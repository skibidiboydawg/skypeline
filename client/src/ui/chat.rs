//! Original Wireline Chat Workspace (100% Dynamic Dark & Light Mode):
//! Minimal 56px header (Call button only — share screen inside the call!),
//! scalable Voice/Screen Call Stage, sleek modern conversational bubbles with Markdown,
//! replies, reactions, floating hover actions, animated typing dots, and pill composer.

use super::{
    anim, avatar_color, day_index, format_day_label, format_discord_timestamp, format_hhmm, icons,
    SkyApp,
};
use crate::sounds::SoundEffect;
use crate::theme::{self, Palette};
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, MessageDto, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_chat_stage(&mut self, ui: &mut Ui) {
        let Some(channel) = self.current_channel().cloned() else {
            return;
        };
        let p = theme::of(ui.ctx());

        // 1. Crisp 60px Workspace Header (38px Avatar + Name + Live Status | Call + Search)
        let (hdr_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 60.0), Sense::hover());
        ui.painter().rect_filled(hdr_rect, 0.0, p.canvas);
        ui.painter().line_segment(
            [hdr_rect.left_bottom(), hdr_rect.right_bottom()],
            p.hairline_stroke(),
        );

        let hdr_inner = hdr_rect.shrink2(egui::vec2(20.0, 8.0));
        let peer_status = channel
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);
        let in_this_call = self.in_voice
            && self.voice_room.as_deref() == Some(Self::channel_room(channel.id).as_str());
        let pulse = self.pulse_time(ui.ctx());

        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(hdr_inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                // Clickable 38px circular peer avatar + status dot
                let (av_r, av_resp) =
                    ui.allocate_exact_size(egui::vec2(38.0, 38.0), Sense::click());
                self.paint_user_avatar(
                    ui,
                    av_r,
                    channel.peer_user_id.unwrap_or(-1),
                    &channel.name,
                    false,
                );
                icons::paint_status_dot_pulse(
                    ui.painter(),
                    av_r.right_bottom() - egui::vec2(3.5, 3.5),
                    peer_status,
                    p.canvas,
                    pulse,
                );
                if av_resp.clicked() {
                    if let Some(peer_id) = channel.peer_user_id {
                        self.open_profile_popup(
                            peer_id,
                            channel.name.clone(),
                            av_r.right_bottom(),
                        );
                    }
                }

                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let name_resp = ui.add(
                        egui::Label::new(
                            theme::semibold(&channel.name, 16.0).color(p.text_primary),
                        )
                        .sense(Sense::click()),
                    );
                    if name_resp.clicked() {
                        if let Some(peer_id) = channel.peer_user_id {
                            self.open_profile_popup(
                                peer_id,
                                channel.name.clone(),
                                name_resp.rect.left_bottom(),
                            );
                        }
                    }
                    ui.label(
                        RichText::new(match peer_status {
                            PresenceStatus::Online => "Online",
                            PresenceStatus::Away => "Idle",
                            PresenceStatus::Dnd => "Do Not Disturb",
                            PresenceStatus::Offline => "Offline",
                        })
                        .size(12.5)
                        .color(if peer_status == PresenceStatus::Online {
                            p.success
                        } else {
                            p.text_muted
                        }),
                    );
                });

                // Right side: Inline Search OR polished Voice Call pill + 36px Search icon
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.chat_search_open {
                        let (xr, xresp) =
                            ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
                        if xresp.hovered() {
                            ui.painter().rect_filled(xr, 8.0, p.hover);
                        }
                        icons::paint_x(ui.painter(), xr.center(), 15.0, p.text_secondary);
                        if xresp.clicked() {
                            self.chat_search_open = false;
                            self.chat_msg_search.clear();
                        }

                        egui::Frame::new()
                            .fill(p.input_bg)
                            .stroke(p.focus_stroke())
                            .corner_radius(10.0)
                            .inner_margin(egui::Margin::symmetric(11, 6))
                            .show(ui, |ui| {
                                let w = (ui.available_width().min(250.0)).max(120.0);
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.chat_msg_search)
                                        .frame(false)
                                        .desired_width(w)
                                        .hint_text("Search messages..."),
                                );
                            });
                    } else {
                        // 36px Search button with hover border
                        let (sr, sresp) =
                            ui.allocate_exact_size(egui::vec2(36.0, 36.0), Sense::click());
                        ui.painter().rect_filled(
                            sr,
                            10.0,
                            if sresp.hovered() {
                                p.hover
                            } else {
                                Color32::TRANSPARENT
                            },
                        );
                        if sresp.hovered() {
                            ui.painter().rect_stroke(
                                sr,
                                10.0,
                                p.border_stroke(),
                                egui::StrokeKind::Inside,
                            );
                        }
                        icons::paint_search(
                            ui.painter(),
                            sr.center(),
                            18.0,
                            if sresp.hovered() {
                                p.text_primary
                            } else {
                                p.text_secondary
                            },
                        );
                        if sresp.on_hover_text("Search conversation").clicked() {
                            self.chat_search_open = true;
                            self.play_sound(SoundEffect::UiClick);
                        }

                        ui.add_space(6.0);

                        // Polished Voice Call pill with hover glow & crisp stroke
                        let call_w = if in_this_call { 96.0 } else { 84.0 };
                        let (cr, cresp) =
                            ui.allocate_exact_size(egui::vec2(call_w, 36.0), Sense::click());
                        let c_bg = if in_this_call {
                            p.success
                        } else if cresp.hovered() {
                            p.accent
                        } else {
                            p.surface
                        };
                        let c_fg = if in_this_call || cresp.hovered() {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        };
                        if cresp.hovered() && !in_this_call {
                            ui.painter().rect_stroke(
                                cr.expand(2.0),
                                12.0,
                                Stroke::new(2.0_f32, p.accent.gamma_multiply(0.32)),
                                egui::StrokeKind::Outside,
                            );
                        }
                        ui.painter().rect_filled(cr, 10.0, c_bg);
                        if !in_this_call && !cresp.hovered() {
                            ui.painter().rect_stroke(
                                cr,
                                10.0,
                                p.border_stroke(),
                                egui::StrokeKind::Inside,
                            );
                        }
                        icons::paint_phone(
                            ui.painter(),
                            cr.left_center() + egui::vec2(18.0, 0.0),
                            17.0,
                            c_fg,
                        );
                        ui.painter().text(
                            cr.left_center() + egui::vec2(33.0, 0.0),
                            Align2::LEFT_CENTER,
                            if in_this_call { "In Call" } else { "Call" },
                            FontId::new(13.5, theme::semibold_family()),
                            c_fg,
                        );
                        if cresp
                            .on_hover_text(if in_this_call {
                                "Show / hide call panel"
                            } else {
                                "Start voice call (share screen once connected)"
                            })
                            .clicked()
                        {
                            if in_this_call {
                                self.call_expanded = !self.call_expanded;
                                self.play_sound(SoundEffect::UiClick);
                            } else {
                                self.start_call(channel.id);
                            }
                        }
                    }
                });
            },
        );

        // 2. Disconnected banner
        if !self.connected {
            let (ban_r, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), Sense::hover());
            ui.painter().rect_filled(
                ban_r,
                0.0,
                if p.dark {
                    Color32::from_rgb(0x3B, 0x2F, 0x16)
                } else {
                    Color32::from_rgb(0xFE, 0xF3, 0xC7)
                },
            );
            ui.painter().text(
                ban_r.center(),
                Align2::CENTER_CENTER,
                "Connecting to Wireline relay...",
                FontId::new(12.0, theme::semibold_family()),
                p.warning,
            );
        }

        // 3. Scalable / Resizable Voice & Screen Call Stage
        if in_this_call && self.call_expanded {
            self.ui_call_stage(ui, &channel);
        }

        // 4. Messages + Floating Pill Composer
        self.ui_messages(ui, &channel);
    }

    fn ui_messages(&mut self, ui: &mut Ui, channel: &ChannelInfo) {
        let p = theme::of(ui.ctx());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let now = Self::now_secs();
        let now_i64 = now as i64;
        let active_typers: Vec<String> = self
            .typing_users
            .get_mut(&channel.id)
            .map(|map| {
                map.retain(|_, (_, exp)| *exp > now);
                map.values().map(|(name, _)| name.clone()).collect()
            })
            .unwrap_or_default();

        // Pinned Floating Pill Composer Dock with subtle 1px top hairline & 980px centered column
        let composer_pad = ((ui.available_width() - 980.0) * 0.5).max(20.0).round() as i8;
        let composer_pad = composer_pad.clamp(20, 120);
        let composer_extra_pad = ((ui.available_width() - 980.0) * 0.5 - composer_pad as f32).max(0.0);
        egui::TopBottomPanel::bottom("chat_composer_dock")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(p.canvas)
                    .inner_margin(egui::Margin {
                        left: 0,
                        right: 0,
                        top: 0,
                        bottom: 14,
                    }),
            )
            .show_inside(ui, |ui| {
                // Crisp 1px subtle separator between chat feed and composer bar
                let sep_r = ui.max_rect();
                ui.painter().line_segment(
                    [sep_r.left_top(), sep_r.right_top()],
                    p.hairline_stroke(),
                );
                let side_margin = ((ui.available_width() - 980.0) * 0.5).max(20.0);
                let _ = (composer_pad, composer_extra_pad);
                ui.horizontal(|ui| {
                    ui.add_space(side_margin);
                    let col_w = (ui.available_width() - side_margin).max(240.0);
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        ui.add_space(4.0);
                        // Typing indicator sits cleanly ABOVE the pill composer
                        let (typ_r, _) =
                            ui.allocate_exact_size(egui::vec2(col_w, 18.0), Sense::hover());
                let is_typing = !active_typers.is_empty();
                if is_typing {
                    self.last_typing_text = if active_typers.len() == 1 {
                        format!("{} is typing...", active_typers[0])
                    } else {
                        format!("{} are typing...", active_typers.join(", "))
                    };
                }
                let dt = ui.ctx().input(|i| i.unstable_dt).clamp(0.001, 0.1);
                let fade_step = if self.config.reduced_motion {
                    1.0
                } else {
                    dt / 0.20
                };
                if is_typing {
                    self.typing_fade = (self.typing_fade + fade_step).min(1.0);
                } else {
                    self.typing_fade = (self.typing_fade - fade_step).max(0.0);
                }
                if self.typing_fade > 0.005 {
                    if is_typing || self.typing_fade < 1.0 {
                        self.animating = true;
                    }
                    let t = ui.ctx().input(|i| i.time);
                    let dot_color = p.text_secondary.gamma_multiply(self.typing_fade);
                    for (dot, delay) in [0.0_f64, 0.15, 0.30].iter().enumerate() {
                        let y_off =
                            anim::typing_dot_offset(t, *delay, self.config.reduced_motion);
                        let c = egui::pos2(
                            typ_r.left() + 10.0 + (dot as f32) * 9.0,
                            typ_r.center().y + y_off,
                        );
                        ui.painter().circle_filled(c, 2.8, dot_color);
                    }
                    ui.painter().text(
                        egui::pos2(typ_r.left() + 38.0, typ_r.center().y),
                        Align2::LEFT_CENTER,
                        &self.last_typing_text,
                        FontId::proportional(11.5),
                        dot_color,
                    );
                }

                // Reply Banner
                if let Some((_, ref author, ref snippet)) = self.reply_to.clone() {
                    egui::Frame::new()
                        .fill(p.surface)
                        .stroke(p.border_stroke())
                        .corner_radius(egui::CornerRadius {
                            nw: 14,
                            ne: 14,
                            sw: 0,
                            se: 0,
                        })
                        .inner_margin(egui::Margin::symmetric(14, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    theme::semibold(format!("Replying to {author}"), 12.0)
                                        .color(p.accent),
                                );
                                let preview: String = if snippet.chars().count() > 48 {
                                    format!(
                                        "{}...",
                                        snippet.chars().take(48).collect::<String>()
                                    )
                                } else {
                                    snippet.clone()
                                };
                                ui.label(RichText::new(preview).small().color(p.text_muted));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (xr, xresp) = ui.allocate_exact_size(
                                            egui::vec2(20.0, 20.0),
                                            Sense::click(),
                                        );
                                        icons::paint_x(
                                            ui.painter(),
                                            xr.center(),
                                            12.0,
                                            if xresp.hovered() {
                                                p.text_primary
                                            } else {
                                                p.text_muted
                                            },
                                        );
                                        if xresp.clicked() {
                                            self.reply_to = None;
                                        }
                                    },
                                );
                            });
                        });
                }

                // Floating Pill Composer (20px radius, subtle focus glow + integrated Send button)
                let top_radius = if self.reply_to.is_some() { 0 } else { 20 };
                let draft_id = egui::Id::new(("chat_draft_input", channel.id));
                let has_focus = ui.memory(|m| m.has_focus(draft_id));
                let composer_resp = egui::Frame::new()
                    .fill(p.input_bg)
                    .stroke(if has_focus {
                        p.focus_stroke()
                    } else {
                        p.border_stroke()
                    })
                    .corner_radius(egui::CornerRadius {
                        nw: top_radius,
                        ne: top_radius,
                        sw: 20,
                        se: 20,
                    })
                    .inner_margin(egui::Margin::symmetric(12, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            // Left "+" button (insert code block)
                            let (pr, presp) =
                                ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                            if presp.hovered() {
                                ui.painter().circle_filled(pr.center(), 14.0, p.hover);
                            }
                            icons::paint_plus_circle(
                                ui.painter(),
                                pr.center(),
                                20.0,
                                if presp.hovered() {
                                    p.text_primary
                                } else {
                                    p.text_secondary
                                },
                            );
                            if presp.on_hover_text("Insert code block").clicked() {
                                self.draft.push_str("```\n\n```");
                                self.refocus_draft = true;
                            }

                            let input_w = (ui.available_width() - 80.0).max(120.0);
                            let field = ui.add_sized(
                                [input_w, 28.0],
                                egui::TextEdit::singleline(&mut self.draft)
                                    .id(draft_id)
                                    .frame(false)
                                    .hint_text(format!("Write to {}...", channel.name)),
                            );
                            if self.refocus_draft {
                                field.request_focus();
                                self.refocus_draft = false;
                            }
                            if field.changed() && !self.draft.trim().is_empty() {
                                self.notify_typing(channel.id);
                            }

                            // Arrow Up when draft is empty -> edit last own message
                            if field.has_focus()
                                && self.draft.is_empty()
                                && self.editing_message.is_none()
                                && ui.input(|i| i.key_pressed(egui::Key::ArrowUp))
                            {
                                if let Some(last_mine) =
                                    self.messages.iter().rev().find(|m| m.user_id == me_id)
                                {
                                    self.editing_message =
                                        Some((last_mine.id, last_mine.content.clone()));
                                }
                            }

                            // Enter to send — immediately keep focus
                            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.send_message();
                                field.request_focus();
                                self.refocus_draft = true;
                                ui.ctx().request_repaint();
                            } else if !field.has_focus()
                                && !field.lost_focus()
                                && !self.show_settings_modal
                                && !self.show_profile_popover
                                && self.emoji_picker_target.is_none()
                                && self.editing_message.is_none()
                                && !self.chat_search_open
                                && !ui.ctx().wants_keyboard_input()
                            {
                                let typed: String = ui.input(|i| {
                                    if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
                                        return String::new();
                                    }
                                    i.events
                                        .iter()
                                        .filter_map(|ev| match ev {
                                            egui::Event::Text(t)
                                                if !t.is_empty()
                                                    && !t.chars().all(char::is_control) =>
                                            {
                                                Some(t.as_str())
                                            }
                                            _ => None,
                                        })
                                        .collect()
                                });
                                if !typed.is_empty() {
                                    self.draft.push_str(&typed);
                                    let end_idx = self.draft.chars().count();
                                    let mut state =
                                        egui::TextEdit::load_state(ui.ctx(), draft_id)
                                            .unwrap_or_default();
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::one(
                                            egui::text::CCursor::new(end_idx),
                                        ),
                                    ));
                                    egui::TextEdit::store_state(ui.ctx(), draft_id, state);
                                    field.request_focus();
                                    self.notify_typing(channel.id);
                                    ui.ctx().request_repaint();
                                }
                            }

                            // Emoji picker button
                            let (er, eresp) =
                                ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                            if eresp.hovered() || self.emoji_picker_target == Some(None) {
                                ui.painter().circle_filled(er.center(), 14.0, p.hover);
                            }
                            icons::paint_smile(
                                ui.painter(),
                                er.center(),
                                18.5,
                                if eresp.hovered() || self.emoji_picker_target == Some(None) {
                                    p.warning
                                } else {
                                    p.text_secondary
                                },
                            );
                            if eresp.on_hover_text("Emoji").clicked() {
                                self.emoji_picker_target =
                                    if self.emoji_picker_target == Some(None) {
                                        None
                                    } else {
                                        Some(None)
                                    };
                            }

                            // Crisp circular Send button
                            let can_send = !self.draft.trim().is_empty();
                            let (sr, sresp) =
                                ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
                            ui.painter().circle_filled(
                                sr.center(),
                                15.0,
                                if can_send { p.accent } else { p.surface_alt },
                            );
                            icons::paint_send(
                                ui.painter(),
                                sr.center() + egui::vec2(1.0, 0.0),
                                15.0,
                                if can_send {
                                    Color32::WHITE
                                } else {
                                    p.text_muted
                                },
                            );
                            if sresp.on_hover_text("Send message").clicked() && can_send {
                                self.send_message();
                                field.request_focus();
                                self.refocus_draft = true;
                            }
                        });
                    });
                    if has_focus {
                        ui.painter().rect_stroke(
                            composer_resp.response.rect.expand(2.0),
                            egui::CornerRadius {
                                nw: top_radius.saturating_add(2),
                                ne: top_radius.saturating_add(2),
                                sw: 22,
                                se: 22,
                            },
                            Stroke::new(2.0_f32, p.accent.gamma_multiply(0.28)),
                            egui::StrokeKind::Outside,
                        );
                    }
                    });
                });
            });

        // Message Stream
        let mut pending_load_older = false;
        let mut pending_save_edit: Option<(i64, String)> = None;
        let mut pending_cancel_edit = false;
        let mut pending_delete: Option<i64> = None;
        let mut pending_toggle_reaction: Option<(i64, String)> = None;
        let mut pending_open_emoji_for: Option<i64> = None;
        let mut pending_reply: Option<(i64, String, String)> = None;
        let mut pending_profile: Option<(i64, String, Pos2)> = None;
        let mut finished_msg_anims: Vec<i64> = Vec::new();

        let search_filter = if self.chat_search_open {
            self.chat_msg_search.trim().to_ascii_lowercase()
        } else {
            String::new()
        };

        let all_messages = self.messages.clone();
        let compact = self.config.compact_mode;
        let font_sz = self.config.font_size;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.canvas))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        if self.has_more_messages {
                            ui.add_space(12.0);
                            ui.vertical_centered(|ui| {
                                let label = if self.loading_older {
                                    "Loading older messages..."
                                } else {
                                    "Load older messages"
                                };
                                if ui
                                    .add_enabled(
                                        !self.loading_older,
                                        egui::Button::new(
                                            RichText::new(label).small().color(p.text_secondary),
                                        )
                                        .fill(p.surface)
                                        .corner_radius(14.0),
                                    )
                                    .clicked()
                                {
                                    pending_load_older = true;
                                }
                            });
                            ui.add_space(10.0);
                        } else {
                            // Clean centered Wireline conversation start hero
                            ui.add_space(28.0);
                            ui.vertical_centered(|ui| {
                                let (av_r, av_resp) =
                                    ui.allocate_exact_size(egui::vec2(68.0, 68.0), Sense::click());
                                self.paint_user_avatar(
                                    ui,
                                    av_r,
                                    channel.peer_user_id.unwrap_or(-1),
                                    &channel.name,
                                    false,
                                );
                                if av_resp.clicked() {
                                    if let Some(peer_id) = channel.peer_user_id {
                                        pending_profile = Some((
                                            peer_id,
                                            channel.name.clone(),
                                            av_r.right_top(),
                                        ));
                                    }
                                }
                                ui.add_space(8.0);
                                ui.label(theme::semibold(&channel.name, 20.0).color(p.text_primary));
                                ui.label(
                                    RichText::new(format!(
                                        "Start of your private conversation with {}.",
                                        channel.name
                                    ))
                                    .size(13.5)
                                    .color(p.text_muted),
                                );
                            });
                            ui.add_space(18.0);
                        }

                        let visible_indices: Vec<usize> = all_messages
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| {
                                !self.is_blocked(m.user_id)
                                    && (search_filter.is_empty()
                                        || m.content
                                            .to_ascii_lowercase()
                                            .contains(&search_filter)
                                        || m.username
                                            .to_ascii_lowercase()
                                            .contains(&search_filter))
                            })
                            .map(|(i, _)| i)
                            .collect();

                        for (vis_pos, &index) in visible_indices.iter().enumerate() {
                            let message = &all_messages[index];
                            let prev_msg = vis_pos
                                .checked_sub(1)
                                .map(|idx| &all_messages[visible_indices[idx]]);

                            let show_date_sep = match prev_msg {
                                None => true,
                                Some(prev) => {
                                    day_index(prev.created_at) != day_index(message.created_at)
                                }
                            };
                            if show_date_sep {
                                let label = format_day_label(message.created_at, now_i64);
                                Self::ui_date_pill(ui, &label, p);
                            }

                            let mine = message.user_id == me_id;
                            let is_call_event = message.content.starts_with("[[call:")
                                && message.content.ends_with("]]");
                            if is_call_event {
                                ui.add_space(8.0);
                                Self::ui_call_event_card(ui, p, message);
                                ui.add_space(8.0);
                                continue;
                            }
                            let prev_was_call = prev_msg.is_some_and(|prev| {
                                prev.content.starts_with("[[call:") && prev.content.ends_with("]]")
                            });
                            let has_reply = message.reply_to_id.is_some();
                            let grouped = !compact
                                && !show_date_sep
                                && !prev_was_call
                                && !has_reply
                                && prev_msg.is_some_and(|prev| {
                                    prev.user_id == message.user_id
                                        && (message.created_at - prev.created_at).abs() < 300
                                });

                            if !grouped && !compact && vis_pos > 0 && !show_date_sep {
                                ui.add_space(14.0);
                            }

                            let is_editing = self
                                .editing_message
                                .as_ref()
                                .is_some_and(|(id, _)| *id == message.id);

                            let (msg_alpha, msg_y_off) =
                                if let Some(&started_at) = self.new_message_anims.get(&message.id) {
                                    let (alpha, y_off, _scale, done) = anim::message_in(
                                        (now - started_at) as f32,
                                        self.config.reduced_motion,
                                    );
                                    if done {
                                        finished_msg_anims.push(message.id);
                                        (1.0, 0.0)
                                    } else {
                                        self.animating = true;
                                        (alpha, y_off)
                                    }
                                } else {
                                    (1.0, 0.0)
                                };

                            let avail_w = ui.available_width();
                            let col_w = avail_w.min(980.0);
                            let side_pad = ((avail_w - col_w) * 0.5).max(0.0);
                            let row_start = ui.cursor().min + egui::vec2(side_pad, 0.0);
                            let row_w = col_w;

                            let row_resp = ui
                                .allocate_new_ui(
                                    egui::UiBuilder::new()
                                        .max_rect(Rect::from_min_size(
                                            row_start + egui::vec2(0.0, msg_y_off),
                                            egui::vec2(row_w, 10_000.0),
                                        ))
                                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                                    |ui| {
                                        if msg_alpha < 0.999 {
                                            ui.multiply_opacity(msg_alpha);
                                        }
                                        egui::Frame::new()
                                            .inner_margin(egui::Margin {
                                                left: 20,
                                                right: 20,
                                                top: if grouped || compact { 1 } else { 3 },
                                                bottom: 1,
                                            })
                                            .show(ui, |ui| {
                                                ui.set_width((ui.available_width()).max(120.0));

                                                if compact {
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.label(
                                                            RichText::new(format_hhmm(
                                                                message.created_at,
                                                            ))
                                                            .size(11.0)
                                                            .color(p.text_muted),
                                                        );
                                                        let u_resp = ui.add(
                                                            egui::Label::new(
                                                                theme::semibold(
                                                                    &message.username,
                                                                    font_sz,
                                                                )
                                                                .color(if mine {
                                                                    p.accent
                                                                } else {
                                                                    p.text_primary
                                                                }),
                                                            )
                                                            .sense(Sense::click()),
                                                        );
                                                        if u_resp.clicked() {
                                                            pending_profile = Some((
                                                                message.user_id,
                                                                message.username.clone(),
                                                                u_resp.rect.right_top(),
                                                            ));
                                                        }
                                                        if is_editing {
                                                            if let Some((_, ref mut buf)) =
                                                                self.editing_message
                                                            {
                                                                Self::ui_inline_editor(
                                                                    ui,
                                                                    p,
                                                                    message.id,
                                                                    buf,
                                                                    &mut pending_save_edit,
                                                                    &mut pending_cancel_edit,
                                                                );
                                                            }
                                                        } else {
                                                            Self::render_rich_message(
                                                                ui,
                                                                p,
                                                                message,
                                                                font_sz,
                                                                p.text_primary,
                                                                false,
                                                                &mut self.revealed_spoilers,
                                                            );
                                                        }
                                                    });
                                                } else {
                                                    // Original Wireline Modern Bubble Layout
                                                    let max_bubble_w =
                                                        (ui.available_width() * 0.72).clamp(220.0, 560.0);

                                                    if mine {
                                                        // Right-aligned own indigo bubble!
                                                        ui.with_layout(
                                                            egui::Layout::top_down(egui::Align::Max),
                                                            |ui| {
                                                                if let Some(reply_id) =
                                                                    message.reply_to_id
                                                                {
                                                                    Self::ui_reply_quote(
                                                                        ui,
                                                                        p,
                                                                        &all_messages,
                                                                        reply_id,
                                                                    );
                                                                }
                                                                egui::Frame::new()
                                                                    .fill(p.own_bubble)
                                                                    .corner_radius(
                                                                        egui::CornerRadius {
                                                                            nw: 16,
                                                                            ne: if grouped { 6 } else { 16 },
                                                                            sw: 16,
                                                                            se: 4,
                                                                        },
                                                                    )
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            14, 9,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        ui.set_max_width(
                                                                            max_bubble_w,
                                                                        );
                                                                        if is_editing {
                                                                            if let Some((
                                                                                _,
                                                                                ref mut buf,
                                                                            )) =
                                                                                self.editing_message
                                                                            {
                                                                                Self::ui_inline_editor(
                                                                                    ui,
                                                                                    p,
                                                                                    message.id,
                                                                                    buf,
                                                                                    &mut pending_save_edit,
                                                                                    &mut pending_cancel_edit,
                                                                                );
                                                                            }
                                                                        } else {
                                                                            Self::render_rich_message(
                                                                                ui,
                                                                                p,
                                                                                message,
                                                                                font_sz,
                                                                                p.own_bubble_text,
                                                                                true,
                                                                                &mut self
                                                                                    .revealed_spoilers,
                                                                            );
                                                                        }
                                                                    });
                                                                if !grouped || message.edited_at.is_some() {
                                                                    ui.horizontal(|ui| {
                                                                        if message.edited_at.is_some() {
                                                                            ui.label(
                                                                                RichText::new("(edited)")
                                                                                    .size(11.5)
                                                                                    .color(p.text_muted),
                                                                            );
                                                                        }
                                                                        ui.label(
                                                                            RichText::new(format_hhmm(
                                                                                message.created_at,
                                                                            ))
                                                                            .size(11.5)
                                                                            .color(p.text_muted),
                                                                        );
                                                                    });
                                                                }
                                                                Self::ui_reactions_row(
                                                                    ui,
                                                                    p,
                                                                    message,
                                                                    me_id,
                                                                    &mut pending_toggle_reaction,
                                                                    &mut pending_open_emoji_for,
                                                                );
                                                            },
                                                        );
                                                    } else {
                                                        // Left-aligned peer bubble with 38px circular avatar
                                                        ui.horizontal_top(|ui| {
                                                            if grouped {
                                                                ui.add_space(48.0);
                                                            } else {
                                                                let (av_r, av_resp) =
                                                                    ui.allocate_exact_size(
                                                                        egui::vec2(38.0, 38.0),
                                                                        Sense::click(),
                                                                    );
                                                                self.paint_user_avatar(
                                                                    ui,
                                                                    av_r,
                                                                    message.user_id,
                                                                    &message.username,
                                                                    false,
                                                                );
                                                                if av_resp.clicked() {
                                                                    pending_profile = Some((
                                                                        message.user_id,
                                                                        message.username.clone(),
                                                                        av_r.right_top(),
                                                                    ));
                                                                }
                                                                ui.add_space(10.0);
                                                            }

                                                            ui.vertical(|ui| {
                                                                if !grouped {
                                                                    ui.horizontal(|ui| {
                                                                        let u_resp = ui.add(
                                                                            egui::Label::new(
                                                                                theme::semibold(
                                                                                    &message.username,
                                                                                    14.0,
                                                                                )
                                                                                .color(p.text_primary),
                                                                            )
                                                                            .sense(Sense::click()),
                                                                        );
                                                                        if u_resp.clicked() {
                                                                            pending_profile = Some((
                                                                                message.user_id,
                                                                                message.username.clone(),
                                                                                u_resp.rect.right_top(),
                                                                            ));
                                                                        }
                                                                        ui.label(
                                                                            RichText::new(
                                                                                format_discord_timestamp(
                                                                                    message.created_at,
                                                                                    now_i64,
                                                                                ),
                                                                            )
                                                                            .size(12.0)
                                                                            .color(p.text_muted),
                                                                        );
                                                                    });
                                                                }
                                                                if let Some(reply_id) =
                                                                    message.reply_to_id
                                                                {
                                                                    Self::ui_reply_quote(
                                                                        ui,
                                                                        p,
                                                                        &all_messages,
                                                                        reply_id,
                                                                    );
                                                                }
                                                                egui::Frame::new()
                                                                    .fill(p.peer_bubble)
                                                                    .stroke(p.border_stroke())
                                                                    .corner_radius(
                                                                        egui::CornerRadius {
                                                                            nw: if grouped { 6 } else { 4 },
                                                                            ne: 16,
                                                                            sw: 16,
                                                                            se: 16,
                                                                        },
                                                                    )
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            14, 9,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        ui.set_max_width(
                                                                            max_bubble_w,
                                                                        );
                                                                        Self::render_rich_message(
                                                                            ui,
                                                                            p,
                                                                            message,
                                                                            font_sz,
                                                                            p.text_primary,
                                                                            false,
                                                                            &mut self
                                                                                .revealed_spoilers,
                                                                        );
                                                                    });
                                                                if message.edited_at.is_some() {
                                                                    ui.label(
                                                                        RichText::new("(edited)")
                                                                            .size(10.5)
                                                                            .color(p.text_muted),
                                                                    );
                                                                }
                                                                Self::ui_reactions_row(
                                                                    ui,
                                                                    p,
                                                                    message,
                                                                    me_id,
                                                                    &mut pending_toggle_reaction,
                                                                    &mut pending_open_emoji_for,
                                                                );
                                                            });
                                                        });
                                                    }
                                                }
                                            });
                                    },
                                )
                                .response;

                            let row_rect = Rect::from_min_max(
                                row_start,
                                egui::pos2(row_start.x + row_w, row_resp.rect.bottom()),
                            );
                            let pointer_in_row = ui.ctx().input(|i| {
                                i.pointer
                                    .hover_pos()
                                    .is_some_and(|pos| row_rect.contains(pos))
                            });

                            // Floating pill hover action bar (cleanly placed on top-right or top-left)
                            if pointer_in_row && !is_editing {
                                let bar_btns = if mine { 4 } else { 2 };
                                let bar_w = (bar_btns as f32) * 30.0 + 8.0;
                                let bar_x = if mine && !compact {
                                    row_rect.left() + 24.0
                                } else {
                                    row_rect.right() - bar_w - 20.0
                                };
                                let bar_rect = Rect::from_min_size(
                                    egui::pos2(
                                        bar_x,
                                        (row_rect.top() - 8.0).max(ui.clip_rect().top()),
                                    ),
                                    egui::vec2(bar_w, 28.0),
                                );
                                ui.painter().rect_filled(bar_rect, 8.0, p.floating);
                                ui.painter().rect_stroke(
                                    bar_rect,
                                    8.0,
                                    p.border_stroke(),
                                    egui::StrokeKind::Inside,
                                );

                                let mut bx = bar_rect.left() + 19.0;
                                let by = bar_rect.center().y;

                                // React
                                let r1 = Rect::from_center_size(
                                    egui::pos2(bx, by),
                                    egui::vec2(26.0, 24.0),
                                );
                                let resp1 = ui.interact(
                                    r1,
                                    ui.id().with(("act_react", message.id)),
                                    Sense::click(),
                                );
                                if resp1.hovered() {
                                    ui.painter().rect_filled(r1, 6.0, p.hover);
                                }
                                icons::paint_smile(
                                    ui.painter(),
                                    r1.center(),
                                    16.0,
                                    if resp1.hovered() {
                                        p.text_primary
                                    } else {
                                        p.text_secondary
                                    },
                                );
                                if resp1.on_hover_text("React").clicked() {
                                    pending_open_emoji_for = Some(message.id);
                                }
                                bx += 30.0;

                                // Reply
                                let r2 = Rect::from_center_size(
                                    egui::pos2(bx, by),
                                    egui::vec2(26.0, 24.0),
                                );
                                let resp2 = ui.interact(
                                    r2,
                                    ui.id().with(("act_reply", message.id)),
                                    Sense::click(),
                                );
                                if resp2.hovered() {
                                    ui.painter().rect_filled(r2, 6.0, p.hover);
                                }
                                icons::paint_reply(
                                    ui.painter(),
                                    r2.center(),
                                    16.0,
                                    if resp2.hovered() {
                                        p.text_primary
                                    } else {
                                        p.text_secondary
                                    },
                                );
                                if resp2.on_hover_text("Reply").clicked() {
                                    pending_reply = Some((
                                        message.id,
                                        message.username.clone(),
                                        message.content.clone(),
                                    ));
                                }

                                if mine {
                                    bx += 30.0;
                                    let r3 = Rect::from_center_size(
                                        egui::pos2(bx, by),
                                        egui::vec2(26.0, 24.0),
                                    );
                                    let resp3 = ui.interact(
                                        r3,
                                        ui.id().with(("act_edit", message.id)),
                                        Sense::click(),
                                    );
                                    if resp3.hovered() {
                                        ui.painter().rect_filled(r3, 6.0, p.hover);
                                    }
                                    icons::paint_pencil(
                                        ui.painter(),
                                        r3.center(),
                                        15.0,
                                        if resp3.hovered() {
                                            p.text_primary
                                        } else {
                                            p.text_secondary
                                        },
                                    );
                                    if resp3.on_hover_text("Edit").clicked() {
                                        self.editing_message =
                                            Some((message.id, message.content.clone()));
                                    }

                                    bx += 30.0;
                                    let r4 = Rect::from_center_size(
                                        egui::pos2(bx, by),
                                        egui::vec2(26.0, 24.0),
                                    );
                                    let resp4 = ui.interact(
                                        r4,
                                        ui.id().with(("act_del", message.id)),
                                        Sense::click(),
                                    );
                                    if resp4.hovered() {
                                        ui.painter().rect_filled(r4, 6.0, p.danger);
                                    }
                                    icons::paint_trash(
                                        ui.painter(),
                                        r4.center(),
                                        15.0,
                                        if resp4.hovered() {
                                            Color32::WHITE
                                        } else {
                                            p.danger
                                        },
                                    );
                                    if resp4.on_hover_text("Delete").clicked() {
                                        pending_delete = Some(message.id);
                                    }
                                }
                            }
                        }

                        ui.add_space(12.0);
                    });
            });

        if pending_load_older {
            if let Some(oldest_id) = self.messages.first().map(|m| m.id) {
                self.load_messages(channel.id, Some(oldest_id));
            }
        }
        if let Some((id, text)) = pending_save_edit {
            self.save_message_edit(id, text);
        }
        if pending_cancel_edit {
            self.editing_message = None;
        }
        if let Some(id) = pending_delete {
            self.delete_message(id);
        }
        if let Some((msg_id, emoji)) = pending_toggle_reaction {
            self.toggle_reaction(channel.id, msg_id, emoji);
        }
        if let Some(msg_id) = pending_open_emoji_for {
            self.emoji_picker_target = Some(Some(msg_id));
        }
        if let Some(r) = pending_reply {
            self.reply_to = Some(r);
            self.refocus_draft = true;
        }
        for id in finished_msg_anims {
            self.new_message_anims.remove(&id);
        }
        if let Some((uid, uname, pos)) = pending_profile {
            self.open_profile_popup(uid, uname, pos);
        }
    }

    fn ui_call_event_card(ui: &mut Ui, p: Palette, message: &MessageDto) {
        let content = message.content.as_str();
        let (icon_col, text) = if content == "[[call:started]]" {
            (
                p.success,
                format!(
                    "{} started a call · {}",
                    message.username,
                    format_hhmm(message.created_at)
                ),
            )
        } else if content == "[[call:missed]]" {
            (
                p.danger,
                format!(
                    "Missed call from {} · {}",
                    message.username,
                    format_hhmm(message.created_at)
                ),
            )
        } else if let Some(dur) = content
            .strip_prefix("[[call:ended:")
            .and_then(|s| s.strip_suffix("]]"))
        {
            (
                p.text_secondary,
                format!(
                    "Call ended · {} · {}",
                    dur,
                    format_hhmm(message.created_at)
                ),
            )
        } else {
            (p.text_secondary, content.to_string())
        };
        let is_missed = content == "[[call:missed]]";
        ui.vertical_centered(|ui| {
            egui::Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_missed {
                        p.danger.gamma_multiply(0.4)
                    } else {
                        p.border
                    },
                ))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(14, 7))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (ir, _) =
                            ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
                        if is_missed {
                            icons::paint_phone_off(ui.painter(), ir.center(), 15.0, icon_col);
                        } else {
                            icons::paint_phone(ui.painter(), ir.center(), 15.0, icon_col);
                        }
                        ui.label(theme::semibold(text, 13.0).color(p.text_primary));
                    });
                });
        });
    }

    fn ui_date_pill(ui: &mut Ui, label: &str, p: Palette) {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            egui::Frame::new()
                .fill(p.surface)
                .stroke(p.border_stroke())
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.label(theme::semibold(label, 12.0).color(p.text_secondary));
                });
        });
        ui.add_space(10.0);
    }

    fn ui_reply_quote(ui: &mut Ui, p: Palette, all_messages: &[MessageDto], reply_id: i64) {
        let ref_msg = all_messages.iter().find(|m| m.id == reply_id);
        egui::Frame::new()
            .fill(p.surface)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(rm) = ref_msg {
                        ui.label(theme::semibold(&rm.username, 11.5).color(p.accent));
                        let clip: String = if rm.content.chars().count() > 44 {
                            format!("{}...", rm.content.chars().take(44).collect::<String>())
                        } else {
                            rm.content.clone()
                        };
                        ui.label(RichText::new(clip).size(11.5).color(p.text_muted));
                    } else {
                        ui.label(
                            RichText::new("Original message deleted")
                                .size(11.5)
                                .italics()
                                .color(p.text_muted),
                        );
                    }
                });
            });
    }

    fn ui_reactions_row(
        ui: &mut Ui,
        p: Palette,
        message: &MessageDto,
        me_id: i64,
        pending_toggle: &mut Option<(i64, String)>,
        pending_open: &mut Option<i64>,
    ) {
        if message.reactions.is_empty() {
            return;
        }
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
            for react in &message.reactions {
                let i_reacted = react.user_ids.contains(&me_id);
                let pill_bg = if i_reacted { p.accent_soft } else { p.surface };
                let pill_stroke = if i_reacted {
                    Stroke::new(1.0_f32, p.accent)
                } else {
                    p.border_stroke()
                };
                let label = format!("{} {}", react.emoji, react.user_ids.len());
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).size(12.5).color(if i_reacted {
                            p.text_primary
                        } else {
                            p.text_secondary
                        }))
                        .fill(pill_bg)
                        .stroke(pill_stroke)
                        .corner_radius(10.0),
                    )
                    .clicked()
                {
                    *pending_toggle = Some((message.id, react.emoji.clone()));
                }
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("+").size(12.5).color(p.text_muted))
                        .fill(p.surface)
                        .stroke(p.border_stroke())
                        .corner_radius(10.0),
                )
                .on_hover_text("Add Reaction")
                .clicked()
            {
                *pending_open = Some(message.id);
            }
        });
    }

    fn ui_inline_editor(
        ui: &mut Ui,
        p: Palette,
        message_id: i64,
        edit_buf: &mut String,
        pending_save: &mut Option<(i64, String)>,
        pending_cancel: &mut bool,
    ) {
        egui::Frame::new()
            .fill(p.input_bg)
            .stroke(p.focus_stroke())
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                let edit_resp = ui.add(
                    egui::TextEdit::singleline(edit_buf)
                        .frame(false)
                        .desired_width((ui.available_width() - 12.0).max(200.0)),
                );
                if edit_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    *pending_save = Some((message_id, edit_buf.clone()));
                }
            });
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(theme::semibold("Save", 11.5).color(Color32::WHITE))
                        .fill(p.accent)
                        .corner_radius(6.0),
                )
                .clicked()
            {
                *pending_save = Some((message_id, edit_buf.clone()));
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Cancel").size(11.5).color(p.text_secondary))
                        .fill(p.surface)
                        .corner_radius(6.0),
                )
                .clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                *pending_cancel = true;
            }
        });
    }

    fn paint_user_avatar(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        user_id: i64,
        username: &str,
        mine: bool,
    ) {
        let hash = if mine {
            self.me.as_ref().and_then(|m| m.avatar_hash.clone())
        } else {
            self.friends
                .iter()
                .find(|f| f.user_id == user_id)
                .and_then(|f| f.avatar_hash.clone())
        };
        let mut drew = false;
        if let Some(h) = hash.as_deref() {
            let tex = self.avatars.texture(
                &ui.ctx().clone(),
                &self.api(),
                self.token.as_deref(),
                user_id,
                Some(h),
                &self.events_tx,
            );
            if let Some(t) = tex {
                let p = theme::of(ui.ctx());
                ui.painter()
                    .circle_filled(rect.center(), rect.width() * 0.5, p.surface_alt);
                ui.painter().image(
                    t.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                drew = true;
            }
        }
        if !drew {
            ui.painter().circle_filled(
                rect.center(),
                rect.width() * 0.5,
                avatar_color(username, mine),
            );
            let initial = username
                .chars()
                .next()
                .unwrap_or('?')
                .to_ascii_uppercase()
                .to_string();
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::new((rect.width() * 0.42).max(12.0), theme::semibold_family()),
                Color32::WHITE,
            );
        }
    }

    fn render_rich_message(
        ui: &mut Ui,
        p: Palette,
        message: &MessageDto,
        font_sz: f32,
        base_color: Color32,
        on_accent: bool,
        revealed_spoilers: &mut std::collections::HashSet<(i64, usize)>,
    ) {
        let content = message.content.as_str();
        let mut spoiler_idx = 0usize;
        let parts: Vec<&str> = content.split("```").collect();
        for (pi, part) in parts.iter().enumerate() {
            if pi % 2 == 1 {
                let trimmed = part.trim_matches('\n');
                let (lang, code_body) = if let Some((first, rest)) = trimmed.split_once('\n') {
                    if !first.contains(' ') && first.len() <= 16 {
                        (Some(first.trim()), rest)
                    } else {
                        (None, trimmed)
                    }
                } else {
                    (None, trimmed)
                };
                egui::Frame::new()
                    .fill(if on_accent {
                        Color32::from_black_alpha(55)
                    } else {
                        p.surface_alt
                    })
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        if let Some(l) = lang {
                            if !l.is_empty() {
                                ui.label(
                                    RichText::new(l)
                                        .size(10.5)
                                        .monospace()
                                        .color(if on_accent {
                                            Color32::from_white_alpha(180)
                                        } else {
                                            p.text_muted
                                        }),
                                );
                            }
                        }
                        ui.label(
                            RichText::new(code_body)
                                .monospace()
                                .size((font_sz - 1.5).max(11.5))
                                .color(base_color),
                        );
                    });
            } else {
                for line in part.lines() {
                    if let Some(quoted) = line.strip_prefix("> ") {
                        ui.horizontal(|ui| {
                            let (bar, _) = ui.allocate_exact_size(
                                egui::vec2(3.0, font_sz + 4.0),
                                Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                bar,
                                1.5,
                                if on_accent {
                                    Color32::from_white_alpha(160)
                                } else {
                                    p.accent
                                },
                            );
                            Self::render_inline_markdown(
                                ui,
                                p,
                                message.id,
                                quoted,
                                font_sz,
                                base_color,
                                on_accent,
                                revealed_spoilers,
                                &mut spoiler_idx,
                            );
                        });
                    } else if !line.is_empty() {
                        Self::render_inline_markdown(
                            ui,
                            p,
                            message.id,
                            line,
                            font_sz,
                            base_color,
                            on_accent,
                            revealed_spoilers,
                            &mut spoiler_idx,
                        );
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_inline_markdown(
        ui: &mut Ui,
        p: Palette,
        msg_id: i64,
        line: &str,
        font_sz: f32,
        base_color: Color32,
        on_accent: bool,
        revealed_spoilers: &mut std::collections::HashSet<(i64, usize)>,
        spoiler_idx: &mut usize,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
            let mut rest = line;
            while !rest.is_empty() {
                if let Some(after) = rest.strip_prefix("||") {
                    if let Some(end) = after.find("||") {
                        let inner = &after[..end];
                        let idx = *spoiler_idx;
                        *spoiler_idx += 1;
                        let revealed = revealed_spoilers.contains(&(msg_id, idx));
                        let hide_bg = if on_accent {
                            Color32::from_black_alpha(120)
                        } else {
                            p.surface_alt
                        };
                        let btn = egui::Button::new(RichText::new(inner).size(font_sz).color(
                            if revealed { base_color } else { hide_bg },
                        ))
                        .fill(if revealed {
                            Color32::from_black_alpha(30)
                        } else {
                            hide_bg
                        })
                        .corner_radius(4.0);
                        if ui.add(btn).clicked() {
                            if revealed {
                                revealed_spoilers.remove(&(msg_id, idx));
                            } else {
                                revealed_spoilers.insert((msg_id, idx));
                            }
                        }
                        rest = &after[end + 2..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix('`') {
                    if let Some(end) = after.find('`') {
                        egui::Frame::new()
                            .fill(if on_accent {
                                Color32::from_black_alpha(55)
                            } else {
                                p.surface_alt
                            })
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(5, 1))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&after[..end])
                                        .monospace()
                                        .size((font_sz - 1.5).max(11.5))
                                        .color(base_color),
                                );
                            });
                        rest = &after[end + 1..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix("**") {
                    if let Some(end) = after.find("**") {
                        ui.label(theme::semibold(&after[..end], font_sz).color(base_color));
                        rest = &after[end + 2..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix("__") {
                    if let Some(end) = after.find("__") {
                        ui.label(
                            RichText::new(&after[..end])
                                .size(font_sz)
                                .underline()
                                .color(base_color),
                        );
                        rest = &after[end + 2..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix("~~") {
                    if let Some(end) = after.find("~~") {
                        ui.label(
                            RichText::new(&after[..end])
                                .size(font_sz)
                                .strikethrough()
                                .color(base_color.gamma_multiply(0.7)),
                        );
                        rest = &after[end + 2..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix('*') {
                    if let Some(end) = after.find('*') {
                        if end > 0 {
                            ui.label(
                                RichText::new(&after[..end])
                                    .size(font_sz)
                                    .italics()
                                    .color(base_color),
                            );
                            rest = &after[end + 1..];
                            continue;
                        }
                    }
                }

                let next_ws = rest.find(char::is_whitespace).unwrap_or(rest.len());
                let token = if next_ws == 0 {
                    let trimmed = rest.trim_start();
                    let consumed = rest.len() - trimmed.len();
                    rest = &rest[consumed..];
                    continue;
                } else {
                    let tok = &rest[..next_ws];
                    rest = &rest[next_ws..];
                    tok
                };

                if token.starts_with("http://") || token.starts_with("https://") {
                    let link_col = if on_accent {
                        Color32::from_rgb(0xE0, 0xE7, 0xFF)
                    } else {
                        p.text_link
                    };
                    if ui
                        .add(egui::Link::new(
                            RichText::new(token).size(font_sz).underline().color(link_col),
                        ))
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(token));
                    }
                } else if token.starts_with('@') && token.len() > 1 {
                    egui::Frame::new()
                        .fill(if on_accent {
                            Color32::from_white_alpha(40)
                        } else {
                            p.accent_soft
                        })
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(4, 0))
                        .show(ui, |ui| {
                            ui.label(theme::semibold(token, font_sz).color(if on_accent {
                                Color32::WHITE
                            } else {
                                p.accent
                            }));
                        });
                } else {
                    ui.label(RichText::new(token).size(font_sz).color(base_color));
                }
            }
        });
    }
}
