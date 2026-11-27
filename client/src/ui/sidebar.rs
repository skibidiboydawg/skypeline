//! 64px navigation rail, 256px secondary sidebar with Ctrl+K search, active call mini-bar, and 56px bottom user dock.

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
                    .inner_margin(egui::Margin::symmetric(12, 14)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Clean W logo badge (40x40 rounded-xl bg-indigo-600)
                    let (brand_rect, _) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::hover());
                    ui.painter()
                        .rect_filled(brand_rect, 12.0, Color32::from_rgb(0x4F, 0x46, 0xE5));
                    ui.painter().text(
                        brand_rect.center(),
                        Align2::CENTER_CENTER,
                        "W",
                        FontId::proportional(19.0),
                        Color32::WHITE,
                    );

                    ui.add_space(14.0);
                    let sep_rect = Rect::from_center_size(
                        ui.cursor().center_top() + egui::vec2(0.0, 1.0),
                        egui::vec2(26.0, 1.0),
                    );
                    ui.painter().rect_filled(sep_rect, 0.5, theme::BORDER);
                    ui.add_space(14.0);

                    let pending_count = self
                        .friends
                        .iter()
                        .filter(|f| {
                            f.status == FriendshipStatus::Pending && f.direction == "incoming"
                        })
                        .count();

                    // 1. Direct Messages (<MessageSquare />)
                    let active_friends = self.selected_channel.is_none()
                        && self.rail_section == RailSection::Friends;
                    let (r1, resp1) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
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
                    icons::paint_message_square(
                        ui.painter(),
                        r1.center(),
                        18.0,
                        if active_friends {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if pending_count > 0 {
                        ui.painter().circle_filled(
                            r1.right_top() + egui::vec2(-3.0, 3.0),
                            5.0,
                            theme::DANGER,
                        );
                    }
                    if resp1.on_hover_text("Direct Messages").clicked() {
                        self.rail_section = RailSection::Friends;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    ui.add_space(10.0);

                    // 2. Call History (<Phone />)
                    let active_calls =
                        self.selected_channel.is_none() && self.rail_section == RailSection::Calls;
                    let (r2, resp2) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
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
                    icons::paint_phone(
                        ui.painter(),
                        r2.center(),
                        17.0,
                        if active_calls {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if resp2.on_hover_text("Call History").clicked() {
                        self.rail_section = RailSection::Calls;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    ui.add_space(10.0);

                    // 3. Groups (<Users />)
                    let active_groups =
                        self.selected_channel.is_none() && self.rail_section == RailSection::Groups;
                    let (r3, resp3) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
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
                    icons::paint_users(
                        ui.painter(),
                        r3.center(),
                        18.0,
                        if active_groups {
                            Color32::WHITE
                        } else {
                            theme::SECONDARY_TEXT
                        },
                    );
                    if resp3.on_hover_text("People & Groups").clicked() {
                        self.rail_section = RailSection::Groups;
                        self.selected_channel = None;
                        self.messages.clear();
                    }

                    // Bottom: <Compass /> (Discover / Theme) — no duplicate gear icon!
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let (rt, respt) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), Sense::click());
                        ui.painter().rect_filled(
                            rt,
                            12.0,
                            if respt.hovered() {
                                theme::HOVER
                            } else {
                                theme::SURFACE_SIDEBAR
                            },
                        );
                        icons::paint_compass(
                            ui.painter(),
                            rt.center(),
                            18.0,
                            theme::SECONDARY_TEXT,
                        );
                        if respt.on_hover_text("Discover / Toggle Theme").clicked() {
                            self.config.dark_mode = !self.config.dark_mode;
                            self.config.save();
                            theme::apply(ctx, self.config.dark_mode);
                        }
                    });
                });
            });
    }

    pub(crate) fn ui_left_sidebar(&mut self, ctx: &Context) {
        let search_id = egui::Id::new("sidebar_search_input");
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            ctx.memory_mut(|m| m.request_focus(search_id));
        }

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
                // Pin Active Voice Call Mini-Bar + strict 56px Bottom User Dock FIRST
                egui::TopBottomPanel::bottom("sidebar_user_dock")
                    .show_separator_line(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::same(0)))
                    .show_inside(ui, |ui| {
                        if self.in_voice {
                            self.ui_active_call_banner(ui);
                        }
                        self.ui_user_dock(ui);
                    });

                // Top + scrollable DM list fills remaining vertical space cleanly
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

                        // Search bar with <Search /> + subtle Ctrl+K / ⌘K hint pill
                        let search_focused = ui.memory(|m| m.has_focus(search_id));
                        egui::Frame::new()
                            .fill(theme::SURFACE_RAIL)
                            .stroke(if search_focused {
                                theme::focus_border()
                            } else {
                                theme::input_border()
                            })
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (ir, _) = ui.allocate_exact_size(
                                        egui::vec2(14.0, 14.0),
                                        Sense::hover(),
                                    );
                                    icons::paint_search(
                                        ui.painter(),
                                        ir.center(),
                                        14.0,
                                        theme::MUTED_TEXT,
                                    );
                                    let input_w = (ui.available_width() - 40.0).max(80.0);
                                    ui.add_sized(
                                        [input_w, 20.0],
                                        egui::TextEdit::singleline(&mut self.chat_search)
                                            .id(search_id)
                                            .frame(false)
                                            .hint_text("Search conversations..."),
                                    );
                                    let (kr, _) = ui.allocate_exact_size(
                                        egui::vec2(34.0, 18.0),
                                        Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        kr,
                                        4.0,
                                        Color32::from_rgb(0x16, 0x1A, 0x24),
                                    );
                                    ui.painter().text(
                                        kr.center(),
                                        Align2::CENTER_CENTER,
                                        "Ctrl K",
                                        FontId::proportional(9.5),
                                        theme::MUTED_TEXT,
                                    );
                                });
                            });
                        ui.add_space(10.0);

                        // Sleek active pill for "Friends & Calls" (<Users />)
                        let home = self.selected_channel.is_none();
                        let (nav_rect, nav_resp) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 36.0),
                            Sense::click(),
                        );
                        ui.painter().rect_filled(
                            nav_rect,
                            8.0,
                            if home {
                                Color32::from_rgb(0x1D, 0x22, 0x30)
                            } else if nav_resp.hovered() {
                                theme::HOVER
                            } else {
                                Color32::TRANSPARENT
                            },
                        );
                        let icon_c = nav_rect.left_center() + egui::vec2(16.0, 0.0);
                        icons::paint_users(
                            ui.painter(),
                            icon_c,
                            16.0,
                            if home {
                                Color32::WHITE
                            } else {
                                theme::SECONDARY_TEXT
                            },
                        );
                        ui.painter().text(
                            nav_rect.left_center() + egui::vec2(34.0, 0.0),
                            Align2::LEFT_CENTER,
                            "Friends & Calls",
                            FontId::proportional(13.5),
                            if home {
                                Color32::WHITE
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
                                        icons::paint_message_circle(
                                            ui.painter(),
                                            er.center(),
                                            24.0,
                                            theme::MUTED_TEXT,
                                        );
                                        ui.add_space(6.0);
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

    /// Active Voice Call Mini-Bar directly above the bottom user dock (<Signal /> + <PhoneOff />).
    fn ui_active_call_banner(&mut self, ui: &mut Ui) {
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, 0.0, Color32::from_rgb(0x0D, 0x17, 0x15));
        ui.painter().line_segment(
            [bar_rect.left_top(), bar_rect.right_top()],
            theme::border(),
        );

        let sig_c = bar_rect.left_center() + egui::vec2(20.0, 0.0);
        icons::paint_signal(ui.painter(), sig_c, 16.0, theme::SUCCESS);

        ui.painter().text(
            bar_rect.left_top() + egui::vec2(36.0, 8.0),
            Align2::LEFT_TOP,
            "Voice Connected",
            FontId::proportional(12.0),
            Color32::from_rgb(0x6E, 0xE7, 0xB7),
        );
        ui.painter().text(
            bar_rect.left_top() + egui::vec2(36.0, 23.0),
            Align2::LEFT_TOP,
            format!("{:02}:{:02} · Relay", elapsed / 60, elapsed % 60),
            FontId::proportional(11.0),
            theme::SECONDARY_TEXT,
        );

        let disc_r = Rect::from_center_size(
            bar_rect.right_center() - egui::vec2(22.0, 0.0),
            egui::vec2(28.0, 28.0),
        );
        let disc_resp = ui.interact(disc_r, ui.id().with("voice_disc_btn"), Sense::click());
        ui.painter().rect_filled(
            disc_r,
            7.0,
            if disc_resp.hovered() {
                theme::DANGER
            } else {
                Color32::from_rgb(0x2A, 0x15, 0x18)
            },
        );
        icons::paint_phone_off(
            ui.painter(),
            disc_r.center(),
            15.0,
            if disc_resp.hovered() {
                Color32::WHITE
            } else {
                theme::DANGER
            },
        );
        if disc_resp.on_hover_text("Disconnect").clicked() {
            self.leave_voice();
        }
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

    /// Strict 56px (h-14) Bottom User Dock on #0B0D12 with vertically centered Avatar, Username, Status, and <Mic /> <Headphones /> <Settings />.
    fn ui_user_dock(&mut self, ui: &mut Ui) {
        let me_name = self
            .me
            .as_ref()
            .map(|m| m.username.clone())
            .unwrap_or_else(|| "User".into());
        let me_id = self.me.as_ref().map(|m| m.user_id).unwrap_or(-1);
        let me_hash = self.me.as_ref().and_then(|m| m.avatar_hash.clone());

        let (dock_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), Sense::hover());
        ui.painter()
            .rect_filled(dock_rect, 0.0, theme::SURFACE_MAIN);
        ui.painter().line_segment(
            [dock_rect.left_top(), dock_rect.right_top()],
            theme::border(),
        );

        let cy = dock_rect.center().y;

        // Profile clickable area (left side)
        let prof_w = (dock_rect.width() - 116.0).max(96.0);
        let prof_rect = Rect::from_min_size(
            egui::pos2(dock_rect.left() + 8.0, cy - 20.0),
            egui::vec2(prof_w, 40.0),
        );
        let prof_resp = ui.interact(prof_rect, ui.id().with("user_dock_profile"), Sense::click());
        if prof_resp.hovered() {
            ui.painter().rect_filled(prof_rect, 8.0, theme::HOVER);
        }

        // 32x32 Avatar + 10px emerald dot with 2px #0B0D12 ring
        let avatar_rect = Rect::from_center_size(
            egui::pos2(prof_rect.left() + 20.0, cy),
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
            ui.painter()
                .circle_filled(avatar_rect.center(), 16.0, avatar_color(&me_name, true));
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
            .circle_filled(dot, 5.0, status_color(self.presence));
        ui.painter()
            .circle_stroke(dot, 5.0, Stroke::new(2.0_f32, theme::SURFACE_MAIN));

        // Vertically centered stacked Username + Status
        let text_x = avatar_rect.right() + 10.0;
        ui.painter().text(
            egui::pos2(text_x, cy - 1.5),
            Align2::LEFT_BOTTOM,
            &me_name,
            FontId::proportional(13.5),
            Color32::WHITE,
        );
        ui.painter().text(
            egui::pos2(text_x, cy + 1.5),
            Align2::LEFT_TOP,
            self.presence.label(),
            FontId::proportional(11.5),
            theme::SECONDARY_TEXT,
        );
        if prof_resp.on_hover_text("Open profile").clicked() {
            self.show_profile_popover = !self.show_profile_popover;
        }

        // Right-aligned three 28x28 icon buttons: <Mic />, <Headphones />, <Settings />
        let btn_size = 28.0;
        let gap = 4.0;
        let right_edge = dock_rect.right() - 10.0;

        // Settings
        let gr = Rect::from_center_size(
            egui::pos2(right_edge - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let gresp = ui.interact(gr, ui.id().with("dock_settings"), Sense::click());
        if gresp.hovered() {
            ui.painter().rect_filled(gr, 6.0, theme::HOVER);
        }
        icons::paint_settings(
            ui.painter(),
            gr.center(),
            16.0,
            if gresp.hovered() {
                Color32::WHITE
            } else {
                theme::SECONDARY_TEXT
            },
        );
        if gresp.on_hover_text("User Settings").clicked() {
            self.show_settings_modal = !self.show_settings_modal;
        }

        // Headphones (Deafen)
        let dr = Rect::from_center_size(
            egui::pos2(gr.left() - gap - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let dresp = ui.interact(dr, ui.id().with("dock_deafen"), Sense::click());
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
        icons::paint_headphones(
            ui.painter(),
            dr.center(),
            16.0,
            if self.deafened || dresp.hovered() {
                Color32::WHITE
            } else {
                theme::SECONDARY_TEXT
            },
            self.deafened,
        );
        if dresp.on_hover_text("Deafen Audio").clicked() {
            self.toggle_deafen();
        }

        // Mic (Mute)
        let mr = Rect::from_center_size(
            egui::pos2(dr.left() - gap - btn_size * 0.5, cy),
            egui::vec2(btn_size, btn_size),
        );
        let mresp = ui.interact(mr, ui.id().with("dock_mute"), Sense::click());
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
        icons::paint_mic(
            ui.painter(),
            mr.center(),
            16.0,
            if self.muted || mresp.hovered() {
                Color32::WHITE
            } else {
                theme::SECONDARY_TEXT
            },
            self.muted,
        );
        if mresp.on_hover_text("Mute Mic").clicked() {
            self.toggle_mute();
        }
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
            .constrain(true)
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
                                                RichText::new("Accept")
                                                    .small()
                                                    .color(Color32::WHITE),
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
