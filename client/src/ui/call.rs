//! Clean Discord Voice & Screen Share Stage (#0e0f12):
//! Live screen-share viewport (local preview + remote streams) with LIVE badge & Fullscreen,
//! centered participant cards, and crisp Phosphor circular call controls.

use super::{avatar_color, icons, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, Sense, Stroke, Ui};
use skypeline_shared::ChannelInfo;

impl SkyApp {
    pub(crate) fn ui_call_stage(&mut self, ui: &mut Ui, channel: &ChannelInfo) {
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
        let rect = ui.available_rect_before_wrap();
        let max_h: f32 = if has_screen { 390.0 } else { 244.0 };
        let height = max_h.min(rect.height() * 0.56).max(196.0);
        let (stage_rect, _) =
            ui.allocate_exact_size(egui::vec2(rect.width(), height), Sense::hover());
        let painter = ui.painter_at(stage_rect);

        // Clean Discord dark call canvas
        painter.rect_filled(stage_rect, 0.0, Color32::from_rgb(0x0E, 0x0F, 0x12));
        painter.line_segment(
            [stage_rect.left_bottom(), stage_rect.right_bottom()],
            theme::hairline(),
        );

        // Top-left status + LIVE pill when screen sharing
        painter.text(
            stage_rect.left_top() + egui::vec2(18.0, 14.0),
            Align2::LEFT_TOP,
            &status,
            FontId::new(13.5, theme::semibold_family()),
            theme::TEXT_PRIMARY,
        );
        if self.sharing || has_screen {
            let live_r = Rect::from_min_size(
                stage_rect.left_top() + egui::vec2(18.0, 35.0),
                egui::vec2(42.0, 18.0),
            );
            painter.rect_filled(live_r, 9.0, theme::DANGER);
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
                "You are sharing your screen"
            } else {
                "Screen Share Active"
            };
            painter.text(
                live_r.right_center() + egui::vec2(8.0, 0.0),
                Align2::LEFT_CENTER,
                owner_label,
                FontId::proportional(11.5),
                theme::TEXT_SECONDARY,
            );
        }

        // Top-right Minimize button
        let col_r = Rect::from_min_size(
            stage_rect.right_top() + egui::vec2(-84.0, 12.0),
            egui::vec2(68.0, 26.0),
        );
        let col_resp = ui.interact(col_r, ui.id().with("call_collapse"), Sense::click());
        painter.rect_filled(
            col_r,
            4.0,
            if col_resp.hovered() {
                theme::BG_ACTIVE
            } else {
                theme::BG_SECONDARY
            },
        );
        painter.text(
            col_r.center(),
            Align2::CENTER_CENTER,
            "Hide",
            FontId::new(12.0, theme::semibold_family()),
            theme::TEXT_SECONDARY,
        );
        if col_resp.clicked() {
            self.call_expanded = false;
        }

        if let Some(texture) = self.screen_texture.clone() {
            let (sw, sh) = self.screen_size;
            if sw > 0 && sh > 0 {
                let pad_top = 56.0;
                let pad_bottom = 72.0;
                let avail_w = (stage_rect.width() - 40.0).max(80.0);
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
                painter.rect_filled(img_rect.expand(1.5), 8.0, Color32::from_rgb(0x1E, 0x1F, 0x22));
                painter.image(
                    texture.id(),
                    img_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let fs_rect = Rect::from_min_size(
                    stage_rect.right_top() + egui::vec2(-180.0, 12.0),
                    egui::vec2(88.0, 26.0),
                );
                let fs_resp = ui.interact(fs_rect, ui.id().with("fs_btn"), Sense::click());
                painter.rect_filled(
                    fs_rect,
                    4.0,
                    if fs_resp.hovered() {
                        theme::ACCENT
                    } else {
                        theme::BG_SECONDARY
                    },
                );
                icons::paint_fullscreen(
                    &painter,
                    fs_rect.left_center() + egui::vec2(14.0, 0.0),
                    13.0,
                    Color32::WHITE,
                );
                painter.text(
                    fs_rect.left_center() + egui::vec2(26.0, 0.0),
                    Align2::LEFT_CENTER,
                    "Expand",
                    FontId::new(12.0, theme::semibold_family()),
                    Color32::WHITE,
                );
                if fs_resp.clicked() {
                    self.screen_fullscreen = true;
                }
            }
        } else {
            // Participant tile grid (#1e1f22 tiles, 8px radius, 60px avatar, green speaking ring)
            let center = stage_rect.center() - egui::vec2(0.0, 12.0);
            let mut shown: Vec<(String, Option<String>, i64, bool, bool)> = Vec::new();
            if let Some(me) = &self.me {
                let speaking = self
                    .participants
                    .iter()
                    .any(|p| p.user_id == me.user_id && p.speaking);
                shown.push((
                    me.username.clone(),
                    me.avatar_hash.clone(),
                    me.user_id,
                    speaking,
                    self.muted,
                ));
            }
            for p in &self.participants {
                if self.me.as_ref().is_some_and(|m| m.user_id == p.user_id) {
                    continue;
                }
                let hash = self
                    .friends
                    .iter()
                    .find(|f| f.user_id == p.user_id)
                    .and_then(|f| f.avatar_hash.clone());
                shown.push((p.username.clone(), hash, p.user_id, p.speaking, p.muted));
            }
            let n = shown.len().max(1);
            let tile_w = 176.0;
            let tile_h = 112.0;
            let gap = 14.0;
            for (idx, (name, hash, user_id, speaking, is_muted)) in shown.into_iter().enumerate() {
                let x = center.x + (idx as f32 - (n as f32 - 1.0) / 2.0) * (tile_w + gap);
                let tile =
                    Rect::from_center_size(egui::pos2(x, center.y), egui::vec2(tile_w, tile_h));
                painter.rect_filled(tile, 8.0, theme::BG_TERTIARY);
                if speaking {
                    painter.rect_stroke(
                        tile,
                        8.0,
                        Stroke::new(2.0_f32, theme::SUCCESS),
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
                            Rect::from_center_size(av_c, egui::vec2(58.0, 58.0)),
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        drew = true;
                    }
                }
                if !drew {
                    painter.circle_filled(
                        av_c,
                        29.0,
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
                        FontId::new(22.0, theme::semibold_family()),
                        Color32::WHITE,
                    );
                }
                painter.text(
                    egui::pos2(tile.left() + 10.0, tile.bottom() - 8.0),
                    Align2::LEFT_BOTTOM,
                    name,
                    FontId::new(12.5, theme::semibold_family()),
                    theme::TEXT_PRIMARY,
                );
                if is_muted {
                    let badge_c = tile.right_bottom() - egui::vec2(16.0, 14.0);
                    painter.circle_filled(badge_c, 10.0, theme::BG_FLOATING);
                    icons::paint_mic(&painter, badge_c, 13.0, theme::DANGER, true);
                }
            }
        }

        // Bottom circular Discord call controls (44px circles: Share Screen, Mute, Deafen, Disconnect)
        let controls_y = stage_rect.bottom() - 34.0;
        let gap = 14.0;
        let btn_d = 44.0;
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
                        theme::SUCCESS
                    } else if resp.hovered() {
                        theme::BG_ACTIVE
                    } else {
                        theme::BG_SECONDARY
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
                        theme::BG_ACTIVE
                    } else {
                        theme::BG_SECONDARY
                    },
                    if self.muted {
                        theme::BG_FLOATING
                    } else {
                        Color32::WHITE
                    },
                    if self.muted { "Unmute" } else { "Mute" },
                ),
                2 => (
                    if self.deafened {
                        Color32::WHITE
                    } else if resp.hovered() {
                        theme::BG_ACTIVE
                    } else {
                        theme::BG_SECONDARY
                    },
                    if self.deafened {
                        theme::BG_FLOATING
                    } else {
                        Color32::WHITE
                    },
                    if self.deafened { "Undeafen" } else { "Deafen" },
                ),
                _ => (theme::DANGER, Color32::WHITE, "Disconnect"),
            };
            painter.circle_filled(c, btn_d * 0.5, bg);
            match i {
                0 => icons::paint_screen_share(&painter, c, 20.0, fg),
                1 => icons::paint_mic(&painter, c, 20.0, fg, self.muted),
                2 => icons::paint_headphones(&painter, c, 20.0, fg, self.deafened),
                _ => icons::paint_phone_off(&painter, c, 20.0, fg),
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
                    ui.label(theme::semibold("Shared screen", 14.0).color(theme::TEXT_PRIMARY));
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
