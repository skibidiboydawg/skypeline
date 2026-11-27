//! WebSocket client for the server-relayed hub.
//!
//! Every chat message, Opus packet and screen frame travels through the server.
//! No peer addresses are ever discovered or used by this client.
//!
//! Everything coming off the wire is untrusted: audio packets and screen
//! frames are size-checked and decoded defensively, and a bad packet is
//! skipped instead of panicking the relay task.

use crate::events::UiEvent;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use skypeline_shared::{WsClientMsg, WsServerMsg};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tmpsc;
use tokio_tungstenite::tungstenite::Message;

const RECONNECT_BASE_MS: u64 = 1_000;
const RECONNECT_MAX_MS: u64 = 15_000;
const KEEPALIVE_SECS: u64 = 45;

// --- untrusted payload limits ------------------------------------------------
const OPUS_FRAME: usize = 960;
const MAX_VOICE_BYTES: usize = 8 * 1024;
const MAX_VOICE_B64_LEN: usize = (MAX_VOICE_BYTES / 3 + 1) * 4 + 8;
const MAX_DECODERS: usize = 32;
const MAX_SCREEN_BYTES: usize = 12 * 1024 * 1024;
const MAX_SCREEN_B64_LEN: usize = (MAX_SCREEN_BYTES / 3 + 1) * 4 + 8;
const MAX_SCREEN_DIM: u32 = 8192;
const MAX_SCREEN_PIXELS: u64 = 16_000_000;

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
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
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
    mut screen_frames: tmpsc::Receiver<WsClientMsg>,
) {
    let url = http_to_ws(&base, &token);
    let mut attempt: u32 = 0;
    let mut screen_open = true;

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
                        screen_msg = screen_frames.recv(), if screen_open => {
                            let Some(msg) = screen_msg else {
                                screen_open = false;
                                continue;
                            };
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
        WsServerMsg::MessageEdited {
            channel_id,
            message_id,
            content,
            edited_at,
        } => {
            let _ = events.send(UiEvent::MessageEdited {
                channel_id,
                message_id,
                content,
                edited_at,
            });
        }
        WsServerMsg::MessageDeleted {
            channel_id,
            message_id,
        } => {
            let _ = events.send(UiEvent::MessageDeleted {
                channel_id,
                message_id,
            });
        }
        WsServerMsg::Typing {
            channel_id,
            user_id,
            username,
        } => {
            let _ = events.send(UiEvent::Typing {
                channel_id,
                user_id,
                username,
            });
        }
        WsServerMsg::FriendsUpdated => {
            let _ = events.send(UiEvent::ReloadFriends);
        }
        WsServerMsg::ChannelsUpdated => {
            let _ = events.send(UiEvent::ReloadChannels);
        }
        WsServerMsg::PresenceSnapshot { users } => {
            let _ = events.send(UiEvent::Presence(users));
        }
        WsServerMsg::VoiceState { participants, .. } => {
            let _ = events.send(UiEvent::VoiceState(participants));
        }
        WsServerMsg::VoicePacket {
            from_user_id,
            opus_b64,
            ..
        } => {
            handle_voice_packet(from_user_id, &opus_b64, events, decoders);
        }
        WsServerMsg::ScreenFrame {
            from_user_id,
            from_username,
            width,
            height,
            jpeg_b64,
            ..
        } => {
            handle_screen_frame(
                from_user_id,
                from_username,
                width,
                height,
                &jpeg_b64,
                events,
            );
        }
        WsServerMsg::ScreenShareStopped { from_user_id, .. } => {
            let _ = events.send(UiEvent::ScreenStopped { from: from_user_id });
        }
        WsServerMsg::Error { message } => {
            let _ = events.send(UiEvent::Error(message));
        }
    }
}

fn handle_voice_packet(
    from_user_id: i64,
    opus_b64: &str,
    events: &mpsc::Sender<UiEvent>,
    decoders: &mut HashMap<i64, opus1::OpusDecoder>,
) {
    // Reject absurd/partial payloads before allocating the decoded buffer.
    if opus_b64.is_empty() || opus_b64.len() > MAX_VOICE_B64_LEN {
        tracing::warn!("skipping oversized voice packet from {from_user_id}");
        return;
    }
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(opus_b64) else {
        tracing::warn!("skipping malformed base64 voice packet from {from_user_id}");
        return;
    };
    if bytes.is_empty() || bytes.len() > MAX_VOICE_BYTES {
        tracing::warn!("skipping oversized voice packet from {from_user_id}");
        return;
    }

    // Cap the decoder map so a spoofed user id cannot grow it without limit.
    if decoders.len() >= MAX_DECODERS && !decoders.contains_key(&from_user_id) {
        if let Some(victim) = decoders.keys().next().copied() {
            decoders.remove(&victim);
        }
    }
    let decoder = match decoders.entry(from_user_id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            match opus1::OpusDecoder::new(opus1::SampleRate::Hz48000, opus1::Channels::Mono) {
                Ok(decoder) => entry.insert(decoder),
                Err(e) => {
                    tracing::warn!("cannot create Opus decoder for {from_user_id}: {e}");
                    return;
                }
            }
        }
    };

    let mut pcm = [0.0f32; OPUS_FRAME];
    // Malformed Opus bitstreams may trip an internal panic: contain it.
    let decoded = catch_unwind(AssertUnwindSafe(|| {
        decoder.decode(&bytes, OPUS_FRAME, &mut pcm)
    }));
    match decoded {
        Ok(Ok(len)) if len == OPUS_FRAME => {}
        Ok(Ok(len)) => {
            tracing::warn!("short Opus decode from {from_user_id}: {len} samples");
            return;
        }
        Ok(Err(e)) => {
            tracing::warn!("Opus decode error from {from_user_id}: {e}");
            return;
        }
        Err(_) => {
            tracing::warn!("Opus decoder panicked on packet from {from_user_id}");
            decoders.remove(&from_user_id);
            return;
        }
    }

    // A hostile packet must not inject NaN/Inf into the playback queue.
    for sample in &mut pcm {
        if !sample.is_finite() {
            *sample = 0.0;
        }
    }
    let _ = events.send(UiEvent::VoicePcm {
        _from: from_user_id,
        pcm: pcm.to_vec(),
    });
}

fn handle_screen_frame(
    from_user_id: i64,
    from_username: String,
    width: u32,
    height: u32,
    jpeg_b64: &str,
    events: &mpsc::Sender<UiEvent>,
) {
    if width == 0
        || height == 0
        || width > MAX_SCREEN_DIM
        || height > MAX_SCREEN_DIM
        || u64::from(width) * u64::from(height) > MAX_SCREEN_PIXELS
    {
        tracing::warn!("skipping screen frame with absurd dimensions from {from_user_id}");
        return;
    }
    let expected_rgba = match (u64::from(width) * u64::from(height))
        .checked_mul(4)
        .and_then(|len| usize::try_from(len).ok())
    {
        Some(len) if len <= MAX_SCREEN_BYTES => len,
        _ => return,
    };
    if jpeg_b64.is_empty() || jpeg_b64.len() > MAX_SCREEN_B64_LEN {
        tracing::warn!("skipping oversized screen frame from {from_user_id}");
        return;
    }
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(jpeg_b64) else {
        tracing::warn!("skipping malformed base64 screen frame from {from_user_id}");
        return;
    };
    if bytes.is_empty() || bytes.len() > MAX_SCREEN_BYTES {
        tracing::warn!("skipping oversized screen frame from {from_user_id}");
        return;
    }

    let loaded = catch_unwind(AssertUnwindSafe(|| image::load_from_memory(&bytes)));
    let img = match loaded {
        Ok(Ok(img)) => img,
        Ok(Err(e)) => {
            tracing::warn!("skipping undecodable screen frame from {from_user_id}: {e}");
            return;
        }
        Err(_) => {
            tracing::warn!("screen frame decoder panicked on packet from {from_user_id}");
            return;
        }
    };
    if img.width() != width || img.height() != height {
        tracing::warn!("skipping screen frame with mismatched dimensions from {from_user_id}");
        return;
    }
    let rgba = img.into_rgba8().into_raw();
    if rgba.len() != expected_rgba {
        tracing::warn!("skipping screen frame with RGBA length mismatch from {from_user_id}");
        return;
    }
    let _ = events.send(UiEvent::ScreenFrame {
        from: from_user_id,
        username: from_username,
        width,
        height,
        rgba,
    });
}
