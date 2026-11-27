//! Friends & Calls home stage: add friends, filter online/all/pending, start DMs or calls.

use super::{avatar_color, status_color, FriendsTab, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::FriendshipStatus;

impl SkyApp {
    pub(crate) fn ui_friends_home(&mut self, ui: &mut Ui) {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.heading(RichText::new("Friends & Calls").size(28.0).strong());
                ui.label(theme::muted(
                    "Start a direct message, create a group, or jump into a call.",
                ));
                ui.add_space(16.0);
                let online_count = self
                    .friends
                    .iter()
                    .filter(|f| f.status == FriendshipStatus::Accepted && f.online)
                    .count();
                let all_count = self
                    .friends
                    .iter()
                    .filter(|f| f.status == FriendshipStatus::Accepted || f.direction == "mutual")
                    .count();
                let pending_count = self
                    .friends
                    .iter()
                    .filter(|f| f.status == FriendshipStatus::Pending)
                    .count();

                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (FriendsTab::Online, format!("Online ({online_count})")),
                        (FriendsTab::All, format!("All ({all_count})")),
                        (FriendsTab::Pending, format!("Pending ({pending_count})")),
                    ] {
                        let active = self.friends_tab == tab;
                        if ui
                            .add(
                                egui::Button::new(RichText::new(label).strong())
                                    .fill(if active {
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
                            self.friends_tab = tab;
                        }
                    }
                });
                ui.add_space(16.0);
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(theme::border())
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Add Friend").strong());
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let field = Self::form_field(
                                ui,
                                &mut self.friend_query,
                                "username or email",
                                false,
                            );
                            let add = egui::Button::new("Add")
                                .fill(theme::ACCENT)
                                .corner_radius(8.0)
                                .min_size(egui::vec2(80.0, 36.0));
                            if ui.add_enabled(!self.friend_busy, add).clicked()
                                || (field.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            {
                                self.send_friend_request();
                            }
                        });
                    });
                ui.add_space(12.0);
                if ui
                    .add(
                        egui::Button::new("+ Create Group Chat")
                            .fill(theme::SURFACE_CARD)
                            .stroke(theme::border())
                            .corner_radius(10.0),
                    )
                    .clicked()
                {
                    self.show_create_group = true;
                }
                if !self.notice.is_empty() {
                    ui.add_space(8.0);
                    self.ui_notice(ui);
                }
                ui.add_space(12.0);
                self.ui_friends_list(ui);
            });
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
            ui.label(theme::muted("No people here yet."));
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for friend in filtered {
                    egui::Frame::new()
                        .fill(theme::SURFACE_CARD)
                        .stroke(theme::border())
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(34.0, 34.0), Sense::hover());
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
                                        rect.center(),
                                        Align2::CENTER_CENTER,
                                        initial,
                                        FontId::proportional(14.0),
                                        Color32::WHITE,
                                    );
                                }
                                if friend.status == FriendshipStatus::Accepted {
                                    let dot = rect.right_bottom() - egui::vec2(4.0, 4.0);
                                    ui.painter().circle_filled(
                                        dot,
                                        5.0,
                                        status_color(friend.presence),
                                    );
                                    ui.painter().circle_stroke(
                                        dot,
                                        5.0,
                                        Stroke::new(1.5_f32, theme::SURFACE_CARD),
                                    );
                                }
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&friend.username).strong());
                                    let status = if friend.status == FriendshipStatus::Pending {
                                        if friend.direction == "incoming" {
                                            "Incoming request".to_string()
                                        } else {
                                            "Outgoing request".to_string()
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
                                                if ui
                                                    .add(
                                                        egui::Button::new("Accept")
                                                            .fill(theme::ACCENT)
                                                            .corner_radius(8.0),
                                                    )
                                                    .clicked()
                                                {
                                                    self.respond_friend(friend.user_id, true);
                                                }
                                                if ui.button("Decline").clicked() {
                                                    self.respond_friend(friend.user_id, false);
                                                }
                                            } else {
                                                ui.label(theme::muted("Pending"));
                                            }
                                        } else {
                                            if ui
                                                .add(
                                                    egui::Button::new("Call")
                                                        .fill(theme::ACCENT)
                                                        .corner_radius(8.0),
                                                )
                                                .clicked()
                                            {
                                                self.open_dm(friend.user_id, true);
                                            }
                                            if ui.button("Message").clicked() {
                                                self.open_dm(friend.user_id, false);
                                            }
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(8.0);
                }
            });
    }
}
