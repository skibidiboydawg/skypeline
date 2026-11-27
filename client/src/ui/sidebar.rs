//! Far-left icon rail and channel drawer with segmented DMs/Groups and call dock.

use super::{avatar_color, status_color, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_app_rail(&mut self, ctx: &Context) {
        egui::SidePanel::left("app_rail")
            .exact_width(64.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_RAIL)
                    .stroke(theme::border())
                    .inner_margin(egui::Margin::symmetric(10, 14)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Brand monogram badge
                    let (brand_rect, _) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::hover());
                    ui.painter().rect_filled(brand_rect, 13.0, theme::ACCENT);
                    ui.painter().text(
                        brand_rect.center(),
                        Align2::CENTER_CENTER,
                        "W",
                        FontId::proportional(20.0),
                        Color32::WHITE,
                    );

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(12.0);

                    // Home / Friends button
                    let home = self.selected_channel.is_none();
                    let pending_count = self
                        .friends
                        .iter()
                        .filter(|f| {
                            f.status == FriendshipStatus::Pending && f.direction == "incoming"
                        })
                        .count();
                    let (home_rect, home_resp) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::click());
                    ui.painter().rect_filled(
                        home_rect,
                        12.0,
                        if home {
                            theme::ACCENT
                        } else if home_resp.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_CARD
                        },
                    );
                    ui.painter().text(
                        home_rect.center(),
                        Align2::CENTER_CENTER,
                        "Home",
                        FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    if pending_count > 0 {
                        let badge = home_rect.right_top() + egui::vec2(-4.0, 4.0);
                        ui.painter().circle_filled(badge, 6.0, theme::DANGER);
                    }
                    if home_resp.on_hover_text("Friends & Calls").clicked() {
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    ui.add_space(10.0);

                    // New Group button
                    let (grp_rect, grp_resp) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::click());
                    ui.painter().rect_filled(
                        grp_rect,
                        12.0,
                        if self.show_create_group {
                            theme::ACCENT
                        } else if grp_resp.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_CARD
                        },
                    );
                    ui.painter().text(
                        grp_rect.center(),
                        Align2::CENTER_CENTER,
                        "+",
                        FontId::proportional(20.0),
                        Color32::WHITE,
                    );
                    if grp_resp.on_hover_text("Create Group Chat").clicked() {
                        self.show_create_group = !self.show_create_group;
                    }

                    // Bottom controls: theme toggle + sign out
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let (out_rect, out_resp) =
                            ui.allocate_exact_size(egui::vec2(42.0, 34.0), Sense::click());
                        ui.painter().rect_filled(
                            out_rect,
                            10.0,
                            if out_resp.hovered() {
                                theme::DANGER
                            } else {
                                theme::SURFACE_CARD
                            },
                        );
                        ui.painter().text(
                            out_rect.center(),
                            Align2::CENTER_CENTER,
                            "Exit",
                            FontId::proportional(11.0),
                            theme::PRIMARY_TEXT,
                        );
                        if out_resp.on_hover_text("Sign out").clicked() {
                            self.sign_out();
                        }

                        ui.add_space(8.0);
                        let (thm_rect, thm_resp) =
                            ui.allocate_exact_size(egui::vec2(42.0, 34.0), Sense::click());
                        ui.painter().rect_filled(
                            thm_rect,
                            10.0,
                            if thm_resp.hovered() {
                                theme::HOVER
                            } else {
                                theme::SURFACE_CARD
                            },
                        );
                        ui.painter().text(
                            thm_rect.center(),
                            Align2::CENTER_CENTER,
                            if self.config.dark_mode {
                                "Light"
                            } else {
                                "Dark"
                            },
                            FontId::proportional(11.0),
                            theme::SECONDARY_TEXT,
                        );
                        if thm_resp.on_hover_text("Toggle theme").clicked() {
                            self.config.dark_mode = !self.config.dark_mode;
                            self.config.save();
                            theme::apply(ctx, self.config.dark_mode);
                        }
                    });
                });
            });
    }

    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("left_sidebar")
            .exact_width(248.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_SIDEBAR)
                    .stroke(theme::border())
                    .inner_margin(egui::Margin::symmetric(12, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Wireline").size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let dot_color = status_color(self.presence);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.5, dot_color);
                    });
                });
                ui.add_space(10.0);

                if self.show_create_group {
                    egui::Frame::new()
                        .fill(theme::SURFACE_CARD)
                        .stroke(theme::accent_border())
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(RichText::new("New Group").small().strong());
                            ui.add_space(4.0);
                            Self::form_field(ui, &mut self.group_name, "Group name", false);
                            ui.add_space(8.0);
                            if ui
                                .add_enabled(
                                    !self.busy,
                                    egui::Button::new("Create")
                                        .fill(theme::ACCENT)
                                        .corner_radius(8.0),
                                )
                                .clicked()
                            {
                                self.create_group();
                            }
                        });
                    ui.add_space(10.0);
                }

                Self::form_field(ui, &mut self.chat_search, "Search chats...", false);
                ui.add_space(12.0);

                let search = self.chat_search.trim().to_ascii_lowercase();
                let mut dms: Vec<ChannelInfo> = self
                    .channels
                    .iter()
                    .filter(|c| c.kind == "dm")
                    .cloned()
                    .collect();
                let mut groups: Vec<ChannelInfo> = self
                    .channels
                    .iter()
                    .filter(|c| c.kind == "group")
                    .cloned()
                    .collect();
                dms.sort_by(|a, b| b.id.cmp(&a.id));
                groups.sort_by(|a, b| b.id.cmp(&a.id));

                let footer_h = if self.in_voice { 136.0 } else { 68.0 };
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height((ui.available_height() - footer_h).max(80.0))
                    .show(ui, |ui| {
                        ui.label(theme::muted("DIRECT MESSAGES").small().strong());
                        ui.add_space(4.0);
                        for ch in dms {
                            if !search.is_empty() && !ch.name.to_ascii_lowercase().contains(&search)
                            {
                                continue;
                            }
                            self.ui_channel_row(ui, &ch);
                        }
                        if !groups.is_empty() {
                            ui.add_space(10.0);
                            ui.label(theme::muted("GROUPS").small().strong());
                            ui.add_space(4.0);
                            for ch in groups {
                                if !search.is_empty()
                                    && !ch.name.to_ascii_lowercase().contains(&search)
                                {
                                    continue;
                                }
                                self.ui_channel_row(ui, &ch);
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

    fn ui_channel_row(&mut self, ui: &mut Ui, ch: &ChannelInfo) {
        let selected = self.selected_channel == Some(ch.id);
        let unread = self.unread.get(&ch.id).copied().unwrap_or(0);
        let presence = ch
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);
        let subtitle = if ch.kind == "group" {
            format!("{} members", ch.members.len())
        } else {
            presence.label().to_string()
        };

        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::click());
        let bg = if selected {
            theme::ACCENT_SOFT
        } else if resp.hovered() {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect(
            rect,
            10.0,
            bg,
            if selected {
                theme::accent_border()
            } else {
                Stroke::NONE
            },
            egui::StrokeKind::Inside,
        );
        let ar = Rect::from_min_size(rect.min + egui::vec2(8.0, 7.0), egui::vec2(34.0, 34.0));
        self.paint_channel_avatar(ui, ar, ch, presence);
        ui.painter().text(
            rect.min + egui::vec2(50.0, 9.0),
            Align2::LEFT_TOP,
            &ch.name,
            FontId::proportional(13.5),
            if selected {
                Color32::WHITE
            } else {
                theme::PRIMARY_TEXT
            },
        );
        ui.painter().text(
            rect.min + egui::vec2(50.0, 27.0),
            Align2::LEFT_TOP,
            subtitle,
            FontId::proportional(11.5),
            theme::MUTED_TEXT,
        );
        if unread > 0 {
            let br = Rect::from_center_size(
                rect.right_center() - egui::vec2(16.0, 0.0),
                egui::vec2(20.0, 20.0),
            );
            ui.painter().circle_filled(br.center(), 9.5, theme::ACCENT);
            ui.painter().text(
                br.center(),
                Align2::CENTER_CENTER,
                unread.min(99).to_string(),
                FontId::proportional(10.5),
                Color32::WHITE,
            );
        }
        if resp.clicked() {
            self.select_channel(ch.id);
        }
    }

    fn ui_active_call_banner(&mut self, ui: &mut Ui) {
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        egui::Frame::new()
            .fill(theme::SURFACE_CARD)
            .stroke(Stroke::new(1.0_f32, theme::SUCCESS))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 7))
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
                                .color(theme::SECONDARY_TEXT),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("End").small().color(Color32::WHITE))
                                    .fill(theme::DANGER)
                                    .corner_radius(7.0),
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
                FontId::proportional(14.0),
                Color32::WHITE,
            );
        }
        if channel.kind == "dm" {
            let dot = rect.right_bottom() - egui::vec2(4.0, 4.0);
            ui.painter().circle_filled(dot, 4.5, status_color(presence));
            ui.painter()
                .circle_stroke(dot, 4.5, Stroke::new(1.5_f32, theme::SURFACE_SIDEBAR));
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
            let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
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
                    .circle_filled(rect.center(), 15.0, avatar_color(&me_name, true));
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
        });
    }
}
