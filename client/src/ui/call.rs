//! Scalable Wireline Voice & Screen-Share Stage:
//! Drag the bottom resize grip freely to scale the call from compact strip up to full canvas,
//! plus quick S / M / L size presets, live 720p/1080p & 15/30/60 FPS controls, and circular call dock.

use super::{avatar_color, icons, SkyApp};
use crate::sounds::SoundEffect;
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, Sense, Stroke, Ui};
use skypeline_shared::ChannelInfo;

impl SkyApp {
    pub(crate) fn ui_call_stage(&mut self, ui: &mut Ui, channel: &ChannelInfo) {
        let p = theme::of(ui.ctx());
        let elapsed = self
            .call_started_at
            .map(|s| (Self::now_secs() - s).max(0.0) as u64)
            .unwrap_or(0);
        let status = if self.participants.len() <= 1 {
            format!("Calling {}...", channel.name)
        } else {
            format!(
                "{} · {:02}:{:02}",
                channel.name,
                elapsed / 60,
                elapsed % 60
            )
        };

        let has_screen =
            self.screen_texture.is_some() && self.screen_size.0 > 0 && self.screen_size.1 > 0;
        let avail = ui.available_rect_before_wrap();
        let min_h = 180.0_f32;
        let max_h = (avail.height() - 120.0).max(min_h + 40.0);
        let height = self.config.call_stage_height.clamp(min_h, max_h);

        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(avail.width(), height + 12.0), Sense::hover());
        let stage_rect = Rect::from_min_max(
            outer_rect.min + egui::vec2(14.0, 10.0),
            egui::pos2(outer_rect.right() - 14.0, outer_rect.bottom() - 12.0),
        );
        let painter = ui.painter_at(stage_rect);

        // Sleek rounded studio call container
        painter.rect_filled(stage_rect, 14.0, p.call_canvas);
        painter.rect_stroke(
            stage_rect,
            14.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x23, 0x29, 0x3A)),
            egui::StrokeKind::Inside,
        );

        // Top-left status + LIVE badge when sharing
        painter.text(
            stage_rect.left_top() + egui::vec2(16.0, 12.0),
            Align2::LEFT_TOP,
            &status,
            FontId::new(13.5, theme::semibold_family()),
            Color32::from_rgb(0xF3, 0xF5, 0xFA),
        );
        if self.sharing || has_screen {
            let live_r = Rect::from_min_size(
                stage_rect.left_top() + egui::vec2(16.0, 33.0),
                egui::vec2(42.0, 18.0),
            );
            painter.rect_filled(live_r, 9.0, p.danger);
            painter.text(
                live_r.center(),
                Align2::CENTER_CENTER,
                "LIVE",
                FontId::new(10.0, theme::semibold_family()),
                Color32::WHITE,
            );
            let owner_label = if self.sharing
                && self.screen_owner == self.me.as_ref().map(|m| m.user_id)
            {
                format!(
                    "{}p · {} FPS",
                    self.config.screen_max_height, self.config.screen_fps
                )
            } else {
                format!("{}x{}", self.screen_size.0, self.screen_size.1)
            };
            painter.text(
                live_r.right_center() + egui::vec2(8.0, 0.0),
                Align2::LEFT_CENTER,
                owner_label,
                FontId::proportional(11.5),
                Color32::from_rgb(0xA6, 0xB0, 0xC6),
            );
        }

        // Top-right controls: [Hide] [Expand] [L][M][S] + live FPS/Res pills when sharing
        let mut rx = stage_rect.right() - 14.0;
        let top_y = stage_rect.top() + 10.0;
        let dark_btn = Color32::from_rgb(0x1E, 0x23, 0x32);
        let dark_btn_hov = Color32::from_rgb(0x2B, 0x32, 0x47);
        let light_txt = Color32::from_rgb(0xD5, 0xDC, 0xEB);

        // Hide button
        rx -= 56.0;
        let hide_r = Rect::from_min_size(egui::pos2(rx, top_y), egui::vec2(56.0, 24.0));
        let hide_resp = ui.interact(hide_r, ui.id().with("call_collapse"), Sense::click());
        painter.rect_filled(
            hide_r,
            6.0,
            if hide_resp.hovered() {
                dark_btn_hov
            } else {
                dark_btn
            },
        );
        painter.text(
            hide_r.center(),
            Align2::CENTER_CENTER,
            "Hide",
            FontId::new(11.5, theme::semibold_family()),
            light_txt,
        );
        if hide_resp.clicked() {
            self.call_expanded = false;
            self.play_sound(SoundEffect::UiClick);
        }

        // Fullscreen button when screen stream active
        if has_screen {
            rx -= 76.0 + 6.0;
            let fs_r = Rect::from_min_size(egui::pos2(rx, top_y), egui::vec2(76.0, 24.0));
            let fs_resp = ui.interact(fs_r, ui.id().with("fs_btn"), Sense::click());
            painter.rect_filled(
                fs_r,
                6.0,
                if fs_resp.hovered() {
                    p.accent
                } else {
                    dark_btn
                },
            );
            icons::paint_fullscreen(
                &painter,
                fs_r.left_center() + egui::vec2(13.0, 0.0),
                12.5,
                Color32::WHITE,
            );
            painter.text(
                fs_r.left_center() + egui::vec2(24.0, 0.0),
                Align2::LEFT_CENTER,
                "Full",
                FontId::new(11.5, theme::semibold_family()),
                Color32::WHITE,
            );
            if fs_resp.clicked() {
                self.screen_fullscreen = true;
                self.play_sound(SoundEffect::UiClick);
            }
        }

        // Quick Scale presets: L / M / S
        rx -= 6.0;
        for (label, target_h) in [("L", 480.0_f32), ("M", 320.0_f32), ("S", 200.0_f32)] {
            rx -= 24.0 + 3.0;
            let sr = Rect::from_min_size(egui::pos2(rx, top_y), egui::vec2(24.0, 24.0));
            let sresp = ui.interact(sr, ui.id().with(("call_sz", label)), Sense::click());
            let active = (self.config.call_stage_height - target_h).abs() < 35.0;
            painter.rect_filled(
                sr,
                6.0,
                if active {
                    p.accent
                } else if sresp.hovered() {
                    dark_btn_hov
                } else {
                    dark_btn
                },
            );
            painter.text(
                sr.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::new(11.0, theme::semibold_family()),
                if active { Color32::WHITE } else { light_txt },
            );
            if sresp
                .on_hover_text(format!("Resize call ({label}) — or drag bottom bar"))
                .clicked()
            {
                self.config.call_stage_height = target_h.clamp(min_h, max_h);
                self.config.save();
                self.play_sound(SoundEffect::UiClick);
            }
        }

        // Stream FPS & Resolution pills right inside the Call panel (before & during sharing!)
        let mut new_fps: Option<u32> = None;
        let mut new_res: Option<u32> = None;
        if stage_rect.width() > 520.0 {
            rx -= 8.0;
            for &fps_val in &[60u32, 30u32, 15u32] {
                let w = 42.0;
                rx -= w + 3.0;
                let r = Rect::from_min_size(egui::pos2(rx, top_y), egui::vec2(w, 24.0));
                let resp = ui.interact(r, ui.id().with(("live_fps", fps_val)), Sense::click());
                let active = self.config.screen_fps == fps_val;
                painter.rect_filled(
                    r,
                    6.0,
                    if active {
                        p.accent
                    } else if resp.hovered() {
                        dark_btn_hov
                    } else {
                        dark_btn
                    },
                );
                painter.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    format!("{fps_val}fps"),
                    FontId::new(10.5, theme::semibold_family()),
                    if active { Color32::WHITE } else { light_txt },
                );
                if resp.on_hover_text("Stream frame rate").clicked() {
                    new_fps = Some(fps_val);
                }
            }
            rx -= 5.0;
            for &res_val in &[1080u32, 720u32, 480u32] {
                let w = if res_val == 1080 { 44.0 } else { 38.0 };
                rx -= w + 3.0;
                let r = Rect::from_min_size(egui::pos2(rx, top_y), egui::vec2(w, 24.0));
                let resp = ui.interact(r, ui.id().with(("live_res", res_val)), Sense::click());
                let active = self.config.screen_max_height == res_val;
                painter.rect_filled(
                    r,
                    6.0,
                    if active {
                        p.accent
                    } else if resp.hovered() {
                        dark_btn_hov
                    } else {
                        dark_btn
                    },
                );
                painter.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    format!("{res_val}p"),
                    FontId::new(10.5, theme::semibold_family()),
                    if active { Color32::WHITE } else { light_txt },
                );
                if resp.on_hover_text("Stream resolution (max 1080p)").clicked() {
                    new_res = Some(res_val);
                }
            }
        }
        if let Some(f) = new_fps {
            self.set_screen_fps(f);
            self.play_sound(SoundEffect::UiClick);
        }
        if let Some(h) = new_res {
            self.set_screen_resolution(h);
            self.play_sound(SoundEffect::UiClick);
        }

        // Main stage content: Screen Share viewport OR Participant cards (scales with height!)
        if let Some(texture) = self.screen_texture.clone() {
            let (sw, sh) = self.screen_size;
            if sw > 0 && sh > 0 {
                let pad_top = 56.0;
                let pad_bottom = 68.0;
                let avail_w = (stage_rect.width() - 32.0).max(80.0);
                let avail_h = (stage_rect.height() - pad_top - pad_bottom).max(60.0);
                let aspect = sw as f32 / sh as f32;
                let draw_w = avail_w.min(avail_h * aspect);
                let draw_h = draw_w / aspect;
                let img_rect = Rect::from_center_size(
                    egui::pos2(
                        stage_rect.center().x,
                        stage_rect.top() + pad_top + avail_h * 0.5,
                    ),
                    egui::vec2(draw_w, draw_h),
                );
                painter.rect_filled(img_rect.expand(1.5), 8.0, p.call_tile);
                painter.image(
                    texture.id(),
                    img_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        } else {
            let pad_top = 46.0;
            let pad_bottom = 68.0;
            let mid_y = stage_rect.top() + pad_top + (stage_rect.height() - pad_top - pad_bottom) * 0.5;
            let scale_f = ((stage_rect.height() - 114.0) / 140.0).clamp(0.75, 1.55);

            let mut shown: Vec<(String, Option<String>, i64, bool, bool)> = Vec::new();
            if let Some(me) = &self.me {
                let speaking = self
                    .participants
                    .iter()
                    .any(|pt| pt.user_id == me.user_id && pt.speaking);
                shown.push((
                    me.username.clone(),
                    me.avatar_hash.clone(),
                    me.user_id,
                    speaking,
                    self.muted,
                ));
            }
            for pt in &self.participants {
                if self.me.as_ref().is_some_and(|m| m.user_id == pt.user_id) {
                    continue;
                }
                let hash = self
                    .friends
                    .iter()
                    .find(|f| f.user_id == pt.user_id)
                    .and_then(|f| f.avatar_hash.clone());
                shown.push((pt.username.clone(), hash, pt.user_id, pt.speaking, pt.muted));
            }
            let n = shown.len().max(1);
            let tile_w = (168.0 * scale_f).min((stage_rect.width() - 48.0) / n as f32 - 12.0);
            let tile_h = (104.0 * scale_f).min((stage_rect.height() - pad_top - pad_bottom).max(70.0));
            let gap = 14.0;
            let av_d = (54.0 * scale_f).clamp(38.0, 82.0);

            for (idx, (name, hash, user_id, speaking, is_muted)) in shown.into_iter().enumerate() {
                let x = stage_rect.center().x + (idx as f32 - (n as f32 - 1.0) / 2.0) * (tile_w + gap);
                let tile = Rect::from_center_size(egui::pos2(x, mid_y), egui::vec2(tile_w, tile_h));
                painter.rect_filled(tile, 12.0, p.call_tile);
                if speaking {
                    painter.rect_stroke(
                        tile,
                        12.0,
                        Stroke::new(2.0_f32, p.success),
                        egui::StrokeKind::Inside,
                    );
                }

                let av_c = tile.center() - egui::vec2(0.0, 8.0);
                let mut drew = false;
                if let Some(hash) = hash.as_deref() {
                    let tex = self.avatars.texture(
                        &ui.ctx().clone(),
                        &self.api(),
                        self.token.as_deref(),
                        user_id,
                        Some(hash),
                        &self.events_tx,
                    );
                    if let Some(t) = tex {
                        painter.image(
                            t.id(),
                            Rect::from_center_size(av_c, egui::vec2(av_d, av_d)),
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        drew = true;
                    }
                }
                if !drew {
                    painter.circle_filled(
                        av_c,
                        av_d * 0.5,
                        avatar_color(
                            &name,
                            self.me.as_ref().is_some_and(|m| m.user_id == user_id),
                        ),
                    );
                    let initial = name
                        .chars()
                        .next()
                        .unwrap_or('?')
                        .to_ascii_uppercase()
                        .to_string();
                    painter.text(
                        av_c,
                        Align2::CENTER_CENTER,
                        initial,
                        FontId::new((av_d * 0.38).max(16.0), theme::semibold_family()),
                        Color32::WHITE,
                    );
                }
                painter.text(
                    egui::pos2(tile.left() + 10.0, tile.bottom() - 8.0),
                    Align2::LEFT_BOTTOM,
                    name,
                    FontId::new(12.5, theme::semibold_family()),
                    Color32::from_rgb(0xF3, 0xF5, 0xFA),
                );
                if is_muted {
                    let badge_c = tile.right_bottom() - egui::vec2(16.0, 14.0);
                    painter.circle_filled(badge_c, 10.0, p.call_canvas);
                    icons::paint_mic(&painter, badge_c, 13.0, p.danger, true);
                }
            }
        }

        // Bottom floating circular controls: Share Screen (inside call!), Mute, Deafen, End
        let controls_y = stage_rect.bottom() - 32.0;
        let gap = 12.0;
        let btn_d = 42.0;
        let total_w = 4.0 * btn_d + 3.0 * gap;
        let start_x = stage_rect.center().x - total_w * 0.5 + btn_d * 0.5;

        let mut actions = [false; 4];
        for i in 0..4 {
            let c = egui::pos2(start_x + (i as f32) * (btn_d + gap), controls_y);
            let r = Rect::from_center_size(c, egui::vec2(btn_d, btn_d));
            let resp = ui.interact(r, ui.id().with(("call_ctrl", i)), Sense::click());
            let (bg, fg, tip) = match i {
                0 => (
                    if self.sharing {
                        p.success
                    } else if resp.hovered() {
                        dark_btn_hov
                    } else {
                        dark_btn
                    },
                    Color32::WHITE,
                    if self.sharing {
                        "Stop Sharing Screen"
                    } else {
                        "Share Your Screen"
                    },
                ),
                1 => (
                    if self.muted {
                        Color32::WHITE
                    } else if resp.hovered() {
                        dark_btn_hov
                    } else {
                        dark_btn
                    },
                    if self.muted {
                        p.call_canvas
                    } else {
                        Color32::WHITE
                    },
                    if self.muted { "Unmute" } else { "Mute" },
                ),
                2 => (
                    if self.deafened {
                        Color32::WHITE
                    } else if resp.hovered() {
                        dark_btn_hov
                    } else {
                        dark_btn
                    },
                    if self.deafened {
                        p.call_canvas
                    } else {
                        Color32::WHITE
                    },
                    if self.deafened { "Undeafen" } else { "Deafen" },
                ),
                _ => (p.danger, Color32::WHITE, "End Call"),
            };
            painter.circle_filled(c, btn_d * 0.5, bg);
            match i {
                0 => icons::paint_screen_share(&painter, c, 19.0, fg),
                1 => icons::paint_mic(&painter, c, 19.0, fg, self.muted),
                2 => icons::paint_headphones(&painter, c, 19.0, fg, self.deafened),
                _ => icons::paint_phone_off(&painter, c, 19.0, fg),
            }
            if resp.on_hover_text(tip).clicked() {
                actions[i] = true;
            }
        }
        if actions[0] {
            self.toggle_screen_share();
        }
        if actions[1] {
            self.toggle_mute();
        }
        if actions[2] {
            self.toggle_deafen();
        }
        if actions[3] {
            self.leave_voice();
        }

        // Interactive Drag-to-Resize Handle along the bottom edge
        let grip_zone = Rect::from_min_max(
            egui::pos2(stage_rect.left(), stage_rect.bottom()),
            egui::pos2(stage_rect.right(), outer_rect.bottom()),
        );
        let grip_resp = ui.interact(
            grip_zone,
            ui.id().with("call_stage_resize_handle"),
            Sense::click_and_drag(),
        );
        if grip_resp.hovered() || grip_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let pill = Rect::from_center_size(grip_zone.center(), egui::vec2(48.0, 4.0));
        ui.painter().rect_filled(
            pill,
            2.0,
            if grip_resp.dragged() || grip_resp.hovered() {
                p.accent
            } else {
                p.border
            },
        );
        if grip_resp.dragged() {
            self.config.call_stage_height =
                (self.config.call_stage_height + grip_resp.drag_delta().y).clamp(min_h, max_h);
            self.animating = true;
        }
        if grip_resp.drag_stopped() {
            self.config.save();
        }
    }

    pub(crate) fn ui_screen_fullscreen(&mut self, ctx: &Context) {
        if !self.screen_fullscreen {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.screen_fullscreen = false;
            return;
        }
        let Some(texture) = self.screen_texture.clone() else {
            self.screen_fullscreen = false;
            return;
        };
        let (w, h) = self.screen_size;
        if w == 0 || h == 0 {
            self.screen_fullscreen = false;
            return;
        }
        let mut open = self.screen_fullscreen;
        let mut close_requested = false;
        egui::Window::new("Shared screen")
            .open(&mut open)
            .default_size(ctx.screen_rect().size() - egui::vec2(40.0, 40.0))
            .frame(egui::Frame::new().fill(Color32::BLACK).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::semibold("Shared screen", 14.0).color(Color32::WHITE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exit fullscreen (Esc)").clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.add_space(8.0);
                let available = ui.available_size();
                let aspect = w as f32 / h as f32;
                let width = available.x.min(available.y * aspect);
                let height = width / aspect;
                ui.horizontal(|ui| {
                    ui.add_space(((available.x - width) / 2.0).max(0.0));
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(
                            texture.id(),
                            egui::vec2(width, height),
                        ))
                        .fit_to_exact_size(egui::vec2(width, height)),
                    );
                });
            });
        if close_requested {
            open = false;
        }
        self.screen_fullscreen = open;
    }
}
