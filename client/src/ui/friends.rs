//! Original Wireline People (Friends) Workspace (100% Dynamic Dark & Light Mode):
//! 56px minimal header with sliding indigo tab underline, centered studio card list,
//! inline search pill, hover slide-in action buttons, and 250ms slide-out on request actions.

use super::{anim, avatar_color, icons, FriendsTab, SkyApp};
use crate::sounds::SoundEffect;
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_friends_home(&mut self, ui: &mut Ui) {
        let p = theme::of(ui.ctx());
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

        // 1. Minimal 56px Top Header Bar
        let (header_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), Sense::hover());
        ui.painter().rect_filled(header_rect, 0.0, p.canvas);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            p.hairline_stroke(),
        );

        let mut active_tab_rect: Option<Rect> = None;
        let header_inner = header_rect.shrink2(egui::vec2(20.0, 8.0));
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(header_inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.label(theme::semibold("People", 16.5).color(p.text_primary));
                ui.add_space(10.0);

                for (tab, label) in [
                    (FriendsTab::Online, "Online"),
                    (FriendsTab::All, "All"),
                    (FriendsTab::Pending, "Requests"),
                    (FriendsTab::Blocked, "Blocked"),
                ] {
                    let active = self.friends_tab == tab;
                    let show_badge = tab == FriendsTab::Pending && pending_count > 0;
                    let btn_w = if show_badge {
                        92.0
                    } else if label == "Requests" {
                        74.0
                    } else if label == "Blocked" {
                        68.0
                    } else if label == "All" {
                        40.0
                    } else {
                        60.0
                    };
                    let (tr, tresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 30.0), Sense::click());
                    if active {
                        active_tab_rect = Some(tr);
                    }
                    if tresp.hovered() && !active {
                        ui.painter().rect_filled(tr, 8.0, p.hover);
                    }
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
                        FontId::new(13.5, theme::semibold_family()),
                        if active {
                            p.text_primary
                        } else {
                            p.text_secondary
                        },
                    );
                    if show_badge {
                        let badge_c = tr.right_center() - egui::vec2(13.0, 0.0);
                        ui.painter()
                            .circle_filled(badge_c, 8.0 * badge_scale, p.success);
                        ui.painter().text(
                            badge_c,
                            Align2::CENTER_CENTER,
                            pending_count.min(99).to_string(),
                            FontId::new((10.0 * badge_scale).max(6.5), theme::semibold_family()),
                            Color32::WHITE,
                        );
                    }
                    if tresp.clicked() {
                        self.friends_tab = tab;
                        self.play_sound(SoundEffect::UiClick);
                    }
                }

                ui.add_space(6.0);
                // Indigo "+ Add Friend" button
                let add_active = self.friends_tab == FriendsTab::AddFriend;
                let (ar, aresp) = ui.allocate_exact_size(egui::vec2(96.0, 30.0), Sense::click());
                if add_active {
                    active_tab_rect = Some(ar);
                }
                ui.painter().rect_filled(
                    ar,
                    8.0,
                    if add_active { p.accent_soft } else { p.accent },
                );
                ui.painter().text(
                    ar.center(),
                    Align2::CENTER_CENTER,
                    "+ Add Friend",
                    FontId::new(13.0, theme::semibold_family()),
                    if add_active { p.accent } else { Color32::WHITE },
                );
                if aresp.clicked() {
                    self.friends_tab = FriendsTab::AddFriend;
                    self.friend_error.clear();
                    self.play_sound(SoundEffect::UiClick);
                }

                // Right: Inbox bell
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (bell_r, bell_resp) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
                    if self.show_inbox_popover || bell_resp.hovered() {
                        ui.painter().rect_filled(bell_r, 8.0, p.hover);
                    }
                    icons::paint_inbox(
                        ui.painter(),
                        bell_r.center(),
                        17.5,
                        if bell_resp.hovered() {
                            p.text_primary
                        } else {
                            p.text_secondary
                        },
                    );
                    if incoming_count > 0 {
                        ui.painter().circle_filled(
                            bell_r.right_top() + egui::vec2(-5.0, 5.0),
                            4.5,
                            p.danger,
                        );
                    }
                    if bell_resp.on_hover_text("Inbox").clicked() {
                        self.show_inbox_popover = !self.show_inbox_popover;
                        self.play_sound(SoundEffect::UiClick);
                    }
                });
            },
        );

        // Sliding Indigo Tab Underline (150ms)
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
            ui.painter().rect_filled(bar, 2.0, p.accent);
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
        let p = theme::of(ui.ctx());
        let avail_w = ui.available_width();
        let col_w = (avail_w - 48.0).clamp(280.0, 520.0);
        let pad = ((avail_w - col_w) * 0.5).max(24.0);

        ui.add_space(40.0);
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    ui.cursor().min + egui::vec2(pad, 0.0),
                    egui::vec2(col_w, 320.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_width(col_w);
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(p.border_stroke())
                    .corner_radius(14.0)
                    .inner_margin(egui::Margin::same(24))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(theme::semibold("Add a Friend", 18.0).color(p.text_primary));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Connect using their Wireline username or email address.")
                                .size(13.5)
                                .color(p.text_secondary),
                        );
                        ui.add_space(16.0);

                        let focus_id = ui.id().with("wireline_add_friend_input");
                        let has_focus = ui.memory(|m| m.has_focus(focus_id));
                        let stroke = if !self.friend_error.is_empty() {
                            Stroke::new(1.5_f32, p.danger)
                        } else if has_focus {
                            p.focus_stroke()
                        } else {
                            p.border_stroke()
                        };

                        egui::Frame::new()
                            .fill(p.canvas)
                            .stroke(stroke)
                            .corner_radius(10.0)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let input_w = (ui.available_width() - 134.0).max(110.0);
                                    let field = ui.add_sized(
                                        [input_w, 30.0],
                                        egui::TextEdit::singleline(&mut self.friend_query)
                                            .id(focus_id)
                                            .frame(false)
                                            .hint_text("Username or email..."),
                                    );
                                    if field.changed() {
                                        self.friend_error.clear();
                                    }
                                    let send_btn = egui::Button::new(
                                        theme::semibold("Send Request", 13.0).color(Color32::WHITE),
                                    )
                                    .fill(p.accent)
                                    .corner_radius(8.0)
                                    .min_size(egui::vec2(124.0, 32.0));
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
                                    .color(p.danger),
                            );
                        } else if !self.notice.is_empty() {
                            ui.add_space(8.0);
                            self.ui_notice(ui);
                        }
                    });
            },
        );
    }

    fn ui_friends_list(&mut self, ui: &mut Ui) {
        let p = theme::of(ui.ctx());
        let now = Self::now_secs();
        let reduced = self.config.reduced_motion;

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

        let q = self.friends_search.trim().to_ascii_lowercase();
        let filtered: Vec<_> = match self.friends_tab {
            FriendsTab::Online => friends
                .into_iter()
                .filter(|f| {
                    f.status == FriendshipStatus::Accepted
                        && f.online
                        && !self.is_blocked(f.user_id)
                        && (q.is_empty() || f.username.to_ascii_lowercase().contains(&q))
                })
                .collect(),
            FriendsTab::All => friends
                .into_iter()
                .filter(|f| {
                    (f.status == FriendshipStatus::Accepted || f.direction == "mutual")
                        && !self.is_blocked(f.user_id)
                        && (q.is_empty() || f.username.to_ascii_lowercase().contains(&q))
                })
                .collect(),
            FriendsTab::Pending => friends
                .into_iter()
                .filter(|f| {
                    f.status == FriendshipStatus::Pending
                        && (q.is_empty() || f.username.to_ascii_lowercase().contains(&q))
                })
                .collect(),
            FriendsTab::Blocked => friends
                .into_iter()
                .filter(|f| {
                    self.is_blocked(f.user_id)
                        && (q.is_empty() || f.username.to_ascii_lowercase().contains(&q))
                })
                .collect(),
            FriendsTab::AddFriend => Vec::new(),
        };

        let header_label = match self.friends_tab {
            FriendsTab::Online => format!("ONLINE — {}", filtered.len()),
            FriendsTab::All => format!("ALL FRIENDS — {}", filtered.len()),
            FriendsTab::Pending => format!("REQUESTS — {}", filtered.len()),
            FriendsTab::Blocked => format!("BLOCKED — {}", filtered.len()),
            FriendsTab::AddFriend => String::new(),
        };
        let pulse = self.pulse_time(ui.ctx());

        let avail_w = ui.available_width();
        let col_w = (avail_w - 48.0).clamp(320.0, 780.0);
        let side_pad = ((avail_w - col_w) * 0.5).max(20.0);

        ui.add_space(18.0);
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    ui.cursor().min + egui::vec2(side_pad, 0.0),
                    egui::vec2(col_w, 10_000.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_width(col_w);

                // Clean rounded search pill
                egui::Frame::new()
                    .fill(p.input_bg)
                    .stroke(p.border_stroke())
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(12, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (sr, _) =
                                ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::hover());
                            icons::paint_search(ui.painter(), sr.center(), 15.0, p.text_muted);
                            let w = (ui.available_width() - 28.0).max(120.0);
                            ui.add_sized(
                                [w, 20.0],
                                egui::TextEdit::singleline(&mut self.friends_search)
                                    .frame(false)
                                    .hint_text("Filter people..."),
                            );
                            if !self.friends_search.is_empty() {
                                let (xr, xresp) =
                                    ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
                                icons::paint_x(ui.painter(), xr.center(), 14.0, p.text_secondary);
                                if xresp.clicked() {
                                    self.friends_search.clear();
                                }
                            }
                        });
                    });
                ui.add_space(18.0);

                if filtered.is_empty() {
                    ui.add_space(56.0);
                    ui.vertical_centered(|ui| {
                        let (title, sub) = match self.friends_tab {
                            FriendsTab::Online => (
                                "No one's online right now",
                                "Friends who come online will appear here.",
                            ),
                            FriendsTab::All => (
                                "No friends yet",
                                "Add someone by username or email to start a conversation.",
                            ),
                            FriendsTab::Pending => (
                                "No pending requests",
                                "Incoming and outgoing friend requests show up here.",
                            ),
                            FriendsTab::Blocked => (
                                "No blocked users",
                                "Users you block won't appear in your chats.",
                            ),
                            FriendsTab::AddFriend => ("", ""),
                        };
                        ui.label(theme::semibold(title, 15.5).color(p.text_primary));
                        ui.add_space(4.0);
                        ui.label(RichText::new(sub).size(13.5).color(p.text_muted));
                        if matches!(self.friends_tab, FriendsTab::Online | FriendsTab::All)
                            && self.friends_search.is_empty()
                        {
                            ui.add_space(14.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        theme::semibold("+ Add Friend", 13.5)
                                            .color(Color32::WHITE),
                                    )
                                    .fill(p.accent)
                                    .corner_radius(8.0)
                                    .min_size(egui::vec2(124.0, 34.0)),
                                )
                                .clicked()
                            {
                                self.friends_tab = FriendsTab::AddFriend;
                            }
                        }
                    });
                    return;
                }

                ui.label(theme::semibold(&header_label, 11.5).color(p.text_muted));
                ui.add_space(8.0);

                for friend in filtered {
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
                    let row_h = (64.0 * (1.0 - rem_p)).max(0.5);

                    let (row_rect, _) = ui.allocate_exact_size(
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
                                .is_some_and(|pos| row_rect.contains(pos))
                        });
                    let card_rect = row_rect
                        .shrink2(egui::vec2(0.0, 3.0))
                        .translate(egui::vec2(slide_x, 0.0));
                    let row_painter = ui.painter_at(row_rect);

                    let card_bg = if row_hovered { p.hover } else { p.surface };
                    row_painter.rect_filled(card_rect, 12.0, card_bg.gamma_multiply(row_alpha));
                    row_painter.rect_stroke(
                        card_rect,
                        12.0,
                        Stroke::new(1.0_f32, p.border.gamma_multiply(row_alpha)),
                        egui::StrokeKind::Inside,
                    );

                    let av_rect = Rect::from_center_size(
                        egui::pos2(card_rect.left() + 28.0, card_rect.center().y),
                        egui::vec2(36.0, 36.0),
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
                            18.0,
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
                            FontId::new(14.5, theme::semibold_family()),
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
                            card_bg,
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

                    let tx = av_rect.right() + 12.0;
                    row_painter.text(
                        egui::pos2(tx, card_rect.center().y - 1.0),
                        Align2::LEFT_BOTTOM,
                        &friend.username,
                        FontId::new(14.5, theme::semibold_family()),
                        p.text_primary.gamma_multiply(row_alpha),
                    );
                    let sub_text = if self.is_blocked(friend.user_id) {
                        "Blocked".to_string()
                    } else if friend.status == FriendshipStatus::Pending {
                        if friend.direction == "incoming" {
                            "Incoming request".to_string()
                        } else {
                            "Outgoing request".to_string()
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
                        egui::pos2(tx, card_rect.center().y + 2.0),
                        Align2::LEFT_TOP,
                        sub_text,
                        FontId::proportional(12.0),
                        p.text_muted.gamma_multiply(row_alpha),
                    );

                    let cy = card_rect.center().y;
                    let right_x = card_rect.right() - 28.0;

                    if self.friends_tab == FriendsTab::Blocked {
                        let ur = Rect::from_center_size(
                            egui::pos2(right_x - 24.0, cy),
                            egui::vec2(80.0, 30.0),
                        );
                        let uresp = ui.interact(
                            ur,
                            ui.id().with(("unblock", friend.user_id)),
                            Sense::click(),
                        );
                        row_painter.rect_filled(
                            ur,
                            8.0,
                            if uresp.hovered() {
                                p.active
                            } else {
                                p.floating
                            },
                        );
                        row_painter.text(
                            ur.center(),
                            Align2::CENTER_CENTER,
                            "Unblock",
                            FontId::new(12.5, theme::semibold_family()),
                            p.text_primary,
                        );
                        if uresp.clicked() {
                            self.toggle_block(friend.user_id);
                        }
                    } else if friend.status == FriendshipStatus::Pending {
                        if friend.direction == "incoming" {
                            let xr = Rect::from_center_size(
                                egui::pos2(right_x, cy),
                                egui::vec2(34.0, 34.0),
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
                                17.0,
                                (if !is_removing && xresp.hovered() {
                                    p.danger
                                } else {
                                    p.floating
                                })
                                .gamma_multiply(row_alpha),
                            );
                            icons::paint_x(
                                &row_painter,
                                xr.center(),
                                15.0,
                                (if !is_removing && xresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    p.text_secondary
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

                            let cr = Rect::from_center_size(
                                egui::pos2(right_x - 44.0, cy),
                                egui::vec2(34.0, 34.0),
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
                                17.0,
                                (if !is_removing && cresp.hovered() {
                                    p.success
                                } else {
                                    p.floating
                                })
                                .gamma_multiply(row_alpha),
                            );
                            icons::paint_check(
                                &row_painter,
                                cr.center(),
                                15.0,
                                (if !is_removing && cresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    p.text_secondary
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
                            let xr = Rect::from_center_size(
                                egui::pos2(right_x, cy),
                                egui::vec2(34.0, 34.0),
                            );
                            let xresp = ui.interact(
                                xr,
                                ui.id().with(("cancel_out", friend.user_id)),
                                if is_removing {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                },
                            );
                            row_painter.circle_filled(
                                xr.center(),
                                17.0,
                                (if !is_removing && xresp.hovered() {
                                    p.danger
                                } else {
                                    p.floating
                                })
                                .gamma_multiply(row_alpha),
                            );
                            icons::paint_x(
                                &row_painter,
                                xr.center(),
                                15.0,
                                (if !is_removing && xresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    p.text_secondary
                                })
                                .gamma_multiply(row_alpha),
                            );
                            if !is_removing
                                && xresp.on_hover_text("Cancel Friend Request").clicked()
                            {
                                if reduced {
                                    self.respond_friend(friend.user_id, false);
                                } else {
                                    self.removing_friends.insert(friend.user_id, (now, false));
                                    self.animating = true;
                                }
                            }
                        }
                    } else {
                        // Hover slide-in Chat & Call buttons
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

                            let cr = Rect::from_center_size(
                                egui::pos2(act_x, cy),
                                egui::vec2(34.0, 34.0),
                            );
                            let cresp = ui.interact(
                                cr,
                                ui.id().with(("call_f", friend.user_id)),
                                Sense::click(),
                            );
                            row_painter.circle_filled(
                                cr.center(),
                                17.0,
                                (if cresp.hovered() {
                                    p.accent
                                } else {
                                    p.floating
                                })
                                .gamma_multiply(hover_t),
                            );
                            icons::paint_phone(
                                &row_painter,
                                cr.center(),
                                15.5,
                                (if cresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    p.text_primary
                                })
                                .gamma_multiply(hover_t),
                            );
                            if cresp.on_hover_text("Voice Call").clicked() {
                                self.open_dm(friend.user_id, true);
                            }

                            let mr = Rect::from_center_size(
                                egui::pos2(act_x - 42.0, cy),
                                egui::vec2(34.0, 34.0),
                            );
                            let mresp = ui.interact(
                                mr,
                                ui.id().with(("msg_f", friend.user_id)),
                                Sense::click(),
                            );
                            row_painter.circle_filled(
                                mr.center(),
                                17.0,
                                (if mresp.hovered() {
                                    p.accent
                                } else {
                                    p.floating
                                })
                                .gamma_multiply(hover_t),
                            );
                            icons::paint_message_circle(
                                &row_painter,
                                mr.center(),
                                15.5,
                                (if mresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    p.text_primary
                                })
                                .gamma_multiply(hover_t),
                            );
                            if mresp.on_hover_text("Message").clicked() {
                                self.open_dm(friend.user_id, false);
                            }
                        }
                    }
                }
            },
        );
    }
}
