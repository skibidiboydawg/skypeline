//! Discord-faithful overlays: Status Popover, 300px User Profile Popup,
//! 360px Emoji Picker, Notifications Inbox, Full-Screen Settings Modal, and Toast Notifications.

use super::{anim, avatar_color, icons, ProfilePopup, SettingsTab, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Pos2, Rect, RichText, Sense, Stroke};
use skypeline_shared::{FriendshipStatus, PresenceStatus};

const EMOJI_CATEGORIES: &[(&str, &[&str])] = &[
    (
        "Smileys",
        &[
            "😀", "😃", "😄", "😁", "😆", "😅", "😂", "🤣", "😊", "😇", "🙂", "😉", "😍", "🥰",
            "😘", "😋", "😜", "🤪", "🤨", "🧐", "🤓", "😎", "🥳", "😏", "😒", "😞", "😔", "😟",
            "😕", "🙁", "😣", "😖", "😫", "😩", "🥺", "😢", "😭", "😤", "😠", "😡", "🤯", "😳",
            "🥵", "🥶", "😱", "😨", "😰", "🤔", "🤭", "🤫", "🤥", "😶", "😐", "😑", "😬", "🙄",
            "😮", "😴", "🤤", "🤮", "🤧", "😷", "🤒", "🤕", "🤠", "🤡", "👻", "💀", "👽", "🤖",
        ],
    ),
    (
        "Hands",
        &[
            "👍", "👎", "👏", "🙌", "👐", "🤲", "🤝", "🙏", "✌️", "🤞", "🤟", "🤘", "🤙", "👌",
            "🤌", "🤏", "👈", "👉", "👆", "👇", "☝️", "✋", "🤚", "🖐️", "🖖", "👋", "💪", "🫡",
        ],
    ),
    (
        "Hearts",
        &[
            "❤️", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "🤎", "💔", "❤️‍🔥", "💕", "💞", "💓",
            "💗", "💖", "💘", "💝", "💟", "✨", "🔥", "💥", "💫", "💯", "🎉", "🎊", "🏆", "⭐",
        ],
    ),
    (
        "Objects",
        &[
            "🎮", "🎧", "🎤", "🎵", "🎶", "💻", "🖥️", "📱", "⌨️", "🖱️", "📷", "🎥", "📺", "☕",
            "🍕", "🍔", "🍟", "🌮", "🍣", "🍩", "🍪", "🍺", "🚀", "⚡", "🌙", "☀️", "🌈", "❄️",
        ],
    ),
];

impl SkyApp {
    /// Status switcher popover anchored above the bottom-left 52px user panel.
    pub(crate) fn ui_profile_popover(&mut self, ctx: &Context) {
        if !self.show_profile_popover {
            return;
        }
        let screen = ctx.screen_rect();
        let anchor = egui::pos2(10.0, (screen.bottom() - 320.0).max(16.0));
        let resp = egui::Area::new(egui::Id::new("profile_status_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::BG_FLOATING)
                    .stroke(theme::border())
                    .corner_radius(8.0)
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(140),
                    })
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(224.0);
                        let name = self
                            .me
                            .as_ref()
                            .map(|m| m.username.clone())
                            .unwrap_or_else(|| "User".into());
                        let email = self
                            .me
                            .as_ref()
                            .map(|m| m.email.clone())
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(&name)
                                .size(15.0)
                                .strong()
                                .color(theme::TEXT_PRIMARY),
                        );
                        if !email.is_empty() {
                            ui.label(RichText::new(&email).small().color(theme::TEXT_MUTED));
                        }

                        ui.add_space(8.0);
                        let (div_r, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(div_r, 0.0, theme::BORDER_SUBTLE);
                        ui.add_space(8.0);

                        ui.label(
                            RichText::new("SET STATUS")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(4.0);

                        for (status, label, desc) in [
                            (PresenceStatus::Online, "Online", "Ready to chat"),
                            (PresenceStatus::Away, "Idle", "Stepped away"),
                            (
                                PresenceStatus::Dnd,
                                "Do Not Disturb",
                                "Mute desktop notifications",
                            ),
                        ] {
                            let active = self.presence == status;
                            let (r, btn) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 34.0),
                                Sense::click(),
                            );
                            if active || btn.hovered() {
                                ui.painter().rect_filled(
                                    r,
                                    4.0,
                                    if active {
                                        theme::BG_ACTIVE
                                    } else {
                                        theme::BG_HOVER
                                    },
                                );
                            }
                            icons::paint_status_dot(
                                ui.painter(),
                                r.left_center() + egui::vec2(14.0, 0.0),
                                status,
                                if active {
                                    theme::BG_ACTIVE
                                } else {
                                    theme::BG_FLOATING
                                },
                            );
                            ui.painter().text(
                                r.left_top() + egui::vec2(30.0, 4.0),
                                Align2::LEFT_TOP,
                                label,
                                FontId::proportional(13.0),
                                theme::TEXT_PRIMARY,
                            );
                            ui.painter().text(
                                r.left_top() + egui::vec2(30.0, 19.0),
                                Align2::LEFT_TOP,
                                desc,
                                FontId::proportional(10.5),
                                theme::TEXT_MUTED,
                            );
                            if btn.clicked() {
                                self.set_presence(status);
                                self.show_profile_popover = false;
                            }
                        }

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("CUSTOM STATUS")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(4.0);
                        let status_resp = egui::Frame::new()
                            .fill(theme::BG_TERTIARY)
                            .stroke(theme::input_border())
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.custom_status)
                                        .frame(false)
                                        .desired_width(ui.available_width())
                                        .hint_text("What's up?"),
                                )
                            })
                            .inner;
                        if status_resp.changed() {
                            self.config.save();
                        }

                        ui.add_space(8.0);
                        let (div2, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(div2, 0.0, theme::BORDER_SUBTLE);
                        ui.add_space(6.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("User Settings").color(theme::TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .min_size(egui::vec2(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.open_settings_modal(self.settings_tab);
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Log Out").color(theme::DANGER))
                                    .fill(Color32::TRANSPARENT)
                                    .min_size(egui::vec2(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.sign_out();
                        }
                    });
            });

        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                let dock_area =
                    Rect::from_min_max(egui::pos2(0.0, screen.bottom() - 56.0), egui::pos2(240.0, screen.bottom()));
                if !resp.response.rect.contains(pos) && !dock_area.contains(pos) {
                    self.show_profile_popover = false;
                }
            }
        }
    }

    /// 300px floating Discord User Profile Card with #8 scale-in (180ms) & reverse exit (150ms).
    pub(crate) fn ui_user_profile_popup(&mut self, ctx: &Context) {
        let Some(popup) = self.profile_popup.clone() else {
            return;
        };
        let now = Self::now_secs();
        let (alpha, scale, _, done_exit) = anim::modal_transition(
            (now - self.profile_popup_opened_at) as f32,
            self.profile_popup_closing_at.map(|c| (now - c) as f32),
            self.config.reduced_motion,
        );
        if done_exit {
            self.profile_popup = None;
            self.profile_popup_closing_at = None;
            return;
        }
        if alpha < 0.999 || self.profile_popup_closing_at.is_some() {
            self.animating = true;
        }

        let screen = ctx.screen_rect();
        let card_w = 300.0 * scale;
        let card_h = 310.0 * scale;
        let base_x = (popup.anchor.x + 12.0).clamp(12.0, (screen.right() - 312.0).max(12.0));
        let base_y = (popup.anchor.y - 20.0).clamp(12.0, (screen.bottom() - 322.0).max(12.0));
        let x = base_x + (300.0 - card_w) * 0.5;
        let y = base_y + (310.0 - card_h) * 0.5;

        let is_me = self.me.as_ref().is_some_and(|m| m.user_id == popup.user_id);
        let is_blocked = self.is_blocked(popup.user_id);
        let pulse = self.pulse_time(ctx);
        let avatar_hash = if is_me {
            self.me.as_ref().and_then(|m| m.avatar_hash.clone())
        } else {
            self.friends
                .iter()
                .find(|f| f.user_id == popup.user_id)
                .and_then(|f| f.avatar_hash.clone())
        };

        let mut close_popup = false;
        let mut open_dm_for: Option<i64> = None;
        let mut toggle_block_for: Option<i64> = None;

        let resp = egui::Area::new(egui::Id::new("user_profile_popup_card"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(x, y))
            .show(ctx, |ui| {
                if alpha < 0.999 {
                    ui.multiply_opacity(alpha);
                }
                egui::Frame::new()
                    .fill(theme::BG_FLOATING)
                    .stroke(theme::border())
                    .corner_radius(8.0)
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha((150.0 * alpha) as u8),
                    })
                    .inner_margin(egui::Margin::same(0))
                    .show(ui, |ui| {
                        ui.set_width(card_w);

                        // 60px top banner + 48px overlap space for 80px avatar
                        let (banner_area, _) =
                            ui.allocate_exact_size(egui::vec2(card_w, 102.0), Sense::hover());
                        let banner = Rect::from_min_size(banner_area.min, egui::vec2(card_w, 60.0));
                        ui.painter().rect_filled(
                            banner,
                            egui::CornerRadius {
                                nw: 8,
                                ne: 8,
                                sw: 0,
                                se: 0,
                            },
                            avatar_color(&popup.username, is_me),
                        );

                        let av_center = banner.left_bottom() + egui::vec2(56.0, 0.0);
                        ui.painter()
                            .circle_filled(av_center, 46.0, theme::BG_FLOATING);
                        let av_rect = Rect::from_center_size(av_center, egui::vec2(80.0, 80.0));

                        let mut drew = false;
                        if let Some(h) = avatar_hash.as_deref() {
                            let tex = self.avatars.texture(
                                &ui.ctx().clone(),
                                &self.api(),
                                self.token.as_deref(),
                                popup.user_id,
                                Some(h),
                                &self.events_tx,
                            );
                            if let Some(t) = tex {
                                ui.painter().image(
                                    t.id(),
                                    av_rect,
                                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                drew = true;
                            }
                        }
                        if !drew {
                            ui.painter().circle_filled(
                                av_center,
                                40.0,
                                avatar_color(&popup.username, is_me),
                            );
                            let initial = popup
                                .username
                                .chars()
                                .next()
                                .unwrap_or('?')
                                .to_ascii_uppercase()
                                .to_string();
                            ui.painter().text(
                                av_center,
                                Align2::CENTER_CENTER,
                                initial,
                                FontId::proportional(30.0),
                                Color32::WHITE,
                            );
                        }
                        icons::paint_status_dot_pulse(
                            ui.painter(),
                            av_center + egui::vec2(28.0, 28.0),
                            popup.status,
                            theme::BG_FLOATING,
                            pulse,
                        );

                        egui::Frame::new()
                            .inner_margin(egui::Margin {
                                left: 16,
                                right: 16,
                                top: 4,
                                bottom: 16,
                            })
                            .show(ui, |ui| {
                                ui.set_width(card_w - 32.0);
                                ui.label(
                                    RichText::new(&popup.username)
                                        .size(18.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "@{} · {}",
                                        popup.username.to_ascii_lowercase().replace(' ', ""),
                                        popup.status.label()
                                    ))
                                    .size(12.5)
                                    .color(theme::TEXT_MUTED),
                                );

                                ui.add_space(10.0);
                                let (div, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 1.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(div, 0.0, theme::BORDER_SUBTLE);
                                ui.add_space(10.0);

                                ui.label(
                                    RichText::new("ABOUT ME")
                                        .size(11.0)
                                        .strong()
                                        .color(theme::TEXT_MUTED),
                                );
                                ui.add_space(2.0);
                                let bio = if is_me && !self.config.bio.trim().is_empty() {
                                    self.config.bio.as_str()
                                } else {
                                    "Wireline desktop member."
                                };
                                ui.label(RichText::new(bio).size(13.0).color(theme::TEXT_SECONDARY));

                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("WIRELINE MEMBER")
                                        .size(11.0)
                                        .strong()
                                        .color(theme::TEXT_MUTED),
                                );
                                ui.label(
                                    RichText::new(format!("User ID #{}", popup.user_id))
                                        .size(12.5)
                                        .color(theme::TEXT_SECONDARY),
                                );

                                if !is_me {
                                    ui.add_space(14.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Send Message")
                                                        .strong()
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(theme::ACCENT)
                                                .corner_radius(4.0)
                                                .min_size(egui::vec2(148.0, 32.0)),
                                            )
                                            .clicked()
                                        {
                                            open_dm_for = Some(popup.user_id);
                                            close_popup = true;
                                        }
                                        let blk_label = if is_blocked { "Unblock" } else { "Block" };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(blk_label).color(if is_blocked {
                                                        theme::TEXT_PRIMARY
                                                    } else {
                                                        theme::DANGER
                                                    }),
                                                )
                                                .fill(theme::BG_SECONDARY)
                                                .corner_radius(4.0)
                                                .min_size(egui::vec2(106.0, 32.0)),
                                            )
                                            .clicked()
                                        {
                                            toggle_block_for = Some(popup.user_id);
                                            close_popup = true;
                                        }
                                    });
                                }
                            });
                    });
            });

        if let Some(uid) = open_dm_for {
            self.open_dm(uid, false);
        }
        if let Some(uid) = toggle_block_for {
            self.toggle_block(uid);
        }
        if close_popup {
            self.close_profile_popup();
            return;
        }
        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !resp.response.rect.contains(pos) && pos.distance(popup.anchor) > 26.0 {
                    self.close_profile_popup();
                }
            }
        }
    }

    /// 360px Discord Emoji Picker floating above the message input bar.
    pub(crate) fn ui_emoji_picker(&mut self, ctx: &Context) {
        let Some(target) = self.emoji_picker_target else {
            return;
        };
        let screen = ctx.screen_rect();
        let w = 360.0;
        let h = 340.0;
        let pos = egui::pos2(
            (screen.right() - w - 20.0).max(250.0),
            (screen.bottom() - h - 76.0).max(60.0),
        );

        let mut picked: Option<String> = None;
        let resp = egui::Area::new(egui::Id::new("discord_emoji_picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::BG_FLOATING)
                    .stroke(theme::border())
                    .corner_radius(8.0)
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(w - 24.0);
                        ui.set_height(h - 24.0);

                        // Search bar
                        egui::Frame::new()
                            .fill(theme::BG_TERTIARY)
                            .stroke(theme::input_border())
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.emoji_search)
                                        .frame(false)
                                        .desired_width(ui.available_width())
                                        .hint_text("Filter emoji or paste any emoji..."),
                                );
                            });
                        ui.add_space(8.0);

                        let query = self.emoji_search.trim().to_ascii_lowercase();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (cat_name, emojis) in EMOJI_CATEGORIES {
                                    if !query.is_empty()
                                        && !cat_name.to_ascii_lowercase().contains(&query)
                                    {
                                        continue;
                                    }
                                    ui.label(
                                        RichText::new(cat_name.to_ascii_uppercase())
                                            .size(11.0)
                                            .strong()
                                            .color(theme::TEXT_MUTED),
                                    );
                                    ui.add_space(4.0);
                                    for row in emojis.chunks(7) {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                            for &emoji in row {
                                                let (r, eresp) = ui.allocate_exact_size(
                                                    egui::vec2(42.0, 36.0),
                                                    Sense::click(),
                                                );
                                                if eresp.hovered() {
                                                    ui.painter().rect_filled(
                                                        r,
                                                        6.0,
                                                        theme::BG_ACTIVE,
                                                    );
                                                }
                                                ui.painter().text(
                                                    r.center(),
                                                    Align2::CENTER_CENTER,
                                                    emoji,
                                                    FontId::proportional(20.0),
                                                    Color32::WHITE,
                                                );
                                                if eresp.clicked() {
                                                    picked = Some(emoji.to_string());
                                                }
                                            }
                                        });
                                    }
                                    ui.add_space(8.0);
                                }
                            });
                    });
            });

        if let Some(emoji) = picked {
            match target {
                Some(message_id) => {
                    if let Some(ch_id) = self.selected_channel {
                        self.toggle_reaction(ch_id, message_id, emoji);
                    }
                }
                None => {
                    self.draft.push_str(&emoji);
                    self.refocus_draft = true;
                }
            }
            self.emoji_picker_target = None;
            self.emoji_search.clear();
            return;
        }

        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                if !resp.response.rect.contains(p) {
                    self.emoji_picker_target = None;
                }
            }
        }
    }

    /// Notifications Inbox popover for incoming friend requests.
    pub(crate) fn ui_inbox_popover(&mut self, ctx: &Context) {
        if !self.show_inbox_popover {
            return;
        }
        let screen = ctx.screen_rect();
        let w = 320.0;
        let pos = egui::pos2((screen.right() - w - 16.0).max(250.0), 54.0);
        let incoming: Vec<_> = self
            .friends
            .iter()
            .filter(|f| f.status == FriendshipStatus::Pending && f.direction == "incoming")
            .cloned()
            .collect();

        let mut respond: Option<(i64, bool)> = None;
        let resp = egui::Area::new(egui::Id::new("notifications_inbox_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::BG_FLOATING)
                    .stroke(theme::border())
                    .corner_radius(8.0)
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(w - 28.0);
                        ui.label(
                            RichText::new("INBOX")
                                .size(12.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(8.0);
                        if incoming.is_empty() {
                            ui.label(
                                RichText::new("Nothing here yet. You're all caught up!")
                                    .color(theme::TEXT_MUTED),
                            );
                        } else {
                            for req in &incoming {
                                egui::Frame::new()
                                    .fill(theme::BG_SECONDARY)
                                    .corner_radius(6.0)
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(&req.username)
                                                        .strong()
                                                        .color(theme::TEXT_PRIMARY),
                                                );
                                                ui.label(
                                                    RichText::new("Incoming friend request")
                                                        .small()
                                                        .color(theme::TEXT_MUTED),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                RichText::new("Decline")
                                                                    .small()
                                                                    .color(theme::DANGER),
                                                            )
                                                            .fill(theme::BG_TERTIARY)
                                                            .corner_radius(4.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        respond = Some((req.user_id, false));
                                                    }
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                RichText::new("Accept")
                                                                    .small()
                                                                    .color(Color32::WHITE),
                                                            )
                                                            .fill(theme::SUCCESS)
                                                            .corner_radius(4.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        respond = Some((req.user_id, true));
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(6.0);
                            }
                        }
                    });
            });

        if let Some((uid, accept)) = respond {
            self.respond_friend(uid, accept);
        }
        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                if !resp.response.rect.contains(p) && p.y > 50.0 {
                    self.show_inbox_popover = false;
                }
            }
        }
    }

    /// Full-Screen Discord Settings Modal with #8 scale-in (180ms) & reverse exit (150ms).
    pub(crate) fn ui_settings_modal(&mut self, ctx: &Context) {
        if !self.show_settings_modal {
            return;
        }
        let now = Self::now_secs();
        let (alpha, scale, backdrop_alpha, done_exit) = anim::modal_transition(
            (now - self.settings_opened_at) as f32,
            self.settings_closing_at.map(|c| (now - c) as f32),
            self.config.reduced_motion,
        );
        if done_exit {
            self.show_settings_modal = false;
            self.settings_closing_at = None;
            return;
        }
        if alpha < 0.999 || self.settings_closing_at.is_some() {
            self.animating = true;
        }

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("discord_settings_modal"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                // Full-screen dim backdrop (180ms linear opacity fade)
                let (bg_rect, bg_resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    bg_rect,
                    0.0,
                    Color32::from_black_alpha((215.0 * backdrop_alpha) as u8),
                );

                let base_w = (screen.width() - 80.0).clamp(560.0, 920.0);
                let base_h = (screen.height() - 80.0).clamp(420.0, 660.0);
                let modal_w = base_w * scale;
                let modal_h = base_h * scale;
                let modal_rect =
                    Rect::from_center_size(screen.center(), egui::vec2(modal_w, modal_h));

                if bg_resp.clicked() {
                    if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                        if !modal_rect.contains(p) {
                            self.close_settings_modal();
                        }
                    }
                }

                if alpha < 0.999 {
                    ui.multiply_opacity(alpha);
                }

                ui.painter().rect_filled(modal_rect, 8.0, theme::BG_PRIMARY);
                ui.painter().rect_stroke(
                    modal_rect,
                    8.0,
                    theme::border(),
                    egui::StrokeKind::Outside,
                );

                // Left 220px navigation sidebar (#2b2d31)
                let nav_rect = Rect::from_min_size(modal_rect.min, egui::vec2(220.0, modal_h));
                ui.painter().rect_filled(
                    nav_rect,
                    egui::CornerRadius {
                        nw: 8,
                        ne: 0,
                        sw: 8,
                        se: 0,
                    },
                    theme::BG_SECONDARY,
                );

                let mut logout_clicked = false;
                let mut close_clicked = false;

                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(nav_rect.shrink2(egui::vec2(14.0, 20.0)))
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.label(
                            RichText::new("USER SETTINGS")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(4.0);
                        self.settings_nav_item(ui, SettingsTab::MyAccount, "My Account");

                        ui.add_space(12.0);
                        let (d1, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(d1, 0.0, theme::BORDER_SUBTLE);
                        ui.add_space(12.0);

                        ui.label(
                            RichText::new("APP SETTINGS")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.add_space(4.0);
                        self.settings_nav_item(ui, SettingsTab::Appearance, "Appearance");
                        self.settings_nav_item(ui, SettingsTab::VoiceVideo, "Voice & Video");
                        self.settings_nav_item(ui, SettingsTab::Keybinds, "Keybinds");

                        ui.add_space(16.0);
                        let (d2, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(d2, 0.0, theme::BORDER_SUBTLE);
                        ui.add_space(10.0);

                        let (lr, lresp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 32.0),
                            Sense::click(),
                        );
                        if lresp.hovered() {
                            ui.painter().rect_filled(
                                lr,
                                4.0,
                                Color32::from_rgb(0x46, 0x26, 0x29),
                            );
                        }
                        ui.painter().text(
                            lr.left_center() + egui::vec2(10.0, 0.0),
                            Align2::LEFT_CENTER,
                            "Log Out",
                            FontId::proportional(14.0),
                            theme::DANGER,
                        );
                        if lresp.clicked() {
                            logout_clicked = true;
                        }
                    },
                );

                // Right content area (#313338)
                let content_rect = Rect::from_min_max(
                    egui::pos2(nav_rect.right(), modal_rect.top()),
                    modal_rect.right_bottom(),
                );
                // Top-right Escape / X button
                let close_c = content_rect.right_top() + egui::vec2(-32.0, 32.0);
                let close_r = Rect::from_center_size(close_c, egui::vec2(34.0, 34.0));
                let close_resp = ui.interact(
                    close_r,
                    ui.id().with("settings_close_x"),
                    Sense::click(),
                );
                ui.painter().circle_stroke(
                    close_c,
                    16.0,
                    Stroke::new(
                        1.5_f32,
                        if close_resp.hovered() {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_MUTED
                        },
                    ),
                );
                icons::paint_x(
                    ui.painter(),
                    close_c,
                    15.0,
                    if close_resp.hovered() {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    },
                );
                ui.painter().text(
                    close_c + egui::vec2(0.0, 24.0),
                    Align2::CENTER_TOP,
                    "ESC",
                    FontId::proportional(10.5),
                    theme::TEXT_MUTED,
                );
                if close_resp.clicked() {
                    close_clicked = true;
                }

                let inner = content_rect.shrink2(egui::vec2(32.0, 28.0));
                let inner = Rect::from_min_max(
                    inner.min,
                    egui::pos2((inner.right() - 42.0).max(inner.left() + 200.0), inner.bottom()),
                );
                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(inner)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.settings_tab {
                                SettingsTab::MyAccount => self.ui_settings_account(ui),
                                SettingsTab::Appearance => self.ui_settings_appearance(ctx, ui),
                                SettingsTab::VoiceVideo => self.ui_settings_voice(ui),
                                SettingsTab::Keybinds => self.ui_settings_keybinds(ui),
                            });
                    },
                );

                if close_clicked {
                    self.close_settings_modal();
                }
                if logout_clicked {
                    self.show_settings_modal = false;
                    self.settings_closing_at = None;
                    self.sign_out();
                }
            });
    }

    fn settings_nav_item(&mut self, ui: &mut egui::Ui, tab: SettingsTab, label: &str) {
        let active = self.settings_tab == tab;
        let (r, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
        if active || resp.hovered() {
            ui.painter().rect_filled(
                r,
                4.0,
                if active {
                    theme::BG_ACTIVE
                } else {
                    theme::BG_HOVER
                },
            );
        }
        ui.painter().text(
            r.left_center() + egui::vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(14.0),
            if active {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            },
        );
        if resp.clicked() {
            self.settings_tab = tab;
        }
    }

    fn ui_settings_account(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("My Account")
                .size(20.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(16.0);

        let name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let email = self
            .me
            .as_ref()
            .map(|m| m.email.clone())
            .unwrap_or_default();
        let uid = self.me.as_ref().map(|m| m.user_id).unwrap_or(0);

        // Discord-style profile banner card
        egui::Frame::new()
            .fill(theme::BG_TERTIARY)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let (top_r, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 96.0),
                    Sense::hover(),
                );
                let banner = Rect::from_min_size(top_r.min, egui::vec2(top_r.width(), 64.0));
                ui.painter().rect_filled(
                    banner,
                    egui::CornerRadius {
                        nw: 8,
                        ne: 8,
                        sw: 0,
                        se: 0,
                    },
                    theme::ACCENT,
                );
                let av_c = banner.left_bottom() + egui::vec2(52.0, 0.0);
                ui.painter().circle_filled(av_c, 40.0, theme::BG_TERTIARY);
                ui.painter().circle_filled(av_c, 34.0, theme::ACCENT);
                let init = name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_ascii_uppercase()
                    .to_string();
                ui.painter().text(
                    av_c,
                    Align2::CENTER_CENTER,
                    init,
                    FontId::proportional(26.0),
                    Color32::WHITE,
                );
                icons::paint_status_dot(
                    ui.painter(),
                    av_c + egui::vec2(24.0, 24.0),
                    self.presence,
                    theme::BG_TERTIARY,
                );
                ui.painter().text(
                    av_c + egui::vec2(48.0, 4.0),
                    Align2::LEFT_TOP,
                    &name,
                    FontId::proportional(18.0),
                    theme::TEXT_PRIMARY,
                );

                egui::Frame::new()
                    .fill(theme::BG_SECONDARY)
                    .corner_radius(6.0)
                    .outer_margin(egui::Margin::same(16))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new("USERNAME")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.label(RichText::new(&name).color(theme::TEXT_PRIMARY));
                        ui.add_space(10.0);

                        ui.label(
                            RichText::new("EMAIL")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.label(RichText::new(&email).color(theme::TEXT_PRIMARY));
                        ui.add_space(10.0);

                        ui.label(
                            RichText::new("USER ID")
                                .size(11.0)
                                .strong()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.label(RichText::new(format!("#{uid}")).color(theme::TEXT_SECONDARY));
                    });
            });

        ui.add_space(16.0);
        ui.label(
            RichText::new("ABOUT ME")
                .size(11.5)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(4.0);
        let bio_resp = egui::Frame::new()
            .fill(theme::BG_TERTIARY)
            .stroke(theme::input_border())
            .corner_radius(4.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.config.bio)
                        .frame(false)
                        .desired_rows(3)
                        .desired_width(ui.available_width())
                        .hint_text("Tell friends a little about yourself..."),
                )
            })
            .inner;
        if bio_resp.changed() {
            self.config.save();
        }
    }

    fn ui_settings_appearance(&mut self, ctx: &Context, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Appearance")
                .size(20.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(16.0);

        // Live preview box
        egui::Frame::new()
            .fill(theme::BG_SECONDARY)
            .stroke(theme::border())
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if !self.config.compact_mode {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 19.0, theme::ACCENT);
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            "W",
                            FontId::proportional(16.0),
                            Color32::WHITE,
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Wireline")
                                        .strong()
                                        .size(self.config.font_size)
                                        .color(theme::TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new("Today at 4:20 PM")
                                        .size(11.5)
                                        .color(theme::TEXT_MUTED),
                                );
                            });
                            ui.label(
                                RichText::new("Looks crisp! This is a live preview of your chat settings.")
                                    .size(self.config.font_size)
                                    .color(theme::TEXT_PRIMARY),
                            );
                        });
                    } else {
                        ui.label(RichText::new("4:20 PM").size(11.5).color(theme::TEXT_MUTED));
                        ui.label(
                            RichText::new("Wireline")
                                .strong()
                                .size(self.config.font_size)
                                .color(theme::TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Compact mode packs more messages onto the screen.")
                                .size(self.config.font_size)
                                .color(theme::TEXT_PRIMARY),
                        );
                    }
                });
            });

        ui.add_space(18.0);
        ui.label(
            RichText::new("THEME")
                .size(11.5)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        let mut theme_changed = false;
        ui.horizontal(|ui| {
            for (is_dark, label) in [(true, "Dark"), (false, "Light")] {
                let active = self.config.dark_mode == is_dark;
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).color(if active {
                            Color32::WHITE
                        } else {
                            theme::TEXT_SECONDARY
                        }))
                        .fill(if active {
                            theme::ACCENT
                        } else {
                            theme::BG_SECONDARY
                        })
                        .corner_radius(6.0)
                        .min_size(egui::vec2(96.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.dark_mode = is_dark;
                    theme_changed = true;
                }
            }
        });

        ui.add_space(18.0);
        ui.label(
            RichText::new("MESSAGE DISPLAY")
                .size(11.5)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (compact, label) in [
                (false, "Cozy: Modern, beautiful, and easy on your eyes."),
                (true, "Compact: Fit more messages on screen at one time."),
            ] {
                let active = self.config.compact_mode == compact;
                if ui
                    .add(
                        egui::Button::new(RichText::new(if compact { "Compact" } else { "Cozy" }).color(
                            if active {
                                Color32::WHITE
                            } else {
                                theme::TEXT_SECONDARY
                            },
                        ))
                        .fill(if active {
                            theme::ACCENT
                        } else {
                            theme::BG_SECONDARY
                        })
                        .corner_radius(6.0)
                        .min_size(egui::vec2(96.0, 34.0)),
                    )
                    .on_hover_text(label)
                    .clicked()
                {
                    self.config.compact_mode = compact;
                    self.config.save();
                }
            }
        });

        ui.add_space(18.0);
        ui.label(
            RichText::new(format!(
                "CHAT FONT SCALING — {}px",
                self.config.font_size.round() as i32
            ))
            .size(11.5)
            .strong()
            .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        if ui
            .add(
                egui::Slider::new(&mut self.config.font_size, 12.0..=20.0)
                    .step_by(1.0)
                    .suffix("px"),
            )
            .changed()
        {
            theme_changed = true;
        }

        if theme_changed {
            self.config.save();
            theme::apply(ctx, self.config.dark_mode, self.config.font_size);
        }

        ui.add_space(18.0);
        ui.label(
            RichText::new("ACCESSIBILITY — REDUCE MOTION")
                .size(11.5)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (rm, label) in [(false, "Full Animations"), (true, "Reduce Motion")] {
                let active = self.config.reduced_motion == rm;
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).color(if active {
                            Color32::WHITE
                        } else {
                            theme::TEXT_SECONDARY
                        }))
                        .fill(if active {
                            theme::ACCENT
                        } else {
                            theme::BG_SECONDARY
                        })
                        .corner_radius(6.0)
                        .min_size(egui::vec2(124.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.reduced_motion = rm;
                    self.config.save();
                }
            }
        });

        ui.add_space(18.0);
        ui.label(
            RichText::new("UPDATES")
                .size(11.5)
                .strong()
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (au, label) in [(true, "Auto-Install Updates"), (false, "Manual Only")] {
                let active = self.config.auto_update == au;
                if ui
                    .add(
                        egui::Button::new(RichText::new(label).color(if active {
                            Color32::WHITE
                        } else {
                            theme::TEXT_SECONDARY
                        }))
                        .fill(if active {
                            theme::ACCENT
                        } else {
                            theme::BG_SECONDARY
                        })
                        .corner_radius(6.0)
                        .min_size(egui::vec2(142.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.auto_update = au;
                    self.config.save();
                }
            }
            if ui
                .add(
                    egui::Button::new(RichText::new("Check Now").color(theme::TEXT_PRIMARY))
                        .fill(theme::BG_ACTIVE)
                        .corner_radius(6.0)
                        .min_size(egui::vec2(96.0, 34.0)),
                )
                .clicked()
            {
                self.check_for_updates(true);
            }
        });
    }

    fn ui_settings_voice(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Voice & Video")
                .size(20.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(16.0);

        egui::Frame::new()
            .fill(theme::BG_SECONDARY)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if self.muted {
                                    "Unmute Microphone"
                                } else {
                                    "Mute Microphone"
                                })
                                .color(Color32::WHITE),
                            )
                            .fill(if self.muted {
                                theme::DANGER
                            } else {
                                theme::ACCENT
                            })
                            .corner_radius(4.0)
                            .min_size(egui::vec2(160.0, 34.0)),
                        )
                        .clicked()
                    {
                        self.toggle_mute();
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(if self.deafened {
                                    "Undeafen Audio"
                                } else {
                                    "Deafen Audio"
                                })
                                .color(Color32::WHITE),
                            )
                            .fill(if self.deafened {
                                theme::DANGER
                            } else {
                                theme::BG_ACTIVE
                            })
                            .corner_radius(4.0)
                            .min_size(egui::vec2(150.0, 34.0)),
                        )
                        .clicked()
                    {
                        self.toggle_deafen();
                    }
                });
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!(
                        "Relay status: {}",
                        if self.connected {
                            "Connected (Opus 48kHz)"
                        } else {
                            "Reconnecting..."
                        }
                    ))
                    .color(if self.connected {
                        theme::SUCCESS
                    } else {
                        theme::WARNING
                    }),
                );
                if self.in_voice {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "Active room: {} ({} participant(s))",
                            self.voice_room.as_deref().unwrap_or("voice"),
                            self.participants.len()
                        ))
                        .color(theme::TEXT_SECONDARY),
                    );
                }
            });

        ui.add_space(18.0);
        ui.label(theme::semibold("SCREEN SHARE", 12.0).color(theme::TEXT_MUTED));
        ui.add_space(8.0);

        egui::Frame::new()
            .fill(theme::BG_SECONDARY)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.label(theme::semibold("RESOLUTION (MAX 1080P)", 11.0).color(theme::TEXT_MUTED));
                ui.add_space(6.0);
                let mut picked_res: Option<u32> = None;
                ui.horizontal(|ui| {
                    for (h, label) in [(480u32, "480p"), (720u32, "720p"), (1080u32, "1080p")] {
                        let active = self.config.screen_max_height == h;
                        if ui
                            .add(
                                egui::Button::new(
                                    theme::semibold(label, 13.0).color(if active {
                                        Color32::WHITE
                                    } else {
                                        theme::TEXT_SECONDARY
                                    }),
                                )
                                .fill(if active {
                                    theme::ACCENT
                                } else {
                                    theme::BG_TERTIARY
                                })
                                .corner_radius(4.0)
                                .min_size(egui::vec2(88.0, 32.0)),
                            )
                            .clicked()
                        {
                            picked_res = Some(h);
                        }
                    }
                });
                if let Some(h) = picked_res {
                    self.set_screen_resolution(h);
                }

                ui.add_space(14.0);
                ui.label(theme::semibold("FRAME RATE", 11.0).color(theme::TEXT_MUTED));
                ui.add_space(6.0);
                let mut picked_fps: Option<u32> = None;
                ui.horizontal(|ui| {
                    for (fps, label) in [
                        (15u32, "15 FPS"),
                        (30u32, "30 FPS"),
                        (60u32, "60 FPS"),
                    ] {
                        let active = self.config.screen_fps == fps;
                        if ui
                            .add(
                                egui::Button::new(
                                    theme::semibold(label, 13.0).color(if active {
                                        Color32::WHITE
                                    } else {
                                        theme::TEXT_SECONDARY
                                    }),
                                )
                                .fill(if active {
                                    theme::ACCENT
                                } else {
                                    theme::BG_TERTIARY
                                })
                                .corner_radius(4.0)
                                .min_size(egui::vec2(88.0, 32.0)),
                            )
                            .clicked()
                        {
                            picked_fps = Some(fps);
                        }
                    }
                });
                if let Some(fps) = picked_fps {
                    self.set_screen_fps(fps);
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Changes apply live even while sharing. Use 720p 60 FPS for fastest motion or 1080p for sharp text.",
                    )
                    .small()
                    .color(theme::TEXT_MUTED),
                );
            });
    }

    fn ui_settings_keybinds(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Keybinds")
                .size(20.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.add_space(12.0);
        let bindings = [
            ("Ctrl + K", "Jump to Quick DM Search"),
            ("Escape", "Close modal, popup, search, or cancel reply/edit"),
            ("Arrow Up", "Edit your last sent message (when input is empty)"),
            ("Ctrl + Shift + M", "Toggle Microphone Mute"),
            ("Ctrl + Shift + D", "Toggle Audio Deafen"),
            ("Ctrl + /", "Open Keyboard Shortcuts"),
        ];
        for (keys, desc) in bindings {
            egui::Frame::new()
                .fill(theme::BG_SECONDARY)
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(desc).color(theme::TEXT_PRIMARY));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                egui::Frame::new()
                                    .fill(theme::BG_TERTIARY)
                                    .stroke(theme::border())
                                    .corner_radius(4.0)
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(keys)
                                                .monospace()
                                                .small()
                                                .color(theme::TEXT_PRIMARY),
                                        );
                                    });
                            },
                        );
                    });
                });
            ui.add_space(6.0);
        }
    }

    /// #9 Top-right Discord Toast Notifications:
    /// 250ms slide-in (-16px, scale 0.97 -> 1), 2px #5865f2 4s progress bar (pauses on hover),
    /// and 200ms slide-out (-8px, fade out).
    pub(crate) fn ui_toasts(&mut self, ctx: &Context) {
        if self.toasts.is_empty() {
            return;
        }
        let now = Self::now_secs();
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.001, 0.1);
        let reduced = self.config.reduced_motion;
        let screen = ctx.screen_rect();
        let base_w = 300.0;
        let base_h = 68.0;
        let mut jump_to: Option<i64> = None;
        let mut remove_indices: Vec<usize> = Vec::new();

        for (idx, toast) in self.toasts.iter_mut().enumerate() {
            let open_elapsed = (now - toast.created_at) as f32;
            let close_elapsed = toast.closing_at.map(|c| (now - c) as f32);
            let (alpha, y_off, scale, done_exit) =
                anim::toast_transition(open_elapsed, close_elapsed, reduced);
            if done_exit {
                remove_indices.push(idx);
                continue;
            }
            self.animating = true;

            let w = base_w * scale;
            let h = base_h * scale;
            let base_y = 58.0 + (idx as f32) * 78.0;
            let pos = egui::pos2(
                (screen.right() - base_w - 16.0).max(16.0) + (base_w - w) * 0.5,
                base_y + y_off + (base_h - h) * 0.5,
            );

            egui::Area::new(egui::Id::new(("discord_toast", idx, toast.channel_id)))
                .order(egui::Order::Tooltip)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    if alpha < 0.999 {
                        ui.multiply_opacity(alpha);
                    }
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(w, h), Sense::click());

                    // Hover pauses the 4s progress countdown (animation-play-state: paused)
                    if !resp.hovered() && toast.closing_at.is_none() {
                        toast.remaining_secs = (toast.remaining_secs - dt).max(0.0);
                        if toast.remaining_secs <= 0.0 {
                            toast.closing_at = Some(now);
                        }
                    }

                    ui.painter().rect_filled(rect, 8.0, theme::BG_FLOATING);
                    ui.painter().rect_stroke(
                        rect,
                        8.0,
                        theme::border(),
                        egui::StrokeKind::Inside,
                    );
                    // Left accent stripe
                    ui.painter().rect_filled(
                        Rect::from_min_max(
                            rect.left_top() + egui::vec2(0.0, 6.0),
                            rect.left_bottom() + egui::vec2(3.0, -8.0),
                        ),
                        1.5,
                        theme::ACCENT,
                    );

                    let av_c = rect.left_center() + egui::vec2(26.0, -1.0);
                    ui.painter()
                        .circle_filled(av_c, 16.0, avatar_color(&toast.sender, false));
                    let init = toast
                        .sender
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase()
                        .to_string();
                    ui.painter().text(
                        av_c,
                        Align2::CENTER_CENTER,
                        init,
                        FontId::proportional(13.0),
                        Color32::WHITE,
                    );

                    ui.painter().text(
                        rect.left_top() + egui::vec2(52.0, 12.0),
                        Align2::LEFT_TOP,
                        &toast.sender,
                        FontId::proportional(13.5),
                        theme::TEXT_PRIMARY,
                    );
                    let snippet: String = if toast.preview.chars().count() > 34 {
                        format!(
                            "{}...",
                            toast.preview.chars().take(34).collect::<String>()
                        )
                    } else {
                        toast.preview.clone()
                    };
                    ui.painter().text(
                        rect.left_top() + egui::vec2(52.0, 33.0),
                        Align2::LEFT_TOP,
                        snippet,
                        FontId::proportional(12.5),
                        theme::TEXT_SECONDARY,
                    );

                    // 2px #5865f2 progress bar shrinking 100% -> 0% over 4s
                    let frac = (toast.remaining_secs / 4.0).clamp(0.0, 1.0);
                    let bar_w = (rect.width() - 8.0) * frac;
                    if bar_w > 0.5 {
                        let pbar = Rect::from_min_max(
                            egui::pos2(rect.left() + 4.0, rect.bottom() - 3.0),
                            egui::pos2(rect.left() + 4.0 + bar_w, rect.bottom() - 1.0),
                        );
                        ui.painter().rect_filled(pbar, 1.0, theme::ACCENT);
                    }

                    if resp.clicked() && toast.closing_at.is_none() {
                        jump_to = Some(toast.channel_id);
                        toast.closing_at = Some(now);
                    }
                });
        }

        for idx in remove_indices.into_iter().rev() {
            if idx < self.toasts.len() {
                self.toasts.remove(idx);
            }
        }
        if let Some(ch_id) = jump_to {
            self.select_channel(ch_id);
        }
    }

    pub(crate) fn open_profile_popup(
        &mut self,
        user_id: i64,
        username: String,
        anchor: Pos2,
    ) {
        let status = if self.me.as_ref().is_some_and(|m| m.user_id == user_id) {
            self.presence
        } else {
            self.online_status_for(user_id)
        };
        self.profile_popup_opened_at = Self::now_secs();
        self.profile_popup_closing_at = None;
        self.profile_popup = Some(ProfilePopup {
            user_id,
            username,
            status,
            anchor,
        });
    }
}
