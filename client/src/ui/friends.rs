//! Main Content Area: 48px top header bar, vertical column max-w-2xl wrapper, Add a Friend card, and centered empty state.

use super::{avatar_color, icons, status_color, FriendsTab, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::FriendshipStatus;

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

        // 1. Top Header Bar (48px height, border-b border-white/5, px-6)
        let (header_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), Sense::hover());
        ui.painter()
            .rect_filled(header_rect, 0.0, theme::SURFACE_SIDEBAR);
        ui.painter().line_segment(
            [header_rect.left_bottom(), header_rect.right_bottom()],
            theme::border(),
        );

        let header_inner = header_rect.shrink2(egui::vec2(24.0, 6.0));
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(header_inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.label(RichText::new("Friends & Calls").size(15.0).strong());
                ui.add_space(6.0);
                let (div_r, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), Sense::hover());
                ui.painter().rect_filled(div_r, 0.0, theme::BORDER);
                ui.add_space(6.0);

                for (tab, label) in [
                    (FriendsTab::Online, "Online"),
                    (FriendsTab::All, "All"),
                    (FriendsTab::Pending, "Pending"),
                ] {
                    let active = self.friends_tab == tab;
                    let show_badge = tab == FriendsTab::Pending && pending_count > 0;
                    let btn_w = if show_badge { 88.0 } else { 64.0 };
                    let (tr, tresp) =
                        ui.allocate_exact_size(egui::vec2(btn_w, 28.0), Sense::click());
                    ui.painter().rect_filled(
                        tr,
                        7.0,
                        if active {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 24)
                        } else if tresp.hovered() {
                            theme::HOVER
                        } else {
                            Color32::TRANSPARENT
                        },
                    );
                    let text_pos = if show_badge {
                        tr.left_center() + egui::vec2(12.0, 0.0)
                    } else {
                        tr.center()
                    };
                    let align = if show_badge {
                        Align2::LEFT_CENTER
                    } else {
                        Align2::CENTER_CENTER
                    };
                    ui.painter().text(
                        text_pos,
                        align,
                        label,
                        FontId::proportional(13.5),
                        if active {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if show_badge {
                        let badge_c = tr.right_center() - egui::vec2(14.0, 0.0);
                        ui.painter().circle_filled(badge_c, 8.5, theme::DANGER);
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

                // Right side: crisp emerald "Update Available" pill + Notifications Inbox icon
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (bell_r, bell_resp) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
                    ui.painter().rect_filled(
                        bell_r,
                        8.0,
                        if self.show_inbox_popover || bell_resp.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_CARD
                        },
                    );
                    icons::paint_inbox(
                        ui.painter(),
                        bell_r.center(),
                        16.0,
                        theme::SECONDARY_TEXT,
                    );
                    if incoming_count > 0 {
                        ui.painter().circle_filled(
                            bell_r.right_top() + egui::vec2(-4.0, 4.0),
                            4.5,
                            theme::DANGER,
                        );
                    }
                    if bell_resp.on_hover_text("Notifications Inbox").clicked() {
                        self.show_inbox_popover = !self.show_inbox_popover;
                    }

                    ui.add_space(6.0);

                    let mint_text = Color32::from_rgb(0x6E, 0xE7, 0xB7);
                    let mint_bg = Color32::from_rgb(0x0E, 0x29, 0x22);
                    let mint_border = Color32::from_rgb(0x1B, 0x4D, 0x3E);
                    let (upd_r, upd_resp) =
                        ui.allocate_exact_size(egui::vec2(148.0, 28.0), Sense::click());
                    ui.painter().rect_filled(
                        upd_r,
                        14.0,
                        if upd_resp.hovered() {
                            Color32::from_rgb(0x05, 0x96, 0x69)
                        } else {
                            mint_bg
                        },
                    );
                    ui.painter().rect_stroke(
                        upd_r,
                        14.0,
                        Stroke::new(1.0_f32, mint_border),
                        egui::StrokeKind::Inside,
                    );
                    let fg = if upd_resp.hovered() {
                        Color32::WHITE
                    } else {
                        mint_text
                    };
                    icons::paint_download(
                        ui.painter(),
                        upd_r.left_center() + egui::vec2(16.0, 0.0),
                        14.0,
                        fg,
                    );
                    ui.painter().text(
                        upd_r.left_center() + egui::vec2(28.0, 0.0),
                        Align2::LEFT_CENTER,
                        "Update Available",
                        FontId::proportional(12.5),
                        fg,
                    );
                    if upd_resp
                        .on_hover_text("Open latest Wireline release")
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(
                            "https://github.com/skibidiboydawg/skypeline/releases/latest",
                        ));
                    }
                });
            },
        );

        // 2. Vertical column container (flex-1 flex flex-col items-center pt-12 px-8)
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let container_w = (avail_w - 64.0).clamp(280.0, 672.0);
                let side_pad = ((avail_w - container_w) * 0.5).max(16.0);

                ui.add_space(48.0);
                // IMPORTANT: top_down layout so children stack vertically, never sideways!
                ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_size(
                            ui.cursor().min + egui::vec2(side_pad, 0.0),
                            egui::vec2(container_w, ui.available_height().max(400.0)),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.set_width(container_w);

                        // "Add a Friend" Card (borderless elevated #131720 surface, p-6)
                        egui::Frame::new()
                            .fill(theme::SURFACE_CARD)
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x18, 0x1D, 0x28)))
                            .corner_radius(12.0)
                            .inner_margin(egui::Margin::same(24))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new("Add a Friend").size(16.0).strong());
                                ui.add_space(4.0);
                                ui.label(
                                    theme::muted(
                                        "You can add friends with their Wireline username or email.",
                                    )
                                    .size(13.5),
                                );
                                ui.add_space(14.0);

                                let focus_id = ui.id().with("add_friend_input");
                                let has_focus = ui.memory(|m| m.has_focus(focus_id));
                                let bar_stroke = if has_focus {
                                    theme::focus_border()
                                } else {
                                    theme::input_border()
                                };

                                egui::Frame::new()
                                    .fill(theme::SURFACE_RAIL)
                                    .stroke(bar_stroke)
                                    .corner_radius(9.0)
                                    .inner_margin(egui::Margin::symmetric(12, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let input_w =
                                                (ui.available_width() - 122.0).max(120.0);
                                            let field = ui.add_sized(
                                                [input_w, 32.0],
                                                egui::TextEdit::singleline(&mut self.friend_query)
                                                    .id(focus_id)
                                                    .frame(false)
                                                    .hint_text("Enter a username or email..."),
                                            );
                                            let send_btn = egui::Button::new(
                                                RichText::new("Send Request")
                                                    .strong()
                                                    .size(13.5)
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(theme::ACCENT)
                                            .stroke(Stroke::NONE)
                                            .corner_radius(7.0)
                                            .min_size(egui::vec2(114.0, 32.0));
                                            if ui.add_enabled(!self.friend_busy, send_btn).clicked()
                                                || (field.lost_focus()
                                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                            {
                                                self.send_friend_request();
                                            }
                                        });
                                    });
                            });

                        if !self.notice.is_empty() {
                            ui.add_space(12.0);
                            self.ui_notice(ui);
                        }

                        ui.add_space(32.0);
                        self.ui_friends_list(ui);
                    },
                );
            });
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
        let filtered: Vec<_> = friends
            .into_iter()
            .filter(|f| match self.friends_tab {
                FriendsTab::Online => f.status == FriendshipStatus::Accepted && f.online,
                FriendsTab::All => {
                    f.status == FriendshipStatus::Accepted || f.direction == "mutual"
                }
                FriendsTab::Pending => f.status == FriendshipStatus::Pending,
            })
            .collect();

        if filtered.is_empty() {
            ui.add_space(32.0);
            ui.vertical_centered(|ui| {
                let (icon_r, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), Sense::hover());
                ui.painter().circle_filled(
                    icon_r.center(),
                    24.0,
                    Color32::from_rgb(0x16, 0x1A, 0x24),
                );
                icons::paint_user_plus(
                    ui.painter(),
                    icon_r.center(),
                    24.0,
                    theme::SECONDARY_TEXT,
                );
                ui.add_space(16.0);
                let (title, subtitle) = match self.friends_tab {
                    FriendsTab::Online => (
                        "No friends online",
                        "When your friends come online, they'll show up here ready to chat or jump into a call.",
                    ),
                    FriendsTab::All => (
                        "No friends added yet",
                        "Use the Add a Friend bar above to connect with someone on Wireline.",
                    ),
                    FriendsTab::Pending => (
                        "No pending requests",
                        "Incoming and outgoing friend requests will show up here.",
                    ),
                };
                ui.label(
                    RichText::new(title)
                        .size(16.0)
                        .strong()
                        .color(Color32::from_rgb(0xE4, 0xE4, 0xE7)),
                );
                ui.add_space(6.0);
                ui.add_sized(
                    [384.0_f32.min(ui.available_width()), 36.0],
                    egui::Label::new(
                        RichText::new(subtitle)
                            .size(14.0)
                            .color(Color32::from_rgb(0x71, 0x71, 0x7A)),
                    )
                    .wrap(),
                );
            });
            ui.add_space(32.0);
            return;
        }

        for friend in filtered {
            egui::Frame::new()
                .fill(theme::SURFACE_CARD)
                .stroke(theme::border())
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(36.0, 36.0), Sense::hover());
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
                                    rect,
                                    Rect::from_min_max(
                                        egui::pos2(0.0, 0.0),
                                        egui::pos2(1.0, 1.0),
                                    ),
                                    Color32::WHITE,
                                );
                                drew = true;
                            }
                        }
                        if !drew {
                            ui.painter().circle_filled(
                                rect.center(),
                                18.0,
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
                                rect.center(),
                                Align2::CENTER_CENTER,
                                initial,
                                FontId::proportional(14.0),
                                Color32::WHITE,
                            );
                        }
                        if friend.status == FriendshipStatus::Accepted {
                            let dot = rect.right_bottom() - egui::vec2(3.0, 3.0);
                            ui.painter()
                                .circle_filled(dot, 5.0, status_color(friend.presence));
                            ui.painter().circle_stroke(
                                dot,
                                5.0,
                                Stroke::new(1.5_f32, theme::SURFACE_CARD),
                            );
                        }
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&friend.username).size(14.5).strong());
                            let status = if friend.status == FriendshipStatus::Pending {
                                if friend.direction == "incoming" {
                                    "Incoming friend request".to_string()
                                } else {
                                    "Outgoing friend request".to_string()
                                }
                            } else {
                                friend.presence.label().to_string()
                            };
                            ui.label(theme::muted(status).small());
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if friend.status == FriendshipStatus::Pending {
                                    if friend.direction == "incoming" {
                                        // Red <X /> Decline icon button
                                        let (xr, xresp) = ui.allocate_exact_size(
                                            egui::vec2(32.0, 32.0),
                                            Sense::click(),
                                        );
                                        ui.painter().circle_filled(
                                            xr.center(),
                                            15.0,
                                            if xresp.hovered() {
                                                theme::DANGER
                                            } else {
                                                Color32::from_rgb(0x28, 0x16, 0x1B)
                                            },
                                        );
                                        icons::paint_x(
                                            ui.painter(),
                                            xr.center(),
                                            15.0,
                                            if xresp.hovered() {
                                                Color32::WHITE
                                            } else {
                                                theme::DANGER
                                            },
                                        );
                                        if xresp.on_hover_text("Decline").clicked() {
                                            self.respond_friend(friend.user_id, false);
                                        }

                                        ui.add_space(6.0);

                                        // Green <Check /> Accept icon button
                                        let (cr, cresp) = ui.allocate_exact_size(
                                            egui::vec2(32.0, 32.0),
                                            Sense::click(),
                                        );
                                        ui.painter().circle_filled(
                                            cr.center(),
                                            15.0,
                                            if cresp.hovered() {
                                                theme::SUCCESS
                                            } else {
                                                Color32::from_rgb(0x10, 0x2A, 0x20)
                                            },
                                        );
                                        icons::paint_check(
                                            ui.painter(),
                                            cr.center(),
                                            15.0,
                                            if cresp.hovered() {
                                                Color32::WHITE
                                            } else {
                                                theme::SUCCESS
                                            },
                                        );
                                        if cresp.on_hover_text("Accept").clicked() {
                                            self.respond_friend(friend.user_id, true);
                                        }
                                    } else {
                                        ui.label(theme::muted("Pending").small());
                                    }
                                } else {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Call").color(Color32::WHITE),
                                            )
                                            .fill(theme::ACCENT)
                                            .corner_radius(8.0),
                                        )
                                        .clicked()
                                    {
                                        self.open_dm(friend.user_id, true);
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new("Message")
                                                .fill(theme::SURFACE_ELEVATED)
                                                .corner_radius(8.0),
                                        )
                                        .clicked()
                                    {
                                        self.open_dm(friend.user_id, false);
                                    }
                                }
                            },
                        );
                    });
                });
            ui.add_space(8.0);
        }
    }
}
