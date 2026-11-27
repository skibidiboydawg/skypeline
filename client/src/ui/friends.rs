//! Pixel-faithful Discord Friends Page:
//! 48px top bar (Friends | Online | All | Pending + badge | Blocked | green Add Friend),
//! centered Add Friend tab (max-w 480px), 62px friend rows with Message/Call/Accept/Decline/Unblock,
//! and Discord-style empty state.

use super::{avatar_color, icons, FriendsTab, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_friends_home(&mut self, ui: &mut Ui) {
        let pending_count = self
            .friends
            .iter()
            .filter(|f| f.status == FriendshipStatus::Pending)
            .count();
        let incoming_count = self
            .friends
            .iter()
            .filter(|f| f.status == FriendshipStatus::Pending && f.direction == "incoming")
            .count();

        // 1. 48px Top Header Bar (--bg-primary, border-bottom 1px subtle)
        let (header_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::hover());
        ui.painter()
            .rect_filled(header_rect, 0.0, theme::BG_PRIMARY);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            theme::border(),
        );

        let header_inner = header_rect.shrink2(egui::vec2(16.0, 6.0));
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(header_inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                let (ir, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
                icons::paint_users(ui.painter(), ir.center(), 18.0, theme::TEXT_MUTED);
                ui.label(
                    RichText::new("Friends")
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.add_space(6.0);
                let (div_r, _) = ui.allocate_exact_size(egui::vec2(1.0, 22.0), Sense::hover());
                ui.painter().rect_filled(div_r, 0.0, theme::BORDER_SUBTLE);
                ui.add_space(6.0);

                for (tab, label) in [
                    (FriendsTab::Online, "Online"),
                    (FriendsTab::All, "All"),
                    (FriendsTab::Pending, "Pending"),
                    (FriendsTab::Blocked, "Blocked"),
                ] {
                    let active = self.friends_tab == tab;
                    let show_badge = tab == FriendsTab::Pending && pending_count > 0;
                    let btn_w = if show_badge {
                        84.0
                    } else if label == "Blocked" {
                        68.0
                    } else if label == "All" {
                        40.0
                    } else {
                        60.0
                    };
                    let (tr, tresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 28.0), Sense::click());
                    ui.painter().rect_filled(
                        tr,
                        4.0,
                        if active {
                            theme::BG_ACTIVE
                        } else if tresp.hovered() {
                            theme::BG_HOVER
                        } else {
                            Color32::TRANSPARENT
                        },
                    );
                    let text_pos = if show_badge {
                        tr.left_center() + egui::vec2(10.0, 0.0)
                    } else {
                        tr.center()
                    };
                    ui.painter().text(
                        text_pos,
                        if show_badge {
                            Align2::LEFT_CENTER
                        } else {
                            Align2::CENTER_CENTER
                        },
                        label,
                        FontId::proportional(14.0),
                        if active {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        },
                    );
                    if show_badge {
                        let badge_c = tr.right_center() - egui::vec2(14.0, 0.0);
                        ui.painter().circle_filled(badge_c, 8.0, theme::DANGER);
                        ui.painter().text(
                            badge_c,
                            Align2::CENTER_CENTER,
                            pending_count.min(99).to_string(),
                            FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                    }
                    if tresp.clicked() {
                        self.friends_tab = tab;
                    }
                }

                ui.add_space(4.0);
                // Green "Add Friend" Discord tab button
                let add_active = self.friends_tab == FriendsTab::AddFriend;
                let (ar, aresp) = ui.allocate_exact_size(egui::vec2(92.0, 28.0), Sense::click());
                ui.painter().rect_filled(
                    ar,
                    4.0,
                    if add_active {
                        Color32::TRANSPARENT
                    } else {
                        theme::SUCCESS
                    },
                );
                ui.painter().text(
                    ar.center(),
                    Align2::CENTER_CENTER,
                    "Add Friend",
                    FontId::proportional(13.5),
                    if add_active {
                        theme::SUCCESS
                    } else {
                        Color32::WHITE
                    },
                );
                if aresp.clicked() {
                    self.friends_tab = FriendsTab::AddFriend;
                    self.friend_error.clear();
                }

                // Right: Inbox icon + Latest release shortcut
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (bell_r, bell_resp) =
                        ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::click());
                    if self.show_inbox_popover || bell_resp.hovered() {
                        ui.painter().rect_filled(bell_r, 4.0, theme::BG_HOVER);
                    }
                    icons::paint_inbox(
                        ui.painter(),
                        bell_r.center(),
                        17.0,
                        if bell_resp.hovered() {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_SECONDARY
                        },
                    );
                    if incoming_count > 0 {
                        ui.painter().circle_filled(
                            bell_r.right_top() + egui::vec2(-4.0, 4.0),
                            4.5,
                            theme::DANGER,
                        );
                    }
                    if bell_resp.on_hover_text("Inbox").clicked() {
                        self.show_inbox_popover = !self.show_inbox_popover;
                    }
                });
            },
        );

        // 2. Main Tab Body
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.friends_tab == FriendsTab::AddFriend {
                    self.ui_add_friend_tab(ui);
                } else {
                    self.ui_friends_list(ui);
                }
            });
    }

    fn ui_add_friend_tab(&mut self, ui: &mut Ui) {
        let avail_w = ui.available_width();
        let col_w = (avail_w - 48.0).clamp(280.0, 480.0);
        let pad = ((avail_w - col_w) * 0.5).max(24.0);

        ui.add_space(36.0);
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    ui.cursor().min + egui::vec2(pad, 0.0),
                    egui::vec2(col_w, 320.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_width(col_w);
                ui.label(
                    RichText::new("ADD FRIEND")
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("You can add friends with their Wireline username or email.")
                        .size(14.0)
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(16.0);

                let focus_id = ui.id().with("discord_add_friend_input");
                let has_focus = ui.memory(|m| m.has_focus(focus_id));
                let stroke = if !self.friend_error.is_empty() {
                    Stroke::new(1.2_f32, theme::DANGER)
                } else if has_focus {
                    theme::focus_border()
                } else {
                    theme::input_border()
                };

                egui::Frame::new()
                    .fill(theme::BG_TERTIARY)
                    .stroke(stroke)
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let input_w = (ui.available_width() - 156.0).max(110.0);
                            let field = ui.add_sized(
                                [input_w, 32.0],
                                egui::TextEdit::singleline(&mut self.friend_query)
                                    .id(focus_id)
                                    .frame(false)
                                    .hint_text("You can add friends with their username."),
                            );
                            if field.changed() {
                                self.friend_error.clear();
                            }
                            let send_btn = egui::Button::new(
                                RichText::new("Send Friend Request")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme::ACCENT)
                            .corner_radius(4.0)
                            .min_size(egui::vec2(146.0, 32.0));
                            if ui.add_enabled(!self.friend_busy, send_btn).clicked()
                                || (field.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                self.send_friend_request();
                            }
                        });
                    });

                if !self.friend_error.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(&self.friend_error)
                            .size(13.0)
                            .color(theme::DANGER),
                    );
                } else if !self.notice.is_empty() {
                    ui.add_space(8.0);
                    self.ui_notice(ui);
                }
            },
        );
    }

    fn ui_friends_list(&mut self, ui: &mut Ui) {
        let mut friends = self.friends.clone();
        friends.sort_by(|a, b| {
            b.online.cmp(&a.online).then_with(|| {
                a.username
                    .to_ascii_lowercase()
                    .cmp(&b.username.to_ascii_lowercase())
            })
        });

        let filtered: Vec<_> = match self.friends_tab {
            FriendsTab::Online => friends
                .into_iter()
                .filter(|f| {
                    f.status == FriendshipStatus::Accepted
                        && f.online
                        && !self.is_blocked(f.user_id)
                })
                .collect(),
            FriendsTab::All => friends
                .into_iter()
                .filter(|f| {
                    (f.status == FriendshipStatus::Accepted || f.direction == "mutual")
                        && !self.is_blocked(f.user_id)
                })
                .collect(),
            FriendsTab::Pending => friends
                .into_iter()
                .filter(|f| f.status == FriendshipStatus::Pending)
                .collect(),
            FriendsTab::Blocked => friends
                .into_iter()
                .filter(|f| self.is_blocked(f.user_id))
                .collect(),
            FriendsTab::AddFriend => Vec::new(),
        };

        let header_label = match self.friends_tab {
            FriendsTab::Online => format!("ONLINE — {}", filtered.len()),
            FriendsTab::All => format!("ALL FRIENDS — {}", filtered.len()),
            FriendsTab::Pending => format!("PENDING — {}", filtered.len()),
            FriendsTab::Blocked => format!("BLOCKED — {}", filtered.len()),
            FriendsTab::AddFriend => String::new(),
        };

        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(24, 16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                if filtered.is_empty() {
                    ui.add_space(72.0);
                    ui.vertical_centered(|ui| {
                        let (icon_r, _) =
                            ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                        ui.painter()
                            .circle_filled(icon_r.center(), 32.0, theme::BG_SECONDARY);
                        icons::paint_users(
                            ui.painter(),
                            icon_r.center(),
                            30.0,
                            theme::TEXT_MUTED,
                        );
                        ui.add_space(16.0);
                        let (title, sub) = match self.friends_tab {
                            FriendsTab::Online => (
                                "It's quiet for now...",
                                "When a friend starts an activity or comes online, we'll show it here!",
                            ),
                            FriendsTab::All => (
                                "It's quiet for now...",
                                "Make some noise by adding a friend or starting a conversation!",
                            ),
                            FriendsTab::Pending => (
                                "There are no pending friend requests.",
                                "Incoming and outgoing friend requests will appear here.",
                            ),
                            FriendsTab::Blocked => (
                                "You haven't blocked anyone.",
                                "Blocked users won't be able to send you notifications.",
                            ),
                            FriendsTab::AddFriend => ("", ""),
                        };
                        ui.label(
                            RichText::new(title)
                                .size(16.0)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.add_space(6.0);
                        ui.label(RichText::new(sub).size(14.0).color(theme::TEXT_MUTED));
                        if matches!(self.friends_tab, FriendsTab::Online | FriendsTab::All) {
                            ui.add_space(16.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Add Friend")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(4.0)
                                    .min_size(egui::vec2(120.0, 34.0)),
                                )
                                .clicked()
                            {
                                self.friends_tab = FriendsTab::AddFriend;
                            }
                        }
                    });
                    return;
                }

                ui.label(
                    RichText::new(header_label)
                        .size(12.0)
                        .strong()
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(8.0);

                for friend in filtered {
                    let (div, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 1.0),
                        Sense::hover(),
                    );
                    ui.painter().rect_filled(div, 0.0, theme::BORDER_SUBTLE);

                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 62.0),
                        Sense::hover(),
                    );
                    let row_hovered = ui.ctx().input(|i| {
                        i.pointer
                            .hover_pos()
                            .is_some_and(|p| row_rect.contains(p))
                    });
                    if row_hovered {
                        ui.painter()
                            .rect_filled(row_rect.shrink2(egui::vec2(0.0, 2.0)), 8.0, theme::BG_HOVER);
                    }
                    let _ = row_resp;

                    let ring_bg = if row_hovered {
                        theme::BG_HOVER
                    } else {
                        theme::BG_PRIMARY
                    };

                    // 32px clickable avatar + Discord status dot
                    let av_rect = Rect::from_center_size(
                        egui::pos2(row_rect.left() + 26.0, row_rect.center().y),
                        egui::vec2(34.0, 34.0),
                    );
                    let av_resp = ui.interact(
                        av_rect,
                        ui.id().with(("friend_av", friend.user_id)),
                        Sense::click(),
                    );
                    let mut drew = false;
                    if let Some(h) = friend.avatar_hash.as_deref() {
                        let tex = self.avatars.texture(
                            &ui.ctx().clone(),
                            &self.api(),
                            self.token.as_deref(),
                            friend.user_id,
                            Some(h),
                            &self.events_tx,
                        );
                        if let Some(t) = tex {
                            ui.painter().image(
                                t.id(),
                                av_rect,
                                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            drew = true;
                        }
                    }
                    if !drew {
                        ui.painter().circle_filled(
                            av_rect.center(),
                            17.0,
                            avatar_color(&friend.username, false),
                        );
                        let initial = friend
                            .username
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_ascii_uppercase()
                            .to_string();
                        ui.painter().text(
                            av_rect.center(),
                            Align2::CENTER_CENTER,
                            initial,
                            FontId::proportional(14.0),
                            Color32::WHITE,
                        );
                    }
                    if friend.status == FriendshipStatus::Accepted
                        && !self.is_blocked(friend.user_id)
                    {
                        icons::paint_status_dot(
                            ui.painter(),
                            av_rect.right_bottom() - egui::vec2(3.0, 3.0),
                            friend.presence,
                            ring_bg,
                        );
                    }
                    if av_resp.clicked() {
                        self.open_profile_popup(
                            friend.user_id,
                            friend.username.clone(),
                            av_rect.right_top(),
                        );
                    }

                    // Name + Status label
                    let tx = av_rect.right() + 12.0;
                    ui.painter().text(
                        egui::pos2(tx, row_rect.center().y - 2.0),
                        Align2::LEFT_BOTTOM,
                        &friend.username,
                        FontId::proportional(15.0),
                        theme::TEXT_PRIMARY,
                    );
                    let sub_text = if self.is_blocked(friend.user_id) {
                        "Blocked".to_string()
                    } else if friend.status == FriendshipStatus::Pending {
                        if friend.direction == "incoming" {
                            "Incoming Friend Request".to_string()
                        } else {
                            "Outgoing Friend Request".to_string()
                        }
                    } else {
                        match friend.presence {
                            PresenceStatus::Online => "Online",
                            PresenceStatus::Away => "Idle",
                            PresenceStatus::Dnd => "Do Not Disturb",
                            PresenceStatus::Offline => "Offline",
                        }
                        .to_string()
                    };
                    ui.painter().text(
                        egui::pos2(tx, row_rect.center().y + 2.0),
                        Align2::LEFT_TOP,
                        sub_text,
                        FontId::proportional(12.5),
                        theme::TEXT_MUTED,
                    );

                    // Right-side circular Discord action buttons (36px)
                    let cy = row_rect.center().y;
                    let right_x = row_rect.right() - 28.0;

                    if self.friends_tab == FriendsTab::Blocked {
                        let ur = Rect::from_center_size(
                            egui::pos2(right_x - 24.0, cy),
                            egui::vec2(84.0, 30.0),
                        );
                        let uresp = ui.interact(
                            ur,
                            ui.id().with(("unblock", friend.user_id)),
                            Sense::click(),
                        );
                        ui.painter().rect_filled(
                            ur,
                            4.0,
                            if uresp.hovered() {
                                theme::BG_ACTIVE
                            } else {
                                theme::BG_SECONDARY
                            },
                        );
                        ui.painter().text(
                            ur.center(),
                            Align2::CENTER_CENTER,
                            "Unblock",
                            FontId::proportional(13.0),
                            theme::TEXT_PRIMARY,
                        );
                        if uresp.clicked() {
                            self.toggle_block(friend.user_id);
                        }
                    } else if friend.status == FriendshipStatus::Pending {
                        if friend.direction == "incoming" {
                            // Decline X
                            let xr = Rect::from_center_size(
                                egui::pos2(right_x, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let xresp = ui.interact(
                                xr,
                                ui.id().with(("decline", friend.user_id)),
                                Sense::click(),
                            );
                            ui.painter().circle_filled(
                                xr.center(),
                                18.0,
                                if xresp.hovered() {
                                    theme::DANGER
                                } else {
                                    theme::BG_SECONDARY
                                },
                            );
                            icons::paint_x(
                                ui.painter(),
                                xr.center(),
                                16.0,
                                if xresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_SECONDARY
                                },
                            );
                            if xresp.on_hover_text("Decline").clicked() {
                                self.respond_friend(friend.user_id, false);
                            }

                            // Accept Check
                            let cr = Rect::from_center_size(
                                egui::pos2(right_x - 46.0, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let cresp = ui.interact(
                                cr,
                                ui.id().with(("accept", friend.user_id)),
                                Sense::click(),
                            );
                            ui.painter().circle_filled(
                                cr.center(),
                                18.0,
                                if cresp.hovered() {
                                    theme::SUCCESS
                                } else {
                                    theme::BG_SECONDARY
                                },
                            );
                            icons::paint_check(
                                ui.painter(),
                                cr.center(),
                                16.0,
                                if cresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_SECONDARY
                                },
                            );
                            if cresp.on_hover_text("Accept").clicked() {
                                self.respond_friend(friend.user_id, true);
                            }
                        } else {
                            ui.painter().text(
                                egui::pos2(row_rect.right() - 12.0, cy),
                                Align2::RIGHT_CENTER,
                                "Pending",
                                FontId::proportional(12.5),
                                theme::TEXT_MUTED,
                            );
                        }
                    } else {
                        // Call button
                        let cr = Rect::from_center_size(
                            egui::pos2(right_x, cy),
                            egui::vec2(36.0, 36.0),
                        );
                        let cresp = ui.interact(
                            cr,
                            ui.id().with(("call_f", friend.user_id)),
                            Sense::click(),
                        );
                        ui.painter().circle_filled(
                            cr.center(),
                            18.0,
                            if cresp.hovered() {
                                theme::BG_TERTIARY
                            } else {
                                theme::BG_SECONDARY
                            },
                        );
                        icons::paint_phone(
                            ui.painter(),
                            cr.center(),
                            16.0,
                            if cresp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if cresp.on_hover_text("Start Voice Call").clicked() {
                            self.open_dm(friend.user_id, true);
                        }

                        // Message button
                        let mr = Rect::from_center_size(
                            egui::pos2(right_x - 46.0, cy),
                            egui::vec2(36.0, 36.0),
                        );
                        let mresp = ui.interact(
                            mr,
                            ui.id().with(("msg_f", friend.user_id)),
                            Sense::click(),
                        );
                        ui.painter().circle_filled(
                            mr.center(),
                            18.0,
                            if mresp.hovered() {
                                theme::BG_TERTIARY
                            } else {
                                theme::BG_SECONDARY
                            },
                        );
                        icons::paint_message_circle(
                            ui.painter(),
                            mr.center(),
                            16.0,
                            if mresp.hovered() {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        );
                        if mresp.on_hover_text("Message").clicked() {
                            self.open_dm(friend.user_id, false);
                        }
                    }
                }
            });
    }
}
