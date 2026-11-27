//! 64px far-left navigation rail, 240px secondary sidebar, bottom user dock, and profile popover.

use super::{avatar_color, icons, status_color, RailSection, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus, PresenceStatus};

impl SkyApp {
    pub(crate) fn ui_app_rail(&mut self, ctx: &Context) {
        egui::SidePanel::left("app_rail")
            .exact_width(64.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_RAIL)
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(10, 14)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Wireline app icon
                    let (brand_rect, _) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                    ui.painter().rect_filled(brand_rect, 12.0, theme::ACCENT);
                    ui.painter().text(
                        brand_rect.center(),
                        Align2::CENTER_CENTER,
                        "W",
                        FontId::proportional(19.0),
                        Color32::WHITE,
                    );

                    ui.add_space(14.0);
                    let sep_rect =
                        Rect::from_center_size(ui.cursor().center_top() + egui::vec2(0.0, 1.0), egui::vec2(28.0, 1.0));
                    ui.painter().rect_filled(sep_rect, 0.5, theme::BORDER);
                    ui.add_space(14.0);

                    let pending_count = self
                        .friends
                        .iter()
                        .filter(|f| {
                            f.status == FriendshipStatus::Pending && f.direction == "incoming"
                        })
                        .count();

                    // 1. Direct Messages / Friends
                    let active_friends =
                        self.selected_channel.is_none() && self.rail_section == RailSection::Friends;
                    let (r1, resp1) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::click());
                    ui.painter().rect_filled(
                        r1,
                        12.0,
                        if active_friends {
                            theme::ACCENT
                        } else if resp1.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_SIDEBAR
                        },
                    );
                    icons::paint_chat_icon(
                        ui.painter(),
                        r1.center(),
                        19.0,
                        if active_friends {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if pending_count > 0 {
                        ui.painter().circle_filled(
                            r1.right_top() + egui::vec2(-4.0, 4.0),
                            5.0,
                            theme::DANGER,
                        );
                    }
                    if resp1.on_hover_text("Direct Messages / Friends").clicked() {
                        self.rail_section = RailSection::Friends;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    ui.add_space(10.0);

                    // 2. Recent Calls
                    let active_calls =
                        self.selected_channel.is_none() && self.rail_section == RailSection::Calls;
                    let (r2, resp2) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::click());
                    ui.painter().rect_filled(
                        r2,
                        12.0,
                        if active_calls {
                            theme::ACCENT
                        } else if resp2.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_SIDEBAR
                        },
                    );
                    icons::paint_phone_icon(
                        ui.painter(),
                        r2.center(),
                        18.0,
                        if active_calls {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if resp2.on_hover_text("Recent Calls").clicked() {
                        self.rail_section = RailSection::Calls;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    ui.add_space(10.0);

                    // 3. Groups / People
                    let active_groups =
                        self.selected_channel.is_none() && self.rail_section == RailSection::Groups;
                    let (r3, resp3) =
                        ui.allocate_exact_size(egui::vec2(42.0, 42.0), Sense::click());
                    ui.painter().rect_filled(
                        r3,
                        12.0,
                        if active_groups {
                            theme::ACCENT
                        } else if resp3.hovered() {
                            theme::HOVER
                        } else {
                            theme::SURFACE_SIDEBAR
                        },
                    );
                    icons::paint_users_icon(
                        ui.painter(),
                        r3.center(),
                        19.0,
                        if active_groups {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if resp3.on_hover_text("Groups & People").clicked() {
                        self.rail_section = RailSection::Groups;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    // Bottom theme toggle icon
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let (rt, respt) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
                        ui.painter().rect_filled(
                            rt,
                            11.0,
                            if respt.hovered() {
                                theme::HOVER
                            } else {
                                theme::SURFACE_SIDEBAR
                            },
                        );
                        icons::paint_sun_moon_icon(
                            ui.painter(),
                            rt.center(),
                            18.0,
                            theme::SECONDARY_TEXT,
                            self.config.dark_mode,
                        );
                        if respt.on_hover_text("Toggle theme").clicked() {
                            self.config.dark_mode = !self.config.dark_mode;
                            self.config.save();
                            theme::apply(ctx, self.config.dark_mode);
                        }
                    });
                });
            });
    }

    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("left_sidebar")
            .exact_width(256.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_SIDEBAR)
                    .stroke(Stroke::NONE)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ctx, |ui| {
                // Pin the Bottom User Dock (and active call banner) FIRST using
                // TopBottomPanel::bottom::show_inside so it NEVER clips at any
                // window size or DPI scale.
                egui::TopBottomPanel::bottom("sidebar_user_dock")
                    .show_separator_line(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(0)))
                    .show_inside(ui, |ui| {
                        if self.in_voice {
                            egui::Frame::new()
                                .fill(theme::SURFACE_SIDEBAR)
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    self.ui_active_call_banner(ui);
                                });
                        }
                        self.ui_user_dock(ui);
                    });

                // Remaining area above the dock fills cleanly with a scrollable DM list.
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(theme::SURFACE_SIDEBAR)
                            .inner_margin(egui::Margin::symmetric(12, 12)),
                    )
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Wireline").size(16.0).strong());
                        });
                        ui.add_space(10.0);

                        // Recessed search bar (#0A0C10) with soft indigo focus ring
                        let search_id = ui.id().with("sidebar_search_input");
                        let search_focused = ui.memory(|m| m.has_focus(search_id));
                        egui::Frame::new()
                            .fill(theme::SURFACE_RAIL)
                            .stroke(if search_focused {
                                theme::focus_border()
                            } else {
                                theme::input_border()
                            })
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (ir, _) = ui.allocate_exact_size(
                                        egui::vec2(14.0, 14.0),
                                        Sense::hover(),
                                    );
                                    icons::paint_search_icon(
                                        ui.painter(),
                                        ir.center(),
                                        13.0,
                                        theme::MUTED_TEXT,
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.chat_search)
                                            .id(search_id)
                                            .frame(false)
                                            .hint_text("Search conversations...")
                                            .desired_width(ui.available_width()),
                                    );
                                });
                            });
                        ui.add_space(10.0);

                        // Sleek active pill for "Friends & Calls"
                        let home = self.selected_channel.is_none();
                        let (nav_rect, nav_resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 36.0),
                            Sense::click(),
                        );
                        ui.painter().rect_filled(
                            nav_rect,
                            8.0,
                            if home {
                                Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                            } else if nav_resp.hovered() {
                                theme::HOVER
                            } else {
                                Color32::TRANSPARENT
                            },
                        );
                        let icon_c = nav_rect.left_center() + egui::vec2(16.0, 0.0);
                        icons::paint_users_icon(
                            ui.painter(),
                            icon_c,
                            15.0,
                            if home {
                                theme::PRIMARY_TEXT
                            } else {
                                theme::SECONDARY_TEXT
                            },
                        );
                        ui.painter().text(
                            nav_rect.left_center() + egui::vec2(32.0, 0.0),
                            Align2::LEFT_CENTER,
                            "Friends & Calls",
                            FontId::proportional(13.5),
                            if home {
                                theme::PRIMARY_TEXT
                            } else {
                                theme::SECONDARY_TEXT
                            },
                        );
                        if nav_resp.clicked() {
                            self.selected_channel = None;
                            self.rail_section = RailSection::Friends;
                            self.messages.clear();
                        }

                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("DIRECT MESSAGES")
                                .size(11.0)
                                .strong()
                                .color(theme::MUTED_TEXT),
                        );
                        ui.add_space(6.0);

                        let search = self.chat_search.trim().to_ascii_lowercase();
                        let mut dms: Vec<ChannelInfo> = self
                            .channels
                            .iter()
                            .filter(|c| c.kind == "dm")
                            .cloned()
                            .collect();
                        dms.sort_by(|a, b| b.id.cmp(&a.id));

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let visible_dms: Vec<ChannelInfo> = dms
                                    .into_iter()
                                    .filter(|ch| {
                                        search.is_empty()
                                            || ch.name.to_ascii_lowercase().contains(&search)
                                    })
                                    .collect();
                                if visible_dms.is_empty() {
                                    ui.add_space(28.0);
                                    ui.vertical_centered(|ui| {
                                        let (er, _) = ui.allocate_exact_size(
                                            egui::vec2(28.0, 28.0),
                                            Sense::hover(),
                                        );
                                        icons::paint_chat_icon(
                                            ui.painter(),
                                            er.center(),
                                            22.0,
                                            theme::MUTED_TEXT,
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new("No conversations yet")
                                                .small()
                                                .color(theme::MUTED_TEXT),
                                        );
                                    });
                                } else {
                                    for ch in visible_dms {
                                        self.ui_channel_row(ui, &ch);
                                    }
                                }
                            });
                    });
            });
    }

    fn ui_channel_row(&mut self, ui: &mut Ui, ch: &ChannelInfo) {
        let selected = self.selected_channel == Some(ch.id);
        let unread = self.unread.get(&ch.id).copied().unwrap_or(0);
        let presence = ch
            .peer_user_id
            .map(|id| self.online_status_for(id))
            .unwrap_or(PresenceStatus::Offline);

        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), Sense::click());
        let bg = if selected {
            theme::SURFACE_CARD
        } else if resp.hovered() {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 8.0, bg);
        let ar = Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::vec2(30.0, 30.0));
        self.paint_channel_avatar(ui, ar, ch, presence);
        ui.painter().text(
            rect.min + egui::vec2(44.0, 14.0),
            Align2::LEFT_TOP,
            &ch.name,
            FontId::proportional(13.5),
            if selected {
                Color32::WHITE
            } else {
                theme::SECONDARY_TEXT
            },
        );
        if unread > 0 {
            let br = Rect::from_center_size(
                rect.right_center() - egui::vec2(14.0, 0.0),
                egui::vec2(18.0, 18.0),
            );
            ui.painter().circle_filled(br.center(), 9.0, theme::DANGER);
            ui.painter().text(
                br.center(),
                Align2::CENTER_CENTER,
                unread.min(99).to_string(),
                FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
        if resp.clicked() {
            self.select_channel(ch.id);
        }
    }

    fn ui_active_call_banner(&mut self, ui: &mut Ui) {
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        egui::Frame::new()
            .fill(theme::SURFACE_CARD)
            .stroke(Stroke::new(1.0_f32, theme::SUCCESS))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Voice Connected")
                                .color(theme::SUCCESS)
                                .small()
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("{:02}:{:02}", elapsed / 60, elapsed % 60))
                                .small()
                                .color(theme::SECONDARY_TEXT),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("End").small().color(Color32::WHITE))
                                    .fill(theme::DANGER)
                                    .corner_radius(7.0),
                            )
                            .clicked()
                        {
                            self.leave_voice();
                        }
                    });
                });
            });
    }

    fn paint_channel_avatar(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        channel: &ChannelInfo,
        presence: PresenceStatus,
    ) {
        let peer = channel.peer_user_id.unwrap_or(-1);
        let hash = self
            .friends
            .iter()
            .find(|f| f.user_id == peer)
            .and_then(|f| f.avatar_hash.clone());
        let mut drew = false;
        if let Some(hash) = hash.as_deref() {
            let tex = self.avatars.texture(
                &ui.ctx().clone(),
                &self.api(),
                self.token.as_deref(),
                peer,
                Some(hash),
                &self.events_tx,
            );
            if let Some(t) = tex {
                ui.painter().image(
                    t.id(),
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                drew = true;
            }
        }
        if !drew {
            ui.painter().circle_filled(
                rect.center(),
                rect.width() / 2.0,
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
                rect.center(),
                Align2::CENTER_CENTER,
                initial,
                FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
        let dot = rect.right_bottom() - egui::vec2(3.0, 3.0);
        ui.painter().circle_filled(dot, 4.5, status_color(presence));
        ui.painter()
            .circle_stroke(dot, 4.5, Stroke::new(1.5_f32, theme::SURFACE_SIDEBAR));
    }

    fn ui_user_dock(&mut self, ui: &mut Ui) {
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());

        egui::Frame::new()
            .fill(theme::SURFACE_DOCK)
            .stroke(Stroke::NONE)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                let top_edge = ui.max_rect().left_top();
                ui.painter().line_segment(
                    [top_edge - egui::vec2(12.0, 10.0), egui::pos2(ui.max_rect().right() + 12.0, top_edge.y - 10.0)],
                    theme::border(),
                );
                ui.set_min_height(38.0);
                ui.horizontal(|ui| {
                    // Clickable Avatar + Name/Status block -> opens Profile Popover Card
                    let prof_w = (ui.available_width() - 98.0).max(100.0);
                    let (prof_rect, prof_resp) =
                        ui.allocate_exact_size(egui::vec2(prof_w, 38.0), Sense::click());
                    if prof_resp.hovered() {
                        ui.painter().rect_filled(prof_rect, 8.0, theme::HOVER);
                    }
                    let avatar_rect = Rect::from_min_size(
                        prof_rect.min + egui::vec2(2.0, 2.0),
                        egui::vec2(32.0, 32.0),
                    );
                    let mut drew = false;
                    if let Some(h) = me_hash.as_deref() {
                        let tex = self.avatars.texture(
                            &ui.ctx().clone(),
                            &self.api(),
                            self.token.as_deref(),
                            me_id,
                            Some(h),
                            &self.events_tx,
                        );
                        if let Some(t) = tex {
                            ui.painter().image(
                                t.id(),
                                avatar_rect,
                                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                            drew = true;
                        }
                    }
                    if !drew {
                        ui.painter().circle_filled(
                            avatar_rect.center(),
                            16.0,
                            avatar_color(&me_name, true),
                        );
                        let initial = me_name
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_ascii_uppercase()
                            .to_string();
                        ui.painter().text(
                            avatar_rect.center(),
                            Align2::CENTER_CENTER,
                            initial,
                            FontId::proportional(13.0),
                            Color32::WHITE,
                        );
                    }
                    let dot = avatar_rect.right_bottom() - egui::vec2(3.0, 3.0);
                    ui.painter()
                        .circle_filled(dot, 4.5, status_color(self.presence));
                    ui.painter()
                        .circle_stroke(dot, 4.5, Stroke::new(1.5_f32, theme::SURFACE_DOCK));

                    ui.painter().text(
                        prof_rect.min + egui::vec2(40.0, 4.0),
                        Align2::LEFT_TOP,
                        &me_name,
                        FontId::proportional(13.0),
                        theme::PRIMARY_TEXT,
                    );
                    ui.painter().text(
                        prof_rect.min + egui::vec2(40.0, 20.0),
                        Align2::LEFT_TOP,
                        self.presence.label(),
                        FontId::proportional(11.0),
                        theme::SECONDARY_TEXT,
                    );
                    if prof_resp.on_hover_text("Open profile").clicked() {
                        self.show_profile_popover = !self.show_profile_popover;
                    }

                    // Right-aligned icon buttons: Mute Mic, Deafen Audio, User Settings (Gear)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (gr, gresp) =
                            ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                        if gresp.hovered() {
                            ui.painter().rect_filled(gr, 6.0, theme::HOVER);
                        }
                        icons::paint_gear_icon(
                            ui.painter(),
                            gr.center(),
                            15.0,
                            theme::SECONDARY_TEXT,
                        );
                        if gresp.on_hover_text("User Settings").clicked() {
                            self.show_settings_modal = !self.show_settings_modal;
                        }

                        let (dr, dresp) =
                            ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                        if dresp.hovered() || self.deafened {
                            ui.painter().rect_filled(
                                dr,
                                6.0,
                                if self.deafened {
                                    theme::DANGER
                                } else {
                                    theme::HOVER
                                },
                            );
                        }
                        icons::paint_headphones_icon(
                            ui.painter(),
                            dr.center(),
                            15.0,
                            if self.deafened {
                                Color32::WHITE
                            } else {
                                theme::SECONDARY_TEXT
                            },
                            self.deafened,
                        );
                        if dresp.on_hover_text("Deafen Audio").clicked() {
                            self.toggle_deafen();
                        }

                        let (mr, mresp) =
                            ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                        if mresp.hovered() || self.muted {
                            ui.painter().rect_filled(
                                mr,
                                6.0,
                                if self.muted {
                                    theme::DANGER
                                } else {
                                    theme::HOVER
                                },
                            );
                        }
                        icons::paint_mic_icon(
                            ui.painter(),
                            mr.center(),
                            15.0,
                            if self.muted {
                                Color32::WHITE
                            } else {
                                theme::SECONDARY_TEXT
                            },
                            self.muted,
                        );
                        if mresp.on_hover_text("Mute Mic").clicked() {
                            self.toggle_mute();
                        }
                    });
                });
            });
    }

    pub(crate) fn ui_profile_popover(&mut self, ctx: &Context) {
        if !self.show_profile_popover {
            return;
        }
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_email = self
            .me
            .as_ref()
            .map(|m| m.email.clone())
            .unwrap_or_default();

        egui::Area::new(egui::Id::new("profile_popover_card"))
            .constrain(true)
            .fixed_pos(egui::pos2(
                74.0,
                (ctx.screen_rect().bottom() - 270.0).max(16.0),
            ))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(theme::border())
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(210.0);
                        ui.label(RichText::new(&me_name).size(15.0).strong());
                        if !me_email.is_empty() {
                            ui.label(theme::muted(&me_email).small());
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("STATUS")
                                .size(10.5)
                                .strong()
                                .color(theme::MUTED_TEXT),
                        );
                        ui.add_space(4.0);
                        for status in [
                            PresenceStatus::Online,
                            PresenceStatus::Away,
                            PresenceStatus::Dnd,
                        ] {
                            let active = self.presence == status;
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(status.label()).color(if active {
                                            Color32::WHITE
                                        } else {
                                            theme::SECONDARY_TEXT
                                        }),
                                    )
                                    .fill(if active {
                                        theme::ACCENT_SOFT
                                    } else {
                                        Color32::TRANSPARENT
                                    })
                                    .stroke(Stroke::NONE)
                                    .corner_radius(7.0)
                                    .min_size(egui::vec2(ui.available_width(), 28.0)),
                                )
                                .clicked()
                            {
                                self.set_presence(status);
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new("Settings")
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(7.0)
                                    .min_size(egui::vec2(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.show_settings_modal = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Sign out").color(theme::DANGER))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .corner_radius(7.0)
                                    .min_size(egui::vec2(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            self.show_profile_popover = false;
                            self.sign_out();
                        }
                    });
            });
    }

    pub(crate) fn ui_settings_modal(&mut self, ctx: &Context) {
        if !self.show_settings_modal {
            return;
        }
        let mut open = self.show_settings_modal;
        egui::Window::new("User Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(theme::border())
                    .corner_radius(14.0)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("Appearance").strong());
                ui.add_space(6.0);
                if ui
                    .checkbox(&mut self.config.dark_mode, "Dark mode")
                    .changed()
                {
                    self.config.save();
                    theme::apply(ctx, self.config.dark_mode);
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Audio").strong());
                ui.add_space(6.0);
                ui.label(theme::muted(format!(
                    "Microphone: {}",
                    if self.muted { "Muted" } else { "Active" }
                )));
                ui.label(theme::muted(format!(
                    "Audio output: {}",
                    if self.deafened { "Deafened" } else { "Active" }
                )));
            });
        self.show_settings_modal = open;
    }

    pub(crate) fn ui_inbox_popover(&mut self, ctx: &Context) {
        if !self.show_inbox_popover {
            return;
        }
        let incoming: Vec<_> = self
            .friends
            .iter()
            .filter(|f| f.status == FriendshipStatus::Pending && f.direction == "incoming")
            .cloned()
            .collect();

        egui::Area::new(egui::Id::new("inbox_popover"))
            .fixed_pos(egui::pos2(ctx.screen_rect().right() - 290.0, 54.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::SURFACE_CARD)
                    .stroke(theme::border())
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.label(RichText::new("Notifications Inbox").strong());
                        ui.add_space(8.0);
                        if incoming.is_empty() {
                            ui.label(theme::muted("You're all caught up.").small());
                        } else {
                            for req in incoming {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!(
                                        "{} sent a friend request",
                                        req.username
                                    )));
                                });
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Accept").small().color(Color32::WHITE),
                                            )
                                            .fill(theme::ACCENT)
                                            .corner_radius(6.0),
                                        )
                                        .clicked()
                                    {
                                        self.respond_friend(req.user_id, true);
                                    }
                                    if ui.small_button("Decline").clicked() {
                                        self.respond_friend(req.user_id, false);
                                    }
                                });
                                ui.add_space(6.0);
                            }
                        }
                    });
            });
    }
}
