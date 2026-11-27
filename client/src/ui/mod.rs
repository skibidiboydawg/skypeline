//! egui application root: state, persistent session restore, event drain, and 3-pane layout.

mod auth;
mod call;
mod chat;
mod friends;
mod icons;
mod sidebar;

use crate::api::ApiClient;
use crate::audio::AudioEngine;
use crate::avatar::AvatarCache;
use crate::config::{Config, SavedSession};
use crate::events::UiEvent;
use crate::net;
use crate::screen::ScreenShare;
use crate::{native, theme};
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, RichText, Sense, TextureOptions, Ui};
use skypeline_shared::{
    AuthResponse, ChannelInfo, FriendInfo, FriendRequest, FriendRespondRequest, MessageDto,
    OnlineUser, PresenceStatus, VoiceParticipant, WsClientMsg,
};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use tokio::sync::mpsc as tmpsc;
use tokio::task::JoinHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

pub const MESSAGE_PAGE_SIZE: i64 = 50;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Login,
    Main,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthTab {
    Login,
    Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthStep {
    Credentials,
    VerifyCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FriendsTab {
    Online,
    All,
    Pending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RailSection {
    Friends,
    Calls,
    Groups,
}

pub struct SkyApp {
    pub(crate) rt: tokio::runtime::Runtime,
    pub(crate) config: Config,
    pub(crate) events_tx: mpsc::Sender<UiEvent>,
    pub(crate) events_rx: mpsc::Receiver<UiEvent>,

    pub(crate) page: Page,
    pub(crate) auth_tab: AuthTab,
    pub(crate) auth_step: AuthStep,
    pub(crate) login_email: String,
    pub(crate) login_password: String,
    pub(crate) register_email: String,
    pub(crate) register_username: String,
    pub(crate) register_password: String,
    pub(crate) verify_email: String,
    pub(crate) verify_code: String,
    pub(crate) busy: bool,
    pub(crate) notice: String,

    pub(crate) token: Option<String>,
    pub(crate) me: Option<AuthResponse>,
    pub(crate) channels: Vec<ChannelInfo>,
    pub(crate) online: Vec<OnlineUser>,
    pub(crate) friends: Vec<FriendInfo>,
    pub(crate) friend_query: String,
    pub(crate) friend_busy: bool,
    pub(crate) avatars: AvatarCache,
    pub(crate) selected_channel: Option<i64>,
    pub(crate) messages: Vec<MessageDto>,
    pub(crate) has_more_messages: bool,
    pub(crate) loading_older: bool,
    pub(crate) editing_message: Option<(i64, String)>,
    pub(crate) draft: String,
    pub(crate) refocus_draft: bool,
    pub(crate) unread: HashMap<i64, usize>,
    pub(crate) typing_users: HashMap<i64, HashMap<i64, (String, f64)>>,
    pub(crate) last_typing_sent: Option<(i64, f64)>,
    pub(crate) presence: PresenceStatus,
    pub(crate) friends_tab: FriendsTab,
    pub(crate) rail_section: RailSection,
    pub(crate) show_profile_popover: bool,
    pub(crate) show_inbox_popover: bool,
    pub(crate) show_settings_modal: bool,
    pub(crate) chat_search: String,

    pub(crate) voice_hint: String,
    pub(crate) voice_room: Option<String>,
    pub(crate) participants: Vec<VoiceParticipant>,
    pub(crate) in_voice: bool,
    pub(crate) muted: bool,
    pub(crate) deafened: bool,
    pub(crate) sharing: bool,
    pub(crate) audio: Option<AudioEngine>,
    pub(crate) screen_share: Option<ScreenShare>,

    pub(crate) ws_tx: Option<tmpsc::UnboundedSender<WsClientMsg>>,
    pub(crate) screen_frame_tx: Option<tmpsc::Sender<WsClientMsg>>,
    pub(crate) ws_task: Option<JoinHandle<()>>,

    pub(crate) screen_texture: Option<egui::TextureHandle>,
    pub(crate) screen_owner: Option<i64>,
    pub(crate) screen_size: (u32, u32),
    pub(crate) screen_fullscreen: bool,

    pub(crate) voice_participants_seen: HashSet<i64>,
    pub(crate) skip_join_chimes: bool,
    pub(crate) call_started_at: Option<f64>,

    pub(crate) _tray: Option<TrayIcon>,
    pub(crate) tray_show_id: Option<tray_icon::menu::MenuId>,
    pub(crate) tray_quit_id: Option<tray_icon::menu::MenuId>,
    pub(crate) allow_close: bool,
    pub(crate) window_visible: bool,
}

impl SkyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        theme::apply(&cc.egui_ctx, config.dark_mode);
        let (events_tx, events_rx) = mpsc::channel();
        let (tray, tray_show_id, tray_quit_id) = Self::init_tray();
        let mut app = Self {
            rt: tokio::runtime::Runtime::new().expect("create tokio runtime"),
            config,
            events_tx,
            events_rx,
            page: Page::Login,
            auth_tab: AuthTab::Login,
            auth_step: AuthStep::Credentials,
            login_email: String::new(),
            login_password: String::new(),
            register_email: String::new(),
            register_username: String::new(),
            register_password: String::new(),
            verify_email: String::new(),
            verify_code: String::new(),
            busy: false,
            notice: String::new(),
            token: None,
            me: None,
            channels: Vec::new(),
            online: Vec::new(),
            friends: Vec::new(),
            friend_query: String::new(),
            friend_busy: false,
            avatars: AvatarCache::new(),
            selected_channel: None,
            messages: Vec::new(),
            has_more_messages: false,
            loading_older: false,
            editing_message: None,
            draft: String::new(),
            refocus_draft: false,
            unread: HashMap::new(),
            typing_users: HashMap::new(),
            last_typing_sent: None,
            presence: PresenceStatus::Online,
            friends_tab: FriendsTab::Online,
            rail_section: RailSection::Friends,
            show_profile_popover: false,
            show_inbox_popover: false,
            show_settings_modal: false,
            chat_search: String::new(),
            voice_hint: String::new(),
            voice_room: None,
            participants: Vec::new(),
            in_voice: false,
            muted: false,
            deafened: false,
            sharing: false,
            audio: None,
            screen_share: None,
            ws_tx: None,
            screen_frame_tx: None,
            ws_task: None,
            screen_texture: None,
            screen_owner: None,
            screen_size: (0, 0),
            screen_fullscreen: false,
            voice_participants_seen: HashSet::new(),
            skip_join_chimes: false,
            call_started_at: None,
            _tray: tray,
            tray_show_id,
            tray_quit_id,
            allow_close: false,
            window_visible: true,
        };
        app.restore_session();
        app
    }

    #[cfg(target_os = "windows")]
    fn init_tray() -> (
        Option<TrayIcon>,
        Option<tray_icon::menu::MenuId>,
        Option<tray_icon::menu::MenuId>,
    ) {
        let icon = tray_icon::Icon::from_rgba(Self::tray_icon_rgba(), 32, 32).ok();
        let show = MenuItem::new("Show Wireline", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append(&show);
        let _ = menu.append(&quit);
        let show_id = show.id().clone();
        let quit_id = quit.id().clone();
        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Wireline");
        if let Some(icon) = icon {
            builder = builder.with_icon(icon);
        }
        match builder.build() {
            Ok(t) => (Some(t), Some(show_id), Some(quit_id)),
            Err(_) => (None, Some(show_id), Some(quit_id)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn init_tray() -> (
        Option<TrayIcon>,
        Option<tray_icon::menu::MenuId>,
        Option<tray_icon::menu::MenuId>,
    ) {
        // On Linux, winit/eframe does not initialize GTK; calling tray-icon's
        // GTK menu API without gtk::init() panics on startup.
        (None, None, None)
    }

    #[cfg(target_os = "windows")]
    fn tray_icon_rgba() -> Vec<u8> {
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let i = (y * 32 + x) * 4;
                let b = x < 2 || x > 29 || y < 2 || y > 29;
                rgba[i..i + 4].copy_from_slice(if b {
                    &[0x18, 0x18, 0x1B, 0xFF]
                } else {
                    &[0x63, 0x66, 0xF1, 0xFF]
                });
            }
        }
        // Simple geometric 'W'
        for y in 10..22 {
            for x in [8, 9, 22, 23] {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[0xFA, 0xFA, 0xFA, 0xFF]);
            }
        }
        for y in 14..22 {
            for x in [15, 16] {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[0xFA, 0xFA, 0xFA, 0xFF]);
            }
        }
        for x in 10..15 {
            let i = (20 * 32 + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[0xFA, 0xFA, 0xFA, 0xFF]);
        }
        for x in 17..22 {
            let i = (20 * 32 + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[0xFA, 0xFA, 0xFA, 0xFF]);
        }
        rgba
    }

    pub(crate) fn now_secs() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    pub(crate) fn channel_room(channel_id: i64) -> String {
        format!("channel:{channel_id}")
    }

    pub(crate) fn api(&self) -> ApiClient {
        ApiClient::new(self.config.base_url())
    }

    fn save_session(&mut self) {
        self.config.session = self.me.as_ref().and_then(|me| {
            self.token.as_ref().map(|t| SavedSession {
                token: t.clone(),
                user_id: me.user_id,
                username: me.username.clone(),
                email: me.email.clone(),
                avatar_hash: me.avatar_hash.clone(),
            })
        });
        self.config.save();
    }

    /// Immediately restores a saved session into the main workspace so the user
    /// is never forced back to the login screen while the relay wakes up.
    /// Only an explicit HTTP 401 invalidates the saved token.
    fn restore_session(&mut self) {
        let Some(session) = self.config.session.clone() else {
            return;
        };
        if !session.is_valid() {
            self.config.session = None;
            self.config.save();
            return;
        }
        let auth = AuthResponse {
            token: session.token.clone(),
            user_id: session.user_id,
            username: session.username,
            email: session.email,
            avatar_hash: session.avatar_hash,
        };
        self.me = Some(auth);
        self.token = Some(session.token.clone());
        self.page = Page::Main;
        self.notice.clear();
        self.connect_ws(&session.token);
        self.load_channels();
        self.load_friends();
    }

    fn connect_ws(&mut self, token: &str) {
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        let (tx, rx) = tmpsc::unbounded_channel();
        let (screen_tx, screen_rx) = tmpsc::channel::<WsClientMsg>(4);
        self.ws_tx = Some(tx);
        self.screen_frame_tx = Some(screen_tx.clone());
        let base = self.config.base_url();
        let token = token.to_string();
        let events = self.events_tx.clone();
        self.ws_task = Some(self.rt.spawn(net::run(base, token, events, rx, screen_rx)));
    }

    pub(crate) fn load_channels(&self) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            match api.channels(&token).await {
                Ok(channels) => {
                    let _ = tx.send(UiEvent::Channels(channels));
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn load_friends(&mut self) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.friend_busy = true;
        self.rt.spawn(async move {
            match api.friends(&token).await {
                Ok(friends) => {
                    let _ = tx.send(UiEvent::Friends(friends));
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn send_friend_request(&mut self) {
        let query = self.friend_query.trim().to_string();
        if query.is_empty() {
            self.notice = "Enter a username or email to add a friend".into();
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.friend_busy = true;
        self.notice = format!("Looking for {query}...");
        self.rt.spawn(async move {
            match api.friend_request(&token, &FriendRequest { query }).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::Status(response.message));
                    let _ = tx.send(UiEvent::ReloadFriends);
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn respond_friend(&mut self, requester_id: i64, accept: bool) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.friend_busy = true;
        self.rt.spawn(async move {
            match api
                .friend_respond(
                    &token,
                    &FriendRespondRequest {
                        requester_id,
                        accept,
                    },
                )
                .await
            {
                Ok(response) => {
                    let _ = tx.send(UiEvent::Status(response.message));
                    let _ = tx.send(UiEvent::ReloadFriends);
                    if accept {
                        let _ = tx.send(UiEvent::ReloadChannels);
                    }
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn load_messages(&mut self, channel_id: i64, before_id: Option<i64>) {
        let Some(token) = self.token.clone() else {
            return;
        };
        if before_id.is_some() {
            self.loading_older = true;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            match api
                .messages(&token, channel_id, before_id, MESSAGE_PAGE_SIZE)
                .await
            {
                Ok(messages) => {
                    let event = if before_id.is_some() {
                        UiEvent::OlderMessages {
                            channel_id,
                            messages,
                        }
                    } else {
                        UiEvent::Messages {
                            channel_id,
                            messages,
                        }
                    };
                    let _ = tx.send(event);
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn open_dm(&self, user_id: i64, start_call: bool) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            match api.open_dm(&token, user_id).await {
                Ok(channel) => {
                    let _ = tx.send(UiEvent::DmOpened {
                        channel,
                        start_call,
                    });
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    pub(crate) fn upsert_channel(&mut self, channel: ChannelInfo) {
        if let Some(existing) = self.channels.iter_mut().find(|c| c.id == channel.id) {
            *existing = channel;
        } else {
            self.channels.push(channel);
        }
    }

    pub(crate) fn select_channel(&mut self, channel_id: i64) {
        if self.selected_channel == Some(channel_id) {
            return;
        }
        self.selected_channel = Some(channel_id);
        self.unread.remove(&channel_id);
        self.messages.clear();
        self.has_more_messages = false;
        self.loading_older = false;
        self.editing_message = None;
        self.load_messages(channel_id, None);
    }

    pub(crate) fn send_message(&mut self) {
        let content = self.draft.trim().to_string();
        let Some(channel_id) = self.selected_channel else {
            return;
        };
        if content.is_empty() {
            return;
        }
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsClientMsg::SendMessage {
                channel_id,
                content,
            });
            self.draft.clear();
            self.refocus_draft = true;
        } else {
            self.notice = "Not connected to the relay".to_string();
        }
    }

    pub(crate) fn notify_typing(&mut self, channel_id: i64) {
        let now = Self::now_secs();
        if let Some((prev_ch, prev_at)) = self.last_typing_sent {
            if prev_ch == channel_id && (now - prev_at) < 2.5 {
                return;
            }
        }
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsClientMsg::Typing { channel_id });
            self.last_typing_sent = Some((channel_id, now));
        }
    }

    pub(crate) fn save_message_edit(&mut self, message_id: i64, content: String) {
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.editing_message = None;
        self.rt.spawn(async move {
            if let Err(e) = api.edit_message(&token, message_id, &trimmed).await {
                let _ = tx.send(UiEvent::Error(e));
            }
        });
    }

    pub(crate) fn delete_message(&mut self, message_id: i64) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            if let Err(e) = api.delete_message(&token, message_id).await {
                let _ = tx.send(UiEvent::Error(e));
            }
        });
    }

    pub(crate) fn start_call(&mut self, channel_id: i64) {
        let Some(channel) = self.channels.iter().find(|c| c.id == channel_id).cloned() else {
            return;
        };
        let room = Self::channel_room(channel_id);
        if self.in_voice && self.voice_room.as_deref() == Some(room.as_str()) {
            return;
        }
        if self.in_voice {
            self.leave_voice();
        }
        let Some(ws) = self.ws_tx.clone() else {
            self.notice = "Not connected to the relay".into();
            return;
        };
        let _ = ws.send(WsClientMsg::JoinVoice { room: room.clone() });
        let audio = self.audio.get_or_insert_with(AudioEngine::start);
        audio.set_muted(self.muted);
        audio.set_deafened(self.deafened);
        match audio.start_capture(room.clone(), ws.clone()) {
            Ok(()) => {
                self.in_voice = true;
                self.voice_room = Some(room);
                self.voice_hint.clear();
                self.voice_participants_seen.clear();
                self.skip_join_chimes = true;
                self.call_started_at = Some(Self::now_secs());
                self.notice = format!("Calling {}", channel.name);
            }
            Err(e) => {
                self.notice = e;
                let _ = ws.send(WsClientMsg::LeaveVoice);
            }
        }
    }

    pub(crate) fn leave_voice(&mut self) {
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsClientMsg::LeaveVoice);
        }
        if let Some(audio) = &mut self.audio {
            audio.stop_capture();
        }
        if let Some(share) = self.screen_share.take() {
            share.stop();
        }
        self.sharing = false;
        self.in_voice = false;
        self.participants.clear();
        self.screen_texture = None;
        self.screen_owner = None;
        self.screen_fullscreen = false;
        self.voice_hint.clear();
        self.voice_room = None;
        self.voice_participants_seen.clear();
        self.skip_join_chimes = false;
        self.call_started_at = None;
        self.notice = "Left voice".into();
    }

    pub(crate) fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(audio) = &self.audio {
            audio.set_muted(self.muted);
        }
        self.send_voice_state();
    }

    pub(crate) fn toggle_deafen(&mut self) {
        self.deafened = !self.deafened;
        if let Some(audio) = &self.audio {
            audio.set_deafened(self.deafened);
        }
        self.send_voice_state();
    }

    pub(crate) fn send_voice_state(&self) {
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsClientMsg::SetVoiceState {
                muted: self.muted,
                deafened: self.deafened,
                speaking: false,
            });
        }
    }

    pub(crate) fn toggle_screen_share(&mut self) {
        if !self.in_voice {
            self.voice_hint = "Join a call before sharing your screen".into();
            return;
        }
        if let Some(share) = self.screen_share.take() {
            share.stop();
            self.sharing = false;
            self.voice_hint = "Screen sharing stopped".into();
            return;
        }
        let room = self.voice_room.clone().unwrap_or_default();
        if room.is_empty() {
            self.voice_hint = "Join a call before sharing your screen".into();
            return;
        }
        let Some(ws) = self.ws_tx.clone() else {
            self.notice = "Not connected to the relay".into();
            return;
        };
        let Some(frames) = self.screen_frame_tx.clone() else {
            self.notice = "Not connected to the relay".into();
            return;
        };
        let events = self.events_tx.clone();
        self.screen_share = Some(ScreenShare::start(room, ws, frames, events));
        self.sharing = true;
        self.voice_hint = "You are sharing your screen through the relay".into();
    }

    pub(crate) fn set_presence(&mut self, status: PresenceStatus) {
        self.presence = status;
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsClientMsg::SetPresence { status });
        }
    }

    pub(crate) fn sign_out(&mut self) {
        self.leave_voice();
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        self.ws_tx = None;
        self.token = None;
        self.me = None;
        self.config.session = None;
        self.config.save();
        self.page = Page::Login;
        self.auth_step = AuthStep::Credentials;
        self.verify_email.clear();
        self.verify_code.clear();
        self.channels.clear();
        self.messages.clear();
        self.has_more_messages = false;
        self.loading_older = false;
        self.editing_message = None;
        self.online.clear();
        self.friends.clear();
        self.friend_query.clear();
        self.friend_busy = false;
        self.friends_tab = FriendsTab::All;
        self.avatars.clear();
        self.unread.clear();
        self.typing_users.clear();
        self.selected_channel = None;
        self.screen_fullscreen = false;
        self.voice_participants_seen.clear();
        self.skip_join_chimes = false;
        self.notice = "Signed out".into();
    }

    pub(crate) fn current_channel(&self) -> Option<&ChannelInfo> {
        self.selected_channel
            .and_then(|id| self.channels.iter().find(|c| c.id == id))
    }

    pub(crate) fn online_status_for(&self, user_id: i64) -> PresenceStatus {
        self.online
            .iter()
            .find(|u| u.user_id == user_id)
            .map(|u| u.status)
            .unwrap_or(PresenceStatus::Offline)
    }

    fn drain_events(&mut self, ctx: &Context) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                UiEvent::SignedIn(response) => {
                    self.busy = false;
                    self.me = Some(response.clone());
                    self.token = Some(response.token.clone());
                    self.page = Page::Main;
                    self.notice.clear();
                    self.selected_channel = None;
                    self.messages.clear();
                    self.save_session();
                    self.connect_ws(&response.token);
                    self.load_channels();
                    self.load_friends();
                }
                UiEvent::RegisterPending(response) => {
                    self.busy = false;
                    self.verify_email = response.email;
                    self.verify_code.clear();
                    self.auth_step = AuthStep::VerifyCode;
                    self.notice = response.message;
                }
                UiEvent::Channels(channels) => {
                    self.channels = channels;
                    if let Some(sel) = self.selected_channel {
                        if !self.channels.iter().any(|c| c.id == sel) {
                            self.selected_channel = None;
                            self.messages.clear();
                        }
                    }
                }
                UiEvent::DmOpened {
                    channel,
                    start_call,
                } => {
                    self.upsert_channel(channel.clone());
                    self.select_channel(channel.id);
                    if start_call {
                        self.start_call(channel.id);
                    }
                }
                UiEvent::GroupCreated(_) => {}
                UiEvent::Messages {
                    channel_id,
                    messages,
                } => {
                    if self.selected_channel == Some(channel_id) {
                        self.has_more_messages = messages.len() as i64 >= MESSAGE_PAGE_SIZE;
                        self.messages = messages;
                    }
                }
                UiEvent::OlderMessages {
                    channel_id,
                    mut messages,
                } => {
                    self.loading_older = false;
                    if self.selected_channel == Some(channel_id) {
                        self.has_more_messages = messages.len() as i64 >= MESSAGE_PAGE_SIZE;
                        let existing: HashSet<i64> = self.messages.iter().map(|m| m.id).collect();
                        messages.retain(|m| !existing.contains(&m.id));
                        messages.extend(self.messages.drain(..));
                        self.messages = messages;
                    }
                }
                UiEvent::NewMessage(message) => {
                    if let Some(ch_typing) = self.typing_users.get_mut(&message.channel_id) {
                        ch_typing.remove(&message.user_id);
                    }
                    let selected = self.selected_channel == Some(message.channel_id);
                    let mine = message.user_id == self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                    if selected {
                        self.messages.push(message);
                    } else if !mine {
                        *self.unread.entry(message.channel_id).or_insert(0) += 1;
                        native::message_chime();
                        let channel_name = self
                            .channels
                            .iter()
                            .find(|c| c.id == message.channel_id)
                            .map(|c| c.name.as_str())
                            .unwrap_or("Wireline");
                        let body = format!(
                            "{} in {}: {}",
                            message.username, channel_name, message.content
                        );
                        native::notify("Wireline", &body);
                        if !self.window_visible {
                            self.notice = body;
                        }
                    }
                }
                UiEvent::MessageEdited {
                    channel_id,
                    message_id,
                    content,
                    edited_at,
                } => {
                    if self.selected_channel == Some(channel_id) {
                        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                            msg.content = content;
                            msg.edited_at = Some(edited_at);
                        }
                    }
                }
                UiEvent::MessageDeleted {
                    channel_id,
                    message_id,
                } => {
                    if self.selected_channel == Some(channel_id) {
                        self.messages.retain(|m| m.id != message_id);
                    }
                }
                UiEvent::Typing {
                    channel_id,
                    user_id,
                    username,
                } => {
                    let expires = Self::now_secs() + 4.0;
                    self.typing_users
                        .entry(channel_id)
                        .or_default()
                        .insert(user_id, (username, expires));
                }
                UiEvent::Presence(users) => {
                    let states: HashMap<i64, PresenceStatus> =
                        users.iter().map(|u| (u.user_id, u.status)).collect();
                    for friend in &mut self.friends {
                        if let Some(status) = states.get(&friend.user_id) {
                            friend.online = true;
                            friend.presence = *status;
                        } else if friend.direction == "mutual" {
                            friend.online = false;
                            friend.presence = PresenceStatus::Offline;
                        }
                    }
                    self.online = users;
                }
                UiEvent::Friends(friends) => {
                    self.friend_busy = false;
                    self.busy = false;
                    self.friends = friends;
                }
                UiEvent::ReloadFriends => self.load_friends(),
                UiEvent::ReloadChannels => self.load_channels(),
                UiEvent::AvatarLoaded {
                    user_id,
                    hash,
                    mime,
                    data_base64,
                } => {
                    let _ = self
                        .avatars
                        .on_loaded(ctx, user_id, hash, mime, data_base64);
                }
                UiEvent::VoiceState(participants) => {
                    let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                    let new_joiner = participants.iter().any(|p| {
                        p.user_id != me_id && !self.voice_participants_seen.contains(&p.user_id)
                    });
                    if new_joiner && !self.skip_join_chimes {
                        native::voice_join_chime();
                    }
                    self.voice_participants_seen = participants
                        .iter()
                        .map(|p| p.user_id)
                        .collect::<HashSet<_>>();
                    self.skip_join_chimes = false;
                    self.participants = participants;
                    if let Some(owner) = self.screen_owner {
                        if !self
                            .participants
                            .iter()
                            .any(|p| p.user_id == owner && p.sharing_screen)
                        {
                            self.screen_texture = None;
                            self.screen_owner = None;
                            self.screen_fullscreen = false;
                        }
                    }
                }
                UiEvent::VoicePcm { pcm, .. } => {
                    let audio = self.audio.get_or_insert_with(AudioEngine::start);
                    audio.push_pcm(&pcm);
                }
                UiEvent::ScreenFrame {
                    from,
                    username,
                    width,
                    height,
                    rgba,
                } => {
                    if rgba.len() != (width as usize) * (height as usize) * 4 {
                        continue;
                    }
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [width as usize, height as usize],
                        &rgba,
                    );
                    match &mut self.screen_texture {
                        Some(texture) => texture.set(image, TextureOptions::LINEAR),
                        None => {
                            self.screen_texture = Some(ctx.load_texture(
                                format!("screen-{from}-{username}"),
                                image,
                                TextureOptions::LINEAR,
                            ));
                        }
                    }
                    self.screen_owner = Some(from);
                    self.screen_size = (width, height);
                }
                UiEvent::ScreenStopped { from } => {
                    if self.screen_owner == Some(from) {
                        self.screen_texture = None;
                        self.screen_owner = None;
                        self.screen_fullscreen = false;
                    }
                }
                UiEvent::Status(status) => {
                    self.busy = false;
                    self.friend_busy = false;
                    if status == "Connected" || status == "Connecting to relay..." {
                        self.notice.clear();
                    } else {
                        self.notice = status;
                    }
                }
                UiEvent::Error(error) => {
                    self.busy = false;
                    self.friend_busy = false;
                    self.loading_older = false;
                    if error.starts_with("HTTP 401") && self.page == Page::Main {
                        self.sign_out();
                        self.notice = "Session expired. Please sign in.".into();
                    } else {
                        self.notice = error;
                    }
                }
                UiEvent::Disconnected => {
                    self.notice = "Reconnecting to relay...".into();
                    self.participants.clear();
                    self.screen_texture = None;
                    self.screen_owner = None;
                    self.screen_fullscreen = false;
                }
                UiEvent::Reconnected => {
                    self.notice.clear();
                    if let Some(ws) = &self.ws_tx {
                        let _ = ws.send(WsClientMsg::SetPresence {
                            status: self.presence,
                        });
                        if self.in_voice {
                            let room = self
                                .voice_room
                                .clone()
                                .unwrap_or_else(|| "channel".to_string());
                            self.skip_join_chimes = true;
                            self.voice_participants_seen.clear();
                            let _ = ws.send(WsClientMsg::JoinVoice { room: room.clone() });
                            let _ = ws.send(WsClientMsg::SetVoiceState {
                                muted: self.muted,
                                deafened: self.deafened,
                                speaking: false,
                            });
                            if self.sharing {
                                let _ = ws.send(WsClientMsg::SetScreenShare { room, active: true });
                            }
                        }
                        self.load_friends();
                        self.load_channels();
                    }
                }
            }
        }
    }

    fn handle_tray(&mut self, ctx: &Context) {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                event,
                TrayIconEvent::Click { .. } | TrayIconEvent::DoubleClick { .. }
            ) {
                self.window_visible = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if Some(event.id.clone()) == self.tray_quit_id {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if Some(event.id.clone()) == self.tray_show_id {
                self.window_visible = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        if self._tray.is_some()
            && ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.window_visible = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    fn ui_main(&mut self, ctx: &Context) {
        self.ui_left_sidebar(ctx);
        self.ui_main_stage(ctx);
        self.ui_screen_fullscreen(ctx);
        self.ui_profile_popover(ctx);
        self.ui_inbox_popover(ctx);
        self.ui_settings_modal(ctx);
    }

    fn ui_main_stage(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::SURFACE_MAIN))
            .show(ctx, |ui| {
                if self.selected_channel.is_none() {
                    self.ui_friends_home(ui);
                } else {
                    self.ui_chat_stage(ui);
                }
            });
    }

    pub(crate) fn form_field(
        ui: &mut Ui,
        value: &mut String,
        hint: &str,
        password: bool,
    ) -> egui::Response {
        egui::Frame::new()
            .fill(theme::SURFACE_CARD)
            .stroke(theme::border())
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(
                    egui::TextEdit::singleline(value)
                        .frame(false)
                        .password(password)
                        .hint_text(hint)
                        .desired_width(ui.available_width()),
                )
            })
            .inner
    }

    pub(crate) fn ui_notice(&self, ui: &mut Ui) {
        if self.notice.is_empty() {
            return;
        }
        let color = if self.notice.starts_with("HTTP") || self.notice.contains("failed") {
            theme::DANGER
        } else {
            theme::SECONDARY_TEXT
        };
        ui.label(RichText::new(&self.notice).color(color).small());
    }
}

pub(crate) fn status_color(status: PresenceStatus) -> Color32 {
    match status {
        PresenceStatus::Online => theme::SUCCESS,
        PresenceStatus::Away => theme::WARNING,
        PresenceStatus::Dnd => theme::DANGER,
        PresenceStatus::Offline => theme::MUTED_TEXT,
    }
}

pub(crate) fn avatar_color(name: &str, mine: bool) -> Color32 {
    if mine {
        return theme::ACCENT;
    }
    let mut h: u32 = 2166136261;
    for b in name.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    let palette = [
        Color32::from_rgb(0x63, 0x66, 0xF1),
        Color32::from_rgb(0x10, 0xB9, 0x81),
        Color32::from_rgb(0xF5, 0x9E, 0x0B),
        Color32::from_rgb(0xEC, 0x48, 0x99),
        Color32::from_rgb(0x06, 0xB6, 0xD4),
    ];
    palette[(h as usize) % palette.len()]
}

pub(crate) fn ui_avatar(ui: &mut Ui, name: &str, mine: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 15.0, avatar_color(name, mine));
    let initial = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_ascii_uppercase()
        .to_string();
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(13.0),
        Color32::WHITE,
    );
}

pub(crate) fn format_hhmm(unix_secs: i64) -> String {
    let secs_in_day = unix_secs.rem_euclid(86_400);
    let hours = secs_in_day / 3_600;
    let mins = (secs_in_day % 3_600) / 60;
    format!("{hours:02}:{mins:02}")
}

impl eframe::App for SkyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray(ctx);
        self.drain_events(ctx);
        match self.page {
            Page::Login => self.ui_login(ctx),
            Page::Main => self.ui_main(ctx),
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_hhmm_formats_expected_clock() {
        assert_eq!(format_hhmm(0), "00:00");
        assert_eq!(format_hhmm(13 * 3600 + 5 * 60), "13:05");
        assert_eq!(format_hhmm(86_399), "23:59");
    }

    #[test]
    fn avatar_color_is_deterministic() {
        assert_eq!(avatar_color("alice", false), avatar_color("alice", false));
        assert_eq!(avatar_color("alice", true), theme::ACCENT);
    }
}
