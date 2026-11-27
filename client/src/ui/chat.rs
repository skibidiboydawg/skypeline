//! Wireline chat stage: sleek header, grouped message stream, hover edit/delete, and pill composer.

use super::{format_hhmm, status_color, ui_avatar, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Color32, RichText, Sense, Ui};
use skypeline_shared::ChannelInfo;

impl SkyApp {
    pub(crate) fn ui_chat_stage(&mut self, ui: &mut Ui) {
        let Some(channel) = self.current_channel().cloned() else {
            return;
        };

        // Top channel bar
        egui::Frame::new()
            .fill(theme::SURFACE_SIDEBAR)
            .stroke(theme::border())
            .inner_margin(egui::Margin::symmetric(20, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui_avatar(ui, &channel.name, false);
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&channel.name).size(17.0).strong());
                        if let Some(peer) = channel.peer_user_id {
                            ui.horizontal(|ui| {
                                let status = self.online_status_for(peer);
                                let (r, _) =
                                    ui.allocate_exact_size(egui::vec2(8.0, 12.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 3.5, status_color(status));
                                ui.label(theme::muted(status.label()).small());
                            });
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let in_this_call = self.in_voice
                            && self.voice_room.as_deref()
                                == Some(Self::channel_room(channel.id).as_str());
                        if ui
                            .add(
                                egui::Button::new(RichText::new(
                                    if in_this_call && self.sharing {
                                        "Stop Share"
                                    } else {
                                        "Share Screen"
                                    },
                                ))
                                .fill(if in_this_call && self.sharing {
                                    theme::ACCENT
                                } else {
                                    theme::SURFACE_CARD
                                })
                                .stroke(theme::border())
                                .corner_radius(16.0)
                                .min_size(egui::vec2(104.0, 32.0)),
                            )
                            .clicked()
                        {
                            if !in_this_call {
                                self.start_call(channel.id);
                            }
                            if self.in_voice {
                                self.toggle_screen_share();
                            }
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(if in_this_call {
                                        "In Call"
                                    } else {
                                        "Voice Call"
                                    })
                                    .strong()
                                    .color(Color32::WHITE),
                                )
                                .fill(if in_this_call {
                                    theme::SUCCESS
                                } else {
                                    theme::ACCENT
                                })
                                .corner_radius(16.0)
                                .min_size(egui::vec2(96.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.start_call(channel.id);
                        }
                    });
                });
            });

        if self.in_voice
            && self.voice_room.as_deref() == Some(Self::channel_room(channel.id).as_str())
        {
            self.ui_call_stage(ui, &channel);
        }
        self.ui_messages(ui, &channel);
    }

    fn ui_messages(&mut self, ui: &mut Ui, channel: &ChannelInfo) {
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let now = Self::now_secs();
        let active_typers: Vec<String> = self
            .typing_users
            .get_mut(&channel.id)
            .map(|map| {
                map.retain(|_, (_, exp)| *exp > now);
                map.values().map(|(name, _)| name.clone()).collect()
            })
            .unwrap_or_default();

        let reserved_bottom = if active_typers.is_empty() { 76.0 } else { 98.0 };
        let scroll_h = (ui.available_height() - reserved_bottom).max(120.0);

        let mut pending_load_older = false;
        let mut pending_save_edit: Option<(i64, String)> = None;
        let mut pending_cancel_edit = false;
        let mut pending_delete: Option<i64> = None;

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(scroll_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(24, 12))
                    .show(ui, |ui| {
                        if self.has_more_messages {
                            ui.vertical_centered(|ui| {
                                let label = if self.loading_older {
                                    "Loading older messages..."
                                } else {
                                    "Load older messages"
                                };
                                if ui
                                    .add_enabled(
                                        !self.loading_older,
                                        egui::Button::new(theme::muted(label).small())
                                            .fill(theme::SURFACE_CARD)
                                            .corner_radius(14.0),
                                    )
                                    .clicked()
                                {
                                    pending_load_older = true;
                                }
                            });
                            ui.add_space(10.0);
                        }

                        if self.messages.is_empty() {
                            ui.add_space(32.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(format!("Welcome to {}", channel.name))
                                        .size(18.0)
                                        .strong(),
                                );
                                ui.label(theme::muted("Send a message to start the conversation."));
                            });
                        }

                        for index in 0..self.messages.len() {
                            let message = self.messages[index].clone();
                            let mine = message.user_id == me_id;
                            let grouped = index > 0
                                && self.messages[index - 1].user_id == message.user_id
                                && (message.created_at - self.messages[index - 1].created_at).abs()
                                    < 300;
                            let time_str = format_hhmm(message.created_at);
                            let is_editing = self
                                .editing_message
                                .as_ref()
                                .is_some_and(|(id, _)| *id == message.id);

                            if !grouped && index > 0 {
                                ui.add_space(10.0);
                            }

                            let row_resp = ui.horizontal(|ui| {
                                if grouped {
                                    ui.add_space(40.0);
                                } else {
                                    ui_avatar(ui, &message.username, mine);
                                    ui.add_space(2.0);
                                }
                                ui.vertical(|ui| {
                                    if !grouped {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(&message.username)
                                                    .strong()
                                                    .color(if mine {
                                                        theme::CYAN_GLOW
                                                    } else {
                                                        theme::PRIMARY_TEXT
                                                    }),
                                            );
                                            ui.label(
                                                RichText::new(&time_str)
                                                    .small()
                                                    .color(theme::MUTED_TEXT),
                                            );
                                            if message.edited_at.is_some() {
                                                ui.label(
                                                    RichText::new("(edited)")
                                                        .small()
                                                        .color(theme::MUTED_TEXT),
                                                );
                                            }
                                        });
                                    }

                                    if is_editing {
                                        if let Some((_, ref mut edit_buf)) = self.editing_message {
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::TextEdit::singleline(edit_buf)
                                                        .desired_width(340.0),
                                                );
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            RichText::new("Save")
                                                                .color(Color32::WHITE),
                                                        )
                                                        .fill(theme::ACCENT)
                                                        .corner_radius(8.0),
                                                    )
                                                    .clicked()
                                                {
                                                    pending_save_edit =
                                                        Some((message.id, edit_buf.clone()));
                                                }
                                                if ui.button("Cancel").clicked() {
                                                    pending_cancel_edit = true;
                                                }
                                            });
                                        }
                                    } else {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new(&message.content)
                                                    .color(theme::PRIMARY_TEXT),
                                            );
                                            if grouped && message.edited_at.is_some() {
                                                ui.label(
                                                    RichText::new("(edited)")
                                                        .small()
                                                        .color(theme::MUTED_TEXT),
                                                );
                                            }
                                        });
                                    }
                                });
                            });

                            if mine && !is_editing && row_resp.response.hovered() {
                                ui.horizontal(|ui| {
                                    ui.add_space(40.0);
                                    if ui
                                        .add(
                                            egui::Button::new(theme::muted("Edit").small())
                                                .fill(theme::SURFACE_CARD)
                                                .corner_radius(6.0),
                                        )
                                        .clicked()
                                    {
                                        self.editing_message =
                                            Some((message.id, message.content.clone()));
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Delete")
                                                    .small()
                                                    .color(theme::DANGER),
                                            )
                                            .fill(theme::SURFACE_CARD)
                                            .corner_radius(6.0),
                                        )
                                        .clicked()
                                    {
                                        pending_delete = Some(message.id);
                                    }
                                });
                            }
                            ui.add_space(2.0);
                        }
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

        if !active_typers.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let text = if active_typers.len() == 1 {
                    format!("{} is typing...", active_typers[0])
                } else {
                    format!("{} are typing...", active_typers.join(", "))
                };
                ui.label(RichText::new(text).small().color(theme::CYAN_GLOW));
            });
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let field_w = (ui.available_width() - 110.0).max(140.0);
            let field = ui
                .allocate_ui(egui::vec2(field_w, 40.0), |ui| {
                    Self::form_field(
                        ui,
                        &mut self.draft,
                        &format!("Message {}", channel.name),
                        false,
                    )
                })
                .inner;
            if self.refocus_draft {
                field.request_focus();
                self.refocus_draft = false;
            }
            if field.changed() && !self.draft.trim().is_empty() {
                self.notify_typing(channel.id);
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Send").strong().color(Color32::WHITE))
                        .fill(theme::ACCENT)
                        .corner_radius(14.0)
                        .min_size(egui::vec2(82.0, 40.0)),
                )
                .clicked()
                || (field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.send_message();
            }
            ui.add_space(20.0);
        });
    }
}
