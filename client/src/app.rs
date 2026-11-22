//! egui application: login/register, settings, channels, DMs, voice and
//! server-relayed screen sharing.

use crate::api::ApiClient;
use crate::audio::AudioEngine;
use crate::avatar::{self, AvatarCache};
use crate::config::Config;
use crate::events::UiEvent;
use crate::net;
use crate::screen::ScreenShare;
use crate::{native, theme};
use base64::Engine as _;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, RichText, Sense, Stroke, TextureOptions, Ui};
use skypeline_shared::{
    AuthResponse, AvatarUpload, ChannelInfo, FriendInfo, FriendRequest, FriendRespondRequest,
    FriendshipStatus, LoginRequest, MessageDto, OnlineUser, PresenceStatus, RegisterRequest,
    ResendCodeRequest, VerifyCodeRequest, VoiceParticipant, WsClientMsg,
};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use tokio::sync::mpsc as tmpsc;
use tokio::task::JoinHandle;

const VOICE_ROOM: &str = "General Voice";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Login,
    Main,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthTab {
    Login,
    Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthStep {
    Credentials,
    VerifyCode,
}

pub struct SkyApp {
    rt: tokio::runtime::Runtime,
    config: Config,
    events_tx: mpsc::Sender<UiEvent>,
    events_rx: mpsc::Receiver<UiEvent>,

    page: Page,
    auth_tab: AuthTab,
    auth_step: AuthStep,
    login_email: String,
    login_password: String,
    register_email: String,
    register_username: String,
    register_password: String,
    verify_email: String,
    verify_code: String,
    busy: bool,
    notice: String,

    token: Option<String>,
    me: Option<AuthResponse>,
    channels: Vec<ChannelInfo>,
    online: Vec<OnlineUser>,
    friends: Vec<FriendInfo>,
    friend_query: String,
    friend_busy: bool,
    avatars: AvatarCache,
    selected_channel: Option<i64>,
    messages: Vec<MessageDto>,
    draft: String,
    unread: HashMap<i64, usize>,
    presence: PresenceStatus,
    settings_open: bool,

    voice_hint: String,
    voice_room: Option<String>,
    voice_label: String,
    viewer_volume: f32,
    chat_collapsed: bool,
    chat_on_side: bool,
    participants: Vec<VoiceParticipant>,
    in_voice: bool,
    muted: bool,
    deafened: bool,
    sharing: bool,
    audio: Option<AudioEngine>,
    screen_share: Option<ScreenShare>,

    ws_tx: Option<tmpsc::UnboundedSender<WsClientMsg>>,
    ws_task: Option<JoinHandle<()>>,

    screen_texture: Option<egui::TextureHandle>,
    screen_owner: Option<i64>,
    screen_size: (u32, u32),
    screen_fullscreen: bool,

    /// Participants already announced with a join chime.
    voice_participants_seen: HashSet<i64>,
    /// The first voice snapshot after joining is the current room, not new joins.
    skip_join_chimes: bool,
}

impl SkyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        theme::apply(&_cc.egui_ctx, config.dark_mode);
        let (events_tx, events_rx) = mpsc::channel();

        Self {
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
            notice: format!("Hosted service: {}", Config::default().base_url()),
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
            draft: String::new(),
            unread: HashMap::new(),
            presence: PresenceStatus::Online,
            settings_open: false,
            voice_hint: String::new(),
            voice_room: None,
            voice_label: VOICE_ROOM.to_string(),
            viewer_volume: 1.0,
            chat_collapsed: false,
            chat_on_side: true,
            participants: Vec::new(),
            in_voice: false,
            muted: false,
            deafened: false,
            sharing: false,
            audio: None,
            screen_share: None,
            ws_tx: None,
            ws_task: None,
            screen_texture: None,
            screen_owner: None,
            screen_size: (0, 0),
            screen_fullscreen: false,
            voice_participants_seen: HashSet::new(),
            skip_join_chimes: false,
        }
    }

    fn api(&self) -> ApiClient {
        ApiClient::new(self.config.base_url())
    }

    // ---------------------------------------------------------------- actions

    fn connect_ws(&mut self, token: &str) {
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        let (tx, rx) = tmpsc::unbounded_channel();
        self.ws_tx = Some(tx);
        let base = self.config.base_url();
        let token = token.to_string();
        let events = self.events_tx.clone();
        self.ws_task = Some(self.rt.spawn(net::run(base, token, events, rx)));
    }

    fn load_channels(&self) {
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

    fn load_friends(&mut self) {
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

    fn send_friend_request(&mut self) {
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

    fn respond_friend(&mut self, requester_id: i64, accept: bool) {
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
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    fn upload_avatar(&self, bytes: Vec<u8>, mime: String, hash: String) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            let upload = AvatarUpload {
                mime,
                data_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
                hash: hash.clone(),
            };
            match api.upload_avatar(&token, &upload).await {
                Ok(_) => {
                    let _ = tx.send(UiEvent::AvatarUpdated { hash });
                    let _ = tx.send(UiEvent::Status(
                        "Profile picture updated (128x128, under 15 KB)".into(),
                    ));
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    fn load_messages(&self, channel_id: i64) {
        let Some(token) = self.token.clone() else {
            return;
        };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            match api.messages(&token, channel_id).await {
                Ok(messages) => {
                    let _ = tx.send(UiEvent::Messages {
                        channel_id,
                        messages,
                    });
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    fn open_dm(&self, user_id: i64, start_call: bool) {
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

    fn upsert_channel(&mut self, channel: ChannelInfo) {
        if let Some(existing) = self.channels.iter_mut().find(|c| c.id == channel.id) {
            *existing = channel;
        } else {
            self.channels.push(channel);
        }
    }

    fn select_channel(&mut self, channel_id: i64) {
        if self.selected_channel == Some(channel_id) {
            return;
        }
        self.selected_channel = Some(channel_id);
        self.unread.remove(&channel_id);
        self.messages.clear();
        self.load_messages(channel_id);
    }

    fn send_message(&mut self) {
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
        } else {
            self.notice = "Not connected to the relay".to_string();
        }
    }

    fn join_voice(&mut self) {
        self.join_voice_room(VOICE_ROOM.to_string(), VOICE_ROOM.to_string());
    }

    fn join_voice_room(&mut self, room: String, label: String) {
        if self.in_voice {
            self.leave_voice();
        }
        let ws = self.ws_tx.clone();
        let Some(ws) = ws else {
            self.notice = "Not connected to the relay".into();
            return;
        };
        let _ = ws.send(WsClientMsg::JoinVoice { room: room.clone() });
        let audio = self.audio.get_or_insert_with(AudioEngine::start);
        audio.set_muted(self.muted);
        audio.set_deafened(self.deafened);
        audio.set_volume(self.viewer_volume);
        match audio.start_capture(room.clone(), ws.clone()) {
            Ok(()) => {
                self.in_voice = true;
                self.voice_room = Some(room);
                self.voice_label = label.clone();
                self.voice_hint =
                    "Voice is connected — audio and screen frames stay server-relayed".into();
                self.voice_participants_seen.clear();
                self.skip_join_chimes = true;
                self.notice = format!("Joined {label}");
            }
            Err(e) => {
                self.notice = e;
                let _ = ws.send(WsClientMsg::LeaveVoice);
            }
        }
    }

    fn leave_voice(&mut self) {
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
        self.voice_label = VOICE_ROOM.to_string();
        self.viewer_volume = 1.0;
        self.voice_participants_seen.clear();
        self.skip_join_chimes = false;
        self.notice = "Left voice".into();
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(audio) = &self.audio {
            audio.set_muted(self.muted);
        }
        self.send_voice_state();
    }

    fn toggle_deafen(&mut self) {
        self.deafened = !self.deafened;
        if self.deafened {
            self.muted = true;
        }
        if let Some(audio) = &self.audio {
            audio.set_deafened(self.deafened);
            audio.set_muted(self.muted);
        }
        self.send_voice_state();
    }

    fn send_voice_state(&self) {
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsClientMsg::SetVoiceState {
                muted: self.muted,
                deafened: self.deafened,
                speaking: false,
            });
        }
    }

    fn toggle_screen_share(&mut self) {
        if !self.in_voice {
            self.voice_hint = "Join voice before sharing your screen".into();
            return;
        }
        if let Some(share) = self.screen_share.take() {
            share.stop();
            self.sharing = false;
            self.voice_hint = "Screen sharing stopped".into();
            return;
        }
        let room = self
            .voice_room
            .clone()
            .unwrap_or_else(|| VOICE_ROOM.to_string());
        if let Some(ws) = &self.ws_tx {
            self.screen_share = Some(ScreenShare::start(room, ws.clone()));
            self.sharing = true;
            self.voice_hint = "You are sharing your screen through the relay".into();
        }
    }

    fn set_presence(&mut self, status: PresenceStatus) {
        self.presence = status;
        if let Some(ws) = &self.ws_tx {
            let _ = ws.send(WsClientMsg::SetPresence { status });
        }
    }

    fn sign_out(&mut self) {
        self.leave_voice();
        if let Some(task) = self.ws_task.take() {
            task.abort();
        }
        self.ws_tx = None;
        self.token = None;
        self.me = None;
        self.page = Page::Login;
        self.auth_step = AuthStep::Credentials;
        self.verify_email.clear();
        self.verify_code.clear();
        self.channels.clear();
        self.messages.clear();
        self.online.clear();
        self.friends.clear();
        self.friend_query.clear();
        self.friend_busy = false;
        self.avatars.clear();
        self.unread.clear();
        self.selected_channel = None;
        self.settings_open = false;
        self.screen_fullscreen = false;
        self.voice_participants_seen.clear();
        self.skip_join_chimes = false;
        self.notice = "Signed out".into();
    }

    // ---------------------------------------------------------------- events

    fn drain_events(&mut self, ctx: &Context) {
        while let Ok(event) = self.events_rx.try_recv() {
            match event {
                UiEvent::SignedIn(response) => {
                    self.busy = false;
                    self.me = Some(response.clone());
                    self.token = Some(response.token.clone());
                    self.page = Page::Main;
                    self.notice = format!("Signed in as {}", response.username);
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
                    for channel in channels {
                        self.upsert_channel(channel);
                    }
                    let first_text = if self.selected_channel.is_none() {
                        self.channels
                            .iter()
                            .find(|c| c.kind == "text")
                            .map(|c| c.id)
                    } else {
                        None
                    };
                    if let Some(id) = first_text {
                        self.select_channel(id);
                    }
                }
                UiEvent::DmOpened {
                    channel,
                    start_call,
                } => {
                    self.upsert_channel(channel.clone());
                    self.unread.remove(&channel.id);
                    self.selected_channel = Some(channel.id);
                    self.messages.clear();
                    self.load_messages(channel.id);
                    if start_call {
                        let room = format!("dm-channel:{}", channel.id);
                        self.join_voice_room(room, format!("Private call · {}", channel.name));
                    }
                }
                UiEvent::Messages {
                    channel_id,
                    messages,
                } => {
                    if self.selected_channel == Some(channel_id) {
                        self.messages = messages;
                    }
                }
                UiEvent::NewMessage(message) => {
                    let selected = self.selected_channel == Some(message.channel_id);
                    let mine = message.user_id == self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                    if selected {
                        self.messages.push(message);
                    } else {
                        if !mine {
                            *self.unread.entry(message.channel_id).or_insert(0) += 1;
                            native::message_chime();
                            let channel_name = self
                                .channels
                                .iter()
                                .find(|c| c.id == message.channel_id)
                                .map(|c| c.name.as_str())
                                .unwrap_or("Skypeline");
                            native::notify(
                                "Skypeline",
                                &format!(
                                    "{} in {}: {}",
                                    message.username, channel_name, message.content
                                ),
                            );
                        }
                    }
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
                    self.friends = friends;
                }
                UiEvent::ReloadFriends => self.load_friends(),
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
                UiEvent::AvatarUpdated { hash } => {
                    if let Some(me) = &mut self.me {
                        me.avatar_hash = Some(hash);
                    }
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
                    self.notice = status;
                }
                UiEvent::Error(error) => {
                    self.busy = false;
                    self.notice = error;
                }
                UiEvent::Disconnected => {
                    self.notice = "Relay connection lost — reconnecting...".into();
                    // Keep the local voice/screen capture running. The background
                    // task retries automatically; state is re-registered below.
                    self.participants.clear();
                    self.screen_texture = None;
                    self.screen_owner = None;
                    self.screen_fullscreen = false;
                }
                UiEvent::Reconnected => {
                    self.notice = "Reconnected to relay".into();
                    if let Some(ws) = &self.ws_tx {
                        let _ = ws.send(WsClientMsg::SetPresence {
                            status: self.presence,
                        });
                        if self.in_voice {
                            let room = self
                                .voice_room
                                .clone()
                                .unwrap_or_else(|| VOICE_ROOM.to_string());
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
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------- UI

    fn ui_login(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading(RichText::new("Skypeline").size(32.0).strong());
                ui.label("Chat, voice and screen sharing — relayed, never peer-to-peer.");
                ui.add_space(18.0);

                match self.auth_step {
                    AuthStep::Credentials => {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.auth_tab, AuthTab::Login, "Sign in");
                            ui.selectable_value(
                                &mut self.auth_tab,
                                AuthTab::Register,
                                "Create account",
                            );
                        });
                        ui.add_space(12.0);

                        match self.auth_tab {
                            AuthTab::Login => {
                                ui.label("Email");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.login_email)
                                        .desired_width(280.0)
                                        .hint_text("you@example.com"),
                                );
                                ui.label("Password");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.login_password)
                                        .password(true)
                                        .desired_width(280.0)
                                        .hint_text("Password"),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .add(
                                        egui::Button::new("Sign in").min_size([180.0, 34.0].into()),
                                    )
                                    .clicked()
                                    && !self.busy
                                {
                                    self.submit_login();
                                }
                            }
                            AuthTab::Register => {
                                ui.label("Email");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.register_email)
                                        .desired_width(280.0)
                                        .hint_text("you@example.com"),
                                );
                                ui.label("Username");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.register_username)
                                        .desired_width(280.0)
                                        .hint_text("Display name"),
                                );
                                ui.label("Password (8+ characters)");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.register_password)
                                        .password(true)
                                        .desired_width(280.0)
                                        .hint_text("Password"),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .add(
                                        egui::Button::new("Create account")
                                            .min_size([180.0, 34.0].into()),
                                    )
                                    .clicked()
                                    && !self.busy
                                {
                                    self.submit_register();
                                }
                            }
                        }
                    }
                    AuthStep::VerifyCode => {
                        ui.heading(RichText::new("Confirm your email").size(22.0).strong());
                        ui.label(format!(
                            "Enter the confirmation code sent to {}",
                            self.verify_email
                        ));
                        ui.add_space(10.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.verify_code)
                                .desired_width(200.0)
                                .hint_text("Confirmation code"),
                        );
                        ui.add_space(8.0);
                        if ui
                            .add(
                                egui::Button::new("Verify & sign in")
                                    .min_size([180.0, 34.0].into()),
                            )
                            .clicked()
                            && !self.busy
                        {
                            self.submit_verify();
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("Resend code").clicked() && !self.busy {
                                self.resend_code();
                            }
                            if ui.small_button("Back to sign in").clicked() && !self.busy {
                                self.auth_step = AuthStep::Credentials;
                                self.auth_tab = AuthTab::Login;
                                self.verify_code.clear();
                                self.notice = "Sign in with your confirmed account.".into();
                            }
                        });
                    }
                }

                ui.add_space(14.0);
                self.ui_notice(ui);
                ui.add_space(20.0);
                ui.separator();
                ui.label(RichText::new("Hosted service").small().strong());
                ui.hyperlink_to(Config::default().base_url(), Config::default().base_url());
            });
        });
    }

    fn submit_login(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = LoginRequest {
            email: self.login_email.clone(),
            password: self.login_password.clone(),
        };
        self.busy = true;
        self.notice = "Signing in...".into();
        self.rt.spawn(async move {
            match api.login(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::SignedIn(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn submit_register(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = RegisterRequest {
            email: self.register_email.clone(),
            username: self.register_username.clone(),
            password: self.register_password.clone(),
        };
        self.busy = true;
        self.notice = "Creating account...".into();
        self.rt.spawn(async move {
            match api.register(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::RegisterPending(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn submit_verify(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = VerifyCodeRequest {
            email: self.verify_email.clone(),
            code: self.verify_code.clone(),
        };
        self.busy = true;
        self.notice = "Confirming code...".into();
        self.rt.spawn(async move {
            match api.verify_code(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::SignedIn(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn resend_code(&mut self) {
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = ResendCodeRequest {
            email: self.verify_email.clone(),
        };
        self.busy = true;
        self.notice = "Sending a new code...".into();
        self.rt.spawn(async move {
            match api.resend_code(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::Status(response.message));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn ui_notice(&self, ui: &mut Ui) {
        if self.notice.is_empty() {
            return;
        }
        let color = if self.notice.starts_with("HTTP") || self.notice.contains("failed") {
            Color32::from_rgb(0xFF, 0x7B, 0x7B)
        } else {
            ui.visuals().weak_text_color()
        };
        ui.label(RichText::new(&self.notice).color(color).small());
    }

    fn ui_main(&mut self, ctx: &Context) {
        self.ui_top_bar(ctx);
        self.ui_left_sidebar(ctx);
        // In a call, voice controls move into the dedicated stage at the bottom.
        if !self.in_voice {
            self.ui_voice_sidebar(ctx);
        }
        self.ui_chat(ctx);
        self.ui_screen_fullscreen(ctx);
        self.ui_settings(ctx);
    }

    fn ui_top_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("skypeline_top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Skypeline").strong());
                ui.separator();
                if let Some(me) = &self.me {
                    self::status_dot(ui, self.presence);
                    ui.label(
                        RichText::new(format!("{} · {}", me.username, self.presence.label()))
                            .strong(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let toggle = if self.config.dark_mode {
                        "☀ Light"
                    } else {
                        "🌙 Dark"
                    };
                    if ui.button(toggle).clicked() {
                        self.config.dark_mode = !self.config.dark_mode;
                        self.config.save();
                        theme::apply(ctx, self.config.dark_mode);
                    }
                    if ui.button("Settings").clicked() {
                        self.settings_open = true;
                    }
                    if ui.button("Sign out").clicked() {
                        self.sign_out();
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("skypeline_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_notice(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Relayed chat · voice · screen")
                            .small()
                            .weak(),
                    );
                });
            });
        });
    }

    fn ui_left_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("channels_panel")
            .resizable(true)
            .default_width(244.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("CHANNELS").small().strong());
                for channel in self.channels.clone() {
                    if channel.kind == "dm" {
                        continue;
                    }
                    let selected = self.selected_channel == Some(channel.id);
                    let icon = if channel.kind == "voice" { "🔊" } else { "#" };
                    let enabled = channel.kind != "voice";
                    if ui
                        .add_sized(
                            [ui.available_width(), 24.0],
                            egui::SelectableLabel::new(
                                selected,
                                format!("{icon} {}", channel.name),
                            ),
                        )
                        .clicked()
                        && enabled
                    {
                        self.select_channel(channel.id);
                    }
                }

                ui.add_space(12.0);
                ui.label(RichText::new("DIRECT MESSAGES").small().strong());
                let dms = self
                    .channels
                    .iter()
                    .filter(|c| c.kind == "dm")
                    .cloned()
                    .collect::<Vec<_>>();
                if dms.is_empty() {
                    ui.label(
                        RichText::new("Add a friend below to start a private chat")
                            .small()
                            .weak(),
                    );
                }
                for channel in dms {
                    let selected = self.selected_channel == Some(channel.id);
                    let unread = self.unread.get(&channel.id).copied().unwrap_or(0);
                    ui.horizontal(|ui| {
                        let label_width = (ui.available_width() - 34.0).max(80.0);
                        let response = ui.add_sized(
                            [label_width, 24.0],
                            egui::SelectableLabel::new(selected, format!("@ {}", channel.name)),
                        );
                        if response.clicked() {
                            self.select_channel(channel.id);
                        }
                        if unread > 0 {
                            ui_badge(ui, &unread.to_string());
                        }
                        if ui
                            .small_button("📞")
                            .on_hover_text("Private voice call")
                            .clicked()
                        {
                            let room = format!("dm-channel:{}", channel.id);
                            self.join_voice_room(room, format!("Private call · {}", channel.name));
                        }
                    });
                }

                ui.add_space(12.0);
                ui.label(RichText::new("FRIENDS / ONLINE").small().strong());
                let friends = self.friends.clone();
                let accepted = friends
                    .iter()
                    .filter(|f| f.status == FriendshipStatus::Accepted || f.direction == "mutual")
                    .cloned()
                    .collect::<Vec<_>>();
                if accepted.is_empty() {
                    ui.label(
                        RichText::new("No friends yet — add someone by username or email")
                            .small()
                            .weak(),
                    );
                }
                for friend in accepted {
                    let dm_selected = self.channels.iter().any(|c| {
                        c.kind == "dm"
                            && c.peer_user_id == Some(friend.user_id)
                            && self.selected_channel == Some(c.id)
                    });
                    ui.horizontal(|ui| {
                        self::status_dot(ui, friend.presence);
                        let response = ui.add_sized(
                            [ui.available_width() - 48.0, 22.0],
                            egui::SelectableLabel::new(dm_selected, friend.username.clone()),
                        );
                        if response.on_hover_text(friend.presence.label()).clicked() {
                            self.open_dm(friend.user_id, false);
                        }
                        if ui
                            .small_button("📞")
                            .on_hover_text("Private voice + screen call")
                            .clicked()
                        {
                            self.open_dm(friend.user_id, true);
                        }
                    });
                }

                let incoming = friends
                    .iter()
                    .filter(|f| f.status == FriendshipStatus::Pending && f.direction == "incoming")
                    .cloned()
                    .collect::<Vec<_>>();
                if !incoming.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("FRIEND REQUESTS").small().strong());
                    for friend in incoming {
                        ui.horizontal(|ui| {
                            ui.label(friend.username);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("Accept").clicked() {
                                        self.respond_friend(friend.user_id, true);
                                    }
                                    if ui.small_button("Decline").clicked() {
                                        self.respond_friend(friend.user_id, false);
                                    }
                                },
                            );
                        });
                    }
                }

                ui.add_space(10.0);
                ui.label(RichText::new("ADD FRIEND").small().strong());
                let busy = self.friend_busy;
                ui.horizontal(|ui| {
                    let response = ui.add_enabled(
                        !busy,
                        egui::TextEdit::singleline(&mut self.friend_query)
                            .desired_width(ui.available_width() - 46.0)
                            .hint_text("username or email")
                            .id_source("friend_query"),
                    );
                    if ui.add_enabled(!busy, egui::Button::new("Add")).clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.send_friend_request();
                        response.request_focus();
                    }
                });
                ui.label(
                    RichText::new("Only accepted friends can message or call you.")
                        .small()
                        .weak(),
                );
            });
    }

    fn ui_voice_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::right("voice_panel")
            .resizable(true)
            .default_width(268.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                let card_fill = if ui.visuals().dark_mode {
                    Color32::from_rgb(0x1B, 0x25, 0x31)
                } else {
                    Color32::from_rgb(0xFF, 0xFF, 0xFF)
                };
                egui::Frame::new()
                    .fill(card_fill)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .corner_radius(10.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("VOICE").small().strong());
                        ui.label(RichText::new(VOICE_ROOM).strong());
                        ui.add_space(8.0);

                        let gap = 6.0;
                        let button_width = (ui.available_width() - gap) / 2.0;
                        egui::Grid::new("voice_controls")
                            .num_columns(2)
                            .spacing([gap, gap])
                            .show(ui, |ui| {
                                let voice_label = if self.in_voice {
                                    "Leave Voice"
                                } else {
                                    "Join Voice"
                                };
                                if ui
                                    .add_sized([button_width, 30.0], egui::Button::new(voice_label))
                                    .clicked()
                                {
                                    if self.in_voice {
                                        self.leave_voice();
                                    } else {
                                        self.join_voice();
                                    }
                                }
                                let mute_label = if self.muted { "Unmute" } else { "Mute" };
                                if ui
                                    .add_sized(
                                        [button_width, 30.0],
                                        egui::SelectableLabel::new(self.muted, mute_label),
                                    )
                                    .clicked()
                                {
                                    self.toggle_mute();
                                }
                                ui.end_row();

                                let deafen_label =
                                    if self.deafened { "Undeafen" } else { "Deafen" };
                                if ui
                                    .add_sized(
                                        [button_width, 30.0],
                                        egui::SelectableLabel::new(self.deafened, deafen_label),
                                    )
                                    .clicked()
                                {
                                    self.toggle_deafen();
                                }
                                if ui
                                    .add_sized(
                                        [button_width, 30.0],
                                        egui::SelectableLabel::new(
                                            self.sharing,
                                            if self.sharing {
                                                "Stop Share"
                                            } else {
                                                "Share Screen"
                                            },
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.toggle_screen_share();
                                }
                                ui.end_row();
                            });

                        if !self.voice_hint.is_empty() {
                            ui.add_space(6.0);
                            ui.label(RichText::new(&self.voice_hint).small().weak());
                        }
                    });

                ui.add_space(12.0);
                ui.label(RichText::new("PARTICIPANTS").small().strong());
                if self.participants.is_empty() {
                    ui.label(RichText::new("Nobody here yet").small().weak());
                }
                for participant in &self.participants {
                    ui.horizontal(|ui| {
                        ui.label(if participant.speaking { "🔈" } else { "👤" });
                        ui.label(&participant.username);
                        if participant.muted {
                            ui.label(RichText::new("muted").small().weak());
                        }
                        if participant.sharing_screen {
                            ui.label(RichText::new("sharing").small().weak());
                        }
                    });
                }

                ui.add_space(12.0);
                ui.label(RichText::new("STATUS").small().strong());
                ui.horizontal(|ui| {
                    for status in [
                        PresenceStatus::Online,
                        PresenceStatus::Away,
                        PresenceStatus::Dnd,
                    ] {
                        self::status_dot(ui, status);
                        if ui
                            .selectable_label(self.presence == status, status.label())
                            .clicked()
                        {
                            self.set_presence(status);
                        }
                    }
                });
            });
    }

    fn ui_chat(&mut self, ctx: &Context) {
        if self.in_voice {
            if !self.chat_collapsed {
                if self.chat_on_side {
                    egui::SidePanel::right("voice_chat_panel")
                        .resizable(true)
                        .default_width(288.0)
                        .show(ctx, |ui| self.chat_content(ui, true));
                } else {
                    egui::TopBottomPanel::bottom("voice_chat_panel")
                        .resizable(true)
                        .default_height(184.0)
                        .show(ctx, |ui| self.chat_content(ui, true));
                }
            }
            egui::CentralPanel::default().show(ctx, |ui| self.ui_voice_stage(ui));
        } else {
            egui::CentralPanel::default().show(ctx, |ui| self.chat_content(ui, false));
        }
    }

    fn chat_content(&mut self, ui: &mut Ui, compact: bool) {
        let title = self
            .channels
            .iter()
            .find(|c| self.selected_channel == Some(c.id))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Select a channel".to_string());
        ui.horizontal(|ui| {
            ui.heading(if compact {
                RichText::new(format!("Chat · {title}")).small().strong()
            } else {
                RichText::new(title).strong()
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if compact {
                    if ui
                        .small_button(if self.chat_on_side { "Bottom" } else { "Side" })
                        .clicked()
                    {
                        self.chat_on_side = !self.chat_on_side;
                    }
                    if ui.small_button("✕").on_hover_text("Hide chat").clicked() {
                        self.chat_collapsed = true;
                    }
                } else {
                    ui.label(RichText::new("Server-relayed • no P2P").small().weak());
                }
            });
        });
        ui.separator();

        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for index in 0..self.messages.len() {
                    let message = self.messages[index].clone();
                    self.ui_message(ui, &message, index, me_id);
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.draft)
                    .desired_width(ui.available_width() - 70.0)
                    .hint_text("Write a message...")
                    .id_source("message_draft"),
            );
            if ui.button("Send").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.send_message();
                response.request_focus();
            }
        });
    }

    fn ui_voice_stage(&mut self, ui: &mut Ui) {
        let label = self.voice_label.clone();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(&label).strong());
            ui.label(
                RichText::new(format!("{} participant(s)", self.participants.len()))
                    .small()
                    .weak(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(if self.chat_collapsed {
                        "Chat"
                    } else {
                        "Hide chat"
                    })
                    .clicked()
                {
                    self.chat_collapsed = !self.chat_collapsed;
                }
                ui.label(RichText::new("Server-relayed • no P2P").small().weak());
            });
        });
        ui.separator();

        let time = ui.input(|i| i.time);
        let pulse = ((time * 3.0).sin().mul_add(0.5, 0.5)) as f32;

        if self.screen_texture.is_some() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Viewer volume").small());
                let before = self.viewer_volume;
                ui.add(
                    egui::Slider::new(&mut self.viewer_volume, 0.0..=2.0)
                        .show_value(false)
                        .text("volume"),
                );
                if (before - self.viewer_volume).abs() > f32::EPSILON {
                    native::set_output_volume(self.viewer_volume);
                    if let Some(audio) = &self.audio {
                        audio.set_volume(self.viewer_volume);
                    }
                }
            });
            self.ui_screen_stage(ui);

            ui.label(RichText::new("IN CALL").small().strong());
            egui::Grid::new("compact_participants")
                .num_columns(4)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (index, participant) in self.participants.iter().enumerate() {
                        self.ui_participant_card(ui, participant, pulse, true);
                        if index % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        } else {
            ui.add_space(12.0);
            let columns = 4usize;
            egui::Grid::new("participant_grid")
                .num_columns(columns)
                .spacing([14.0, 14.0])
                .show(ui, |ui| {
                    for (index, participant) in self.participants.clone().iter().enumerate() {
                        self.ui_participant_card(ui, participant, pulse, false);
                        if index % columns == columns - 1 {
                            ui.end_row();
                        }
                    }
                });
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let button_width = 112.0;
                let mute_label = if self.muted { "Unmute" } else { "Mute" };
                if ui
                    .add_sized(
                        [button_width, 32.0],
                        egui::SelectableLabel::new(self.muted, mute_label),
                    )
                    .clicked()
                {
                    self.toggle_mute();
                }
                let deafen_label = if self.deafened { "Undeafen" } else { "Deafen" };
                if ui
                    .add_sized(
                        [button_width, 32.0],
                        egui::SelectableLabel::new(self.deafened, deafen_label),
                    )
                    .clicked()
                {
                    self.toggle_deafen();
                }
                let share_label = if self.sharing {
                    "Stop Share"
                } else {
                    "Share Screen"
                };
                if ui
                    .add_sized(
                        [button_width, 32.0],
                        egui::SelectableLabel::new(self.sharing, share_label),
                    )
                    .clicked()
                {
                    self.toggle_screen_share();
                }
                let disconnect =
                    egui::Button::new(RichText::new("Disconnect").strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(0xD9, 0x3B, 0x3B));
                if ui.add_sized([button_width, 32.0], disconnect).clicked() {
                    self.leave_voice();
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new(&self.voice_hint).small().weak());
        });
    }

    fn ui_participant_card(
        &self,
        ui: &mut Ui,
        participant: &VoiceParticipant,
        pulse: f32,
        compact: bool,
    ) {
        egui::Frame::new()
            .fill(if ui.visuals().dark_mode {
                Color32::from_rgb(0x1B, 0x25, 0x31)
            } else {
                Color32::from_rgb(0xFF, 0xFF, 0xFF)
            })
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let size = if compact { 28.0 } else { 38.0 };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
                    if participant.speaking {
                        let color = Color32::from_rgba_premultiplied(
                            0x30,
                            0xD1,
                            0x58,
                            (90.0 + 140.0 * pulse) as u8,
                        );
                        ui.painter().circle_stroke(
                            rect.center(),
                            size / 2.0,
                            Stroke::new(2.0 + 2.0 * pulse, color),
                        );
                    }
                    let initial = participant
                        .username
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase()
                        .to_string();
                    ui.painter().circle_filled(
                        rect.center(),
                        size / 2.0 - 3.0,
                        avatar_color(&participant.username, false),
                    );
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        initial,
                        FontId::proportional(size * 0.42),
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&participant.username).strong());
                        let icons = if participant.muted {
                            "🎤 muted"
                        } else {
                            "🎤 open"
                        };
                        ui.label(
                            RichText::new(if participant.deafened {
                                format!("{icons} · deafened")
                            } else {
                                icons.to_string()
                            })
                            .small()
                            .weak(),
                        );
                        if participant.sharing_screen {
                            ui.label(RichText::new("sharing screen").small().weak());
                        }
                    });
                });
            });
    }

    fn ui_screen_stage(&mut self, ui: &mut Ui) {
        let Some(texture) = &self.screen_texture else {
            return;
        };
        let texture_id = texture.id();
        let (source_w, source_h) = self.screen_size;
        let owner = self
            .screen_owner
            .and_then(|id| {
                self.participants
                    .iter()
                    .find(|p| p.user_id == id)
                    .map(|p| p.username.clone())
            })
            .unwrap_or_else(|| "Participant".to_string());

        let stage_width = ui.available_width().min(900.0).max(240.0);
        let stage = egui::vec2(stage_width, stage_width * 9.0 / 16.0);
        let aspect = (source_w.max(1) as f32 / source_h.max(1) as f32).max(0.1);
        let image_width = stage.x.min(stage.y * aspect);
        let image_height = image_width / aspect;

        egui::Frame::new()
            .fill(Color32::from_rgb(0x0A, 0x10, 0x18))
            .corner_radius(10.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{owner} is sharing")).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("⛶ Fullscreen").clicked() {
                            self.screen_fullscreen = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(((stage.x - image_width) / 2.0).max(0.0));
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            texture_id,
                            egui::vec2(image_width, image_height),
                        ))
                        .fit_to_exact_size(egui::vec2(image_width, image_height)),
                    );
                });
            });
        ui.add_space(8.0);
    }

    fn ui_screen_fullscreen(&mut self, ctx: &Context) {
        if !self.screen_fullscreen {
            return;
        }
        let Some(texture) = &self.screen_texture else {
            self.screen_fullscreen = false;
            return;
        };
        let texture_id = texture.id();
        let (source_w, source_h) = self.screen_size;
        let aspect = (source_w.max(1) as f32 / source_h.max(1) as f32).max(0.1);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.screen_fullscreen = false;
            return;
        }

        let mut open = self.screen_fullscreen;
        let mut close_requested = false;
        egui::Window::new("Skypeline — shared screen")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .default_size(ctx.screen_rect().size())
            .frame(egui::Frame::new().fill(Color32::BLACK).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Shared screen").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exit fullscreen (Esc)").clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.add_space(8.0);
                let available = ui.available_size();
                let width = available.x.min(available.y * aspect);
                let height = width / aspect;
                ui.horizontal(|ui| {
                    ui.add_space(((available.x - width) / 2.0).max(0.0));
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            texture_id,
                            egui::vec2(width, height),
                        ))
                        .fit_to_exact_size(egui::vec2(width, height)),
                    );
                });
            });
        if close_requested {
            open = false;
        }
        self.screen_fullscreen = open;
    }

    fn ui_message(&mut self, ui: &mut Ui, message: &MessageDto, index: usize, me_id: i64) {
        let ctx = ui.ctx().clone();
        let grouped = index > 0
            && self.messages[index - 1].user_id == message.user_id
            && message
                .created_at
                .saturating_sub(self.messages[index - 1].created_at)
                <= 120;
        let mine = message.user_id == me_id;
        let time = chrono_like_time(message.created_at);
        let avatar_hash = if mine {
            self.me.as_ref().and_then(|m| m.avatar_hash.clone())
        } else {
            self.friends
                .iter()
                .find(|f| f.user_id == message.user_id)
                .and_then(|f| f.avatar_hash.clone())
        };
        let fill = if ui.visuals().dark_mode {
            if mine {
                Color32::from_rgb(0x14, 0x3B, 0x5E)
            } else {
                Color32::from_rgb(0x1B, 0x25, 0x31)
            }
        } else if mine {
            Color32::from_rgb(0xD9, 0xEC, 0xFF)
        } else {
            Color32::from_rgb(0xFF, 0xFF, 0xFF)
        };

        let response = egui::Frame::new()
            .fill(fill)
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .outer_margin(egui::Margin {
                left: if grouped { 42 } else { 0 },
                right: 0,
                top: if grouped { 2 } else { 8 },
                bottom: 0,
            })
            .show(ui, |ui| {
                if grouped {
                    ui.label(&message.content);
                } else {
                    ui.horizontal_top(|ui| {
                        self.ui_person_avatar(
                            ui,
                            &ctx,
                            message.user_id,
                            &message.username,
                            mine,
                            avatar_hash.as_deref(),
                        );
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            let name_color = if mine {
                                Color32::from_rgb(0x2E, 0x9C, 0xF0)
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&message.username).strong().color(name_color),
                                );
                                ui.label(RichText::new(&time).small().weak());
                            });
                            ui.label(&message.content);
                        });
                    });
                }
            })
            .response;
        response.on_hover_text(format!("{} · {}", message.username, time));
    }

    fn ui_person_avatar(
        &mut self,
        ui: &mut Ui,
        ctx: &Context,
        user_id: i64,
        name: &str,
        mine: bool,
        hash: Option<&str>,
    ) {
        let api = self.api();
        let token = self.token.clone();
        let tx = self.events_tx.clone();
        if let Some(texture) = self
            .avatars
            .texture(ctx, &api, token.as_deref(), user_id, hash, &tx)
        {
            ui.add(
                egui::Image::new(egui::load::SizedTexture::new(
                    texture.id(),
                    egui::vec2(30.0, 30.0),
                ))
                .fit_to_exact_size(egui::vec2(30.0, 30.0))
                .corner_radius(15.0),
            );
        } else {
            ui_avatar(ui, name, mine);
        }
    }

    fn ui_settings(&mut self, ctx: &Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        let hosted_url = self.config.base_url();
        let me = self.me.clone();
        let me_id = me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_name = me.as_ref().map(|m| m.username.clone()).unwrap_or_default();
        let me_hash = me.as_ref().and_then(|m| m.avatar_hash.clone());
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Hosted service");
                ui.hyperlink_to(hosted_url.clone(), hosted_url);
                ui.label(
                    RichText::new("Skypeline always connects to the hosted relay.")
                        .small()
                        .weak(),
                );

                ui.add_space(8.0);
                ui.label(RichText::new("Profile picture").small().strong());
                ui.horizontal(|ui| {
                    self.ui_person_avatar(ui, ctx, me_id, &me_name, true, me_hash.as_deref());
                    ui.vertical(|ui| {
                        if ui.button("Choose image...").clicked() {
                            if let Some((bytes, filename)) = avatar::pick_avatar_file() {
                                match avatar::process_avatar_bytes(&bytes, &filename) {
                                    Ok((jpeg, mime, hash)) => {
                                        self.upload_avatar(jpeg, mime, hash);
                                        self.notice = "Uploading profile picture...".into();
                                    }
                                    Err(error) => self.notice = error,
                                }
                            }
                        }
                        ui.label(
                            RichText::new("PNG / JPG / WebP · 128x128 · under 15 KB")
                                .small()
                                .weak(),
                        );
                    });
                });

                ui.add_space(8.0);
                let mut dark = self.config.dark_mode;
                if ui.checkbox(&mut dark, "Dark mode").changed() {
                    self.config.dark_mode = dark;
                    self.config.save();
                    theme::apply(ctx, dark);
                }

                ui.separator();
                if ui.button("Sign out").clicked() {
                    self.sign_out();
                }
            });
        self.settings_open = open;
    }
}

fn status_color(status: PresenceStatus) -> Color32 {
    match status {
        PresenceStatus::Online => Color32::from_rgb(0x30, 0xD1, 0x58),
        PresenceStatus::Away => Color32::from_rgb(0xFF, 0xD6, 0x0A),
        PresenceStatus::Dnd => Color32::from_rgb(0xFF, 0x45, 0x3A),
        PresenceStatus::Offline => Color32::from_gray(0x80),
    }
}

fn status_dot(ui: &mut Ui, status: PresenceStatus) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 4.5, status_color(status));
}

fn ui_badge(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x00, 0x68, 0xC8))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(Color32::WHITE));
        });
}

fn avatar_color(name: &str, mine: bool) -> Color32 {
    if mine {
        return Color32::from_rgb(0x00, 0x68, 0xC8);
    }
    let hash = name.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    });
    match hash % 6 {
        0 => Color32::from_rgb(0x6C, 0x4F, 0xC1),
        1 => Color32::from_rgb(0x00, 0x8C, 0x82),
        2 => Color32::from_rgb(0xB3, 0x5E, 0x00),
        3 => Color32::from_rgb(0xC2, 0x18, 0x5A),
        4 => Color32::from_rgb(0x2E, 0x7D, 0xB2),
        _ => Color32::from_rgb(0x4C, 0x7C, 0x28),
    }
}

fn ui_avatar(ui: &mut Ui, name: &str, mine: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 14.0, avatar_color(name, mine));
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

impl eframe::App for SkyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);
        match self.page {
            Page::Login => self.ui_login(ctx),
            Page::Main => self.ui_main(ctx),
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn chrono_like_time(timestamp: i64) -> String {
    let secs = timestamp.rem_euclid(86_400);
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}
