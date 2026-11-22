//! egui application: login/register, settings, channels, DMs, voice and
//! server-relayed screen sharing.

use crate::api::ApiClient;
use crate::audio::AudioEngine;
use crate::config::{Config, LOCAL_SERVER_URL};
use crate::events::UiEvent;
use crate::net;
use crate::screen::ScreenShare;
use crate::{native, theme};
use eframe::egui;
use egui::{Color32, Context, RichText, TextureOptions, Ui};
use skypeline_shared::{
    AuthResponse, ChannelInfo, LoginRequest, MessageDto, OnlineUser, PresenceStatus,
    RegisterRequest, VoiceParticipant, WsClientMsg,
};
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

pub struct SkyApp {
    rt: tokio::runtime::Runtime,
    config: Config,
    events_tx: mpsc::Sender<UiEvent>,
    events_rx: mpsc::Receiver<UiEvent>,

    page: Page,
    auth_tab: AuthTab,
    login_email: String,
    login_password: String,
    register_email: String,
    register_username: String,
    register_password: String,
    busy: bool,
    notice: String,

    token: Option<String>,
    me: Option<AuthResponse>,
    channels: Vec<ChannelInfo>,
    online: Vec<OnlineUser>,
    selected_channel: Option<i64>,
    messages: Vec<MessageDto>,
    draft: String,
    presence: PresenceStatus,
    settings_open: bool,
    settings_url: String,

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
}

impl SkyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        theme::apply(&_cc.egui_ctx, config.dark_mode);
        let (events_tx, events_rx) = mpsc::channel();
        let settings_url = config.base_url();

        Self {
            rt: tokio::runtime::Runtime::new().expect("create tokio runtime"),
            config,
            events_tx,
            events_rx,
            page: Page::Login,
            auth_tab: AuthTab::Login,
            login_email: String::new(),
            login_password: String::new(),
            register_email: String::new(),
            register_username: String::new(),
            register_password: String::new(),
            busy: false,
            notice: format!("Server: {}", settings_url),
            token: None,
            me: None,
            channels: Vec::new(),
            online: Vec::new(),
            selected_channel: None,
            messages: Vec::new(),
            draft: String::new(),
            presence: PresenceStatus::Online,
            settings_open: false,
            settings_url,
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
        let Some(token) = self.token.clone() else { return };
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

    fn load_messages(&self, channel_id: i64) {
        let Some(token) = self.token.clone() else { return };
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

    fn open_dm(&self, user_id: i64) {
        let Some(token) = self.token.clone() else { return };
        let api = self.api();
        let tx = self.events_tx.clone();
        self.rt.spawn(async move {
            match api.open_dm(&token, user_id).await {
                Ok(channel) => {
                    let _ = tx.send(UiEvent::Channels(vec![channel]));
                }
                Err(e) => {
                    let _ = tx.send(UiEvent::Error(e));
                }
            }
        });
    }

    fn send_message(&mut self) {
        let content = self.draft.trim().to_string();
        let Some(channel_id) = self.selected_channel else { return };
        if content.is_empty() {
            return;
        }
        if let Some(tx) = &self.ws_tx {
            let _ = tx.send(WsClientMsg::SendMessage { channel_id, content });
            self.draft.clear();
        } else {
            self.notice = "Not connected to the relay".to_string();
        }
    }

    fn join_voice(&mut self) {
        let ws = self.ws_tx.clone();
        let Some(ws) = ws else {
            self.notice = "Not connected to the relay".into();
            return;
        };
        let _ = ws.send(WsClientMsg::JoinVoice {
            room: VOICE_ROOM.to_string(),
        });
        let audio = self.audio.get_or_insert_with(AudioEngine::start);
        audio.set_muted(self.muted);
        audio.set_deafened(self.deafened);
        match audio.start_capture(VOICE_ROOM.to_string(), ws.clone()) {
            Ok(()) => {
                self.in_voice = true;
                self.notice = "Joined General Voice — all audio is server-relayed".into();
            }
            Err(e) => self.notice = e,
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
            self.notice = "Join voice before sharing your screen".into();
            return;
        }
        if let Some(share) = self.screen_share.take() {
            share.stop();
            self.sharing = false;
            self.notice = "Stopped sharing your screen".into();
            return;
        }
        if let Some(ws) = &self.ws_tx {
            self.screen_share = Some(ScreenShare::start(VOICE_ROOM.to_string(), ws.clone()));
            self.sharing = true;
            self.notice = "Sharing screen through the relay server".into();
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
        self.channels.clear();
        self.messages.clear();
        self.online.clear();
        self.selected_channel = None;
        self.settings_open = false;
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
                }
                UiEvent::Channels(channels) => {
                    for channel in channels {
                        if !self.channels.iter().any(|c| c.id == channel.id) {
                            self.channels.push(channel);
                        } else if let Some(existing) =
                            self.channels.iter_mut().find(|c| c.id == channel.id)
                        {
                            *existing = channel;
                        }
                    }
                    if self.selected_channel.is_none() {
                        if let Some(channel) = self.channels.iter().find(|c| c.kind == "text") {
                            self.selected_channel = Some(channel.id);
                            self.load_messages(channel.id);
                        }
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
                    if selected {
                        self.messages.push(message);
                    } else if message.user_id != self.me.as_ref().map(|m| m.user_id).unwrap_or(-1) {
                        let channel_name = self
                            .channels
                            .iter()
                            .find(|c| c.id == message.channel_id)
                            .map(|c| c.name.as_str())
                            .unwrap_or("Skypeline");
                        native::beep();
                        native::notify(
                            "Skypeline",
                            &format!("{} in {}: {}", message.username, channel_name, message.content),
                        );
                    }
                }
                UiEvent::Presence(users) => self.online = users,
                UiEvent::VoiceState(participants) => {
                    self.participants = participants;
                    if let Some(owner) = self.screen_owner {
                        if !self.participants.iter().any(|p| p.user_id == owner && p.sharing_screen) {
                            self.screen_texture = None;
                            self.screen_owner = None;
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
                    }
                }
                UiEvent::Status(status) => self.notice = status,
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
                }
                UiEvent::Reconnected => {
                    self.notice = "Reconnected to relay".into();
                    if let Some(ws) = &self.ws_tx {
                        let _ = ws.send(WsClientMsg::SetPresence { status: self.presence });
                        if self.in_voice {
                            let _ = ws.send(WsClientMsg::JoinVoice {
                                room: VOICE_ROOM.to_string(),
                            });
                            let _ = ws.send(WsClientMsg::SetVoiceState {
                                muted: self.muted,
                                deafened: self.deafened,
                                speaking: false,
                            });
                            if self.sharing {
                                let _ = ws.send(WsClientMsg::SetScreenShare {
                                    room: VOICE_ROOM.to_string(),
                                    active: true,
                                });
                            }
                        }
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

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.auth_tab, AuthTab::Login, "Sign in");
                    ui.selectable_value(&mut self.auth_tab, AuthTab::Register, "Create account");
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
                        if ui.add(egui::Button::new("Sign in").min_size([180.0, 34.0].into())).clicked()
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
                        ui.label("Password (6+ characters)");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.register_password)
                                .password(true)
                                .desired_width(280.0)
                                .hint_text("Password"),
                        );
                        ui.add_space(8.0);
                        if ui
                            .add(egui::Button::new("Create account").min_size([180.0, 34.0].into()))
                            .clicked()
                            && !self.busy
                        {
                            self.submit_register();
                        }
                    }
                }

                ui.add_space(14.0);
                self.ui_notice(ui);
                ui.add_space(20.0);
                ui.separator();
                ui.label("Server URL (editable in Settings)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_url).desired_width(360.0),
                );
                ui.hyperlink_to("Hugging Face Space", "https://huggingface.co/spaces/Rezixyzz/Co");
                ui.label(format!("Local testing: {LOCAL_SERVER_URL}"));
                if ui.small_button("Use local server").clicked() {
                    self.settings_url = LOCAL_SERVER_URL.to_string();
                    self.apply_server_url();
                }
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
                    let _ = tx.send(UiEvent::SignedIn(response));
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
        egui::TopBottomPanel::top("skypeline_top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Skypeline");
                ui.separator();
                if let Some(me) = &self.me {
                    ui.label(&me.username);
                }
                ui.label(self.presence.label());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Settings").clicked() {
                        self.settings_open = true;
                        self.settings_url = self.config.base_url();
                    }
                    if ui.button("Sign out").clicked() {
                        self.sign_out();
                    }
                    ui.label(RichText::new(&self.notice).small().weak());
                });
            });
        });

        egui::SidePanel::left("channels_panel")
            .resizable(true)
            .default_width(210.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("CHANNELS").small().strong());
                for channel in self.channels.clone() {
                    let selected = self.selected_channel == Some(channel.id);
                    let icon = match channel.kind.as_str() {
                        "voice" => "🔊",
                        "dm" => "@",
                        _ => "#",
                    };
                    if ui
                        .selectable_label(selected, format!("{icon} {}", channel.name))
                        .clicked()
                        && channel.kind != "voice"
                    {
                        self.selected_channel = Some(channel.id);
                        self.load_messages(channel.id);
                    }
                }

                ui.add_space(14.0);
                ui.label(RichText::new("ONLINE").small().strong());
                let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
                for user in self.online.clone() {
                    if user.user_id == me_id {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(0x55, 0xC8, 0x8A), "●");
                        if ui.link(&user.username).clicked() {
                            self.open_dm(user.user_id);
                        }
                        ui.label(RichText::new(user.status.label()).small().weak());
                    });
                }
            });

        egui::SidePanel::right("voice_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("VOICE — GENERAL VOICE").small().strong());
                if !self.in_voice {
                    if ui.button("Join voice").clicked() {
                        self.join_voice();
                    }
                } else if ui.button("Leave voice").clicked() {
                    self.leave_voice();
                }

                ui.horizontal(|ui| {
                    if ui.selectable_label(self.muted, "🎤 Mute").clicked() {
                        self.toggle_mute();
                    }
                    if ui.selectable_label(self.deafened, "🔇 Deafen").clicked() {
                        self.toggle_deafen();
                    }
                });
                if ui
                    .selectable_label(self.sharing, "🖥 Share screen")
                    .clicked()
                {
                    self.toggle_screen_share();
                }

                ui.add_space(10.0);
                ui.label(RichText::new("PARTICIPANTS").small().strong());
                for participant in &self.participants {
                    ui.horizontal(|ui| {
                        ui.label(if participant.speaking { "🔈" } else { "👤" });
                        ui.label(&participant.username);
                        if participant.muted {
                            ui.label("(muted)");
                        }
                        if participant.sharing_screen {
                            ui.label("(sharing)");
                        }
                    });
                }

                ui.add_space(10.0);
                ui.label(RichText::new("STATUS").small().strong());
                ui.horizontal(|ui| {
                    for status in [
                        PresenceStatus::Online,
                        PresenceStatus::Away,
                        PresenceStatus::Dnd,
                    ] {
                        if ui
                            .selectable_label(self.presence == status, status.label())
                            .clicked()
                        {
                            self.set_presence(status);
                        }
                    }
                });

                if let Some(texture) = &self.screen_texture {
                    ui.add_space(10.0);
                    ui.label(RichText::new("SHARED SCREEN (relayed)").small().strong());
                    let available = ui.available_width().max(160.0);
                    let (w, h) = self.screen_size;
                    let scale = (available / w.max(1) as f32).min(1.0);
                    let size = egui::vec2(w as f32 * scale, h as f32 * scale);
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(texture.id(), size))
                            .fit_to_exact_size(size),
                    );
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let title = self
                .channels
                .iter()
                .find(|c| self.selected_channel == Some(c.id))
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Select a channel".to_string());
            ui.horizontal(|ui| {
                ui.heading(&title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Server-relayed • no P2P").small().weak());
                });
            });
            ui.separator();

            let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for message in &self.messages {
                        ui.horizontal_wrapped(|ui| {
                            let mine = message.user_id == me_id;
                            let color = if mine {
                                Color32::from_rgb(0x5A, 0xB4, 0xFF)
                            } else {
                                ui.visuals().text_color()
                            };
                            ui.label(RichText::new(&message.username).strong().color(color));
                            ui.label(
                                RichText::new(format!(
                                    "{}",
                                    chrono_like_time(message.created_at)
                                ))
                                .small()
                                .weak(),
                            );
                        });
                        ui.label(&message.content);
                        ui.add_space(6.0);
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
        });

        self.ui_settings(ctx);
    }

    fn ui_settings(&mut self, ctx: &Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Server URL");
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings_url).desired_width(340.0),
                );
                ui.label(
                    RichText::new(format!(
                        "Default: https://rezixyzz-co.hf.space\nLocal testing: {LOCAL_SERVER_URL}"
                    ))
                    .small()
                    .weak(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Use Hugging Face Space").clicked() {
                        self.settings_url = "https://rezixyzz-co.hf.space".into();
                    }
                    if ui.button("Use local server").clicked() {
                        self.settings_url = LOCAL_SERVER_URL.into();
                    }
                });

                ui.add_space(8.0);
                let mut dark = self.config.dark_mode;
                if ui.checkbox(&mut dark, "Dark mode").changed() {
                    self.config.dark_mode = dark;
                    self.config.save();
                    theme::apply(ctx, dark);
                }

                ui.add_space(8.0);
                if ui.button("Apply & reconnect").clicked() {
                    self.apply_server_url();
                }
                ui.separator();
                if ui.button("Sign out").clicked() {
                    self.sign_out();
                }
            });
        self.settings_open = open;
    }

    fn apply_server_url(&mut self) {
        let url = self.settings_url.trim();
        if url.is_empty() {
            self.notice = "Server URL cannot be empty".into();
            return;
        }
        self.config.server_url = url.to_string();
        self.config.save();
        self.settings_url = self.config.base_url();
        self.notice = format!("Server set to {}", self.settings_url);
        if let Some(token) = self.token.clone() {
            self.connect_ws(&token);
            self.load_channels();
        }
    }
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
