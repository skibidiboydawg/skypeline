//! Chat stage: channel header, message history with pagination/edit/delete, typing indicator, and composer.

use super::{format_hhmm, ui_avatar, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Color32, RichText, Ui};
use skypeline_shared::ChannelInfo;

impl SkyApp {
    pub(crate) fn ui_chat_stage(&mut self, ui: &mut Ui) {
        let Some(channel) = self.current_channel().cloned() else {
            return;
        };
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&channel.name).size(22.0).strong());
                let subtitle = if channel.kind == "group" {
                    format!("{} members", channel.members.len())
                } else if let Some(peer) = channel.peer_user_id {
                    self.online_status_for(peer).label().to_string()
                } else {
                    "Direct message".into()
                };
                ui.label(theme::muted(subtitle).small());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                let in_this_call = self.in_voice
                    && self.voice_room.as_deref() == Some(Self::channel_room(channel.id).as_str());
                if ui
                    .add(
                        egui::Button::new(if in_this_call && self.sharing {
                            "Stop Share"
                        } else {
                            "Share Screen"
                        })
                        .fill(if in_this_call && self.sharing {
                            theme::ACCENT
                        } else {
                            theme::SURFACE_CARD
                        })
                        .stroke(theme::border())
                        .corner_radius(10.0),
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
                        egui::Button::new(if in_this_call {
                            "In Call"
                        } else {
                            "Voice Call"
                        })
                        .fill(if in_this_call {
                            theme::SUCCESS
                        } else {
                            theme::ACCENT
                        })
                        .corner_radius(10.0),
                    )
                    .clicked()
                {
                    self.start_call(channel.id);
                }
            });
        });
        ui.add_space(10.0);
        ui.separator();
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

        let reserved_bottom = if active_typers.is_empty() { 74.0 } else { 96.0 };
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
                    .inner_margin(egui::Margin::symmetric(24, 8))
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
                                            .corner_radius(12.0),
                                    )
                                    .clicked()
                                {
                                    pending_load_older = true;
                                }
                            });
                            ui.add_space(8.0);
                        }

                        if self.messages.is_empty() {
                            ui.add_space(24.0);
                            ui.vertical_centered(|ui| {
                                ui.label(theme::muted(format!(
                                    "No messages in {} yet. Say hello!",
                                    channel.name
                                )));
                            });
                        }

                        for index in 0..self.messages.len() {
                            let message = self.messages[index].clone();
                            let mine = message.user_id == me_id;
                            let time_str = format_hhmm(message.created_at);
                            let is_editing = self
                                .editing_message
                                .as_ref()
                                .is_some_and(|(id, _)| *id == message.id);

                            ui.horizontal(|ui| {
                                if !mine {
                                    ui_avatar(ui, &message.username, false);
                                }
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&message.username)
                                                .strong()
                                                .small()
                                                .color(if mine {
                                                    theme::ACCENT
                                                } else {
                                                    theme::PRIMARY_TEXT
                                                }),
                                        );
                                        ui.label(theme::muted(&time_str).small());
                                        if message.edited_at.is_some() {
                                            ui.label(theme::muted("(edited)").small());
                                        }
                                        if mine && !is_editing {
                                            if ui.small_button("Edit").clicked() {
                                                self.editing_message =
                                                    Some((message.id, message.content.clone()));
                                            }
                                            if ui
                                                .small_button(
                                                    RichText::new("Delete").color(theme::DANGER),
                                                )
                                                .clicked()
                                            {
                                                pending_delete = Some(message.id);
                                            }
                                        }
                                    });

                                    if is_editing {
                                        if let Some((_, ref mut edit_buf)) = self.editing_message {
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::TextEdit::singleline(edit_buf)
                                                        .desired_width(320.0),
                                                );
                                                if ui
                                                    .add(
                                                        egui::Button::new("Save")
                                                            .fill(theme::ACCENT)
                                                            .corner_radius(6.0),
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
                                        egui::Frame::new()
                                            .fill(if mine {
                                                Color32::from_rgb(0x1D, 0x4E, 0xD8)
                                            } else {
                                                theme::SURFACE_CARD
                                            })
                                            .stroke(theme::border())
                                            .corner_radius(10.0)
                                            .inner_margin(egui::Margin::symmetric(10, 8))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&message.content)
                                                        .color(theme::PRIMARY_TEXT),
                                                );
                                            });
                                    }
                                });
                            });
                            ui.add_space(6.0);
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
                ui.label(theme::muted(text).small());
            });
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            let field_w = (ui.available_width() - 108.0).max(140.0);
            let field = ui
                .allocate_ui(egui::vec2(field_w, 38.0), |ui| {
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
                    egui::Button::new("Send")
                        .fill(theme::ACCENT)
                        .corner_radius(9.0)
                        .min_size(egui::vec2(80.0, 38.0)),
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
