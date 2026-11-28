//! Pixel-faithful Discord DM Chat View:
//! 48px top bar with inline search, disconnected banner, 80px conversation header,
//! date separators, Cozy (Format A & B) + Compact modes, rich Discord markdown (bold, italic,
//! underline, strike, inline/block code, blockquote, clickable spoilers, @mentions, links),
//! quoted replies, persisted reactions, floating hover action bar, animated typing dots,
//! and Discord #383a40 composer with Emoji Picker + Arrow-Up edit last message.

use super::{
    anim, avatar_color, day_index, format_day_label, format_discord_timestamp, format_hhmm, icons,
    SkyApp,
};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, MessageDto, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_chat_stage(&mut self, ui: &mut Ui) {
        let Some(channel) = self.current_channel().cloned() else {
            return;
        };

        // 1. 48px Top Bar (--bg-primary, border-bottom 1px subtle)
        let (hdr_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::hover());
        ui.painter().rect_filled(hdr_rect, 0.0, theme::BG_PRIMARY);
        ui.painter().line_segment(
            [hdr_rect.left_bottom(), hdr_rect.right_bottom()],
            theme::border(),
        );

        let hdr_inner = hdr_rect.shrink2(egui::vec2(16.0, 6.0));
        let peer_status = channel
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);
        let in_this_call = self.in_voice
            && self.voice_room.as_deref() == Some(Self::channel_room(channel.id).as_str());

        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(hdr_inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                // Left: @ symbol + Name + Discord status dot
                ui.label(RichText::new("@").size(19.0).strong().color(theme::TEXT_MUTED));
                ui.add_space(2.0);
                let name_resp = ui.add(
                    egui::Label::new(
                        RichText::new(&channel.name)
                            .size(16.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
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
                ui.add_space(4.0);
                let (dot_r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                let pulse = self.pulse_time(ui.ctx());
                icons::paint_status_dot_pulse(
                    ui.painter(),
                    dot_r.center(),
                    peer_status,
                    theme::BG_PRIMARY,
                    pulse,
                );

                // Right side: inline Search OR icon buttons (Phone, Video/Share, Pin, Search)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.chat_search_open {
                        let (xr, xresp) =
                            ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                        if xresp.hovered() {
                            ui.painter().rect_filled(xr, 4.0, theme::BG_HOVER);
                        }
                        icons::paint_x(ui.painter(), xr.center(), 14.0, theme::TEXT_SECONDARY);
                        if xresp.clicked() {
                            self.chat_search_open = false;
                            self.chat_msg_search.clear();
                        }

                        egui::Frame::new()
                            .fill(theme::INPUT_BG)
                            .stroke(theme::input_border())
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(8, 4))
                            .show(ui, |ui| {
                                let w = (ui.available_width().min(240.0)).max(120.0);
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.chat_msg_search)
                                        .frame(false)
                                        .desired_width(w)
                                        .hint_text("Search in conversation"),
                                );
                            });
                    } else {
                        // Search toggle
                        let (sr, sresp) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                        if sresp.hovered() {
                            ui.painter().rect_filled(sr, 4.0, theme::BG_HOVER);
                        }
                        icons::paint_search(
                            ui.painter(),
                            sr.center(),
                            17.0,
                            if sresp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if sresp.on_hover_text("Search in conversation").clicked() {
                            self.chat_search_open = true;
                        }

                        // Pin icon (toggles call stage visibility if in call, or shows hint)
                        let (pr, presp) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                        if presp.hovered() {
                            ui.painter().rect_filled(pr, 4.0, theme::BG_HOVER);
                        }
                        icons::paint_pin(
                            ui.painter(),
                            pr.center(),
                            17.0,
                            if presp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if presp.on_hover_text("Pinned Messages").clicked() {
                            self.notice = "No pinned messages in this conversation yet.".into();
                        }

                        // Video / Screen share icon
                        let (vr, vresp) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                        if vresp.hovered() || (in_this_call && self.sharing) {
                            ui.painter().rect_filled(
                                vr,
                                4.0,
                                if in_this_call && self.sharing {
                                    theme::ACCENT
                                } else {
                                    theme::BG_HOVER
                                },
                            );
                        }
                        icons::paint_video(
                            ui.painter(),
                            vr.center(),
                            18.0,
                            if in_this_call && self.sharing {
                                Color32::WHITE
                            } else if vresp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if vresp
                            .on_hover_text(if in_this_call && self.sharing {
                                "Stop Screen Share"
                            } else {
                                "Start Video / Screen Share"
                            })
                            .clicked()
                        {
                            if !in_this_call {
                                self.start_call(channel.id);
                            }
                            if self.in_voice {
                                self.toggle_screen_share();
                            }
                        }

                        // Phone icon (Start Voice Call)
                        let (cr, cresp) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                        if cresp.hovered() || in_this_call {
                            ui.painter().rect_filled(
                                cr,
                                4.0,
                                if in_this_call {
                                    theme::SUCCESS
                                } else {
                                    theme::BG_HOVER
                                },
                            );
                        }
                        icons::paint_phone(
                            ui.painter(),
                            cr.center(),
                            17.0,
                            if in_this_call {
                                Color32::WHITE
                            } else if cresp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if cresp
                            .on_hover_text(if in_this_call {
                                "Show / Expand Voice Call"
                            } else {
                                "Start Voice Call"
                            })
                            .clicked()
                        {
                            if in_this_call {
                                self.call_expanded = !self.call_expanded;
                            } else {
                                self.start_call(channel.id);
                            }
                        }
                    }
                });
            },
        );

        // 2. Disconnected warning banner
        if !self.connected {
            let (ban_r, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), Sense::hover());
            ui.painter()
                .rect_filled(ban_r, 0.0, Color32::from_rgb(0x47, 0x39, 0x1A));
            ui.painter().text(
                ban_r.center(),
                Align2::CENTER_CENTER,
                "Connecting to Wireline relay...",
                FontId::proportional(12.5),
                theme::WARNING,
            );
        }

        // 3. Expandable Voice/Video Call Stage
        if in_this_call && self.call_expanded {
            self.ui_call_stage(ui, &channel);
        }

        // 4. Messages + Pinned Composer Dock
        self.ui_messages(ui, &channel);
    }

    fn ui_messages(&mut self, ui: &mut Ui, channel: &ChannelInfo) {
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

        // Pin Typing Indicator + Reply Banner + Discord #383a40 Input Bar FIRST
        egui::TopBottomPanel::bottom("chat_composer_dock")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_PRIMARY)
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: 2,
                        bottom: 14,
                    }),
            )
            .show_inside(ui, |ui| {
                // Reply Banner
                if let Some((_, ref author, ref snippet)) = self.reply_to.clone() {
                    egui::Frame::new()
                        .fill(theme::BG_SECONDARY)
                        .corner_radius(egui::CornerRadius {
                            nw: 8,
                            ne: 8,
                            sw: 0,
                            se: 0,
                        })
                        .inner_margin(egui::Margin::symmetric(12, 6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Replying to @{author}"))
                                        .small()
                                        .strong()
                                        .color(theme::TEXT_SECONDARY),
                                );
                                let preview: String = if snippet.chars().count() > 48 {
                                    format!(
                                        "{}...",
                                        snippet.chars().take(48).collect::<String>()
                                    )
                                } else {
                                    snippet.clone()
                                };
                                ui.label(
                                    RichText::new(preview).small().color(theme::TEXT_MUTED),
                                );
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
                                                theme::TEXT_PRIMARY
                                            } else {
                                                theme::TEXT_MUTED
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

                // Input Bar (#383a40, 8px radius, min-height 44px)
                let top_radius = if self.reply_to.is_some() { 0 } else { 8 };
                egui::Frame::new()
                    .fill(theme::INPUT_BG)
                    .corner_radius(egui::CornerRadius {
                        nw: top_radius,
                        ne: top_radius,
                        sw: 8,
                        se: 8,
                    })
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            // Left "+" button (circle)
                            let (pr, presp) =
                                ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                            ui.painter().circle_filled(
                                pr.center(),
                                11.0,
                                if presp.hovered() {
                                    theme::TEXT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                },
                            );
                            icons::paint_plus(ui.painter(), pr.center(), 13.0, theme::INPUT_BG);
                            if presp.on_hover_text("Insert code block").clicked() {
                                self.draft.push_str("```\n\n```");
                                self.refocus_draft = true;
                            }

                            ui.add_space(4.0);

                            // Text input (stable ID so focus persists across reply banner toggles & sends)
                            let draft_id = egui::Id::new(("chat_draft_input", channel.id));
                            let input_w = (ui.available_width() - 40.0).max(120.0);
                            let field = ui.add_sized(
                                [input_w, 26.0],
                                egui::TextEdit::singleline(&mut self.draft)
                                    .id(draft_id)
                                    .frame(false)
                                    .hint_text(format!("Message @{}", channel.name)),
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

                            // Enter to send — immediately keep focus in the message input
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
                                // Discord-style type-anywhere auto-focus: runs AFTER TextEdit so
                                // Event::Text is never doubled and the first keystroke is never dropped.
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

                            // Right Emoji Picker smile button
                            let (er, eresp) =
                                ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::click());
                            if eresp.hovered() || self.emoji_picker_target == Some(None) {
                                ui.painter().rect_filled(er, 4.0, theme::BG_HOVER);
                            }
                            icons::paint_smile(
                                ui.painter(),
                                er.center(),
                                19.0,
                                if eresp.hovered() || self.emoji_picker_target == Some(None) {
                                    theme::WARNING
                                } else {
                                    theme::TEXT_SECONDARY
                                },
                            );
                            if eresp.on_hover_text("Pick Emoji").clicked() {
                                self.emoji_picker_target =
                                    if self.emoji_picker_target == Some(None) {
                                        None
                                    } else {
                                        Some(None)
                                    };
                            }
                        });
                    });

                // Animated 3-dot Typing Indicator below composer (200ms fade in/out + staggered 1.2s bounce)
                let (typ_r, _) =
                    ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), Sense::hover());
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
                    let dot_alpha = (self.typing_fade * 255.0).round().clamp(0.0, 255.0) as u8;
                    let dot_color = Color32::from_rgba_unmultiplied(0xB5, 0xBA, 0xC1, dot_alpha);
                    for (dot, delay) in [0.0_f64, 0.15, 0.30].iter().enumerate() {
                        let y_off =
                            anim::typing_dot_offset(t, *delay, self.config.reduced_motion);
                        let c = egui::pos2(
                            typ_r.left() + 8.0 + (dot as f32) * 9.0,
                            typ_r.center().y + 1.0 + y_off,
                        );
                        ui.painter().circle_filled(c, 3.0, dot_color);
                    }
                    ui.painter().text(
                        egui::pos2(typ_r.left() + 36.0, typ_r.center().y),
                        Align2::LEFT_CENTER,
                        &self.last_typing_text,
                        FontId::proportional(12.0),
                        dot_color,
                    );
                }
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
            .frame(egui::Frame::new().fill(theme::BG_PRIMARY))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        // Load older button OR Top-of-conversation 80px Discord header
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
                                            RichText::new(label)
                                                .small()
                                                .color(theme::TEXT_SECONDARY),
                                        )
                                        .fill(theme::BG_SECONDARY)
                                        .corner_radius(14.0),
                                    )
                                    .clicked()
                                {
                                    pending_load_older = true;
                                }
                            });
                            ui.add_space(12.0);
                        } else {
                            // Top of conversation header (80px avatar + 32px bold name + subtext)
                            egui::Frame::new()
                                .inner_margin(egui::Margin {
                                    left: 16,
                                    right: 16,
                                    top: 24,
                                    bottom: 16,
                                })
                                .show(ui, |ui| {
                                    let (av_r, av_resp) = ui.allocate_exact_size(
                                        egui::vec2(80.0, 80.0),
                                        Sense::click(),
                                    );
                                    ui.painter().circle_filled(
                                        av_r.center(),
                                        40.0,
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
                                        av_r.center(),
                                        Align2::CENTER_CENTER,
                                        initial,
                                        FontId::proportional(34.0),
                                        Color32::WHITE,
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
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new(&channel.name)
                                            .size(28.0)
                                            .strong()
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "This is the beginning of your direct message history with @{}.",
                                            channel.name
                                        ))
                                        .size(15.0)
                                        .color(theme::TEXT_MUTED),
                                    );
                                    ui.add_space(16.0);
                                    let (div, _) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), 1.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().rect_filled(div, 0.0, theme::BORDER_SUBTLE);
                                });
                        }

                        // Filtered message slice
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
                                .map(|p| &all_messages[visible_indices[p]]);

                            // Date separator when day changes
                            let show_date_sep = match prev_msg {
                                None => true,
                                Some(prev) => {
                                    day_index(prev.created_at) != day_index(message.created_at)
                                }
                            };
                            if show_date_sep {
                                let label = format_day_label(message.created_at, now_i64);
                                Self::ui_date_separator(ui, &label);
                            }

                            let mine = message.user_id == me_id;
                            let has_reply = message.reply_to_id.is_some();
                            let grouped = !compact
                                && !show_date_sep
                                && !has_reply
                                && prev_msg.is_some_and(|prev| {
                                    prev.user_id == message.user_id
                                        && (message.created_at - prev.created_at).abs() < 300
                                });

                            if !grouped && !compact && vis_pos > 0 && !show_date_sep {
                                ui.add_space(12.0);
                            }

                            let is_editing = self
                                .editing_message
                                .as_ref()
                                .is_some_and(|(id, _)| *id == message.id);

                            // #2 Message send/receive spring bounce (freshly appended messages only)
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

                            // Measure row hover by pre-allocating a background shape placeholder
                            let bg_shape = ui.painter().add(egui::Shape::Noop);
                            let row_start = ui.cursor().min;

                            let row_resp = ui
                                .allocate_new_ui(
                                    egui::UiBuilder::new()
                                        .max_rect(Rect::from_min_size(
                                            row_start + egui::vec2(0.0, msg_y_off),
                                            egui::vec2(ui.available_width(), 10_000.0),
                                        ))
                                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                                    |ui| {
                                        if msg_alpha < 0.999 {
                                            ui.multiply_opacity(msg_alpha);
                                        }
                                        egui::Frame::new()
                                            .inner_margin(egui::Margin {
                                                left: 16,
                                                right: 16,
                                                top: if grouped || compact { 2 } else { 4 },
                                                bottom: 2,
                                            })
                                            .show(ui, |ui| {
                                                ui.set_width((ui.available_width()).max(120.0));

                                                // Quoted Reply reference bar
                                                if let Some(reply_id) = message.reply_to_id {
                                                    let ref_msg = all_messages
                                                        .iter()
                                                        .find(|m| m.id == reply_id);
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(20.0);
                                                        let (conn, _) = ui.allocate_exact_size(
                                                            egui::vec2(30.0, 14.0),
                                                            Sense::hover(),
                                                        );
                                                        let c_s = Stroke::new(
                                                            1.5_f32,
                                                            theme::TEXT_MUTED,
                                                        );
                                                        ui.painter().line_segment(
                                                            [
                                                                egui::pos2(conn.left(), conn.bottom()),
                                                                egui::pos2(conn.left(), conn.center().y),
                                                            ],
                                                            c_s,
                                                        );
                                                        ui.painter().line_segment(
                                                            [
                                                                egui::pos2(conn.left(), conn.center().y),
                                                                egui::pos2(conn.right(), conn.center().y),
                                                            ],
                                                            c_s,
                                                        );
                                                        if let Some(rm) = ref_msg {
                                                            ui.label(
                                                                RichText::new(format!(
                                                                    "@{}",
                                                                    rm.username
                                                                ))
                                                                .size(12.0)
                                                                .strong()
                                                                .color(theme::TEXT_SECONDARY),
                                                            );
                                                            let clip: String =
                                                                if rm.content.chars().count() > 64 {
                                                                    format!(
                                                                        "{}...",
                                                                        rm.content
                                                                            .chars()
                                                                            .take(64)
                                                                            .collect::<String>()
                                                                    )
                                                                } else {
                                                                    rm.content.clone()
                                                                };
                                                            ui.label(
                                                                RichText::new(clip)
                                                                    .size(12.0)
                                                                    .color(theme::TEXT_MUTED),
                                                            );
                                                        } else {
                                                            ui.label(
                                                                RichText::new(
                                                                    "Original message was deleted",
                                                                )
                                                                .size(12.0)
                                                                .italics()
                                                                .color(theme::TEXT_MUTED),
                                                            );
                                                        }
                                                    });
                                                }

                                                if compact {
                                                    // Compact Mode: [HH:MM] username  message
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.label(
                                                            RichText::new(format_hhmm(
                                                                message.created_at,
                                                            ))
                                                            .size(11.0)
                                                            .color(theme::TEXT_MUTED),
                                                        );
                                                        let u_resp = ui.add(
                                                            egui::Label::new(
                                                                RichText::new(&message.username)
                                                                    .size(font_sz)
                                                                    .strong()
                                                                    .color(if mine {
                                                                        theme::TEXT_PRIMARY
                                                                    } else {
                                                                        avatar_color(
                                                                            &message.username,
                                                                            false,
                                                                        )
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
                                                                    message.id,
                                                                    buf,
                                                                    &mut pending_save_edit,
                                                                    &mut pending_cancel_edit,
                                                                );
                                                            }
                                                        } else {
                                                            Self::render_rich_message(
                                                                ui,
                                                                message,
                                                                font_sz,
                                                                &mut self.revealed_spoilers,
                                                            );
                                                            if message.edited_at.is_some() {
                                                                ui.label(
                                                                    RichText::new("(edited)")
                                                                        .size(10.5)
                                                                        .color(theme::TEXT_MUTED),
                                                                );
                                                            }
                                                        }
                                                    });
                                                } else {
                                                    // Cozy Mode: Format A (40px avatar) & Format B (56px gutter)
                                                    ui.horizontal_top(|ui| {
                                                        if grouped {
                                                            let (gr, _) = ui.allocate_exact_size(
                                                                egui::vec2(48.0, 18.0),
                                                                Sense::hover(),
                                                            );
                                                            // Hover timestamp drawn later if hovered
                                                            let _ = gr;
                                                        } else {
                                                            let (av_r, av_resp) =
                                                                ui.allocate_exact_size(
                                                                    egui::vec2(40.0, 40.0),
                                                                    Sense::click(),
                                                                );
                                                            self.paint_user_avatar(
                                                                ui,
                                                                av_r,
                                                                message.user_id,
                                                                &message.username,
                                                                mine,
                                                            );
                                                            if av_resp.clicked() {
                                                                pending_profile = Some((
                                                                    message.user_id,
                                                                    message.username.clone(),
                                                                    av_r.right_top(),
                                                                ));
                                                            }
                                                            ui.add_space(8.0);
                                                        }

                                                        ui.vertical(|ui| {
                                                            ui.set_width(
                                                                (ui.available_width() - 8.0)
                                                                    .max(120.0),
                                                            );
                                                            if !grouped {
                                                                ui.horizontal(|ui| {
                                                                    let u_resp = ui.add(
                                                                        egui::Label::new(
                                                                            RichText::new(
                                                                                &message.username,
                                                                            )
                                                                            .size(font_sz)
                                                                            .strong()
                                                                            .color(
                                                                                theme::TEXT_PRIMARY,
                                                                            ),
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
                                                                        .size(11.5)
                                                                        .color(theme::TEXT_MUTED),
                                                                    );
                                                                });
                                                            }

                                                            if is_editing {
                                                                if let Some((_, ref mut buf)) =
                                                                    self.editing_message
                                                                {
                                                                    Self::ui_inline_editor(
                                                                        ui,
                                                                        message.id,
                                                                        buf,
                                                                        &mut pending_save_edit,
                                                                        &mut pending_cancel_edit,
                                                                    );
                                                                }
                                                            } else {
                                                                Self::render_rich_message(
                                                                    ui,
                                                                    message,
                                                                    font_sz,
                                                                    &mut self.revealed_spoilers,
                                                                );
                                                                if message.edited_at.is_some() {
                                                                    ui.label(
                                                                        RichText::new("(edited)")
                                                                            .size(10.5)
                                                                            .color(
                                                                                theme::TEXT_MUTED,
                                                                            ),
                                                                    );
                                                                }
                                                            }

                                                            // Reactions pill row
                                                            if !message.reactions.is_empty() {
                                                                ui.add_space(4.0);
                                                                ui.horizontal_wrapped(|ui| {
                                                                    ui.spacing_mut().item_spacing =
                                                                        egui::vec2(5.0, 4.0);
                                                                    for react in &message.reactions
                                                                    {
                                                                        let i_reacted = react
                                                                            .user_ids
                                                                            .contains(&me_id);
                                                                        let pill_bg = if i_reacted {
                                                                            theme::ACCENT_SOFT
                                                                        } else {
                                                                            theme::BG_SECONDARY
                                                                        };
                                                                        let pill_stroke =
                                                                            if i_reacted {
                                                                                Stroke::new(
                                                                                    1.0_f32,
                                                                                    theme::ACCENT,
                                                                                )
                                                                            } else {
                                                                                Stroke::new(
                                                                                    1.0_f32,
                                                                                    theme::BORDER_SUBTLE,
                                                                                )
                                                                            };
                                                                        let label = format!(
                                                                            "{} {}",
                                                                            react.emoji,
                                                                            react.user_ids.len()
                                                                        );
                                                                        if ui
                                                                            .add(
                                                                                egui::Button::new(
                                                                                    RichText::new(
                                                                                        label,
                                                                                    )
                                                                                    .size(13.0)
                                                                                    .color(
                                                                                        if i_reacted {
                                                                                            theme::TEXT_PRIMARY
                                                                                        } else {
                                                                                            theme::TEXT_SECONDARY
                                                                                        },
                                                                                    ),
                                                                                )
                                                                                .fill(pill_bg)
                                                                                .stroke(pill_stroke)
                                                                                .corner_radius(6.0),
                                                                            )
                                                                            .clicked()
                                                                        {
                                                                            pending_toggle_reaction =
                                                                                Some((
                                                                                    message.id,
                                                                                    react
                                                                                        .emoji
                                                                                        .clone(),
                                                                                ));
                                                                        }
                                                                    }
                                                                    if ui
                                                                        .add(
                                                                            egui::Button::new(
                                                                                RichText::new("+")
                                                                                    .size(13.0)
                                                                                    .color(
                                                                                        theme::TEXT_MUTED,
                                                                                    ),
                                                                            )
                                                                            .fill(
                                                                                theme::BG_SECONDARY,
                                                                            )
                                                                            .stroke(theme::border())
                                                                            .corner_radius(6.0),
                                                                        )
                                                                        .on_hover_text(
                                                                            "Add Reaction",
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        pending_open_emoji_for =
                                                                            Some(message.id);
                                                                    }
                                                                });
                                                            }
                                                        });
                                                    });
                                                }
                                            });
                                    },
                                )
                                .response;

                            let row_rect = Rect::from_min_max(
                                row_start,
                                egui::pos2(
                                    row_start.x + ui.available_width(),
                                    row_resp.rect.bottom(),
                                ),
                            );
                            let pointer_in_row = ui.ctx().input(|i| {
                                i.pointer
                                    .hover_pos()
                                    .is_some_and(|p| row_rect.contains(p))
                            });

                            if pointer_in_row {
                                ui.painter().set(
                                    bg_shape,
                                    egui::Shape::rect_filled(row_rect, 0.0, theme::MESSAGE_HOVER),
                                );
                                // Format B continuation gutter timestamp on hover
                                if grouped && !compact {
                                    ui.painter().text(
                                        egui::pos2(row_rect.left() + 18.0, row_rect.top() + 11.0),
                                        Align2::LEFT_CENTER,
                                        format_hhmm(message.created_at),
                                        FontId::proportional(10.5),
                                        theme::TEXT_MUTED,
                                    );
                                }

                                // Floating top-right Discord Hover Action Bar
                                if !is_editing {
                                    let bar_btns = if mine { 4 } else { 2 };
                                    let bar_w = (bar_btns as f32) * 30.0 + 8.0;
                                    let bar_rect = Rect::from_min_size(
                                        egui::pos2(
                                            row_rect.right() - bar_w - 16.0,
                                            (row_rect.top() - 10.0).max(ui.clip_rect().top()),
                                        ),
                                        egui::vec2(bar_w, 28.0),
                                    );
                                    ui.painter().rect_filled(bar_rect, 4.0, theme::BG_FLOATING);
                                    ui.painter().rect_stroke(
                                        bar_rect,
                                        4.0,
                                        theme::border(),
                                        egui::StrokeKind::Inside,
                                    );

                                    let mut bx = bar_rect.left() + 19.0;
                                    let by = bar_rect.center().y;

                                    // 1. Add Reaction
                                    let r1 =
                                        Rect::from_center_size(egui::pos2(bx, by), egui::vec2(26.0, 24.0));
                                    let resp1 = ui.interact(
                                        r1,
                                        ui.id().with(("act_react", message.id)),
                                        Sense::click(),
                                    );
                                    if resp1.hovered() {
                                        ui.painter().rect_filled(r1, 4.0, theme::BG_HOVER);
                                    }
                                    icons::paint_smile(
                                        ui.painter(),
                                        r1.center(),
                                        16.0,
                                        if resp1.hovered() {
                                            theme::TEXT_PRIMARY
                                        } else {
                                            theme::TEXT_SECONDARY
                                        },
                                    );
                                    if resp1.on_hover_text("Add Reaction").clicked() {
                                        pending_open_emoji_for = Some(message.id);
                                    }
                                    bx += 30.0;

                                    // 2. Reply
                                    let r2 =
                                        Rect::from_center_size(egui::pos2(bx, by), egui::vec2(26.0, 24.0));
                                    let resp2 = ui.interact(
                                        r2,
                                        ui.id().with(("act_reply", message.id)),
                                        Sense::click(),
                                    );
                                    if resp2.hovered() {
                                        ui.painter().rect_filled(r2, 4.0, theme::BG_HOVER);
                                    }
                                    icons::paint_reply(
                                        ui.painter(),
                                        r2.center(),
                                        16.0,
                                        if resp2.hovered() {
                                            theme::TEXT_PRIMARY
                                        } else {
                                            theme::TEXT_SECONDARY
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
                                        // 3. Edit
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
                                            ui.painter().rect_filled(r3, 4.0, theme::BG_HOVER);
                                        }
                                        icons::paint_pencil(
                                            ui.painter(),
                                            r3.center(),
                                            15.0,
                                            if resp3.hovered() {
                                                theme::TEXT_PRIMARY
                                            } else {
                                                theme::TEXT_SECONDARY
                                            },
                                        );
                                        if resp3.on_hover_text("Edit").clicked() {
                                            self.editing_message =
                                                Some((message.id, message.content.clone()));
                                        }

                                        bx += 30.0;
                                        // 4. Delete
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
                                            ui.painter().rect_filled(r4, 4.0, theme::DANGER);
                                        }
                                        icons::paint_trash(
                                            ui.painter(),
                                            r4.center(),
                                            15.0,
                                            if resp4.hovered() {
                                                Color32::WHITE
                                            } else {
                                                theme::DANGER
                                            },
                                        );
                                        if resp4.on_hover_text("Delete").clicked() {
                                            pending_delete = Some(message.id);
                                        }
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

    fn ui_date_separator(ui: &mut Ui, label: &str) {
        ui.add_space(12.0);
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), Sense::hover());
        let cy = r.center().y;
        let galley = ui.painter().layout_no_wrap(
            label.to_string(),
            FontId::proportional(11.5),
            theme::TEXT_MUTED,
        );
        let text_w = galley.size().x + 18.0;
        let cx = r.center().x;
        let left_end = (cx - text_w * 0.5).max(r.left() + 16.0);
        let right_start = (cx + text_w * 0.5).min(r.right() - 16.0);
        ui.painter().line_segment(
            [egui::pos2(r.left() + 16.0, cy), egui::pos2(left_end, cy)],
            theme::border(),
        );
        ui.painter().line_segment(
            [egui::pos2(right_start, cy), egui::pos2(r.right() - 16.0, cy)],
            theme::border(),
        );
        ui.painter().text(
            egui::pos2(cx, cy),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.5),
            theme::TEXT_MUTED,
        );
        ui.add_space(8.0);
    }

    fn ui_inline_editor(
        ui: &mut Ui,
        message_id: i64,
        edit_buf: &mut String,
        pending_save: &mut Option<(i64, String)>,
        pending_cancel: &mut bool,
    ) {
        egui::Frame::new()
            .fill(theme::INPUT_BG)
            .stroke(theme::focus_border())
            .corner_radius(6.0)
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
            ui.label(
                RichText::new("escape to cancel · enter to")
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
            if ui
                .add(
                    egui::Button::new(RichText::new("save").size(11.5).color(theme::TEXT_LINK))
                        .frame(false),
                )
                .clicked()
            {
                *pending_save = Some((message_id, edit_buf.clone()));
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("cancel").size(11.5).color(theme::TEXT_MUTED))
                        .frame(false),
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
                FontId::proportional((rect.width() * 0.42).max(12.0)),
                Color32::WHITE,
            );
        }
    }

    /// Rich Discord markdown renderer: code blocks, blockquotes, bold, italic, underline,
    /// strikethrough, inline code, clickable ||spoilers||, @mentions, and clickable URLs.
    fn render_rich_message(
        ui: &mut Ui,
        message: &MessageDto,
        font_sz: f32,
        revealed_spoilers: &mut std::collections::HashSet<(i64, usize)>,
    ) {
        let content = message.content.as_str();
        // Split out triple-backtick code blocks first
        let mut spoiler_idx = 0usize;
        let parts: Vec<&str> = content.split("```").collect();
        for (pi, part) in parts.iter().enumerate() {
            if pi % 2 == 1 {
                // Code block
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
                    .fill(theme::BG_TERTIARY)
                    .stroke(theme::border())
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width((ui.available_width() - 16.0).clamp(160.0, 620.0));
                        if let Some(l) = lang {
                            if !l.is_empty() {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        ui.label(
                                            RichText::new(l)
                                                .size(10.5)
                                                .monospace()
                                                .color(theme::TEXT_MUTED),
                                        );
                                    },
                                );
                            }
                        }
                        ui.label(
                            RichText::new(code_body)
                                .monospace()
                                .size((font_sz - 1.5).max(11.5))
                                .color(theme::TEXT_PRIMARY),
                        );
                    });
            } else {
                for line in part.lines() {
                    if let Some(quoted) = line.strip_prefix("> ") {
                        ui.horizontal(|ui| {
                            let (bar, _) =
                                ui.allocate_exact_size(egui::vec2(4.0, font_sz + 4.0), Sense::hover());
                            ui.painter().rect_filled(bar, 2.0, theme::TEXT_MUTED);
                            ui.add_space(4.0);
                            Self::render_inline_markdown(
                                ui,
                                message.id,
                                quoted,
                                font_sz,
                                theme::TEXT_SECONDARY,
                                revealed_spoilers,
                                &mut spoiler_idx,
                            );
                        });
                    } else if !line.is_empty() {
                        Self::render_inline_markdown(
                            ui,
                            message.id,
                            line,
                            font_sz,
                            theme::TEXT_PRIMARY,
                            revealed_spoilers,
                            &mut spoiler_idx,
                        );
                    }
                }
            }
        }
    }

    fn render_inline_markdown(
        ui: &mut Ui,
        msg_id: i64,
        line: &str,
        font_sz: f32,
        base_color: Color32,
        revealed_spoilers: &mut std::collections::HashSet<(i64, usize)>,
        spoiler_idx: &mut usize,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
            let mut rest = line;
            while !rest.is_empty() {
                // Find earliest special delimiter: ||, **, __, ~~, `, *
                if let Some(after) = rest.strip_prefix("||") {
                    if let Some(end) = after.find("||") {
                        let inner = &after[..end];
                        let idx = *spoiler_idx;
                        *spoiler_idx += 1;
                        let revealed = revealed_spoilers.contains(&(msg_id, idx));
                        let btn = egui::Button::new(
                            RichText::new(inner).size(font_sz).color(if revealed {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::BG_ACTIVE
                            }),
                        )
                        .fill(if revealed {
                            theme::BG_HOVER
                        } else {
                            theme::BG_ACTIVE
                        })
                        .corner_radius(3.0);
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
                        let inner = &after[..end];
                        egui::Frame::new()
                            .fill(theme::BG_SECONDARY)
                            .corner_radius(3.0)
                            .inner_margin(egui::Margin::symmetric(4, 1))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(inner)
                                        .monospace()
                                        .size((font_sz - 1.5).max(11.5))
                                        .color(theme::TEXT_PRIMARY),
                                );
                            });
                        rest = &after[end + 1..];
                        continue;
                    }
                }
                if let Some(after) = rest.strip_prefix("**") {
                    if let Some(end) = after.find("**") {
                        ui.label(
                            RichText::new(&after[..end])
                                .size(font_sz)
                                .strong()
                                .color(base_color),
                        );
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
                                .color(theme::TEXT_MUTED),
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

                // Consume until next whitespace-delimited token or markdown marker
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
                    if ui
                        .add(
                            egui::Link::new(
                                RichText::new(token).size(font_sz).color(theme::TEXT_LINK),
                            ),
                        )
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(token));
                    }
                } else if token.starts_with('@') && token.len() > 1 {
                    egui::Frame::new()
                        .fill(theme::ACCENT_SOFT)
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(3, 0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(token)
                                    .size(font_sz)
                                    .strong()
                                    .color(Color32::from_rgb(0xC9, 0xCD, 0xFB)),
                            );
                        });
                } else {
                    ui.label(RichText::new(token).size(font_sz).color(base_color));
                }
            }
        });
    }
}
