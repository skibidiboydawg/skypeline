//! Pixel-faithful Discord Friends Page with smooth animations:
//! #1 Sliding #5865f2 tab underline (150ms), #3 online pulse dot, #5 friend request 250ms slide-out,
//! #6 Pending badge pop (300ms spring), and #7 hover slide-in action buttons (150ms).

use super::{anim, avatar_color, icons, FriendsTab, SkyApp};
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
        let badge_scale = self.pending_badge_scale(pending_count);
        let reduced = self.config.reduced_motion;

        // 1. 48px Top Header Bar (--bg-primary, border-bottom 1px subtle)
        let (header_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::hover());
        ui.painter()
            .rect_filled(header_rect, 0.0, theme::BG_PRIMARY);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            theme::border(),
        );

        let mut active_tab_rect: Option<Rect> = None;
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
                    if active {
                        active_tab_rect = Some(tr);
                    }
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
                    // #6 Pending Badge Pop (green pill scaling 0.5 -> 1.15 -> 1.0)
                    if show_badge {
                        let badge_c = tr.right_center() - egui::vec2(14.0, 0.0);
                        ui.painter()
                            .circle_filled(badge_c, 8.0 * badge_scale, theme::SUCCESS);
                        ui.painter().text(
                            badge_c,
                            Align2::CENTER_CENTER,
                            pending_count.min(99).to_string(),
                            FontId::proportional((10.0 * badge_scale).max(6.5)),
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
                if add_active {
                    active_tab_rect = Some(ar);
                }
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

                // Right: Inbox icon
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

        // #1 Sliding Tab Underline: 2px #5865f2 rounded bar gliding over 150ms
        if let Some(target_r) = active_tab_rect {
            let dur = if reduced { 0.001 } else { 0.15 };
            let anim_left = ui.ctx().animate_value_with_time(
                egui::Id::new("friends_tab_underline_x"),
                target_r.left(),
                dur,
            );
            let anim_w = ui.ctx().animate_value_with_time(
                egui::Id::new("friends_tab_underline_w"),
                target_r.width(),
                dur,
            );
            if (anim_left - target_r.left()).abs() > 0.5
                || (anim_w - target_r.width()).abs() > 0.5
            {
                self.animating = true;
            }
            let bar = Rect::from_min_max(
                egui::pos2(anim_left, header_rect.bottom() - 2.5),
                egui::pos2(anim_left + anim_w, header_rect.bottom() - 0.5),
            );
            ui.painter().rect_filled(bar, 2.0, theme::ACCENT);
        }

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
        let now = Self::now_secs();
        let reduced = self.config.reduced_motion;

        // #5 Flush any completed 250ms friend-request slide-out animations
        let finished_removals: Vec<(i64, bool)> = self
            .removing_friends
            .iter()
            .filter_map(|(&uid, &(started, accept))| {
                if reduced || (now - started) >= 0.25 {
                    Some((uid, accept))
                } else {
                    None
                }
            })
            .collect();
        for (uid, accept) in finished_removals {
            self.removing_friends.remove(&uid);
            self.friends
                .retain(|f| !(f.user_id == uid && f.status == FriendshipStatus::Pending));
            self.respond_friend(uid, accept);
        }

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
        let pulse = self.pulse_time(ui.ctx());

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
                    // #5 Friend Request 250ms slideOutRight animation state
                    let removing_state = self
                        .removing_friends
                        .get(&friend.user_id)
                        .copied()
                        .map(|(started, _)| {
                            let raw = ((now - started) / 0.25).clamp(0.0, 1.0) as f32;
                            anim::ease_in(raw)
                        });
                    let is_removing = removing_state.is_some();
                    if is_removing {
                        self.animating = true;
                    }
                    let rem_p = removing_state.unwrap_or(0.0);
                    let row_alpha = (1.0 - rem_p).clamp(0.0, 1.0);
                    let slide_x = 40.0 * rem_p;
                    let row_h = (62.0 * (1.0 - rem_p)).max(0.5);

                    if !is_removing {
                        let (div, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(div, 0.0, theme::BORDER_SUBTLE);
                    }

                    let (row_rect, _row_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_h),
                        Sense::hover(),
                    );
                    if row_h < 8.0 {
                        continue;
                    }

                    let row_hovered = !is_removing
                        && ui.ctx().input(|i| {
                            i.pointer
                                .hover_pos()
                                .is_some_and(|p| row_rect.contains(p))
                        });
                    let shifted_rect = row_rect.translate(egui::vec2(slide_x, 0.0));
                    let row_painter = ui.painter_at(row_rect);

                    if row_hovered {
                        row_painter.rect_filled(
                            row_rect.shrink2(egui::vec2(0.0, 2.0)),
                            8.0,
                            theme::BG_HOVER,
                        );
                    }

                    let ring_bg = if row_hovered {
                        theme::BG_HOVER
                    } else {
                        theme::BG_PRIMARY
                    };

                    // 34px clickable avatar + Discord status dot (#3 pulse on online)
                    let av_rect = Rect::from_center_size(
                        egui::pos2(shifted_rect.left() + 26.0, shifted_rect.center().y),
                        egui::vec2(34.0, 34.0),
                    );
                    let av_resp = ui.interact(
                        av_rect,
                        ui.id().with(("friend_av", friend.user_id)),
                        if is_removing {
                            Sense::hover()
                        } else {
                            Sense::click()
                        },
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
                            row_painter.image(
                                t.id(),
                                av_rect,
                                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE.gamma_multiply(row_alpha),
                            );
                            drew = true;
                        }
                    }
                    if !drew {
                        row_painter.circle_filled(
                            av_rect.center(),
                            17.0,
                            avatar_color(&friend.username, false).gamma_multiply(row_alpha),
                        );
                        let initial = friend
                            .username
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_ascii_uppercase()
                            .to_string();
                        row_painter.text(
                            av_rect.center(),
                            Align2::CENTER_CENTER,
                            initial,
                            FontId::proportional(14.0),
                            Color32::WHITE.gamma_multiply(row_alpha),
                        );
                    }
                    if friend.status == FriendshipStatus::Accepted
                        && !self.is_blocked(friend.user_id)
                    {
                        icons::paint_status_dot_pulse(
                            &row_painter,
                            av_rect.right_bottom() - egui::vec2(3.0, 3.0),
                            friend.presence,
                            ring_bg,
                            pulse,
                        );
                    }
                    if !is_removing && av_resp.clicked() {
                        self.open_profile_popup(
                            friend.user_id,
                            friend.username.clone(),
                            av_rect.right_top(),
                        );
                    }

                    // Name + Status label
                    let tx = av_rect.right() + 12.0;
                    row_painter.text(
                        egui::pos2(tx, shifted_rect.center().y - 2.0),
                        Align2::LEFT_BOTTOM,
                        &friend.username,
                        FontId::proportional(15.0),
                        theme::TEXT_PRIMARY.gamma_multiply(row_alpha),
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
                    row_painter.text(
                        egui::pos2(tx, shifted_rect.center().y + 2.0),
                        Align2::LEFT_TOP,
                        sub_text,
                        FontId::proportional(12.5),
                        theme::TEXT_MUTED.gamma_multiply(row_alpha),
                    );

                    // Right-side circular Discord action buttons (36px)
                    let cy = shifted_rect.center().y;
                    let right_x = shifted_rect.right() - 28.0;

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
                        row_painter.rect_filled(
                            ur,
                            4.0,
                            if uresp.hovered() {
                                theme::BG_ACTIVE
                            } else {
                                theme::BG_SECONDARY
                            },
                        );
                        row_painter.text(
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
                            // Decline X (triggers #5 250ms slideOutRight)
                            let xr = Rect::from_center_size(
                                egui::pos2(right_x, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let xresp = ui.interact(
                                xr,
                                ui.id().with(("decline", friend.user_id)),
                                if is_removing {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                },
                            );
                            row_painter.circle_filled(
                                xr.center(),
                                18.0,
                                (if !is_removing && xresp.hovered() {
                                    theme::DANGER
                                } else {
                                    theme::BG_SECONDARY
                                })
                                .gamma_multiply(row_alpha),
                            );
                            icons::paint_x(
                                &row_painter,
                                xr.center(),
                                16.0,
                                (if !is_removing && xresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_SECONDARY
                                })
                                .gamma_multiply(row_alpha),
                            );
                            if !is_removing && xresp.on_hover_text("Decline").clicked() {
                                if reduced {
                                    self.respond_friend(friend.user_id, false);
                                } else {
                                    self.removing_friends.insert(friend.user_id, (now, false));
                                    self.animating = true;
                                }
                            }

                            // Accept Check (triggers #5 250ms slideOutRight)
                            let cr = Rect::from_center_size(
                                egui::pos2(right_x - 46.0, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let cresp = ui.interact(
                                cr,
                                ui.id().with(("accept", friend.user_id)),
                                if is_removing {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                },
                            );
                            row_painter.circle_filled(
                                cr.center(),
                                18.0,
                                (if !is_removing && cresp.hovered() {
                                    theme::SUCCESS
                                } else {
                                    theme::BG_SECONDARY
                                })
                                .gamma_multiply(row_alpha),
                            );
                            icons::paint_check(
                                &row_painter,
                                cr.center(),
                                16.0,
                                (if !is_removing && cresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_SECONDARY
                                })
                                .gamma_multiply(row_alpha),
                            );
                            if !is_removing && cresp.on_hover_text("Accept").clicked() {
                                if reduced {
                                    self.respond_friend(friend.user_id, true);
                                } else {
                                    self.removing_friends.insert(friend.user_id, (now, true));
                                    self.animating = true;
                                }
                            }
                        } else {
                            row_painter.text(
                                egui::pos2(shifted_rect.right() - 12.0, cy),
                                Align2::RIGHT_CENTER,
                                "Pending",
                                FontId::proportional(12.5),
                                theme::TEXT_MUTED.gamma_multiply(row_alpha),
                            );
                        }
                    } else {
                        // #7 Hover: Action buttons slide in from right (+8px -> 0, opacity 0 -> 1, 150ms)
                        let hover_t = ui.ctx().animate_bool_with_time(
                            ui.id().with(("friend_actions_hover", friend.user_id)),
                            row_hovered,
                            if reduced { 0.001 } else { 0.15 },
                        );
                        if hover_t > 0.01 && hover_t < 0.99 {
                            self.animating = true;
                        }
                        if hover_t > 0.02 {
                            let slide_off = (1.0 - hover_t) * 8.0;
                            let act_x = right_x + slide_off;

                            // Call button
                            let cr = Rect::from_center_size(
                                egui::pos2(act_x, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let cresp = ui.interact(
                                cr,
                                ui.id().with(("call_f", friend.user_id)),
                                Sense::click(),
                            );
                            row_painter.circle_filled(
                                cr.center(),
                                18.0,
                                (if cresp.hovered() {
                                    theme::BG_TERTIARY
                                } else {
                                    theme::BG_SECONDARY
                                })
                                .gamma_multiply(hover_t),
                            );
                            icons::paint_phone(
                                &row_painter,
                                cr.center(),
                                16.0,
                                (if cresp.hovered() {
                                    theme::TEXT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                })
                                .gamma_multiply(hover_t),
                            );
                            if cresp.on_hover_text("Start Voice Call").clicked() {
                                self.open_dm(friend.user_id, true);
                            }

                            // Message button
                            let mr = Rect::from_center_size(
                                egui::pos2(act_x - 46.0, cy),
                                egui::vec2(36.0, 36.0),
                            );
                            let mresp = ui.interact(
                                mr,
                                ui.id().with(("msg_f", friend.user_id)),
                                Sense::click(),
                            );
                            row_painter.circle_filled(
                                mr.center(),
                                18.0,
                                (if mresp.hovered() {
                                    theme::BG_TERTIARY
                                } else {
                                    theme::BG_SECONDARY
                                })
                                .gamma_multiply(hover_t),
                            );
                            icons::paint_message_circle(
                                &row_painter,
                                mr.center(),
                                16.0,
                                (if mresp.hovered() {
                                    theme::TEXT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                })
                                .gamma_multiply(hover_t),
                            );
                            if mresp.on_hover_text("Message").clicked() {
                                self.open_dm(friend.user_id, false);
                            }
                        }
                    }
                }
            });
    }
}
