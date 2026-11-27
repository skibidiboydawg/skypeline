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
    #[serde(default)]
    pub avatar_hash: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FriendshipStatus {
    Pending,
    Accepted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendRequest {
    pub query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendRespondRequest {
    pub requester_id: i64,
    pub accept: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FriendInfo {
    pub user_id: i64,
    pub username: String,
    pub status: FriendshipStatus,
    /// `"incoming"`, `"outgoing"`, or `"mutual"`.
    pub direction: String,
    pub online: bool,
    pub presence: PresenceStatus,
    #[serde(default)]
    pub avatar_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvatarUpload {
    pub mime: String,
    pub data_base64: String,
    pub hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvatarInfo {
    pub user_id: i64,
    pub hash: String,
    pub mime: String,
    pub data_base64: String,
}

/// Sign-up intentionally does not sign the user in. Neon Auth sends an email
/// verification code that must be submitted through `/api/verify-code` first.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub email: String,
    pub verification_required: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyCodeRequest {
    pub email: String,
    pub code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResendCodeRequest {
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelMemberInfo {
    pub user_id: i64,
    pub username: String,
    #[serde(default)]
    pub avatar_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: i64,
    /// Channel name (for DMs: the peer's username, for groups: group name).
    pub name: String,
    /// `"dm"` or `"group"`
    pub kind: String,
    /// For DM channels: the other participant's user id.
    #[serde(default)]
    pub peer_user_id: Option<i64>,
    #[serde(default)]
    pub members: Vec<ChannelMemberInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    #[serde(default)]
    pub member_ids: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    pub user_id: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionDto {
    pub emoji: String,
    pub user_ids: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDto {
    pub id: i64,
    pub channel_id: i64,
    pub user_id: i64,
    pub username: String,
    pub content: String,
    pub created_at: i64,
    #[serde(default)]
    pub edited_at: Option<i64>,
    #[serde(default)]
    pub reply_to_id: Option<i64>,
    #[serde(default)]
    pub reactions: Vec<ReactionDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
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
        #[serde(default)]
        reply_to_id: Option<i64>,
    },
    ToggleReaction {
        channel_id: i64,
        message_id: i64,
        emoji: String,
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
    /// No-op keepalive so short-lived hosts (e.g. Render free) see inbound
    /// WebSocket traffic and do not spin the service down while you're online.
    Ping,
    /// Ephemeral typing indicator for a channel (never persisted).
    Typing {
        channel_id: i64,
    },
}

/// Server -> Client WebSocket messages (no user IPs are ever included).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMsg {
    NewMessage {
        message: MessageDto,
    },
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
    ReactionUpdated {
        channel_id: i64,
        message_id: i64,
        emoji: String,
        user_id: i64,
        added: bool,
    },
    Typing {
        channel_id: i64,
        user_id: i64,
        username: String,
    },
    FriendsUpdated,
    ChannelsUpdated,
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
