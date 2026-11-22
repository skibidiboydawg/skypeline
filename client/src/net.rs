//! WebSocket client for the server-relayed hub.
//!
//! Every chat message, Opus packet and screen frame travels through the server.
//! No peer addresses are ever discovered or used by this client.

use crate::events::UiEvent;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use skypeline_shared::{WsClientMsg, WsServerMsg};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tmpsc;
use tokio_tungstenite::tungstenite::Message;

const RECONNECT_BASE_MS: u64 = 1_000;
const RECONNECT_MAX_MS: u64 = 15_000;
const KEEPALIVE_SECS: u64 = 45;

pub fn http_to_ws(base: &str, token: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let (scheme, rest) = if let Some(r) = base.strip_prefix("https://") {
        ("wss", r)
    } else if let Some(r) = base.strip_prefix("http://") {
        ("ws", r)
    } else {
        ("wss", base)
    };
    let encoded = token
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect::<String>();
    format!("{scheme}://{rest}/ws?token={encoded}")
}

/// Runs until the UI drops `outgoing`. Drops and failed connects are retried
/// with exponential backoff — essential for hosts that spin down when idle.
pub async fn run(
    base: String,
    token: String,
    events: mpsc::Sender<UiEvent>,
    mut outgoing: tmpsc::UnboundedReceiver<WsClientMsg>,
) {
    let url = http_to_ws(&base, &token);
    let mut attempt: u32 = 0;

    while !outgoing.is_closed() {
        let status = if attempt == 0 {
            "Connecting to relay...".to_string()
        } else {
            format!("Reconnecting to relay (attempt {})...", attempt + 1)
        };
        let _ = events.send(UiEvent::Status(status));

        match tokio_tungstenite::connect_async(&url).await {
            Ok((mut socket, _)) => {
                let was_reconnect = attempt > 0;
                attempt = 0;
                let _ = events.send(UiEvent::Status("Connected".into()));
                if was_reconnect {
                    let _ = events.send(UiEvent::Reconnected);
                }

                let mut decoders: HashMap<i64, opus1::OpusDecoder> = HashMap::new();
                let mut keepalive = tokio::time::interval(Duration::from_secs(KEEPALIVE_SECS));
                keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                keepalive.tick().await; // consume the immediate first tick

                loop {
                    tokio::select! {
                        _ = keepalive.tick() => {
                            let Ok(json) = serde_json::to_string(&WsClientMsg::Ping) else {
                                continue;
                            };
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        outgoing_msg = outgoing.recv() => {
                            let Some(msg) = outgoing_msg else { return };
                            let Ok(json) = serde_json::to_string(&msg) else { continue };
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        incoming = socket.next() => {
                            let Some(incoming) = incoming else { break };
                            let msg = match incoming {
                                Ok(Message::Text(text)) => text,
                                Ok(Message::Close(_)) => break,
                                Ok(_) => continue,
                                Err(e) => {
                                    let _ = events.send(UiEvent::Error(format!("Relay error: {e}")));
                                    break;
                                }
                            };
                            let Ok(server_msg) = serde_json::from_str::<WsServerMsg>(&msg) else {
                                continue;
                            };
                            handle_server_msg(server_msg, &events, &mut decoders);
                        }
                    }
                }
            }
            Err(e) => {
                let _ = events.send(UiEvent::Error(format!("Relay unavailable: {e} — retrying")));
            }
        }

        let _ = events.send(UiEvent::Disconnected);
        if outgoing.is_closed() {
            break;
        }
        let shift = attempt.min(4);
        let delay_ms = RECONNECT_BASE_MS
            .saturating_mul(1u64 << shift)
            .min(RECONNECT_MAX_MS);
        attempt = attempt.saturating_add(1);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

fn handle_server_msg(
    msg: WsServerMsg,
    events: &mpsc::Sender<UiEvent>,
    decoders: &mut HashMap<i64, opus1::OpusDecoder>,
) {
    match msg {
        WsServerMsg::NewMessage { message } => {
            let _ = events.send(UiEvent::NewMessage(message));
        }
        WsServerMsg::PresenceSnapshot { users } => {
            let _ = events.send(UiEvent::Presence(users));
        }
        WsServerMsg::VoiceState {
            participants, ..
        } => {
            let _ = events.send(UiEvent::VoiceState(participants));
        }
        WsServerMsg::VoicePacket {
            from_user_id,
            opus_b64,
            ..
        } => {
            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(opus_b64) else {
                return;
            };
            let decoder = decoders.entry(from_user_id).or_insert_with(|| {
                opus1::OpusDecoder::new(opus1::SampleRate::Hz48000, opus1::Channels::Mono)
                    .expect("create Opus decoder")
            });
            let mut pcm = [0.0f32; 960];
            if decoder.decode(&bytes, 960, &mut pcm).is_ok() {
                let _ = events.send(UiEvent::VoicePcm {
                    _from: from_user_id,
                    pcm: pcm.to_vec(),
                });
            }
        }
        WsServerMsg::ScreenFrame {
            from_user_id,
            from_username,
            width,
            height,
            jpeg_b64,
            ..
        } => {
            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(jpeg_b64) else {
                return;
            };
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgba = img.to_rgba8();
                let _ = events.send(UiEvent::ScreenFrame {
                    from: from_user_id,
                    username: from_username,
                    width,
                    height,
                    rgba: rgba.into_raw(),
                });
            }
        }
        WsServerMsg::ScreenShareStopped { from_user_id, .. } => {
            let _ = events.send(UiEvent::ScreenStopped { from: from_user_id });
        }
        WsServerMsg::Error { message } => {
            let _ = events.send(UiEvent::Error(message));
        }
    }
}
