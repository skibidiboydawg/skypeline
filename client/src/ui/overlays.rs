//! Original Wireline overlays (100% Dynamic Dark & Light Mode):
//! Top-anchored Status Popover, 300px User Profile Popup, Emoji Picker,
//! Notifications Inbox, Settings Modal (with Dark/Light, UI Sounds, Screen Share FPS/Res,
//! Auto-Update, Reduce Motion), and Toast Notifications with 4s progress bar.

use super::{anim, avatar_color, icons, ProfilePopup, SettingsTab, SkyApp};
use crate::sounds::SoundEffect;
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
    /// Status switcher popover anchored cleanly below the top-left profile bar.
    pub(crate) fn ui_profile_popover(&mut self, ctx: &Context) {
        if !self.show_profile_popover {
            return;
        }
        let p = theme::of(ctx);
        let anchor = egui::pos2(12.0, 62.0);
        let resp = egui::Area::new(egui::Id::new("profile_status_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.floating)
                    .stroke(p.border_stroke())
                    .corner_radius(12.0)
                    .shadow(p.shadow())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(236.0);
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
                        ui.label(theme::semibold(&name, 15.0).color(p.text_primary));
                        if !email.is_empty() {
                            ui.label(RichText::new(&email).small().color(p.text_muted));
                        }

                        ui.add_space(8.0);
                        let (div_r, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(div_r, 0.0, p.hairline);
                        ui.add_space(8.0);

                        ui.label(theme::semibold("STATUS", 10.5).color(p.text_muted));
                        ui.add_space(4.0);

                        for (status, label, desc) in [
                            (PresenceStatus::Online, "Online", "Available to chat"),
                            (PresenceStatus::Away, "Idle", "Stepped away"),
                            (PresenceStatus::Dnd, "Do Not Disturb", "Mute alerts"),
                        ] {
                            let active = self.presence == status;
                            let (r, btn) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 34.0),
                                Sense::click(),
                            );
                            if active || btn.hovered() {
                                ui.painter().rect_filled(
                                    r,
                                    8.0,
                                    if active { p.active } else { p.hover },
                                );
                            }
                            icons::paint_status_dot(
                                ui.painter(),
                                r.left_center() + egui::vec2(14.0, 0.0),
                                status,
                                if active { p.active } else { p.floating },
                            );
                            ui.painter().text(
                                r.left_top() + egui::vec2(30.0, 4.0),
                                Align2::LEFT_TOP,
                                label,
                                FontId::new(13.0, theme::semibold_family()),
                                p.text_primary,
                            );
                            ui.painter().text(
                                r.left_top() + egui::vec2(30.0, 19.0),
                                Align2::LEFT_TOP,
                                desc,
                                FontId::proportional(10.5),
                                p.text_muted,
                            );
                            if btn.clicked() {
                                self.set_presence(status);
                                self.show_profile_popover = false;
                                self.play_sound(SoundEffect::UiClick);
                            }
                        }

                        ui.add_space(8.0);
                        ui.label(theme::semibold("CUSTOM STATUS", 10.5).color(p.text_muted));
                        ui.add_space(4.0);
                        let status_resp = egui::Frame::new()
                            .fill(p.input_bg)
                            .stroke(p.border_stroke())
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.config.custom_status)
                                        .frame(false)
                                        .desired_width(ui.available_width())
                                        .hint_text("What are you up to?"),
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
                        ui.painter().rect_filled(div2, 0.0, p.hairline);
                        ui.add_space(6.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    theme::semibold("Upload Profile Picture", 13.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(p.accent)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(ui.available_width(), 32.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.upload_my_avatar();
                            self.play_sound(SoundEffect::UiClick);
                        }
                        ui.add_space(5.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    theme::semibold("Settings", 13.0).color(p.text_primary),
                                )
                                .fill(p.surface)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(ui.available_width(), 30.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.open_settings_modal(self.settings_tab);
                            self.play_sound(SoundEffect::UiClick);
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(theme::semibold("Log Out", 13.0).color(p.danger))
                                    .fill(Color32::TRANSPARENT)
                                    .corner_radius(8.0)
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
                let top_area = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(268.0, 62.0));
                if !resp.response.rect.contains(pos) && !top_area.contains(pos) {
                    self.show_profile_popover = false;
                }
            }
        }
    }

    /// 300px floating User Profile Card with scale-in (180ms) & reverse exit (150ms).
    pub(crate) fn ui_user_profile_popup(&mut self, ctx: &Context) {
        let Some(popup) = self.profile_popup.clone() else {
            return;
        };
        let p = theme::of(ctx);
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
                    .fill(p.floating)
                    .stroke(p.border_stroke())
                    .corner_radius(14.0)
                    .shadow(p.shadow())
                    .inner_margin(egui::Margin::same(0))
                    .show(ui, |ui| {
                        ui.set_width(card_w);

                        let (banner_area, _) =
                            ui.allocate_exact_size(egui::vec2(card_w, 100.0), Sense::hover());
                        let banner = Rect::from_min_size(banner_area.min, egui::vec2(card_w, 58.0));
                        ui.painter().rect_filled(
                            banner,
                            egui::CornerRadius {
                                nw: 14,
                                ne: 14,
                                sw: 0,
                                se: 0,
                            },
                            avatar_color(&popup.username, is_me),
                        );

                        let av_center = banner.left_bottom() + egui::vec2(54.0, 0.0);
                        ui.painter().circle_filled(av_center, 44.0, p.floating);
                        let av_rect = Rect::from_center_size(av_center, egui::vec2(76.0, 76.0));

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
                                38.0,
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
                                FontId::new(28.0, theme::semibold_family()),
                                Color32::WHITE,
                            );
                        }
                        icons::paint_status_dot_pulse(
                            ui.painter(),
                            av_center + egui::vec2(26.0, 26.0),
                            popup.status,
                            p.floating,
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
                                    theme::semibold(&popup.username, 18.0).color(p.text_primary),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "@{} · {}",
                                        popup.username.to_ascii_lowercase().replace(' ', ""),
                                        popup.status.label()
                                    ))
                                    .size(12.5)
                                    .color(p.text_muted),
                                );

                                ui.add_space(10.0);
                                let (div, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 1.0),
                                    Sense::hover(),
                                );
                                ui.painter().rect_filled(div, 0.0, p.hairline);
                                ui.add_space(10.0);

                                ui.label(theme::semibold("ABOUT", 10.5).color(p.text_muted));
                                ui.add_space(2.0);
                                let bio = if is_me && !self.config.bio.trim().is_empty() {
                                    self.config.bio.as_str()
                                } else {
                                    "Wireline member."
                                };
                                ui.label(RichText::new(bio).size(13.0).color(p.text_secondary));

                                if !is_me {
                                    ui.add_space(14.0);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    theme::semibold("Message", 13.0)
                                                        .color(Color32::WHITE),
                                                )
                                                .fill(p.accent)
                                                .corner_radius(8.0)
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
                                                    theme::semibold(blk_label, 13.0).color(
                                                        if is_blocked {
                                                            p.text_primary
                                                        } else {
                                                            p.danger
                                                        },
                                                    ),
                                                )
                                                .fill(p.surface)
                                                .corner_radius(8.0)
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

    /// 360px Emoji Picker floating above the composer.
    pub(crate) fn ui_emoji_picker(&mut self, ctx: &Context) {
        let Some(target) = self.emoji_picker_target else {
            return;
        };
        let p = theme::of(ctx);
        let screen = ctx.screen_rect();
        let w = 360.0;
        let h = 340.0;
        let pos = egui::pos2(
            (screen.right() - w - 20.0).max(274.0),
            (screen.bottom() - h - 76.0).max(60.0),
        );

        let mut picked: Option<String> = None;
        let resp = egui::Area::new(egui::Id::new("wireline_emoji_picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.floating)
                    .stroke(p.border_stroke())
                    .corner_radius(14.0)
                    .shadow(p.shadow())
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(w - 24.0);
                        ui.set_height(h - 24.0);

                        egui::Frame::new()
                            .fill(p.input_bg)
                            .stroke(p.border_stroke())
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(8, 5))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.emoji_search)
                                        .frame(false)
                                        .desired_width(ui.available_width())
                                        .hint_text("Filter emoji..."),
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
                                        theme::semibold(cat_name.to_ascii_uppercase(), 10.5)
                                            .color(p.text_muted),
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
                                                    ui.painter().rect_filled(r, 8.0, p.hover);
                                                }
                                                ui.painter().text(
                                                    r.center(),
                                                    Align2::CENTER_CENTER,
                                                    emoji,
                                                    FontId::proportional(20.0),
                                                    p.text_primary,
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
                    self.play_sound(SoundEffect::UiClick);
                }
            }
            self.emoji_picker_target = None;
            self.emoji_search.clear();
            return;
        }

        if ctx.input(|i| i.pointer.any_click()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !resp.response.rect.contains(pos) {
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
        let p = theme::of(ctx);
        let screen = ctx.screen_rect();
        let w = 320.0;
        let pos = egui::pos2((screen.right() - w - 16.0).max(274.0), 60.0);
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
                    .fill(p.floating)
                    .stroke(p.border_stroke())
                    .corner_radius(14.0)
                    .shadow(p.shadow())
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(w - 28.0);
                        ui.label(theme::semibold("INBOX", 11.5).color(p.text_muted));
                        ui.add_space(8.0);
                        if incoming.is_empty() {
                            ui.label(
                                RichText::new("All caught up!").color(p.text_muted),
                            );
                        } else {
                            for req in &incoming {
                                egui::Frame::new()
                                    .fill(p.surface)
                                    .corner_radius(10.0)
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    theme::semibold(&req.username, 13.5)
                                                        .color(p.text_primary),
                                                );
                                                ui.label(
                                                    RichText::new("Incoming friend request")
                                                        .small()
                                                        .color(p.text_muted),
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
                                                                    .color(p.danger),
                                                            )
                                                            .fill(p.floating)
                                                            .corner_radius(6.0),
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
                                                            .fill(p.success)
                                                            .corner_radius(6.0),
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
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !resp.response.rect.contains(pos) && pos.y > 58.0 {
                    self.show_inbox_popover = false;
                }
            }
        }
    }

    /// Full-Screen Wireline Settings Modal (100% Dynamic Dark & Light Mode, scale-in/out).
    pub(crate) fn ui_settings_modal(&mut self, ctx: &Context) {
        if !self.show_settings_modal {
            return;
        }
        let p = theme::of(ctx);
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
        egui::Area::new(egui::Id::new("wireline_settings_modal"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (bg_rect, bg_resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    bg_rect,
                    0.0,
                    Color32::from_black_alpha(
                        ((if p.dark { 200.0 } else { 110.0 }) * backdrop_alpha) as u8,
                    ),
                );

                let base_w = (screen.width() - 80.0).clamp(560.0, 900.0);
                let base_h = (screen.height() - 80.0).clamp(440.0, 660.0);
                let modal_w = base_w * scale;
                let modal_h = base_h * scale;
                let modal_rect =
                    Rect::from_center_size(screen.center(), egui::vec2(modal_w, modal_h));

                if bg_resp.clicked() {
                    if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                        if !modal_rect.contains(pos) {
                            self.close_settings_modal();
                        }
                    }
                }

                if alpha < 0.999 {
                    ui.multiply_opacity(alpha);
                }

                ui.painter().rect_filled(modal_rect, 16.0, p.canvas);
                ui.painter().rect_stroke(
                    modal_rect,
                    16.0,
                    p.border_stroke(),
                    egui::StrokeKind::Outside,
                );

                // Left 210px navigation sidebar
                let nav_rect = Rect::from_min_size(modal_rect.min, egui::vec2(210.0, modal_h));
                ui.painter().rect_filled(
                    nav_rect,
                    egui::CornerRadius {
                        nw: 16,
                        ne: 0,
                        sw: 16,
                        se: 0,
                    },
                    p.sidebar,
                );
                ui.painter().line_segment(
                    [nav_rect.right_top(), nav_rect.right_bottom()],
                    p.hairline_stroke(),
                );

                let mut logout_clicked = false;
                let mut close_clicked = false;

                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(nav_rect.shrink2(egui::vec2(14.0, 20.0)))
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.label(theme::semibold("SETTINGS", 11.0).color(p.text_muted));
                        ui.add_space(6.0);
                        self.settings_nav_item(ui, SettingsTab::MyAccount, "My Account");
                        self.settings_nav_item(ui, SettingsTab::Appearance, "Appearance");
                        self.settings_nav_item(ui, SettingsTab::VoiceVideo, "Audio & Devices");
                        self.settings_nav_item(ui, SettingsTab::Keybinds, "Shortcuts");

                        ui.add_space(16.0);
                        let (d2, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(d2, 0.0, p.hairline);
                        ui.add_space(10.0);

                        let (lr, lresp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 34.0),
                            Sense::click(),
                        );
                        if lresp.hovered() {
                            ui.painter().rect_filled(lr, 8.0, p.danger.gamma_multiply(0.14));
                        }
                        ui.painter().text(
                            lr.left_center() + egui::vec2(10.0, 0.0),
                            Align2::LEFT_CENTER,
                            "Log Out",
                            FontId::new(13.5, theme::semibold_family()),
                            p.danger,
                        );
                        if lresp.clicked() {
                            logout_clicked = true;
                        }
                    },
                );

                // Right content area
                let content_rect = Rect::from_min_max(
                    egui::pos2(nav_rect.right(), modal_rect.top()),
                    modal_rect.right_bottom(),
                );
                let close_c = content_rect.right_top() + egui::vec2(-30.0, 30.0);
                let close_r = Rect::from_center_size(close_c, egui::vec2(32.0, 32.0));
                let close_resp =
                    ui.interact(close_r, ui.id().with("settings_close_x"), Sense::click());
                ui.painter().circle_filled(
                    close_c,
                    16.0,
                    if close_resp.hovered() {
                        p.hover
                    } else {
                        p.surface
                    },
                );
                icons::paint_x(
                    ui.painter(),
                    close_c,
                    14.0,
                    if close_resp.hovered() {
                        p.text_primary
                    } else {
                        p.text_secondary
                    },
                );
                if close_resp.on_hover_text("Close (Esc)").clicked() {
                    close_clicked = true;
                }

                let inner = content_rect.shrink2(egui::vec2(32.0, 26.0));
                let inner = Rect::from_min_max(
                    inner.min,
                    egui::pos2(
                        (inner.right() - 36.0).max(inner.left() + 200.0),
                        inner.bottom(),
                    ),
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
        let p = theme::of(ui.ctx());
        let active = self.settings_tab == tab;
        let (r, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::click());
        if active || resp.hovered() {
            ui.painter()
                .rect_filled(r, 8.0, if active { p.active } else { p.hover });
        }
        ui.painter().text(
            r.left_center() + egui::vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            FontId::new(13.5, theme::semibold_family()),
            if active {
                p.text_primary
            } else {
                p.text_secondary
            },
        );
        if resp.clicked() {
            self.settings_tab = tab;
            self.play_sound(SoundEffect::UiClick);
        }
    }

    fn ui_settings_account(&mut self, ui: &mut egui::Ui) {
        let p = theme::of(ui.ctx());
        ui.label(theme::semibold("My Account", 20.0).color(p.text_primary));
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
        let my_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());
        let mut want_upload = false;

        egui::Frame::new()
            .fill(p.surface)
            .stroke(p.border_stroke())
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (av_r, av_resp) =
                        ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::click());
                    // Crisp circular backing so transparent PNG avatars look clean.
                    ui.painter().circle_filled(av_r.center(), 32.0, p.surface_alt);
                    let mut drew = false;
                    if let Some(h) = my_hash.as_deref() {
                        let tex = self.avatars.texture(
                            &ui.ctx().clone(),
                            &self.api(),
                            self.token.as_deref(),
                            uid,
                            Some(h),
                            &self.events_tx,
                        );
                        if let Some(t) = tex {
                            ui.painter().image(
                                t.id(),
                                av_r,
                                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            drew = true;
                        }
                    }
                    if !drew {
                        ui.painter().circle_filled(av_r.center(), 32.0, p.accent);
                        let init = name
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_ascii_uppercase()
                            .to_string();
                        ui.painter().text(
                            av_r.center(),
                            Align2::CENTER_CENTER,
                            init,
                            FontId::new(24.0, theme::semibold_family()),
                            Color32::WHITE,
                        );
                    }
                    ui.painter().circle_stroke(
                        av_r.center(),
                        32.0,
                        if av_resp.hovered() {
                            p.focus_stroke()
                        } else {
                            p.border_stroke()
                        },
                    );
                    icons::paint_status_dot_pulse(
                        ui.painter(),
                        av_r.right_bottom() - egui::vec2(5.0, 5.0),
                        self.presence,
                        p.surface,
                        self.pulse_time(ui.ctx()),
                    );
                    if av_resp.on_hover_text("Click to upload profile picture").clicked() {
                        want_upload = true;
                    }
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.label(theme::semibold(&name, 17.5).color(p.text_primary));
                        ui.label(RichText::new(&email).size(13.5).color(p.text_secondary));
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    theme::semibold("Upload Profile Picture", 13.0)
                                        .color(Color32::WHITE),
                                )
                                .fill(p.accent)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(170.0, 32.0)),
                            )
                            .clicked()
                        {
                            want_upload = true;
                        }
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new(format!(
                                "PNG, JPG or WebP (transparency supported) · ID #{uid}"
                            ))
                            .size(11.5)
                            .color(p.text_muted),
                        );
                    });
                });
            });
        if want_upload {
            self.upload_my_avatar();
            self.play_sound(SoundEffect::UiClick);
        }

        ui.add_space(16.0);
        ui.label(theme::semibold("ABOUT ME", 11.0).color(p.text_muted));
        ui.add_space(6.0);
        let bio_resp = egui::Frame::new()
            .fill(p.input_bg)
            .stroke(p.border_stroke())
            .corner_radius(10.0)
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
        let p = theme::of(ctx);
        ui.label(theme::semibold("Appearance", 20.0).color(p.text_primary));
        ui.add_space(16.0);

        ui.label(theme::semibold("THEME", 11.0).color(p.text_muted));
        ui.add_space(6.0);
        let mut theme_changed = false;
        ui.horizontal(|ui| {
            for (is_dark, label) in [(true, "Midnight Dark"), (false, "Daylight Light")] {
                let active = self.config.dark_mode == is_dark;
                if ui
                    .add(
                        egui::Button::new(theme::semibold(label, 13.0).color(if active {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        }))
                        .fill(if active { p.accent } else { p.surface })
                        .stroke(if active {
                            Stroke::NONE
                        } else {
                            p.border_stroke()
                        })
                        .corner_radius(8.0)
                        .min_size(egui::vec2(128.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.dark_mode = is_dark;
                    theme_changed = true;
                    self.play_sound(SoundEffect::UiClick);
                }
            }
        });

        ui.add_space(16.0);
        ui.label(theme::semibold("MESSAGE STYLE", 11.0).color(p.text_muted));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (compact, label) in [(false, "Modern Bubbles"), (true, "Compact Feed")] {
                let active = self.config.compact_mode == compact;
                if ui
                    .add(
                        egui::Button::new(theme::semibold(label, 13.0).color(if active {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        }))
                        .fill(if active { p.accent } else { p.surface })
                        .stroke(if active {
                            Stroke::NONE
                        } else {
                            p.border_stroke()
                        })
                        .corner_radius(8.0)
                        .min_size(egui::vec2(128.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.compact_mode = compact;
                    self.config.save();
                    self.play_sound(SoundEffect::UiClick);
                }
            }
        });

        ui.add_space(16.0);
        ui.label(
            theme::semibold(
                format!("FONT SIZE — {}px", self.config.font_size.round() as i32),
                11.0,
            )
            .color(p.text_muted),
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

        ui.add_space(16.0);
        ui.label(theme::semibold("MOTION", 11.0).color(p.text_muted));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (rm, label) in [(false, "Full Animations"), (true, "Reduce Motion")] {
                let active = self.config.reduced_motion == rm;
                if ui
                    .add(
                        egui::Button::new(theme::semibold(label, 13.0).color(if active {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        }))
                        .fill(if active { p.accent } else { p.surface })
                        .stroke(if active {
                            Stroke::NONE
                        } else {
                            p.border_stroke()
                        })
                        .corner_radius(8.0)
                        .min_size(egui::vec2(128.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.reduced_motion = rm;
                    self.config.save();
                    self.play_sound(SoundEffect::UiClick);
                }
            }
        });

        ui.add_space(16.0);
        ui.label(theme::semibold("UPDATES", 11.0).color(p.text_muted));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (au, label) in [(true, "Auto-Install Updates"), (false, "Manual Only")] {
                let active = self.config.auto_update == au;
                if ui
                    .add(
                        egui::Button::new(theme::semibold(label, 13.0).color(if active {
                            Color32::WHITE
                        } else {
                            p.text_primary
                        }))
                        .fill(if active { p.accent } else { p.surface })
                        .stroke(if active {
                            Stroke::NONE
                        } else {
                            p.border_stroke()
                        })
                        .corner_radius(8.0)
                        .min_size(egui::vec2(142.0, 34.0)),
                    )
                    .clicked()
                {
                    self.config.auto_update = au;
                    self.config.save();
                    self.play_sound(SoundEffect::UiClick);
                }
            }
            if ui
                .add(
                    egui::Button::new(theme::semibold("Check Now", 13.0).color(p.text_primary))
                        .fill(p.surface)
                        .stroke(p.border_stroke())
                        .corner_radius(8.0)
                        .min_size(egui::vec2(96.0, 34.0)),
                )
                .clicked()
            {
                self.check_for_updates(true);
                self.play_sound(SoundEffect::UiClick);
            }
        });
    }

    fn ui_settings_voice(&mut self, ui: &mut egui::Ui) {
        let p = theme::of(ui.ctx());
        if !self.devices_loaded {
            self.refresh_audio_devices();
        }

        ui.horizontal(|ui| {
            ui.label(theme::semibold("Audio & Devices", 20.0).color(p.text_primary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            theme::semibold("Refresh Devices", 12.0).color(p.text_primary),
                        )
                        .fill(p.surface)
                        .stroke(p.border_stroke())
                        .corner_radius(8.0)
                        .min_size(egui::vec2(112.0, 28.0)),
                    )
                    .clicked()
                {
                    self.refresh_audio_devices();
                    self.play_sound(SoundEffect::UiClick);
                }
            });
        });
        ui.add_space(16.0);

        // Hardware Input & Output Device Selectors
        egui::Frame::new()
            .fill(p.surface)
            .stroke(p.border_stroke())
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Microphone Input Device
                ui.label(theme::semibold("MICROPHONE (INPUT DEVICE)", 11.0).color(p.text_muted));
                ui.add_space(6.0);
                let current_in = self
                    .config
                    .input_device
                    .clone()
                    .unwrap_or_else(|| "System Default".to_string());
                let mut chosen_in: Option<Option<String>> = None;
                egui::ComboBox::from_id_salt("settings_input_device")
                    .selected_text(&current_in)
                    .width((ui.available_width() - 8.0).max(220.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.config.input_device.is_none(), "System Default")
                            .clicked()
                        {
                            chosen_in = Some(None);
                        }
                        for dev in &self.input_devices {
                            let sel = self.config.input_device.as_deref() == Some(dev.as_str());
                            if ui.selectable_label(sel, dev).clicked() {
                                chosen_in = Some(Some(dev.clone()));
                            }
                        }
                    });
                if let Some(dev) = chosen_in {
                    self.set_input_device(dev);
                    self.play_sound(SoundEffect::UiClick);
                }

                ui.add_space(14.0);

                // Speaker / Headphones Output Device
                ui.label(
                    theme::semibold("SPEAKERS / HEADPHONES (OUTPUT DEVICE)", 11.0)
                        .color(p.text_muted),
                );
                ui.add_space(6.0);
                let current_out = self
                    .config
                    .output_device
                    .clone()
                    .unwrap_or_else(|| "System Default".to_string());
                let mut chosen_out: Option<Option<String>> = None;
                egui::ComboBox::from_id_salt("settings_output_device")
                    .selected_text(&current_out)
                    .width((ui.available_width() - 8.0).max(220.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.config.output_device.is_none(), "System Default")
                            .clicked()
                        {
                            chosen_out = Some(None);
                        }
                        for dev in &self.output_devices {
                            let sel = self.config.output_device.as_deref() == Some(dev.as_str());
                            if ui.selectable_label(sel, dev).clicked() {
                                chosen_out = Some(Some(dev.clone()));
                            }
                        }
                    });
                if let Some(dev) = chosen_out {
                    self.set_output_device(dev);
                    self.play_sound(SoundEffect::Reaction);
                }
            });

        ui.add_space(14.0);

        // Volumes & Mic Sensitivity
        egui::Frame::new()
            .fill(p.surface)
            .stroke(p.border_stroke())
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut in_pct = (self.config.input_volume * 100.0).round();
                ui.label(
                    theme::semibold(format!("INPUT VOLUME — {}%", in_pct as i32), 11.0)
                        .color(p.text_muted),
                );
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Slider::new(&mut in_pct, 0.0..=200.0)
                            .step_by(5.0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    self.set_input_volume(in_pct / 100.0);
                }

                ui.add_space(12.0);
                let mut out_pct = (self.config.output_volume * 100.0).round();
                ui.label(
                    theme::semibold(format!("OUTPUT VOLUME — {}%", out_pct as i32), 11.0)
                        .color(p.text_muted),
                );
                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Slider::new(&mut out_pct, 0.0..=200.0)
                            .step_by(5.0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    self.set_output_volume(out_pct / 100.0);
                }

                ui.add_space(12.0);
                let mut vad_pct = (self.config.vad_threshold * 1000.0).round();
                ui.label(
                    theme::semibold(
                        format!("VOICE DETECTION SENSITIVITY — {}", vad_pct as i32),
                        11.0,
                    )
                    .color(p.text_muted),
                );
                ui.add_space(4.0);
                if ui
                    .add(egui::Slider::new(&mut vad_pct, 4.0..=60.0).step_by(1.0))
                    .changed()
                {
                    self.set_vad_threshold(vad_pct / 1000.0);
                }
                ui.label(
                    RichText::new("Lower values pick up quieter speech; higher values reduce background noise.")
                        .small()
                        .color(p.text_muted),
                );
            });

        ui.add_space(14.0);

        // App Sound Effects & Speaker Test
        egui::Frame::new()
            .fill(p.surface)
            .stroke(p.border_stroke())
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(theme::semibold("APP SOUND EFFECTS", 11.0).color(p.text_muted));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    for (enabled, label) in [(true, "Sounds On"), (false, "Sounds Off")] {
                        let active = self.config.ui_sounds == enabled;
                        if ui
                            .add(
                                egui::Button::new(theme::semibold(label, 13.0).color(if active {
                                    Color32::WHITE
                                } else {
                                    p.text_primary
                                }))
                                .fill(if active { p.accent } else { p.floating })
                                .stroke(if active {
                                    Stroke::NONE
                                } else {
                                    p.border_stroke()
                                })
                                .corner_radius(8.0)
                                .min_size(egui::vec2(108.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.config.ui_sounds = enabled;
                            self.config.save();
                            if enabled {
                                self.play_sound(SoundEffect::CallJoin);
                            }
                        }
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                theme::semibold("Test Sound", 13.0).color(p.text_primary),
                            )
                            .fill(p.floating)
                            .stroke(p.border_stroke())
                            .corner_radius(8.0)
                            .min_size(egui::vec2(104.0, 32.0)),
                        )
                        .clicked()
                    {
                        let clip = crate::sounds::synthesize(SoundEffect::CallJoin);
                        self.ensure_audio().play_sfx(&clip);
                    }
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Call controls (Mute, Deafen, Screen Share resolution & FPS) live inside the active Call panel.",
                    )
                    .small()
                    .color(p.text_muted),
                );
            });
    }

    fn ui_settings_keybinds(&mut self, ui: &mut egui::Ui) {
        let p = theme::of(ui.ctx());
        ui.label(theme::semibold("Keyboard Shortcuts", 20.0).color(p.text_primary));
        ui.add_space(12.0);
        let bindings = [
            ("Ctrl + K", "Search conversations"),
            ("Escape", "Close modal, popup, search, or cancel reply/edit"),
            ("Arrow Up", "Edit your last sent message (when input is empty)"),
            ("Ctrl + Shift + M", "Toggle Microphone Mute"),
            ("Ctrl + Shift + D", "Toggle Audio Deafen"),
            ("Ctrl + /", "Open Shortcuts"),
        ];
        for (keys, desc) in bindings {
            egui::Frame::new()
                .fill(p.surface)
                .stroke(p.border_stroke())
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(desc).color(p.text_primary));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                egui::Frame::new()
                                    .fill(p.floating)
                                    .stroke(p.border_stroke())
                                    .corner_radius(6.0)
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(keys)
                                                .monospace()
                                                .small()
                                                .color(p.text_primary),
                                        );
                                    });
                            },
                        );
                    });
                });
            ui.add_space(6.0);
        }
    }

    /// Top-right Toast Notifications (250ms slide-in, 4s progress bar pausing on hover, 200ms slide-out).
    pub(crate) fn ui_toasts(&mut self, ctx: &Context) {
        if self.toasts.is_empty() {
            return;
        }
        let p = theme::of(ctx);
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
            let base_y = 64.0 + (idx as f32) * 78.0;
            let pos = egui::pos2(
                (screen.right() - base_w - 16.0).max(16.0) + (base_w - w) * 0.5,
                base_y + y_off + (base_h - h) * 0.5,
            );

            egui::Area::new(egui::Id::new(("wireline_toast", idx, toast.channel_id)))
                .order(egui::Order::Tooltip)
                .fixed_pos(pos)
                .show(ctx, |ui| {
                    if alpha < 0.999 {
                        ui.multiply_opacity(alpha);
                    }
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(w, h), Sense::click());

                    if !resp.hovered() && toast.closing_at.is_none() {
                        toast.remaining_secs = (toast.remaining_secs - dt).max(0.0);
                        if toast.remaining_secs <= 0.0 {
                            toast.closing_at = Some(now);
                        }
                    }

                    ui.painter().rect_filled(rect, 12.0, p.floating);
                    ui.painter().rect_stroke(
                        rect,
                        12.0,
                        p.border_stroke(),
                        egui::StrokeKind::Inside,
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
                        FontId::new(13.0, theme::semibold_family()),
                        Color32::WHITE,
                    );

                    ui.painter().text(
                        rect.left_top() + egui::vec2(52.0, 12.0),
                        Align2::LEFT_TOP,
                        &toast.sender,
                        FontId::new(13.5, theme::semibold_family()),
                        p.text_primary,
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
                        p.text_secondary,
                    );

                    let frac = (toast.remaining_secs / 4.0).clamp(0.0, 1.0);
                    let bar_w = (rect.width() - 16.0) * frac;
                    if bar_w > 0.5 {
                        let pbar = Rect::from_min_max(
                            egui::pos2(rect.left() + 8.0, rect.bottom() - 4.0),
                            egui::pos2(rect.left() + 8.0 + bar_w, rect.bottom() - 2.0),
                        );
                        ui.painter().rect_filled(pbar, 1.0, p.accent);
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
        self.play_sound(SoundEffect::UiClick);
    }
}
