//! Messages from background async tasks to the egui event loop.

use skypeline_shared::{AuthResponse, ChannelInfo, MessageDto, OnlineUser, VoiceParticipant};

#[derive(Debug)]
pub enum UiEvent {
    SignedIn(AuthResponse),
    Channels(Vec<ChannelInfo>),
    Messages {
        channel_id: i64,
        messages: Vec<MessageDto>,
    },
    NewMessage(MessageDto),
    Presence(Vec<OnlineUser>),
    VoiceState(Vec<VoiceParticipant>),
    /// Decoded 48 kHz mono PCM from a remote participant, ready to enqueue.
    VoicePcm {
        _from: i64,
        pcm: Vec<f32>,
    },
    /// Decoded RGBA screen frame relayed by another participant.
    ScreenFrame {
        from: i64,
        username: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    ScreenStopped {
        from: i64,
    },
    Status(String),
    Error(String),
    Disconnected,
    /// A WebSocket connection was re-established after a drop.
    Reconnected,
}
