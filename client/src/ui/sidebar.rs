//! Clean 2-pane left sidebar: Friends & Calls, Direct Messages, active call dock, and user bar.

use super::{avatar_color, status_color, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("left_sidebar")
            .exact_width(268.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_SIDEBAR)
                    .stroke(theme::border())
                    .inner_margin(egui::Margin::symmetric(14, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (brand_rect, _) =
                        ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
                    ui.painter().rect_filled(brand_rect, 8.0, theme::ACCENT);
                    ui.painter().text(
                        brand_rect.center(),
                        Align2::CENTER_CENTER,
                        "W",
                        FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("Wireline").size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if self.config.dark_mode {
                                        "Light"
                                    } else {
                                        "Dark"
                                    })
                                    .small()
                                    .color(theme::SECONDARY_TEXT),
                                )
                                .fill(theme::SURFACE_CARD)
                                .corner_radius(8.0),
                            )
                            .clicked()
                        {
                            self.config.dark_mode = !self.config.dark_mode;
                            self.config.save();
                            theme::apply(ctx, self.config.dark_mode);
                        }
                    });
                });
                ui.add_space(12.0);

                let home = self.selected_channel.is_none();
                let pending_count = self
                    .friends
                    .iter()
                    .filter(|f| {
                        f.status == FriendshipStatus::Pending && f.direction == "incoming"
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
                        egui::Button::new(RichText::new(home_label).strong().color(if home {
                            Color32::WHITE
                        } else {
                            theme::PRIMARY_TEXT
                        }))
                        .fill(if home {
                            theme::ACCENT
                        } else {
                            theme::SURFACE_CARD
                        })
                        .stroke(if home { Stroke::NONE } else { theme::border() })
                        .corner_radius(10.0),
                    )
                    .clicked()
                {
                    self.selected_channel = None;
                    self.messages.clear();
                }

                ui.add_space(10.0);
                Self::form_field(ui, &mut self.chat_search, "Search people...", false);
                ui.add_space(12.0);
                ui.label(theme::muted("DIRECT MESSAGES").small().strong());
                ui.add_space(6.0);

                let search = self.chat_search.trim().to_ascii_lowercase();
                let mut dms: Vec<ChannelInfo> = self
                    .channels
                    .iter()
                    .filter(|c| c.kind == "dm")
                    .cloned()
                    .collect();
                dms.sort_by(|a, b| b.id.cmp(&a.id));

                let footer_h = if self.in_voice { 138.0 } else { 70.0 };
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height((ui.available_height() - footer_h).max(80.0))
                    .show(ui, |ui| {
                        if dms.is_empty() {
                            ui.add_space(8.0);
                            ui.label(theme::muted("No conversations yet.").small());
                        }
                        for ch in dms {
                            if !search.is_empty() && !ch.name.to_ascii_lowercase().contains(&search)
                            {
                                continue;
                            }
                            self.ui_channel_row(ui, &ch);
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
            presence.label(),
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
        let peer = channel.peer_user_id.unwrap_or(-1);
        let hash = self
            .friends
            .iter()
            .find(|f| f.user_id == peer)
            .and_then(|f| f.avatar_hash.clone());
        let mut drew = false;
        if let Some(hash) = hash.as_deref() {
            let tex = self.avatars.texture(
                &ui.ctx().clone(),
                &self.api(),
                self.token.as_deref(),
                peer,
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
                avatar_color(&channel.name, false),
            );
            let initial = channel
                .name
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
        let dot = rect.right_bottom() - egui::vec2(4.0, 4.0);
        ui.painter().circle_filled(dot, 4.5, status_color(presence));
        ui.painter()
            .circle_stroke(dot, 4.5, Stroke::new(1.5_f32, theme::SURFACE_SIDEBAR));
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Sign out").small())
                            .fill(theme::SURFACE_CARD)
                            .corner_radius(7.0),
                    )
                    .clicked()
                {
                    self.sign_out();
                }
            });
        });
    }
}
