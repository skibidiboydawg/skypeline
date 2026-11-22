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
use tokio::sync::mpsc as tmpsc;
use tokio_tungstenite::tungstenite::Message;

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

/// Runs until the UI drops `outgoing` or the connection fails.
pub async fn run(
    base: String,
    token: String,
    events: mpsc::Sender<UiEvent>,
    mut outgoing: tmpsc::UnboundedReceiver<WsClientMsg>,
) {
    let url = http_to_ws(&base, &token);
    let _ = events.send(UiEvent::Status("Connecting to relay...".into()));

    let (mut socket, _) = match tokio_tungstenite::connect_async(&url).await {
        Ok(pair) => {
            let _ = events.send(UiEvent::Status("Connected".into()));
            pair
        }
        Err(e) => {
            let _ = events.send(UiEvent::Error(format!("WebSocket failed: {e}")));
            let _ = events.send(UiEvent::Disconnected);
            return;
        }
    };

    let mut decoders: HashMap<i64, opus1::OpusDecoder> = HashMap::new();

    loop {
        tokio::select! {
            outgoing_msg = outgoing.recv() => {
                let Some(msg) = outgoing_msg else { break };
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
                let Ok(server_msg) = serde_json::from_str::<WsServerMsg>(&msg) else { continue };
                handle_server_msg(server_msg, &events, &mut decoders);
            }
        }
    }

    let _ = events.send(UiEvent::Disconnected);
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
