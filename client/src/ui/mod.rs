//! Wireline Discord-style DM desktop app root: state, WebSocket events, shortcuts, and layout.

pub(crate) mod anim;
mod auth;
mod call;
mod chat;
mod friends;
mod icons;
mod overlays;
mod sidebar;

use crate::api::ApiClient;
use crate::audio::AudioEngine;
use crate::avatar::AvatarCache;
use crate::config::{Config, SavedSession};
use crate::events::UiEvent;
use crate::net;
use crate::screen::ScreenShare;
use crate::sounds::{self, SoundEffect};
use crate::{theme, updater};
use eframe::egui;
use egui::{Color32, Context, Pos2, RichText, TextureOptions, Ui};
use skypeline_shared::{
    AuthResponse, ChannelInfo, FriendInfo, FriendRequest, FriendRespondRequest, MessageDto,
    OnlineUser, PresenceStatus, ReactionDto, VoiceParticipant, WsClientMsg,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use tokio::sync::mpsc as tmpsc;
use tokio::task::JoinHandle;
#[cfg(target_os = "windows")]
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
#[cfg(target_os = "windows")]
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
    Blocked,
    AddFriend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    MyAccount,
    Appearance,
    VoiceVideo,
    Keybinds,
}

#[derive(Clone, Debug)]
pub(crate) struct ToastItem {
    pub channel_id: i64,
    pub sender: String,
    pub preview: String,
    pub created_at: f64,
    pub remaining_secs: f32,
    pub closing_at: Option<f64>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProfilePopup {
    pub user_id: i64,
    pub username: String,
    pub status: PresenceStatus,
    pub anchor: Pos2,
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
    pub(crate) connected: bool,

    pub(crate) token: Option<String>,
    pub(crate) me: Option<AuthResponse>,
    pub(crate) channels: Vec<ChannelInfo>,
    pub(crate) last_messages: HashMap<i64, (String, i64)>,
    pub(crate) online: Vec<OnlineUser>,
    pub(crate) friends: Vec<FriendInfo>,
    pub(crate) friends_search: String,
    pub(crate) friend_query: String,
    pub(crate) friend_error: String,
    pub(crate) friend_busy: bool,
    pub(crate) avatars: AvatarCache,
    pub(crate) selected_channel: Option<i64>,
    pub(crate) messages: Vec<MessageDto>,
    pub(crate) has_more_messages: bool,
    pub(crate) loading_older: bool,
    pub(crate) editing_message: Option<(i64, String)>,
    pub(crate) reply_to: Option<(i64, String, String)>,
    pub(crate) revealed_spoilers: HashSet<(i64, usize)>,
    pub(crate) draft: String,
    pub(crate) refocus_draft: bool,
    pub(crate) unread: HashMap<i64, usize>,
    pub(crate) typing_users: HashMap<i64, HashMap<i64, (String, f64)>>,
    pub(crate) last_typing_sent: Option<(i64, f64)>,
    pub(crate) presence: PresenceStatus,
    pub(crate) friends_tab: FriendsTab,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) show_profile_popover: bool,
    pub(crate) show_inbox_popover: bool,
    pub(crate) show_settings_modal: bool,
    pub(crate) profile_popup: Option<ProfilePopup>,
    pub(crate) emoji_picker_target: Option<Option<i64>>,
    pub(crate) emoji_search: String,
    pub(crate) chat_search_open: bool,
    pub(crate) chat_msg_search: String,
    pub(crate) incoming_call: Option<(i64, String)>,
    pub(crate) call_expanded: bool,
    pub(crate) toasts: Vec<ToastItem>,
    pub(crate) chat_search: String,

    // Animation state (all respect config.reduced_motion)
    pub(crate) new_message_anims: HashMap<i64, f64>,
    pub(crate) typing_fade: f32,
    pub(crate) last_typing_text: String,
    pub(crate) removing_friends: HashMap<i64, (f64, bool)>,
    pub(crate) last_pending_count: usize,
    pub(crate) pending_badge_pop_at: Option<f64>,
    pub(crate) settings_opened_at: f64,
    pub(crate) settings_closing_at: Option<f64>,
    pub(crate) profile_popup_opened_at: f64,
    pub(crate) profile_popup_closing_at: Option<f64>,
    pub(crate) animating: bool,

    pub(crate) voice_hint: String,
    pub(crate) voice_room: Option<String>,
    pub(crate) participants: Vec<VoiceParticipant>,
    pub(crate) active_room_calls: HashMap<i64, Vec<VoiceParticipant>>,
    pub(crate) in_voice: bool,
    pub(crate) muted: bool,
    pub(crate) deafened: bool,
    pub(crate) sharing: bool,
    pub(crate) stream_minimized: bool,
    pub(crate) audio: Option<AudioEngine>,
    pub(crate) input_devices: Vec<String>,
    pub(crate) output_devices: Vec<String>,
    pub(crate) devices_loaded: bool,
    pub(crate) screen_share: Option<ScreenShare>,
    pub(crate) screen_fps: Arc<AtomicU32>,
    pub(crate) screen_max_height: Arc<AtomicU32>,

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

    #[cfg(target_os = "windows")]
    pub(crate) _tray: Option<TrayIcon>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_show_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) tray_quit_id: Option<tray_icon::menu::MenuId>,
    #[cfg(target_os = "windows")]
    pub(crate) allow_close: bool,
    pub(crate) window_visible: bool,
}

impl SkyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        theme::setup_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, config.dark_mode, config.font_size);
        let (events_tx, events_rx) = mpsc::channel();
        #[cfg(target_os = "windows")]
        let (tray, tray_show_id, tray_quit_id) = Self::init_tray();
        let init_fps = config.screen_fps;
        let init_max_h = config.screen_max_height;
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
            connected: false,
            token: None,
            me: None,
            channels: Vec::new(),
            last_messages: HashMap::new(),
            online: Vec::new(),
            friends: Vec::new(),
            friends_search: String::new(),
            friend_query: String::new(),
            friend_error: String::new(),
            friend_busy: false,
            avatars: AvatarCache::new(),
            selected_channel: None,
            messages: Vec::new(),
            has_more_messages: false,
            loading_older: false,
            editing_message: None,
            reply_to: None,
            revealed_spoilers: HashSet::new(),
            draft: String::new(),
            refocus_draft: false,
            unread: HashMap::new(),
            typing_users: HashMap::new(),
            last_typing_sent: None,
            presence: PresenceStatus::Online,
            friends_tab: FriendsTab::Online,
            settings_tab: SettingsTab::MyAccount,
            show_profile_popover: false,
            show_inbox_popover: false,
            show_settings_modal: false,
            profile_popup: None,
            emoji_picker_target: None,
            emoji_search: String::new(),
            chat_search_open: false,
            chat_msg_search: String::new(),
            incoming_call: None,
            call_expanded: true,
            toasts: Vec::new(),
            chat_search: String::new(),
            new_message_anims: HashMap::new(),
            typing_fade: 0.0,
            last_typing_text: String::new(),
            removing_friends: HashMap::new(),
            last_pending_count: 0,
            pending_badge_pop_at: None,
            settings_opened_at: 0.0,
            settings_closing_at: None,
            profile_popup_opened_at: 0.0,
            profile_popup_closing_at: None,
            animating: false,
            voice_hint: String::new(),
            voice_room: None,
            participants: Vec::new(),
            active_room_calls: HashMap::new(),
            in_voice: false,
            muted: false,
            deafened: false,
            sharing: false,
            stream_minimized: false,
            audio: None,
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            devices_loaded: false,
            screen_share: None,
            screen_fps: Arc::new(AtomicU32::new(init_fps)),
            screen_max_height: Arc::new(AtomicU32::new(init_max_h)),
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
            #[cfg(target_os = "windows")]
            _tray: tray,
            #[cfg(target_os = "windows")]
            tray_show_id,
            #[cfg(target_os = "windows")]
            tray_quit_id,
            #[cfg(target_os = "windows")]
            allow_close: false,
            window_visible: true,
        };
        app.restore_session();
        app.check_for_updates(false);
        app
    }

    pub(crate) fn check_for_updates(&self, manual: bool) {
        let auto = self.config.auto_update;
        let last = self.config.last_installed_update.clone();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            updater::check_for_update(auto, last, manual, tx).await;
        });
    }

    pub(crate) fn ensure_audio(&mut self) -> &mut AudioEngine {
        let out_dev = self.config.output_device.clone();
        let in_vol = self.config.input_volume;
        let out_vol = self.config.output_volume;
        let vad = self.config.vad_threshold;
        self.audio
            .get_or_insert_with(|| AudioEngine::start(out_dev, in_vol, out_vol, vad))
    }

    pub(crate) fn refresh_audio_devices(&mut self) {
        let (ins, outs) = crate::audio::list_audio_devices();
        self.input_devices = ins;
        self.output_devices = outs;
        self.devices_loaded = true;
    }

    pub(crate) fn set_input_device(&mut self, device: Option<String>) {
        self.config.input_device = device.clone();
        self.config.save();
        if self.in_voice {
            if let (Some(room), Some(ws)) = (self.voice_room.clone(), self.ws_tx.clone()) {
                let _ = self.ensure_audio().start_capture(room, ws, device);
            }
        }
    }

    pub(crate) fn set_output_device(&mut self, device: Option<String>) {
        self.config.output_device = device.clone();
        self.config.save();
        if let Some(audio) = &self.audio {
            audio.set_output_device(device);
        }
    }

    pub(crate) fn set_input_volume(&mut self, vol: f32) {
        let v = vol.clamp(0.0, 2.0);
        self.config.input_volume = v;
        if let Some(audio) = &self.audio {
            audio.set_input_volume(v);
        }
        self.config.save();
    }

    pub(crate) fn set_output_volume(&mut self, vol: f32) {
        let v = vol.clamp(0.0, 2.0);
        self.config.output_volume = v;
        if let Some(audio) = &self.audio {
            audio.set_output_volume(v);
        }
        self.config.save();
    }

    pub(crate) fn set_vad_threshold(&mut self, threshold: f32) {
        let t = threshold.clamp(0.004, 0.060);
        self.config.vad_threshold = t;
        if let Some(audio) = &self.audio {
            audio.set_vad_threshold(t);
        }
        self.config.save();
    }

    pub(crate) fn play_sound(&mut self, effect: SoundEffect) {
        if !self.config.ui_sounds || self.deafened {
            return;
        }
        let clip = sounds::synthesize(effect);
        self.ensure_audio().play_sfx(&clip);
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

    #[cfg(target_os = "windows")]
    fn tray_icon_rgba() -> Vec<u8> {
        let mut rgba = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                let i = (y * 32 + x) * 4;
                let b = x < 2 || x > 29 || y < 2 || y > 29;
                rgba[i..i + 4].copy_from_slice(if b {
                    &[0x1E, 0x1F, 0x22, 0xFF]
                } else {
                    &[0x58, 0x65, 0xF2, 0xFF]
                });
            }
        }
        for y in 10..22 {
            for x in [8, 9, 22, 23] {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[0xF2, 0xF3, 0xF5, 0xFF]);
            }
        }
        for y in 14..22 {
            for x in [15, 16] {
                let i = (y * 32 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[0xF2, 0xF3, 0xF5, 0xFF]);
            }
        }
        for x in 10..15 {
            let i = (20 * 32 + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[0xF2, 0xF3, 0xF5, 0xFF]);
        }
        for x in 17..22 {
            let i = (20 * 32 + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[0xF2, 0xF3, 0xF5, 0xFF]);
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
        let (screen_tx, screen_rx) = tmpsc::channel::<WsClientMsg>(2);
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
            self.friend_error = "Enter a username or email to add a friend.".into();
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.friend_query.clear();
        self.friend_error.clear();
        self.friend_busy = true;
        self.play_sound(SoundEffect::FriendAction);
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
        if accept {
            self.play_sound(SoundEffect::FriendAction);
        } else {
            self.play_sound(SoundEffect::UiClick);
        }
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

    pub(crate) fn open_dm(&mut self, user_id: i64, start_call: bool) {
        let Some(token) = self.token.clone() else {
            return;
        };
        self.config.closed_dms.retain(|id| *id != user_id);
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
        self.config.closed_dms.retain(|id| *id != channel.id);
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
        self.reply_to = None;
        self.chat_search_open = false;
        self.chat_msg_search.clear();
        self.new_message_anims.clear();
        self.typing_fade = 0.0;
        self.last_typing_text.clear();
        self.refocus_draft = true;
        self.play_sound(SoundEffect::UiClick);
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
        let reply_to_id = self.reply_to.take().map(|(id, _, _)| id);
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsClientMsg::SendMessage {
                channel_id,
                content,
                reply_to_id,
            });
            self.draft.clear();
            self.refocus_draft = true;
            self.play_sound(SoundEffect::MessageSend);
        } else {
            self.notice = "Not connected to the relay".to_string();
        }
    }

    pub(crate) fn toggle_reaction(&mut self, channel_id: i64, message_id: i64, emoji: String) {
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsClientMsg::ToggleReaction {
                channel_id,
                message_id,
                emoji,
            });
            self.play_sound(SoundEffect::Reaction);
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

    pub(crate) fn is_channel_muted(&self, channel_id: i64) -> bool {
        self.config.muted_channels.contains(&channel_id)
    }

    pub(crate) fn toggle_mute_channel(&mut self, channel_id: i64) {
        if let Some(pos) = self
            .config
            .muted_channels
            .iter()
            .position(|id| *id == channel_id)
        {
            self.config.muted_channels.remove(pos);
        } else {
            self.config.muted_channels.push(channel_id);
        }
        self.config.save();
    }

    pub(crate) fn close_dm(&mut self, channel_id: i64) {
        if !self.config.closed_dms.contains(&channel_id) {
            self.config.closed_dms.push(channel_id);
            self.config.save();
        }
        if self.selected_channel == Some(channel_id) {
            self.selected_channel = None;
            self.messages.clear();
        }
    }

    pub(crate) fn is_blocked(&self, user_id: i64) -> bool {
        self.config.blocked_users.contains(&user_id)
    }

    pub(crate) fn upload_my_avatar(&mut self) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let Some((raw_bytes, filename)) = crate::avatar::pick_avatar_file() else {
            return;
        };
        let (encoded, mime, hash) = match crate::avatar::process_avatar_bytes(&raw_bytes, &filename)
        {
            Ok(res) => res,
            Err(err) => {
                self.notice = err;
                return;
            }
        };
        use base64::Engine as _;
        let upload = skypeline_shared::AvatarUpload {
            mime,
            data_base64: base64::engine::general_purpose::STANDARD.encode(&encoded),
            hash,
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.notice = "Uploading profile picture...".into();
        self.rt.spawn(async move {
            match api.upload_avatar(&token, &upload).await {
                Ok(info) => {
                    let _ = tx.send(UiEvent::AvatarLoaded {
                        user_id: info.user_id,
                        hash: info.hash,
                        mime: info.mime,
                        data_base64: info.data_base64,
                    });
                    let _ = tx.send(UiEvent::Status("Profile picture updated".into()));
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(format!("Avatar upload failed: {e}")));
                }
            }
        });
    }

    pub(crate) fn toggle_block(&mut self, user_id: i64) {
        if let Some(pos) = self
            .config
            .blocked_users
            .iter()
            .position(|id| *id == user_id)
        {
            self.config.blocked_users.remove(pos);
        } else {
            self.config.blocked_users.push(user_id);
        }
        self.config.save();
    }

    pub(crate) fn start_call(&mut self, channel_id: i64) {
        let Some(channel) = self.channels.iter().find(|c| c.id == channel_id).cloned() else {
            return;
        };
        let room = Self::channel_room(channel_id);
        if self.in_voice && self.voice_room.as_deref() == Some(room.as_str()) {
            self.call_expanded = true;
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
        let muted = self.muted;
        let deafened = self.deafened;
        let input_dev = self.config.input_device.clone();
        let cap_res = {
            let audio = self.ensure_audio();
            audio.set_muted(muted);
            audio.set_deafened(deafened);
            audio.start_capture(room.clone(), ws.clone(), input_dev)
        };
        match cap_res {
            Ok(()) => {
                self.in_voice = true;
                self.call_expanded = true;
                self.incoming_call = None;
                self.voice_room = Some(room);
                self.voice_hint.clear();
                self.voice_participants_seen.clear();
                self.skip_join_chimes = true;
                self.call_started_at = Some(Self::now_secs());
                self.notice = format!("Calling {}", channel.name);
                self.play_sound(SoundEffect::CallJoin);
            }
            Err(e) => {
                self.notice = e;
                let _ = ws.send(WsClientMsg::LeaveVoice);
            }
        }
    }

    pub(crate) fn leave_voice(&mut self) {
        let was_in_voice = self.in_voice;
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
        self.stream_minimized = false;
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
        if was_in_voice {
            self.play_sound(SoundEffect::CallLeave);
        }
    }

    pub(crate) fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(audio) = &self.audio {
            audio.set_muted(self.muted);
        }
        self.send_voice_state();
        self.play_sound(if self.muted {
            SoundEffect::Mute
        } else {
            SoundEffect::Unmute
        });
    }

    pub(crate) fn toggle_deafen(&mut self) {
        self.deafened = !self.deafened;
        if let Some(audio) = &self.audio {
            audio.set_deafened(self.deafened);
        }
        self.send_voice_state();
        if self.config.ui_sounds {
            let clip = sounds::synthesize(if self.deafened {
                SoundEffect::Deafen
            } else {
                SoundEffect::Undeafen
            });
            self.ensure_audio().play_sfx(&clip);
        }
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
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "You".into());
        if let Some(share) = self.screen_share.take() {
            share.stop();
            self.sharing = false;
            if self.screen_owner == Some(me_id) {
                self.screen_texture = None;
                self.screen_owner = None;
                self.screen_fullscreen = false;
            }
            self.play_sound(SoundEffect::ScreenShareStop);
            return;
        }
        let room = self.voice_room.clone().unwrap_or_default();
        if room.is_empty() {
            return;
        }
        let Some(ws) = self.ws_tx.clone() else {
            return;
        };
        let Some(frames) = self.screen_frame_tx.clone() else {
            return;
        };
        let events = self.events_tx.clone();
        self.screen_share = Some(ScreenShare::start(
            room,
            me_id,
            me_name,
            Arc::clone(&self.screen_fps),
            Arc::clone(&self.screen_max_height),
            ws,
            frames,
            events,
        ));
        self.sharing = true;
        self.call_expanded = true;
        self.play_sound(SoundEffect::ScreenShareStart);
    }

    pub(crate) fn set_screen_fps(&mut self, fps: u32) {
        let clamped = match fps {
            15 => 15,
            60 => 60,
            _ => 30,
        };
        self.config.screen_fps = clamped;
        self.screen_fps.store(clamped, Ordering::Relaxed);
        self.config.save();
    }

    pub(crate) fn set_screen_resolution(&mut self, max_height: u32) {
        let clamped = match max_height {
            480 => 480,
            720 => 720,
            _ => 1080,
        };
        self.config.screen_max_height = clamped;
        self.screen_max_height.store(clamped, Ordering::Relaxed);
        self.config.save();
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
        self.connected = false;
        self.config.session = None;
        self.config.save();
        self.page = Page::Login;
        self.auth_step = AuthStep::Credentials;
        self.verify_email.clear();
        self.verify_code.clear();
        self.channels.clear();
        self.last_messages.clear();
        self.messages.clear();
        self.has_more_messages = false;
        self.loading_older = false;
        self.editing_message = None;
        self.reply_to = None;
        self.online.clear();
        self.friends.clear();
        self.friend_query.clear();
        self.friend_error.clear();
        self.friend_busy = false;
        self.friends_tab = FriendsTab::Online;
        self.avatars.clear();
        self.unread.clear();
        self.typing_users.clear();
        self.toasts.clear();
        self.profile_popup = None;
        self.show_settings_modal = false;
        self.show_profile_popover = false;
        self.show_inbox_popover = false;
        self.emoji_picker_target = None;
        self.incoming_call = None;
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
        let mut latest_screen_frame: Option<(i64, String, u32, u32, Vec<u8>)> = None;
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
                    if response.verification_required {
                        self.verify_email = response.email;
                        self.verify_code.clear();
                        self.auth_step = AuthStep::VerifyCode;
                        self.notice = response.message;
                    } else {
                        self.login_email = response.email;
                        self.login_password = self.register_password.clone();
                        self.auth_step = AuthStep::Credentials;
                        self.auth_tab = AuthTab::Login;
                        self.notice = "Account created! Signing in...".into();
                        self.submit_login();
                    }
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
                    if let Some(last) = messages.last() {
                        self.last_messages
                            .insert(channel_id, (last.content.clone(), last.created_at));
                    }
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
                    self.config.closed_dms.retain(|id| *id != message.channel_id);
                    self.last_messages
                        .insert(message.channel_id, (message.content.clone(), message.created_at));
                    if let Some(ch_typing) = self.typing_users.get_mut(&message.channel_id) {
                        ch_typing.remove(&message.user_id);
                    }
                    let selected = self.selected_channel == Some(message.channel_id);
                    let mine = message.user_id == self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                    if selected {
                        if !self.config.reduced_motion {
                            self.new_message_anims.insert(message.id, Self::now_secs());
                        }
                        if !mine {
                            self.play_sound(SoundEffect::MessageReceive);
                        }
                        self.messages.push(message);
                        if self.messages.len() > 300 {
                            let excess = self.messages.len() - 300;
                            self.messages.drain(..excess);
                        }
                    } else if !mine && !self.is_blocked(message.user_id) {
                        *self.unread.entry(message.channel_id).or_insert(0) += 1;
                        if !self.is_channel_muted(message.channel_id) {
                            self.play_sound(SoundEffect::MessageReceive);
                            // Push non-intrusive in-app toast (stack up to 3, 4s progress bar)
                            self.toasts.push(ToastItem {
                                channel_id: message.channel_id,
                                sender: message.username.clone(),
                                preview: message.content.clone(),
                                created_at: Self::now_secs(),
                                remaining_secs: 4.0,
                                closing_at: None,
                            });
                            if self.toasts.len() > 3 {
                                self.toasts.remove(0);
                            }
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
                UiEvent::ReactionUpdated {
                    channel_id,
                    message_id,
                    emoji,
                    user_id,
                    added,
                } => {
                    if self.selected_channel == Some(channel_id) {
                        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                            if added {
                                if let Some(r) =
                                    msg.reactions.iter_mut().find(|r| r.emoji == emoji)
                                {
                                    if !r.user_ids.contains(&user_id) {
                                        r.user_ids.push(user_id);
                                    }
                                } else {
                                    msg.reactions.push(ReactionDto {
                                        emoji,
                                        user_ids: vec![user_id],
                                    });
                                }
                            } else {
                                if let Some(r) =
                                    msg.reactions.iter_mut().find(|r| r.emoji == emoji)
                                {
                                    r.user_ids.retain(|u| *u != user_id);
                                }
                                msg.reactions.retain(|r| !r.user_ids.is_empty());
                            }
                        }
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
                    if let Some(me) = self.me.as_mut() {
                        if me.user_id == user_id {
                            me.avatar_hash = Some(hash.clone());
                            if let Some(sess) = self.config.session.as_mut() {
                                sess.avatar_hash = Some(hash.clone());
                                self.config.save();
                            }
                        }
                    }
                    let _ = self
                        .avatars
                        .on_loaded(ctx, user_id, hash, mime, data_base64);
                }
                UiEvent::VoiceState { room, participants } => {
                    let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                    if let Some(ch_id) = room
                        .strip_prefix("channel:")
                        .and_then(|s| s.parse::<i64>().ok())
                    {
                        if participants.is_empty() {
                            self.active_room_calls.remove(&ch_id);
                        } else {
                            self.active_room_calls.insert(ch_id, participants.clone());
                        }
                    }
                    let in_this_room =
                        self.in_voice && self.voice_room.as_deref() == Some(room.as_str());
                    if !in_this_room {
                        if let Some(ch_id) = room
                            .strip_prefix("channel:")
                            .and_then(|s| s.parse::<i64>().ok())
                        {
                            if let Some(caller) = participants.iter().find(|p| p.user_id != me_id) {
                                let already_ringing = self
                                    .incoming_call
                                    .as_ref()
                                    .is_some_and(|(id, _)| *id == ch_id);
                                if !already_ringing {
                                    self.incoming_call =
                                        Some((ch_id, caller.username.clone()));
                                    self.play_sound(SoundEffect::CallIncoming);
                                }
                            } else if self
                                .incoming_call
                                .as_ref()
                                .is_some_and(|(id, _)| *id == ch_id)
                            {
                                self.incoming_call = None;
                            }
                        }
                        continue;
                    }

                    let new_joiner = participants.iter().any(|p| {
                        p.user_id != me_id && !self.voice_participants_seen.contains(&p.user_id)
                    });
                    if new_joiner && !self.skip_join_chimes {
                        self.play_sound(SoundEffect::CallJoin);
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
                    self.ensure_audio().push_pcm(&pcm);
                }
                UiEvent::ScreenFrame {
                    from,
                    username,
                    width,
                    height,
                    rgba,
                } => {
                    if rgba.len() == (width as usize) * (height as usize) * 4 {
                        latest_screen_frame = Some((from, username, width, height, rgba));
                    }
                }
                UiEvent::ScreenStopped { from } => {
                    if latest_screen_frame
                        .as_ref()
                        .is_some_and(|(f, _, _, _, _)| *f == from)
                    {
                        latest_screen_frame = None;
                    }
                    if self.screen_owner == Some(from) {
                        self.screen_texture = None;
                        self.screen_owner = None;
                        self.screen_fullscreen = false;
                    }
                }
                UiEvent::Status(status) => {
                    self.busy = false;
                    self.friend_busy = false;
                    if status == "Connected" {
                        self.connected = true;
                        self.notice.clear();
                    } else if status.starts_with("Connecting") || status.starts_with("Reconnecting")
                    {
                        self.connected = false;
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
                        if self.friends_tab == FriendsTab::AddFriend && self.selected_channel.is_none() {
                            self.friend_error = error.clone();
                        }
                        self.notice = error;
                    }
                }
                UiEvent::Disconnected => {
                    self.connected = false;
                    self.participants.clear();
                    self.active_room_calls.clear();
                    self.screen_texture = None;
                    self.screen_owner = None;
                    self.screen_fullscreen = false;
                }
                UiEvent::Reconnected => {
                    self.connected = true;
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
                UiEvent::UpdateSynced(ts) => {
                    if self.config.last_installed_update.as_deref() != Some(ts.as_str()) {
                        self.config.last_installed_update = Some(ts);
                        self.config.save();
                    }
                }
                UiEvent::UpdateReady {
                    asset_updated_at,
                    installer_path,
                } => {
                    self.config.last_installed_update = Some(asset_updated_at);
                    self.config.save();
                    if updater::launch_silent_install_and_relaunch(&installer_path) {
                        #[cfg(target_os = "windows")]
                        {
                            self.allow_close = true;
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
        if let Some((from, username, width, height, rgba)) = latest_screen_frame {
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
            self.animating = true;
        }
    }

    #[cfg(target_os = "windows")]
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

    #[cfg(not(target_os = "windows"))]
    fn handle_tray(&mut self, _ctx: &Context) {}

    fn handle_shortcuts(&mut self, ctx: &Context) {
        ctx.input(|i| {
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::M) {
                self.toggle_mute();
            }
            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::D) {
                self.toggle_deafen();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Slash) {
                self.open_settings_modal(SettingsTab::Keybinds);
            }
            if i.key_pressed(egui::Key::Escape) {
                if self.show_settings_modal {
                    self.close_settings_modal();
                } else if self.emoji_picker_target.is_some() {
                    self.emoji_picker_target = None;
                } else if self.profile_popup.is_some() {
                    self.close_profile_popup();
                } else if self.show_profile_popover {
                    self.show_profile_popover = false;
                } else if self.show_inbox_popover {
                    self.show_inbox_popover = false;
                } else if self.chat_search_open {
                    self.chat_search_open = false;
                    self.chat_msg_search.clear();
                } else if self.reply_to.is_some() {
                    self.reply_to = None;
                } else if self.editing_message.is_some() {
                    self.editing_message = None;
                }
            }
        });
    }

    pub(crate) fn open_settings_modal(&mut self, tab: SettingsTab) {
        self.settings_tab = tab;
        if !self.show_settings_modal || self.settings_closing_at.is_some() {
            self.show_settings_modal = true;
            self.settings_opened_at = Self::now_secs();
            self.settings_closing_at = None;
        }
    }

    pub(crate) fn close_settings_modal(&mut self) {
        if self.config.reduced_motion {
            self.show_settings_modal = false;
            self.settings_closing_at = None;
        } else if self.show_settings_modal && self.settings_closing_at.is_none() {
            self.settings_closing_at = Some(Self::now_secs());
        }
    }

    pub(crate) fn close_profile_popup(&mut self) {
        if self.config.reduced_motion {
            self.profile_popup = None;
            self.profile_popup_closing_at = None;
        } else if self.profile_popup.is_some() && self.profile_popup_closing_at.is_none() {
            self.profile_popup_closing_at = Some(Self::now_secs());
        }
    }

    pub(crate) fn pulse_time(&self, ctx: &Context) -> Option<f64> {
        if self.config.reduced_motion {
            None
        } else {
            Some(ctx.input(|i| i.time))
        }
    }

    pub(crate) fn pending_badge_scale(&mut self, current_count: usize) -> f32 {
        let now = Self::now_secs();
        if current_count != self.last_pending_count {
            self.last_pending_count = current_count;
            if current_count > 0 && !self.config.reduced_motion {
                self.pending_badge_pop_at = Some(now);
            }
        }
        if let Some(started) = self.pending_badge_pop_at {
            let (scale, done) =
                anim::badge_pop_scale((now - started) as f32, self.config.reduced_motion);
            if done {
                self.pending_badge_pop_at = None;
            } else {
                self.animating = true;
            }
            scale
        } else {
            1.0
        }
    }

    fn ui_main(&mut self, ctx: &Context) {
        self.animating = false;
        self.handle_shortcuts(ctx);
        self.ui_left_sidebar(ctx);
        self.ui_main_stage(ctx);
        self.ui_screen_fullscreen(ctx);
        self.ui_profile_popover(ctx);
        self.ui_user_profile_popup(ctx);
        self.ui_emoji_picker(ctx);
        self.ui_inbox_popover(ctx);
        self.ui_settings_modal(ctx);
        self.ui_toasts(ctx);
    }

    fn ui_main_stage(&mut self, ctx: &Context) {
        let p = theme::of(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.canvas))
            .show(ctx, |ui| {
                if self.selected_channel.is_none() {
                    self.ui_friends_home(ui);
                } else {
                    self.ui_chat_stage(ui);
                }
            });
    }

    pub(crate) fn ui_notice(&self, ui: &mut Ui) {
        if self.notice.is_empty() {
            return;
        }
        let p = theme::of(ui.ctx());
        let color = if self.notice.starts_with("HTTP") || self.notice.contains("failed") {
            p.danger
        } else {
            p.text_secondary
        };
        ui.label(RichText::new(&self.notice).color(color).small());
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
        Color32::from_rgb(0x0E, 0xA5, 0xE9),
        Color32::from_rgb(0x14, 0xB8, 0xA6),
        Color32::from_rgb(0xF5, 0x9E, 0x0B),
        Color32::from_rgb(0xF4, 0x3F, 0x5E),
        Color32::from_rgb(0x3B, 0x82, 0xF6),
    ];
    palette[(h as usize) % palette.len()]
}

pub(crate) fn format_hhmm(unix_secs: i64) -> String {
    let secs_in_day = unix_secs.rem_euclid(86_400);
    let hours = secs_in_day / 3_600;
    let mins = (secs_in_day % 3_600) / 60;
    format!("{hours:02}:{mins:02}")
}

pub(crate) fn day_index(unix_secs: i64) -> i64 {
    unix_secs.div_euclid(86_400)
}

pub(crate) fn format_day_label(unix_secs: i64, now_secs: i64) -> String {
    let d = day_index(unix_secs);
    let today = day_index(now_secs);
    if d == today {
        "Today".to_string()
    } else if d == today - 1 {
        "Yesterday".to_string()
    } else {
        // Simple YYYY-MM-DD civil date from epoch days
        let z = d + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if month <= 2 { y + 1 } else { y };
        format!("{month:02}/{day:02}/{year:04}")
    }
}

#[allow(dead_code)]
pub(crate) fn format_discord_timestamp(unix_secs: i64, now_secs: i64) -> String {
    let day = format_day_label(unix_secs, now_secs);
    if day == "Today" || day == "Yesterday" {
        format!("{day} at {}", format_hhmm(unix_secs))
    } else {
        format!("{day} {}", format_hhmm(unix_secs))
    }
}

impl eframe::App for SkyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray(ctx);
        self.drain_events(ctx);
        match self.page {
            Page::Login => self.ui_login(ctx),
            Page::Main => self.ui_main(ctx),
        }
        if self.animating || self.sharing || self.screen_texture.is_some() {
            ctx.request_repaint();
        } else if !self.config.reduced_motion && self.page == Page::Main {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
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
