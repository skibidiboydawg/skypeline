//! Left navigation sidebar: Friends & Calls, group creation, channel list, and user bar.

use super::{avatar_color, status_color, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("left_sidebar")
            .exact_width(280.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_SIDEBAR)
                    .stroke(theme::border())
                    .inner_margin(egui::Margin::symmetric(14, 14)),
            )
            .show(ctx, |ui| {
                let home = self.selected_channel.is_none();
                let pending_count = self
                    .friends
                    .iter()
                    .filter(|f| {
                        f.status == skypeline_shared::FriendshipStatus::Pending
                            && f.direction == "incoming"
                    })
                    .count();
                let home_label = if pending_count > 0 {
                    format!("Friends & Calls ({pending_count})")
                } else {
                    "Friends & Calls".to_string()
                };
                if ui
                    .add_sized(
                        [ui.available_width(), 38.0],
                        egui::Button::new(RichText::new(home_label).strong())
                            .fill(if home {
                                theme::HOVER
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(theme::border())
                            .corner_radius(10.0),
                    )
                    .clicked()
                {
                    self.selected_channel = None;
                    self.messages.clear();
                }
                ui.add_space(8.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new("+ New Group")
                            .fill(if self.show_create_group {
                                theme::HOVER
                            } else {
                                theme::SURFACE_CARD
                            })
                            .stroke(theme::border())
                            .corner_radius(10.0),
                    )
                    .clicked()
                {
                    self.show_create_group = !self.show_create_group;
                }
                if self.show_create_group {
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(theme::SURFACE_CARD)
                        .stroke(theme::border())
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            Self::form_field(ui, &mut self.group_name, "Group name", false);
                            ui.add_space(8.0);
                            if ui
                                .add_enabled(
                                    !self.busy,
                                    egui::Button::new("Create group")
                                        .fill(theme::ACCENT)
                                        .corner_radius(8.0),
                                )
                                .clicked()
                            {
                                self.create_group();
                            }
                        });
                }
                ui.add_space(10.0);
                Self::form_field(ui, &mut self.chat_search, "Search chats...", false);
                ui.add_space(12.0);
                ui.label(
                    theme::muted("DIRECT MESSAGES & GROUP CHATS")
                        .small()
                        .strong(),
                );
                ui.add_space(8.0);
                let search = self.chat_search.trim().to_ascii_lowercase();
                let mut channels: Vec<_> = self
                    .channels
                    .iter()
                    .filter(|c| c.kind == "dm" || c.kind == "group")
                    .cloned()
                    .collect();
                channels.sort_by(|a, b| b.id.cmp(&a.id));

                let footer_h = if self.in_voice { 150.0 } else { 96.0 };
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height((ui.available_height() - footer_h).max(80.0))
                    .show(ui, |ui| {
                        for ch in channels {
                            let label = Self::channel_label(&ch);
                            if !search.is_empty() && !label.to_ascii_lowercase().contains(&search) {
                                continue;
                            }
                            let selected = self.selected_channel == Some(ch.id);
                            let unread = self.unread.get(&ch.id).copied().unwrap_or(0);
                            let preview = self
                                .messages
                                .iter()
                                .rev()
                                .find(|m| m.channel_id == ch.id)
                                .map(|m| m.content.clone())
                                .unwrap_or_else(|| {
                                    if ch.kind == "group" {
                                        format!("{} members", ch.members.len())
                                    } else {
                                        "Direct message".into()
                                    }
                                });
                            let presence = ch
                                .peer_user_id
                                .map(|id| self.online_status_for(id))
                                .unwrap_or(PresenceStatus::Offline);
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 58.0),
                                Sense::click(),
                            );
                            ui.painter().rect(
                                rect,
                                10.0,
                                if selected {
                                    theme::HOVER
                                } else if resp.hovered() {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 10)
                                } else {
                                    Color32::TRANSPARENT
                                },
                                Stroke::NONE,
                                egui::StrokeKind::Inside,
                            );
                            let ar = Rect::from_min_size(
                                rect.min + egui::vec2(8.0, 9.0),
                                egui::vec2(40.0, 40.0),
                            );
                            self.paint_channel_avatar(ui, ar, &ch, presence);
                            ui.painter().text(
                                rect.min + egui::vec2(58.0, 12.0),
                                Align2::LEFT_TOP,
                                &label,
                                FontId::proportional(14.0),
                                theme::PRIMARY_TEXT,
                            );
                            let prev = if preview.chars().count() > 28 {
                                let mut t: String = preview.chars().take(28).collect();
                                t.push('…');
                                t
                            } else {
                                preview
                            };
                            ui.painter().text(
                                rect.min + egui::vec2(58.0, 32.0),
                                Align2::LEFT_TOP,
                                prev,
                                FontId::proportional(12.0),
                                theme::SECONDARY_TEXT,
                            );
                            if unread > 0 {
                                let br = Rect::from_center_size(
                                    rect.right_center() - egui::vec2(18.0, 0.0),
                                    egui::vec2(22.0, 22.0),
                                );
                                ui.painter().circle_filled(br.center(), 10.0, theme::ACCENT);
                                ui.painter().text(
                                    br.center(),
                                    Align2::CENTER_CENTER,
                                    unread.min(99).to_string(),
                                    FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }
                            if resp.clicked() {
                                self.select_channel(ch.id);
                            }
                        }
                    });

                if self.in_voice {
                    ui.add_space(6.0);
                    self.ui_active_call_banner(ui);
                }
                ui.separator();
                self.ui_user_bar(ui);
            });
    }

    fn ui_active_call_banner(&mut self, ui: &mut Ui) {
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        egui::Frame::new()
            .fill(Color32::from_rgb(0x0C, 0x35, 0x54))
            .stroke(Stroke::new(1.0_f32, theme::ACCENT))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Voice Connected")
                                .color(theme::SUCCESS)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{:02}:{:02}", elapsed / 60, elapsed % 60))
                                .small()
                                .color(theme::PRIMARY_TEXT),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("End").small())
                                    .fill(theme::DANGER)
                                    .corner_radius(6.0),
                            )
                            .clicked()
                        {
                            self.leave_voice();
                        }
                    });
                });
            });
    }

    fn paint_channel_avatar(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        channel: &ChannelInfo,
        presence: PresenceStatus,
    ) {
        let (name, hash, user_id) = if channel.kind == "dm" {
            let peer = channel.peer_user_id.unwrap_or(-1);
            let hash = self
                .friends
                .iter()
                .find(|f| f.user_id == peer)
                .and_then(|f| f.avatar_hash.clone());
            (channel.name.clone(), hash, peer)
        } else {
            (channel.name.clone(), None, -channel.id)
        };
        let mut drew = false;
        if let Some(hash) = hash.as_deref() {
            let tex = self.avatars.texture(
                &ui.ctx().clone(),
                &self.api(),
                self.token.as_deref(),
                user_id,
                Some(hash),
                &self.events_tx,
            );
            if let Some(t) = tex {
                ui.painter().image(
                    t.id(),
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                drew = true;
            }
        }
        if !drew {
            ui.painter().circle_filled(
                rect.center(),
                rect.width() / 2.0,
                avatar_color(&name, false),
            );
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
                FontId::proportional(15.0),
                Color32::WHITE,
            );
        }
        if channel.kind == "dm" {
            let dot = rect.right_bottom() - egui::vec2(5.0, 5.0);
            ui.painter().circle_filled(dot, 5.0, status_color(presence));
            ui.painter()
                .circle_stroke(dot, 5.0, Stroke::new(1.5_f32, theme::SURFACE_SIDEBAR));
        }
    }

    fn ui_user_bar(&mut self, ui: &mut Ui) {
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::hover());
            let mut drew = false;
            if let Some(h) = me_hash.as_deref() {
                let tex = self.avatars.texture(
                    &ui.ctx().clone(),
                    &self.api(),
                    self.token.as_deref(),
                    me_id,
                    Some(h),
                    &self.events_tx,
                );
                if let Some(t) = tex {
                    ui.painter().image(
                        t.id(),
                        rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    drew = true;
                }
            }
            if !drew {
                ui.painter()
                    .circle_filled(rect.center(), 16.0, avatar_color(&me_name, true));
                let initial = me_name
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
            ui.vertical(|ui| {
                ui.label(RichText::new(&me_name).strong().small());
                egui::ComboBox::from_id_salt("presence_combo")
                    .selected_text(self.presence.label())
                    .show_ui(ui, |ui| {
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
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Sign out").clicked() {
                    self.sign_out();
                }
            });
        });
    }
}
