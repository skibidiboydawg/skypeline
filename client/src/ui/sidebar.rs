//! Discord-faithful 240px left sidebar (#2b2d31): Quick Search, Friends button,
//! Direct Messages list with right-click context menu, Voice Connected bar, and 52px User Panel (#1e1f22).

use super::{avatar_color, format_day_label, format_hhmm, icons, FriendsTab, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        let search_id = egui::Id::new("sidebar_search_input");
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }

        egui::SidePanel::left("left_sidebar")
            .exact_width(240.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_SECONDARY)
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| {
                // Pin Voice Connected / Incoming Call bar + strict 52px User Panel FIRST
                egui::TopBottomPanel::bottom("sidebar_user_dock")
                    .show_separator_line(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(0)))
                    .show_inside(ui, |ui| {
                        if self.in_voice {
                            self.ui_active_call_banner(ui);
                        } else if let Some((call_ch_id, caller_name)) = self.incoming_call.clone() {
                            self.ui_incoming_call_banner(ui, call_ch_id, &caller_name);
                        }
                        self.ui_user_dock(ui);
                    });

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(theme::BG_SECONDARY)
                            .inner_margin(egui::Margin::same(0)),
                    )
                    .show_inside(ui, |ui| {
                        // Top 48px search header (border-bottom 1px subtle)
                        let (top_r, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 48.0),
                            Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [top_r.left_bottom(), top_r.right_bottom()],
                            theme::border(),
                        );
                        let search_box = top_r.shrink2(egui::vec2(10.0, 10.0));
                        ui.painter()
                            .rect_filled(search_box, 4.0, theme::BG_TERTIARY);
                        ui.allocate_new_ui(
                            egui::UiBuilder::new()
                                .max_rect(search_box.shrink2(egui::vec2(8.0, 4.0)))
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 20.0],
                                    egui::TextEdit::singleline(&mut self.chat_search)
                                        .id(search_id)
                                        .frame(false)
                                        .hint_text("Find or start a conversation"),
                                );
                            },
                        );

                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(8, 8))
                            .show(ui, |ui| {
                                // "Friends" navigation item
                                let pending_count = self
                                    .friends
                                    .iter()
                                    .filter(|f| {
                                        f.status == FriendshipStatus::Pending
                                            && f.direction == "incoming"
                                    })
                                    .count();
                                let home = self.selected_channel.is_none();
                                let (fr_r, fr_resp) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 42.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    fr_r,
                                    4.0,
                                    if home {
                                        theme::BG_ACTIVE
                                    } else if fr_resp.hovered() {
                                        theme::BG_HOVER
                                    } else {
                                        Color32::TRANSPARENT
                                    },
                                );
                                icons::paint_users(
                                    ui.painter(),
                                    fr_r.left_center() + egui::vec2(20.0, 0.0),
                                    18.0,
                                    if home {
                                        theme::TEXT_PRIMARY
                                    } else {
                                        theme::TEXT_SECONDARY
                                    },
                                );
                                ui.painter().text(
                                    fr_r.left_center() + egui::vec2(42.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    "Friends",
                                    FontId::proportional(15.0),
                                    if home {
                                        theme::TEXT_PRIMARY
                                    } else {
                                        theme::TEXT_SECONDARY
                                    },
                                );
                                if pending_count > 0 {
                                    let badge_c = fr_r.right_center() - egui::vec2(16.0, 0.0);
                                    ui.painter().circle_filled(badge_c, 8.5, theme::DANGER);
                                    ui.painter().text(
                                        badge_c,
                                        Align2::CENTER_CENTER,
                                        pending_count.min(99).to_string(),
                                        FontId::proportional(11.0),
                                        Color32::WHITE,
                                    );
                                }
                                if fr_resp.clicked() {
                                    self.selected_channel = None;
                                    self.messages.clear();
                                }

                                ui.add_space(12.0);

                                // "DIRECT MESSAGES" header + "+" button
                                ui.horizontal(|ui| {
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new("DIRECT MESSAGES")
                                            .size(11.5)
                                            .strong()
                                            .color(theme::TEXT_MUTED),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (pr, presp) = ui.allocate_exact_size(
                                                egui::vec2(20.0, 20.0),
                                                Sense::click(),
                                            );
                                            if presp.hovered() {
                                                ui.painter().rect_filled(pr, 4.0, theme::BG_HOVER);
                                            }
                                            icons::paint_plus(
                                                ui.painter(),
                                                pr.center(),
                                                14.0,
                                                if presp.hovered() {
                                                    theme::TEXT_PRIMARY
                                                } else {
                                                    theme::TEXT_MUTED
                                                },
                                            );
                                            if presp.on_hover_text("New DM").clicked() {
                                                self.selected_channel = None;
                                                self.friends_tab = FriendsTab::All;
                                            }
                                        },
                                    );
                                });
                                ui.add_space(4.0);

                                // Scrollable DM List
                                let search = self.chat_search.trim().to_ascii_lowercase();
                                let mut dms: Vec<ChannelInfo> = self
                                    .channels
                                    .iter()
                                    .filter(|c| {
                                        c.kind == "dm" && !self.config.closed_dms.contains(&c.id)
                                    })
                                    .cloned()
                                    .collect();
                                dms.sort_by(|a, b| {
                                    let ta = self
                                        .last_messages
                                        .get(&a.id)
                                        .map(|(_, t)| *t)
                                        .unwrap_or(a.id);
                                    let tb = self
                                        .last_messages
                                        .get(&b.id)
                                        .map(|(_, t)| *t)
                                        .unwrap_or(b.id);
                                    tb.cmp(&ta)
                                });

                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let visible_dms: Vec<ChannelInfo> = dms
                                            .into_iter()
                                            .filter(|ch| {
                                                search.is_empty()
                                                    || ch.name.to_ascii_lowercase().contains(&search)
                                            })
                                            .collect();
                                        if visible_dms.is_empty() {
                                            ui.add_space(28.0);
                                            ui.vertical_centered(|ui| {
                                                let (er, _) = ui.allocate_exact_size(
                                                    egui::vec2(28.0, 28.0),
                                                    Sense::hover(),
                                                );
                                                icons::paint_message_circle(
                                                    ui.painter(),
                                                    er.center(),
                                                    22.0,
                                                    theme::TEXT_MUTED,
                                                );
                                                ui.add_space(6.0);
                                                ui.label(
                                                    RichText::new("No conversations yet")
                                                        .small()
                                                        .color(theme::TEXT_MUTED),
                                                );
                                            });
                                        } else {
                                            for ch in visible_dms {
                                                self.ui_channel_row(ui, &ch);
                                            }
                                        }
                                    });
                            });
                    });
            });
    }

    fn ui_channel_row(&mut self, ui: &mut Ui, ch: &ChannelInfo) {
        let selected = self.selected_channel == Some(ch.id);
        let unread = self.unread.get(&ch.id).copied().unwrap_or(0);
        let muted = self.is_channel_muted(ch.id);
        let presence = ch
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);

        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::click());
        let bg = if selected {
            theme::BG_ACTIVE
        } else if resp.hovered() {
            theme::BG_HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 4.0, bg);

        // Unread left 2px blurple bar
        if unread > 0 && !selected {
            let bar = Rect::from_min_max(
                rect.left_top() + egui::vec2(0.0, 8.0),
                rect.left_bottom() + egui::vec2(2.5, -8.0),
            );
            ui.painter().rect_filled(bar, 1.0, theme::ACCENT);
        }

        let ar = Rect::from_min_size(rect.min + egui::vec2(8.0, 6.0), egui::vec2(32.0, 32.0));
        let ring_bg = if selected {
            theme::BG_ACTIVE
        } else {
            theme::BG_SECONDARY
        };
        self.paint_channel_avatar(ui, ar, ch, presence, ring_bg);

        let name_color = if selected || unread > 0 {
            theme::TEXT_PRIMARY
        } else if muted {
            theme::TEXT_MUTED
        } else {
            theme::TEXT_SECONDARY
        };

        let last_info = self.last_messages.get(&ch.id).cloned();
        let has_preview = !selected && last_info.is_some();
        let name_y = if has_preview {
            rect.top() + 7.0
        } else {
            rect.center().y - 7.0
        };
        ui.painter().text(
            egui::pos2(rect.left() + 50.0, name_y),
            Align2::LEFT_TOP,
            &ch.name,
            FontId::proportional(14.0),
            name_color,
        );

        if let Some((ref preview, ts)) = last_info {
            let now = Self::now_secs() as i64;
            let day = format_day_label(ts, now);
            let ts_str = if day == "Today" {
                format_hhmm(ts)
            } else {
                day
            };
            ui.painter().text(
                egui::pos2(rect.right() - 8.0, rect.top() + 8.0),
                Align2::RIGHT_TOP,
                ts_str,
                FontId::proportional(10.5),
                theme::TEXT_MUTED,
            );
            if has_preview {
                let clipped: String = if preview.chars().count() > 20 {
                    format!("{}...", preview.chars().take(20).collect::<String>())
                } else {
                    preview.clone()
                };
                ui.painter().text(
                    egui::pos2(rect.left() + 50.0, rect.top() + 24.0),
                    Align2::LEFT_TOP,
                    clipped,
                    FontId::proportional(12.0),
                    if unread > 0 {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_MUTED
                    },
                );
            }
        }

        if unread > 0 {
            let badge_c = rect.right_center() - egui::vec2(14.0, -6.0);
            let pill = Rect::from_center_size(badge_c, egui::vec2(18.0, 16.0));
            ui.painter().rect_filled(
                pill,
                8.0,
                if muted { theme::TEXT_MUTED } else { theme::DANGER },
            );
            ui.painter().text(
                pill.center(),
                Align2::CENTER_CENTER,
                unread.min(99).to_string(),
                FontId::proportional(10.5),
                Color32::WHITE,
            );
        }

        if resp.clicked() {
            self.select_channel(ch.id);
        }

        // Right-click Discord context menu on DM entry
        resp.context_menu(|ui| {
            ui.set_min_width(160.0);
            if ui.button("Mark As Read").clicked() {
                self.unread.remove(&ch.id);
                ui.close_menu();
            }
            let mute_label = if muted {
                "Unmute Conversation"
            } else {
                "Mute Conversation"
            };
            if ui.button(mute_label).clicked() {
                self.toggle_mute_channel(ch.id);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(RichText::new("Close DM").color(theme::DANGER))
                .clicked()
            {
                self.close_dm(ch.id);
                ui.close_menu();
            }
        });
    }

    /// Persistent Voice Connected bar (52px, --bg-tertiary) above bottom user panel.
    fn ui_active_call_banner(&mut self, ui: &mut Ui) {
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, 0.0, theme::BG_TERTIARY);
        ui.painter().line_segment(
            [bar_rect.left_top(), bar_rect.right_top()],
            theme::border(),
        );

        let sig_c = bar_rect.left_center() + egui::vec2(18.0, -6.0);
        icons::paint_signal(ui.painter(), sig_c, 15.0, theme::SUCCESS);

        ui.painter().text(
            bar_rect.left_top() + egui::vec2(32.0, 10.0),
            Align2::LEFT_TOP,
            "Voice Connected",
            FontId::proportional(13.0),
            theme::SUCCESS,
        );
        ui.painter().text(
            bar_rect.left_top() + egui::vec2(32.0, 27.0),
            Align2::LEFT_TOP,
            format!("{:02}:{:02} · Server Relay", elapsed / 60, elapsed % 60),
            FontId::proportional(11.5),
            theme::TEXT_MUTED,
        );

        let disc_r = Rect::from_center_size(
            bar_rect.right_center() - egui::vec2(22.0, 0.0),
            egui::vec2(30.0, 30.0),
        );
        let disc_resp = ui.interact(disc_r, ui.id().with("voice_disc_btn"), Sense::click());
        ui.painter().rect_filled(
            disc_r,
            4.0,
            if disc_resp.hovered() {
                theme::DANGER
            } else {
                theme::BG_HOVER
            },
        );
        icons::paint_phone_off(
            ui.painter(),
            disc_r.center(),
            16.0,
            if disc_resp.hovered() {
                Color32::WHITE
            } else {
                theme::TEXT_SECONDARY
            },
        );
        if disc_resp.on_hover_text("Disconnect").clicked() {
            self.leave_voice();
        }
    }

    fn ui_incoming_call_banner(&mut self, ui: &mut Ui, channel_id: i64, caller: &str) {
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, 0.0, theme::BG_FLOATING);
        ui.painter().line_segment(
            [bar_rect.left_top(), bar_rect.right_top()],
            Stroke::new(1.5_f32, theme::ACCENT),
        );
        ui.painter().text(
            bar_rect.left_top() + egui::vec2(12.0, 10.0),
            Align2::LEFT_TOP,
            caller,
            FontId::proportional(13.5),
            theme::TEXT_PRIMARY,
        );
        ui.painter().text(
            bar_rect.left_top() + egui::vec2(12.0, 28.0),
            Align2::LEFT_TOP,
            "Incoming voice call...",
            FontId::proportional(11.5),
            theme::SUCCESS,
        );

        let dec_r = Rect::from_center_size(
            bar_rect.right_center() - egui::vec2(20.0, 0.0),
            egui::vec2(28.0, 28.0),
        );
        let dec_resp = ui.interact(dec_r, ui.id().with("inc_call_decline"), Sense::click());
        ui.painter().circle_filled(
            dec_r.center(),
            14.0,
            if dec_resp.hovered() {
                theme::DANGER
            } else {
                theme::BG_SECONDARY
            },
        );
        icons::paint_x(
            ui.painter(),
            dec_r.center(),
            14.0,
            if dec_resp.hovered() {
                Color32::WHITE
            } else {
                theme::DANGER
            },
        );
        if dec_resp.on_hover_text("Dismiss").clicked() {
            self.incoming_call = None;
        }

        let ans_r = Rect::from_center_size(
            bar_rect.right_center() - egui::vec2(54.0, 0.0),
            egui::vec2(28.0, 28.0),
        );
        let ans_resp = ui.interact(ans_r, ui.id().with("inc_call_accept"), Sense::click());
        ui.painter().circle_filled(
            ans_r.center(),
            14.0,
            if ans_resp.hovered() {
                theme::SUCCESS
            } else {
                theme::BG_SECONDARY
            },
        );
        icons::paint_phone(
            ui.painter(),
            ans_r.center(),
            14.0,
            if ans_resp.hovered() {
                Color32::WHITE
            } else {
                theme::SUCCESS
            },
        );
        if ans_resp.on_hover_text("Answer call").clicked() {
            self.incoming_call = None;
            self.select_channel(channel_id);
            self.start_call(channel_id);
        }
    }

    fn paint_channel_avatar(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        channel: &ChannelInfo,
        presence: PresenceStatus,
        ring_bg: Color32,
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
                FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
        let dot = rect.right_bottom() - egui::vec2(3.0, 3.0);
        icons::paint_status_dot(ui.painter(), dot, presence, ring_bg);
    }

    /// Strict 52px Discord Bottom User Panel (--bg-tertiary #1e1f22).
    fn ui_user_dock(&mut self, ui: &mut Ui) {
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());

        let (dock_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 52.0), Sense::hover());
        ui.painter()
            .rect_filled(dock_rect, 0.0, theme::BG_TERTIARY);

        let cy = dock_rect.center().y;

        // Clickable profile section
        let prof_w = (dock_rect.width() - 108.0).max(92.0);
        let prof_rect = Rect::from_min_size(
            egui::pos2(dock_rect.left() + 6.0, cy - 19.0),
            egui::vec2(prof_w, 38.0),
        );
        let prof_resp = ui.interact(prof_rect, ui.id().with("user_dock_profile"), Sense::click());
        if prof_resp.hovered() {
            ui.painter().rect_filled(prof_rect, 4.0, theme::BG_HOVER);
        }

        let avatar_rect = Rect::from_center_size(
            egui::pos2(prof_rect.left() + 19.0, cy),
            egui::vec2(32.0, 32.0),
        );
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
                    avatar_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                drew = true;
            }
        }
        if !drew {
            ui.painter()
                .circle_filled(avatar_rect.center(), 16.0, avatar_color(&me_name, true));
            let initial = me_name
                .chars()
                .next()
                .unwrap_or('?')
                .to_ascii_uppercase()
                .to_string();
            ui.painter().text(
                avatar_rect.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
        let dot = avatar_rect.right_bottom() - egui::vec2(3.0, 3.0);
        icons::paint_status_dot(ui.painter(), dot, self.presence, theme::BG_TERTIARY);

        let text_x = avatar_rect.right() + 8.0;
        ui.painter().text(
            egui::pos2(text_x, cy - 1.0),
            Align2::LEFT_BOTTOM,
            &me_name,
            FontId::proportional(13.5),
            theme::TEXT_PRIMARY,
        );
        let sub = if !self.config.custom_status.trim().is_empty() {
            self.config.custom_status.as_str()
        } else {
            self.presence.label()
        };
        ui.painter().text(
            egui::pos2(text_x, cy + 1.0),
            Align2::LEFT_TOP,
            sub,
            FontId::proportional(11.5),
            theme::TEXT_MUTED,
        );
        if prof_resp.on_hover_text("Change Status").clicked() {
            self.show_profile_popover = !self.show_profile_popover;
        }

        // Right: 3x 32px icon buttons (Mic, Headphones, Settings)
        let btn_size = 30.0;
        let gap = 2.0;
        let right_edge = dock_rect.right() - 6.0;

        let gr = Rect::from_center_size(
            egui::pos2(right_edge - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let gresp = ui.interact(gr, ui.id().with("dock_settings"), Sense::click());
        if gresp.hovered() {
            ui.painter().rect_filled(gr, 4.0, theme::BG_HOVER);
        }
        icons::paint_settings(
            ui.painter(),
            gr.center(),
            17.0,
            if gresp.hovered() {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            },
        );
        if gresp.on_hover_text("User Settings").clicked() {
            self.show_settings_modal = true;
        }

        let dr = Rect::from_center_size(
            egui::pos2(gr.left() - gap - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let dresp = ui.interact(dr, ui.id().with("dock_deafen"), Sense::click());
        if dresp.hovered() || self.deafened {
            ui.painter().rect_filled(
                dr,
                4.0,
                if self.deafened {
                    Color32::from_rgb(0x46, 0x26, 0x29)
                } else {
                    theme::BG_HOVER
                },
            );
        }
        icons::paint_headphones(
            ui.painter(),
            dr.center(),
            17.0,
            if self.deafened {
                theme::DANGER
            } else if dresp.hovered() {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            },
            self.deafened,
        );
        if dresp.on_hover_text("Deafen (Ctrl+Shift+D)").clicked() {
            self.toggle_deafen();
        }

        let mr = Rect::from_center_size(
            egui::pos2(dr.left() - gap - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let mresp = ui.interact(mr, ui.id().with("dock_mute"), Sense::click());
        if mresp.hovered() || self.muted {
            ui.painter().rect_filled(
                mr,
                4.0,
                if self.muted {
                    Color32::from_rgb(0x46, 0x26, 0x29)
                } else {
                    theme::BG_HOVER
                },
            );
        }
        icons::paint_mic(
            ui.painter(),
            mr.center(),
            17.0,
            if self.muted {
                theme::DANGER
            } else if mresp.hovered() {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            },
            self.muted,
        );
        if mresp.on_hover_text("Mute (Ctrl+Shift+M)").clicked() {
            self.toggle_mute();
        }
    }
}
