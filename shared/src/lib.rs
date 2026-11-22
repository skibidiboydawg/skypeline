use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Away,
    Dnd,
    Offline,
}

impl PresenceStatus {
    pub fn label(self) -> &'static str {
        match self {
            PresenceStatus::Online => "Online",
            PresenceStatus::Away => "Away",
            PresenceStatus::Dnd => "DND",
            PresenceStatus::Offline => "Offline",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: i64,
    /// Channel name (for DMs: the peer's username).
    pub name: String,
    /// `"text"`, `"dm"`, or `"voice"`
    pub kind: String,
    /// For DM channels: the other participant's user id.
    #[serde(default)]
    pub peer_user_id: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDto {
    pub id: i64,
    pub channel_id: i64,
    pub user_id: i64,
    pub username: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnlineUser {
    pub user_id: i64,
    pub username: String,
    pub status: PresenceStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceParticipant {
    pub user_id: i64,
    pub username: String,
    pub muted: bool,
    pub deafened: bool,
    pub speaking: bool,
    pub sharing_screen: bool,
}

/// Client -> Server WebSocket messages (100% server-relayed, never P2P).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMsg {
    SendMessage {
        channel_id: i64,
        content: String,
    },
    SetPresence {
        status: PresenceStatus,
    },
    JoinVoice {
        room: String,
    },
    LeaveVoice,
    SetVoiceState {
        muted: bool,
        deafened: bool,
        speaking: bool,
    },
    /// Base64-encoded 20ms Opus audio packet relayed through the server.
    VoicePacket {
        room: String,
        opus_b64: String,
    },
    SetScreenShare {
        room: String,
        active: bool,
    },
    /// Base64-encoded compressed 720p JPEG screen frame relayed through the server.
    ScreenFrame {
        room: String,
        width: u32,
        height: u32,
        jpeg_b64: String,
    },
}

/// Server -> Client WebSocket messages (no user IPs are ever included).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMsg {
    NewMessage {
        message: MessageDto,
    },
    PresenceSnapshot {
        users: Vec<OnlineUser>,
    },
    VoiceState {
        room: String,
        participants: Vec<VoiceParticipant>,
    },
    VoicePacket {
        room: String,
        from_user_id: i64,
        opus_b64: String,
    },
    ScreenFrame {
        room: String,
        from_user_id: i64,
        from_username: String,
        width: u32,
        height: u32,
        jpeg_b64: String,
    },
    ScreenShareStopped {
        room: String,
        from_user_id: i64,
    },
    Error {
        message: String,
    },
}
