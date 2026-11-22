use crate::{auth, db, AppState};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use skypeline_shared::{OnlineUser, PresenceStatus, VoiceParticipant, WsClientMsg, WsServerMsg};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// Max base64 payload sizes accepted for relay (defensive limits).
const MAX_VOICE_B64: usize = 8 * 1024;
const MAX_FRAME_B64: usize = 2 * 1024 * 1024;

/// Who should receive a relayed payload.
pub enum Scope {
    All,
    Room(String),
    Users(Vec<i64>),
}

pub struct Relay {
    pub scope: Scope,
    pub exclude: Option<i64>,
    pub json: String,
}

/// Centralized, strictly NON-P2P relay hub.
/// Every chat message, presence event, Opus voice packet, and screen frame is
/// relayed exclusively through this server; peer IP addresses are never shared.
pub struct RelayHub {
    tx: broadcast::Sender<Arc<Relay>>,
    pub online_users: DashMap<i64, OnlineUser>,
    /// room name -> (user_id -> participant)
    pub voice_rooms: DashMap<String, DashMap<i64, VoiceParticipant>>,
    /// user_id -> voice room they are currently in
    pub user_rooms: DashMap<i64, String>,
}

impl RelayHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(4096);
        let rooms = DashMap::new();
        rooms.insert("General Voice".to_string(), DashMap::new());
        Self {
            tx,
            online_users: DashMap::new(),
            voice_rooms: rooms,
            user_rooms: DashMap::new(),
        }
    }

    pub fn send(&self, scope: Scope, exclude: Option<i64>, msg: &WsServerMsg) {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = self.tx.send(Arc::new(Relay { scope, exclude, json }));
        }
    }

    fn should_deliver(&self, relay: &Relay, user_id: i64) -> bool {
        if relay.exclude == Some(user_id) {
            return false;
        }
        match &relay.scope {
            Scope::All => true,
            Scope::Users(ids) => ids.contains(&user_id),
            Scope::Room(room) => self.user_rooms.get(&user_id).is_some_and(|r| r.as_str() == room),
        }
    }

    fn presence_msg(&self) -> WsServerMsg {
        WsServerMsg::PresenceSnapshot {
            users: self.online_users.iter().map(|e| e.value().clone()).collect(),
        }
    }

    pub fn broadcast_presence(&self) {
        self.send(Scope::All, None, &self.presence_msg());
    }

    fn voice_room_msg(&self, room: &str) -> WsServerMsg {
        let participants = self
            .voice_rooms
            .get(room)
            .map(|m| m.iter().map(|e| e.value().clone()).collect())
            .unwrap_or_default();
        WsServerMsg::VoiceState {
            room: room.to_string(),
            participants,
        }
    }

    pub fn broadcast_voice_room(&self, room: &str) {
        self.send(Scope::All, None, &self.voice_room_msg(room));
    }

    pub fn leave_voice(&self, user_id: i64) {
        let Some((_, room)) = self.user_rooms.remove(&user_id) else {
            return;
        };
        let was_sharing = self
            .voice_rooms
            .get(&room)
            .and_then(|m| m.remove(&user_id))
            .is_some_and(|(_, p)| p.sharing_screen);
        if was_sharing {
            self.send(
                Scope::Room(room.clone()),
                None,
                &WsServerMsg::ScreenShareStopped {
                    room: room.clone(),
                    from_user_id: user_id,
                },
            );
        }
        self.broadcast_voice_room(&room);
    }
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let claims = auth::verify_jwt(&state.jwt_secret, &query.token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid ws token".to_string()))?;
    Ok(ws
        .max_message_size(4 * 1024 * 1024)
        .on_upgrade(move |socket| handle_socket(socket, state, claims)))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, claims: auth::Claims) {
    let user_id = claims.sub;
    let username = claims.username.clone();
    let hub = &state.hub;

    let (mut sender, mut receiver) = socket.split();
    let mut rx = hub.tx.subscribe();

    hub.online_users.insert(
        user_id,
        OnlineUser {
            user_id,
            username: username.clone(),
            status: PresenceStatus::Online,
        },
    );

    // Send an initial snapshot directly to this client, then announce presence.
    let mut initial = vec![hub.presence_msg()];
    for room in hub.voice_rooms.iter().map(|e| e.key().clone()).collect::<Vec<_>>() {
        initial.push(hub.voice_room_msg(&room));
    }
    for msg in initial {
        if let Ok(json) = serde_json::to_string(&msg) {
            if sender.send(Message::Text(json.into())).await.is_err() {
                hub.online_users.remove(&user_id);
                return;
            }
        }
    }
    hub.broadcast_presence();

    let state_for_send = state.clone();
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(relay) => {
                    if !state_for_send.hub.should_deliver(&relay, user_id) {
                        continue;
                    }
                    if sender.send(Message::Text(relay.json.clone().into())).await.is_err() {
                        break;
                    }
                }
                // Slow client: skip dropped frames instead of disconnecting.
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(client_msg) = serde_json::from_str::<WsClientMsg>(&text) else {
            continue;
        };
        handle_client_msg(&state, user_id, &username, client_msg).await;
    }

    send_task.abort();
    hub.leave_voice(user_id);
    hub.online_users.remove(&user_id);
    hub.broadcast_presence();
}

async fn handle_client_msg(state: &Arc<AppState>, user_id: i64, username: &str, msg: WsClientMsg) {
    let hub = &state.hub;
    match msg {
        WsClientMsg::SendMessage { channel_id, content } => {
            let trimmed = content.trim();
            if trimmed.is_empty() || trimmed.len() > 4000 {
                return;
            }
            let channel = match db::get_channel(&state.db, channel_id).await {
                Ok(Some(c)) => c,
                _ => return,
            };
            let scope = match channel.kind.as_str() {
                "text" => Scope::All,
                "dm" => match db::dm_members(&channel.name) {
                    Some((a, b)) if a == user_id || b == user_id => Scope::Users(vec![a, b]),
                    _ => return,
                },
                _ => return,
            };
            match db::insert_message(&state.db, channel.id, user_id, username, trimmed).await {
                Ok(saved) => hub.send(scope, None, &WsServerMsg::NewMessage { message: saved }),
                Err(e) => tracing::warn!("message insert failed: {e}"),
            }
        }
        WsClientMsg::SetPresence { status } => {
            hub.online_users.insert(
                user_id,
                OnlineUser {
                    user_id,
                    username: username.to_string(),
                    status,
                },
            );
            hub.broadcast_presence();
        }
        WsClientMsg::JoinVoice { room } => {
            if !hub.voice_rooms.contains_key(&room) {
                return;
            }
            hub.leave_voice(user_id);
            if let Some(members) = hub.voice_rooms.get(&room) {
                members.insert(
                    user_id,
                    VoiceParticipant {
                        user_id,
                        username: username.to_string(),
                        muted: false,
                        deafened: false,
                        speaking: false,
                        sharing_screen: false,
                    },
                );
            }
            hub.user_rooms.insert(user_id, room.clone());
            hub.broadcast_voice_room(&room);
        }
        WsClientMsg::LeaveVoice => hub.leave_voice(user_id),
        WsClientMsg::SetVoiceState { muted, deafened, speaking } => {
            let Some(room) = hub.user_rooms.get(&user_id).map(|r| r.clone()) else {
                return;
            };
            let mut changed = false;
            if let Some(members) = hub.voice_rooms.get(&room) {
                if let Some(mut p) = members.get_mut(&user_id) {
                    let speaking = speaking && !muted;
                    changed = p.muted != muted || p.deafened != deafened || p.speaking != speaking;
                    p.muted = muted;
                    p.deafened = deafened;
                    p.speaking = speaking;
                }
            }
            if changed {
                hub.broadcast_voice_room(&room);
            }
        }
        WsClientMsg::VoicePacket { opus_b64, .. } => {
            // Always relay into the sender's ACTUAL room (ignore client-supplied room).
            let Some(room) = hub.user_rooms.get(&user_id).map(|r| r.clone()) else {
                return;
            };
            if opus_b64.len() > MAX_VOICE_B64 {
                return;
            }
            hub.send(
                Scope::Room(room.clone()),
                Some(user_id),
                &WsServerMsg::VoicePacket {
                    room,
                    from_user_id: user_id,
                    opus_b64,
                },
            );
        }
        WsClientMsg::SetScreenShare { active, .. } => {
            let Some(room) = hub.user_rooms.get(&user_id).map(|r| r.clone()) else {
                return;
            };
            if let Some(members) = hub.voice_rooms.get(&room) {
                if let Some(mut p) = members.get_mut(&user_id) {
                    p.sharing_screen = active;
                }
            }
            if !active {
                hub.send(
                    Scope::Room(room.clone()),
                    None,
                    &WsServerMsg::ScreenShareStopped {
                        room: room.clone(),
                        from_user_id: user_id,
                    },
                );
            }
            hub.broadcast_voice_room(&room);
        }
        WsClientMsg::ScreenFrame { width, height, jpeg_b64, .. } => {
            let Some(room) = hub.user_rooms.get(&user_id).map(|r| r.clone()) else {
                return;
            };
            if jpeg_b64.len() > MAX_FRAME_B64 || width > 1920 || height > 1080 {
                return;
            }
            hub.send(
                Scope::Room(room.clone()),
                Some(user_id),
                &WsServerMsg::ScreenFrame {
                    room,
                    from_user_id: user_id,
                    from_username: username.to_string(),
                    width,
                    height,
                    jpeg_b64,
                },
            );
        }
        // Keepalive only: intentional no-op so idle hosts stay awake.
        WsClientMsg::Ping => {}
    }
}
