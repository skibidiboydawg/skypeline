//! Messages from background async tasks to the egui event loop.

use skypeline_shared::{
    AuthResponse, ChannelInfo, FriendInfo, MessageDto, OnlineUser, RegisterResponse,
    VoiceParticipant,
};

#[derive(Debug)]
pub enum UiEvent {
    SignedIn(AuthResponse),
    /// Sign-up succeeded but the emailed code still needs to be confirmed.
    RegisterPending(RegisterResponse),
    Channels(Vec<ChannelInfo>),
    /// A DM/group channel was opened (created or reused) and should be selected.
    DmOpened {
        channel: ChannelInfo,
        start_call: bool,
    },
    #[allow(dead_code)]
    GroupCreated(ChannelInfo),
    Messages {
        channel_id: i64,
        messages: Vec<MessageDto>,
    },
    OlderMessages {
        channel_id: i64,
        messages: Vec<MessageDto>,
    },
    NewMessage(MessageDto),
    MessageEdited {
        channel_id: i64,
        message_id: i64,
        content: String,
        edited_at: i64,
    },
    MessageDeleted {
        channel_id: i64,
        message_id: i64,
    },
    Typing {
        channel_id: i64,
        user_id: i64,
        username: String,
    },
    Presence(Vec<OnlineUser>),
    /// Accepted friends, incoming requests and outgoing requests.
    Friends(Vec<FriendInfo>),
    /// Refresh the friends list after a request/response mutation.
    ReloadFriends,
    /// Refresh the channels list after a group/friendship mutation.
    ReloadChannels,
    VoiceState {
        room: String,
        participants: Vec<VoiceParticipant>,
    },
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
    /// An avatar downloaded from the relay and ready for disk/memory caching.
    AvatarLoaded {
        user_id: i64,
        hash: String,
        mime: String,
        data_base64: String,
    },
    Status(String),
    Error(String),
    Disconnected,
    /// A WebSocket connection was re-established after a drop.
    Reconnected,
}
