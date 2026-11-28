//! Original Wireline Sidebar (268px):
//! Top user profile bar (with instant Dark/Light toggle & Settings), segmented
//! Chats/People pill switcher, rounded search pill, spacious 54px conversation cards,
//! and a floating Active Call / Incoming Call card pinned cleanly at the bottom when active.

use super::{avatar_color, format_day_label, format_hhmm, icons, FriendsTab, SkyApp};
use crate::sounds::SoundEffect;
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        let p = theme::of(ctx);
        let search_id = egui::Id::new("sidebar_search_input");
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }

        egui::SidePanel::left("left_sidebar")
            .exact_width(278.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.sidebar)
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| {
                // Subtle right hairline separating sidebar from workspace canvas
                let panel_r = ui.max_rect();
                ui.painter().line_segment(
                    [panel_r.right_top(), panel_r.right_bottom()],
                    p.hairline_stroke(),
                );

                // Floating Call Card pinned cleanly at bottom ONLY when in a call or ringing
                if self.in_voice || self.incoming_call.is_some() {
                    egui::TopBottomPanel::bottom("sidebar_floating_call")
                        .show_separator_line(false)
                        .frame(
                            egui::Frame::new()
                                .fill(p.sidebar)
                                .inner_margin(egui::Margin::same(10)),
                        )
                        .show_inside(ui, |ui| {
                            if self.in_voice {
                                self.ui_active_call_card(ui);
                            } else if let Some((call_ch_id, caller_name)) =
                                self.incoming_call.clone()
                            {
                                self.ui_incoming_call_card(ui, call_ch_id, &caller_name);
                            }
                        });
                }

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(p.sidebar)
                            .inner_margin(egui::Margin::same(0)),
                    )
                    .show_inside(ui, |ui| {
                        // 1. Original Top Profile & Controls Header (60px)
                        self.ui_top_profile_bar(ctx, ui);

                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                // 2. Segmented Switcher Pill: [ Chats | People ]
                                let pending_count = self
                                    .friends
                                    .iter()
                                    .filter(|f| {
                                        f.status == FriendshipStatus::Pending
                                            && f.direction == "incoming"
                                    })
                                    .count();
                                let on_people = self.selected_channel.is_none();

                                let (seg_r, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 36.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(seg_r, 10.0, p.surface);
                                let half = (seg_r.width() - 6.0) * 0.5;
                                let left_pill = Rect::from_min_size(
                                    seg_r.min + egui::vec2(3.0, 3.0),
                                    egui::vec2(half, 30.0),
                                );
                                let right_pill = Rect::from_min_size(
                                    egui::pos2(left_pill.right(), seg_r.top() + 3.0),
                                    egui::vec2(half, 30.0),
                                );

                                // Chats tab
                                let chats_resp = ui.interact(
                                    left_pill,
                                    ui.id().with("nav_chats_tab"),
                                    Sense::click(),
                                );
                                if !on_people {
                                    ui.painter().rect_filled(left_pill, 8.0, p.floating);
                                    ui.painter().rect_stroke(
                                        left_pill,
                                        8.0,
                                        p.border_stroke(),
                                        egui::StrokeKind::Inside,
                                    );
                                } else if chats_resp.hovered() {
                                    ui.painter().rect_filled(left_pill, 8.0, p.hover);
                                }
                                let c_fg = if !on_people {
                                    p.text_primary
                                } else {
                                    p.text_secondary
                                };
                                icons::paint_message_circle(
                                    ui.painter(),
                                    left_pill.center() - egui::vec2(22.0, 0.0),
                                    15.0,
                                    c_fg,
                                );
                                ui.painter().text(
                                    left_pill.center() + egui::vec2(8.0, 0.0),
                                    Align2::CENTER_CENTER,
                                    "Chats",
                                    FontId::new(13.0, theme::semibold_family()),
                                    c_fg,
                                );
                                if chats_resp.clicked() && on_people {
                                    if let Some(first_dm) = self
                                        .channels
                                        .iter()
                                        .find(|c| {
                                            c.kind == "dm"
                                                && !self.config.closed_dms.contains(&c.id)
                                        })
                                        .map(|c| c.id)
                                    {
                                        self.select_channel(first_dm);
                                    }
                                }

                                // People tab
                                let people_resp = ui.interact(
                                    right_pill,
                                    ui.id().with("nav_people_tab"),
                                    Sense::click(),
                                );
                                if on_people {
                                    ui.painter().rect_filled(right_pill, 8.0, p.floating);
                                    ui.painter().rect_stroke(
                                        right_pill,
                                        8.0,
                                        p.border_stroke(),
                                        egui::StrokeKind::Inside,
                                    );
                                } else if people_resp.hovered() {
                                    ui.painter().rect_filled(right_pill, 8.0, p.hover);
                                }
                                let p_fg = if on_people {
                                    p.text_primary
                                } else {
                                    p.text_secondary
                                };
                                icons::paint_users(
                                    ui.painter(),
                                    right_pill.center() - egui::vec2(26.0, 0.0),
                                    15.0,
                                    p_fg,
                                );
                                ui.painter().text(
                                    right_pill.center() + egui::vec2(6.0, 0.0),
                                    Align2::CENTER_CENTER,
                                    "People",
                                    FontId::new(13.0, theme::semibold_family()),
                                    p_fg,
                                );
                                if pending_count > 0 {
                                    let scale = self.pending_badge_scale(pending_count);
                                    let bc = right_pill.right_center() - egui::vec2(13.0, 0.0);
                                    ui.painter().circle_filled(bc, 8.0 * scale, p.accent);
                                    ui.painter().text(
                                        bc,
                                        Align2::CENTER_CENTER,
                                        pending_count.min(99).to_string(),
                                        FontId::new(
                                            (10.0 * scale).max(7.0),
                                            theme::semibold_family(),
                                        ),
                                        Color32::WHITE,
                                    );
                                }
                                if people_resp.clicked() && !on_people {
                                    self.selected_channel = None;
                                    self.messages.clear();
                                    self.play_sound(SoundEffect::UiClick);
                                }

                                ui.add_space(10.0);

                                // 3. Rounded Search Pill
                                egui::Frame::new()
                                    .fill(p.input_bg)
                                    .stroke(p.border_stroke())
                                    .corner_radius(10.0)
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let (sr, _) = ui.allocate_exact_size(
                                                egui::vec2(16.0, 16.0),
                                                Sense::hover(),
                                            );
                                            icons::paint_search(
                                                ui.painter(),
                                                sr.center(),
                                                14.5,
                                                p.text_muted,
                                            );
                                            let w = (ui.available_width() - 8.0).max(100.0);
                                            ui.add_sized(
                                                [w, 18.0],
                                                egui::TextEdit::singleline(&mut self.chat_search)
                                                    .id(search_id)
                                                    .frame(false)
                                                    .hint_text("Search conversations..."),
                                            );
                                        });
                                    });

                                ui.add_space(12.0);

                                // Section label + New Chat button
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(
                                        theme::semibold("CONVERSATIONS", 11.0).color(p.text_muted),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let (pr, presp) = ui.allocate_exact_size(
                                                egui::vec2(22.0, 22.0),
                                                Sense::click(),
                                            );
                                            if presp.hovered() {
                                                ui.painter().rect_filled(pr, 6.0, p.hover);
                                            }
                                            icons::paint_plus(
                                                ui.painter(),
                                                pr.center(),
                                                14.0,
                                                if presp.hovered() {
                                                    p.text_primary
                                                } else {
                                                    p.text_muted
                                                },
                                            );
                                            if presp.on_hover_text("Start a new chat").clicked() {
                                                self.selected_channel = None;
                                                self.friends_tab = FriendsTab::All;
                                                self.play_sound(SoundEffect::UiClick);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(6.0);

                                // 4. Spacious 54px Conversation Cards
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
                                            ui.add_space(32.0);
                                            ui.vertical_centered(|ui| {
                                                ui.label(
                                                    RichText::new("No conversations yet")
                                                        .size(13.0)
                                                        .color(p.text_muted),
                                                );
                                            });
                                        } else {
                                            for ch in visible_dms {
                                                self.ui_channel_card(ui, &ch);
                                                ui.add_space(2.0);
                                            }
                                        }
                                    });
                            });
                    });
            });
    }

    /// Original Top User Profile & Theme/Settings Header (replaces Discord's bottom user panel).
    fn ui_top_profile_bar(&mut self, ctx: &Context, ui: &mut Ui) {
        let p = theme::of(ctx);
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());

        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), Sense::hover());
        ui.painter().line_segment(
            [bar_rect.left_bottom(), bar_rect.right_bottom()],
            p.hairline_stroke(),
        );
        let cy = bar_rect.center().y;

        // Clickable profile chip on left
        let chip_w = (bar_rect.width() - 94.0).max(116.0);
        let chip_r = Rect::from_min_size(
            egui::pos2(bar_rect.left() + 10.0, cy - 22.0),
            egui::vec2(chip_w, 44.0),
        );
        let chip_resp = ui.interact(chip_r, ui.id().with("top_profile_chip"), Sense::click());
        if chip_resp.hovered() {
            ui.painter().rect_filled(chip_r, 10.0, p.hover);
            ui.painter().rect_stroke(
                chip_r,
                10.0,
                p.border_stroke(),
                egui::StrokeKind::Inside,
            );
        }

        let av_r = Rect::from_center_size(
            egui::pos2(chip_r.left() + 24.0, cy),
            egui::vec2(38.0, 38.0),
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
                // Backing circle keeps transparent PNG avatars crisp.
                ui.painter().circle_filled(av_r.center(), 19.0, p.surface_alt);
                ui.painter().image(
                    t.id(),
                    av_r,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                drew = true;
            }
        }
        if !drew {
            ui.painter()
                .circle_filled(av_r.center(), 19.0, avatar_color(&me_name, true));
            let init = me_name
                .chars()
                .next()
                .unwrap_or('?')
                .to_ascii_uppercase()
                .to_string();
            ui.painter().text(
                av_r.center(),
                Align2::CENTER_CENTER,
                init,
                FontId::new(15.5, theme::semibold_family()),
                Color32::WHITE,
            );
        }
        let dot = av_r.right_bottom() - egui::vec2(3.0, 3.0);
        let pulse = self.pulse_time(ctx);
        icons::paint_status_dot_pulse(ui.painter(), dot, self.presence, p.sidebar, pulse);

        let tx = av_r.right() + 10.0;
        ui.painter().text(
            egui::pos2(tx, cy - 1.0),
            Align2::LEFT_BOTTOM,
            &me_name,
            FontId::new(14.5, theme::semibold_family()),
            p.text_primary,
        );
        let sub = if !self.config.custom_status.trim().is_empty() {
            self.config.custom_status.as_str()
        } else {
            self.presence.label()
        };
        ui.painter().text(
            egui::pos2(tx, cy + 1.0),
            Align2::LEFT_TOP,
            sub,
            FontId::proportional(12.0),
            p.text_muted,
        );
        if chip_resp.on_hover_text("Status, Avatar & Account").clicked() {
            self.show_profile_popover = !self.show_profile_popover;
            self.play_sound(SoundEffect::UiClick);
        }

        // Right: Instant Dark/Light Theme Toggle + Settings button (36px targets)
        let set_r = Rect::from_center_size(
            egui::pos2(bar_rect.right() - 24.0, cy),
            egui::vec2(35.0, 35.0),
        );
        let set_resp = ui.interact(set_r, ui.id().with("top_settings_btn"), Sense::click());
        if set_resp.hovered() {
            ui.painter().rect_filled(set_r, 9.0, p.hover);
        }
        icons::paint_settings(
            ui.painter(),
            set_r.center(),
            18.5,
            if set_resp.hovered() {
                p.text_primary
            } else {
                p.text_secondary
            },
        );
        if set_resp.on_hover_text("Settings").clicked() {
            self.open_settings_modal(self.settings_tab);
            self.play_sound(SoundEffect::UiClick);
        }

        let th_r = Rect::from_center_size(
            egui::pos2(set_r.left() - 22.0, cy),
            egui::vec2(35.0, 35.0),
        );
        let th_resp = ui.interact(th_r, ui.id().with("top_theme_toggle"), Sense::click());
        if th_resp.hovered() {
            ui.painter().rect_filled(th_r, 8.0, p.hover);
        }
        icons::paint_sun_moon(
            ui.painter(),
            th_r.center(),
            17.0,
            if th_resp.hovered() {
                p.accent
            } else {
                p.text_secondary
            },
            self.config.dark_mode,
        );
        if th_resp
            .on_hover_text(if self.config.dark_mode {
                "Switch to Light Mode"
            } else {
                "Switch to Dark Mode"
            })
            .clicked()
        {
            self.config.dark_mode = !self.config.dark_mode;
            self.config.save();
            theme::apply(ctx, self.config.dark_mode, self.config.font_size);
            self.play_sound(SoundEffect::UiClick);
        }
    }

    fn ui_channel_card(&mut self, ui: &mut Ui, ch: &ChannelInfo) {
        let p = theme::of(ui.ctx());
        let selected = self.selected_channel == Some(ch.id);
        let unread = self.unread.get(&ch.id).copied().unwrap_or(0);
        let muted = self.is_channel_muted(ch.id);
        let presence = ch
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);

        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), Sense::click());
        let bg = if selected {
            p.active
        } else if resp.hovered() {
            p.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 11.0, bg);
        if selected || resp.hovered() {
            ui.painter().rect_stroke(
                rect,
                11.0,
                if selected {
                    Stroke::new(1.0_f32, p.accent.gamma_multiply(0.45))
                } else {
                    p.border_stroke()
                },
                egui::StrokeKind::Inside,
            );
        }

        let ar = Rect::from_center_size(
            egui::pos2(rect.left() + 28.0, rect.center().y),
            egui::vec2(38.0, 38.0),
        );
        let ring_bg = if selected {
            p.active
        } else if resp.hovered() {
            p.hover
        } else {
            p.sidebar
        };
        self.paint_channel_avatar(ui, ar, ch, presence, ring_bg);

        let name_color = if selected || unread > 0 {
            p.text_primary
        } else if muted {
            p.text_muted
        } else {
            p.text_secondary
        };

        let last_info = self.last_messages.get(&ch.id).cloned();
        let tx = ar.right() + 11.0;
        let name_y = if last_info.is_some() {
            rect.top() + 10.0
        } else {
            rect.center().y - 9.0
        };
        ui.painter().text(
            egui::pos2(tx, name_y),
            Align2::LEFT_TOP,
            &ch.name,
            FontId::new(14.5, theme::semibold_family()),
            name_color,
        );

        if let Some((ref raw_preview, ts)) = last_info {
            let preview_clean = if raw_preview == "[[call:started]]" {
                "Started a call".to_string()
            } else if raw_preview == "[[call:missed]]" {
                "Missed call".to_string()
            } else if let Some(dur) = raw_preview
                .strip_prefix("[[call:ended:")
                .and_then(|s| s.strip_suffix("]]"))
            {
                format!("Call ended · {dur}")
            } else {
                raw_preview.clone()
            };
            let now = Self::now_secs() as i64;
            let day = format_day_label(ts, now);
            let ts_str = if day == "Today" {
                format_hhmm(ts)
            } else {
                day
            };
            ui.painter().text(
                egui::pos2(rect.right() - 10.0, rect.top() + 12.0),
                Align2::RIGHT_TOP,
                ts_str,
                FontId::proportional(11.5),
                p.text_muted,
            );
            let clipped: String = if preview_clean.chars().count() > 22 {
                format!("{}...", preview_clean.chars().take(22).collect::<String>())
            } else {
                preview_clean
            };
            ui.painter().text(
                egui::pos2(tx, rect.top() + 31.0),
                Align2::LEFT_TOP,
                clipped,
                FontId::proportional(12.5),
                if unread > 0 {
                    p.text_primary
                } else {
                    p.text_muted
                },
            );
        }

        let active_callers = self
            .active_room_calls
            .get(&ch.id)
            .map(|v| v.len())
            .unwrap_or(0);
        if active_callers > 0 {
            let pill = Rect::from_min_size(
                egui::pos2(rect.right() - 62.0, rect.bottom() - 25.0),
                egui::vec2(52.0, 18.0),
            );
            ui.painter()
                .rect_filled(pill, 9.0, p.success.gamma_multiply(0.2));
            ui.painter().rect_stroke(
                pill,
                9.0,
                Stroke::new(1.0_f32, p.success.gamma_multiply(0.65)),
                egui::StrokeKind::Inside,
            );
            icons::paint_phone(
                ui.painter(),
                pill.left_center() + egui::vec2(10.0, 0.0),
                10.5,
                p.success,
            );
            ui.painter().text(
                pill.left_center() + egui::vec2(18.0, 0.0),
                Align2::LEFT_CENTER,
                "Call",
                FontId::new(10.5, theme::semibold_family()),
                p.success,
            );
        } else if unread > 0 {
            let badge_c = egui::pos2(rect.right() - 18.0, rect.bottom() - 16.0);
            let pill = Rect::from_center_size(badge_c, egui::vec2(19.0, 17.0));
            ui.painter()
                .rect_filled(pill, 8.5, if muted { p.text_muted } else { p.accent });
            ui.painter().text(
                pill.center(),
                Align2::CENTER_CENTER,
                unread.min(99).to_string(),
                FontId::new(10.5, theme::semibold_family()),
                Color32::WHITE,
            );
        }

        if resp.clicked() {
            self.select_channel(ch.id);
        }

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
                .button(RichText::new("Close Chat").color(p.danger))
                .clicked()
            {
                self.close_dm(ch.id);
                ui.close_menu();
            }
        });
    }

    /// Floating Active Call Card inside the sidebar with Mute, Deafen, Share Screen & End Call.
    fn ui_active_call_card(&mut self, ui: &mut Ui) {
        let p = theme::of(ui.ctx());
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);

        egui::Frame::new()
            .fill(p.floating)
            .stroke(p.border_stroke())
            .corner_radius(12.0)
            .shadow(p.shadow())
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (sr, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                    icons::paint_signal(ui.painter(), sr.center(), 16.0, p.success);
                    ui.vertical(|ui| {
                        ui.label(theme::semibold("Voice Active", 13.0).color(p.success));
                        ui.label(
                            RichText::new(format!(
                                "{:02}:{:02}{}",
                                elapsed / 60,
                                elapsed % 60,
                                if self.sharing { " · Sharing" } else { "" }
                            ))
                            .small()
                            .color(p.text_muted),
                        );
                    });
                });

                ui.add_space(10.0);

                // 4 clean pill controls: Mute | Deafen | Share Screen | End
                let btn_w = ((ui.available_width() - 18.0) / 4.0).max(40.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // Mute
                    let (mr, mresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 34.0), Sense::click());
                    ui.painter().rect_filled(
                        mr,
                        8.0,
                        if self.muted {
                            p.danger
                        } else if mresp.hovered() {
                            p.hover
                        } else {
                            p.surface
                        },
                    );
                    icons::paint_mic(
                        ui.painter(),
                        mr.center(),
                        17.0,
                        if self.muted {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        },
                        self.muted,
                    );
                    if mresp.on_hover_text("Mute (Ctrl+Shift+M)").clicked() {
                        self.toggle_mute();
                    }

                    // Deafen
                    let (dr, dresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 34.0), Sense::click());
                    ui.painter().rect_filled(
                        dr,
                        8.0,
                        if self.deafened {
                            p.danger
                        } else if dresp.hovered() {
                            p.hover
                        } else {
                            p.surface
                        },
                    );
                    icons::paint_headphones(
                        ui.painter(),
                        dr.center(),
                        17.0,
                        if self.deafened {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        },
                        self.deafened,
                    );
                    if dresp.on_hover_text("Deafen (Ctrl+Shift+D)").clicked() {
                        self.toggle_deafen();
                    }

                    // Share Screen (only inside an active call!)
                    let (scr, sresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 34.0), Sense::click());
                    ui.painter().rect_filled(
                        scr,
                        8.0,
                        if self.sharing {
                            p.success
                        } else if sresp.hovered() {
                            p.hover
                        } else {
                            p.surface
                        },
                    );
                    icons::paint_screen_share(
                        ui.painter(),
                        scr.center(),
                        17.0,
                        if self.sharing {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        },
                    );
                    if sresp
                        .on_hover_text(if self.sharing {
                            "Stop Sharing Screen"
                        } else {
                            "Share Your Screen"
                        })
                        .clicked()
                    {
                        self.toggle_screen_share();
                    }

                    // End Call
                    let (er, eresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 34.0), Sense::click());
                    ui.painter().rect_filled(er, 8.0, p.danger);
                    icons::paint_phone_off(ui.painter(), er.center(), 17.0, Color32::WHITE);
                    if eresp.on_hover_text("End Call").clicked() {
                        self.leave_voice();
                    }
                });
            });
    }

    fn ui_incoming_call_card(&mut self, ui: &mut Ui, channel_id: i64, caller: &str) {
        let p = theme::of(ui.ctx());
        egui::Frame::new()
            .fill(p.floating)
            .stroke(Stroke::new(1.5_f32, p.accent))
            .corner_radius(12.0)
            .shadow(p.shadow())
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(theme::semibold(caller, 14.0).color(p.text_primary));
                ui.label(RichText::new("Incoming voice call...").small().color(p.success));
                ui.add_space(10.0);
                let half = ((ui.available_width() - 8.0) * 0.5).max(60.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(theme::semibold("Answer", 13.0).color(Color32::WHITE))
                                .fill(p.success)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(half, 32.0)),
                        )
                        .clicked()
                    {
                        self.incoming_call = None;
                        self.select_channel(channel_id);
                        self.start_call(channel_id);
                    }
                    if ui
                        .add(
                            egui::Button::new(theme::semibold("Decline", 13.0).color(Color32::WHITE))
                                .fill(p.danger)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(half, 32.0)),
                        )
                        .clicked()
                    {
                        self.incoming_call = None;
                        self.play_sound(SoundEffect::UiClick);
                    }
                });
            });
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
                let p = theme::of(ui.ctx());
                ui.painter()
                    .circle_filled(rect.center(), rect.width() * 0.5, p.surface_alt);
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
                FontId::new(14.0, theme::semibold_family()),
                Color32::WHITE,
            );
        }
        let dot = rect.right_bottom() - egui::vec2(3.0, 3.0);
        let pulse = self.pulse_time(ui.ctx());
        icons::paint_status_dot_pulse(ui.painter(), dot, presence, ring_bg, pulse);
    }
}
