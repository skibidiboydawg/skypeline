//! Skype-style voice & screen-share call stage and fullscreen screen viewer.

use super::{avatar_color, SkyApp};
use crate::theme;
use eframe::egui;
use egui::{Align2, Color32, Context, FontId, Rect, RichText, Sense, Stroke, Ui};
use skypeline_shared::{ChannelInfo, FriendshipStatus};

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
                "In call ({} people) • {:02}:{:02}",
                self.participants.len(),
                elapsed / 60,
                elapsed % 60
            )
        };

        let has_screen =
            self.screen_texture.is_some() && self.screen_size.0 > 0 && self.screen_size.1 > 0;
        let rect = ui.available_rect_before_wrap();
        let max_h: f32 = if has_screen { 360.0 } else { 250.0 };
        let height = max_h.min(rect.height() * 0.52).max(180.0);
        let (stage_rect, _) =
            ui.allocate_exact_size(egui::vec2(rect.width(), height), Sense::hover());
        let painter = ui.painter_at(stage_rect);

        let top = Color32::from_rgb(0x02, 0x84, 0xC7);
        let bottom = Color32::from_rgb(0x08, 0x2F, 0x49);
        painter.rect_filled(stage_rect, 0.0, bottom);
        let mut mesh = egui::epaint::Mesh::default();
        let i0 = mesh.vertices.len() as u32;
        mesh.colored_vertex(stage_rect.left_top(), top);
        mesh.colored_vertex(stage_rect.right_top(), top);
        mesh.colored_vertex(stage_rect.right_bottom(), bottom);
        mesh.colored_vertex(stage_rect.left_bottom(), bottom);
        mesh.add_triangle(i0, i0 + 1, i0 + 2);
        mesh.add_triangle(i0, i0 + 2, i0 + 3);
        painter.add(egui::Shape::mesh(mesh));

        painter.text(
            stage_rect.left_top() + egui::vec2(20.0, 14.0),
            Align2::LEFT_TOP,
            status,
            FontId::proportional(15.0),
            Color32::WHITE,
        );

        if let Some(texture) = self.screen_texture.clone() {
            let (sw, sh) = self.screen_size;
            if sw > 0 && sh > 0 {
                let pad_top = 42.0;
                let pad_bottom = 78.0;
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
                painter.rect_filled(img_rect.expand(2.0), 8.0, Color32::BLACK);
                painter.image(
                    texture.id(),
                    img_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let fs_rect = Rect::from_min_size(
                    stage_rect.right_top() + egui::vec2(-116.0, 10.0),
                    egui::vec2(96.0, 26.0),
                );
                let fs_resp = ui.interact(fs_rect, ui.id().with("fs_btn"), Sense::click());
                painter.rect_filled(
                    fs_rect,
                    8.0,
                    Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                );
                painter.text(
                    fs_rect.center(),
                    Align2::CENTER_CENTER,
                    "Fullscreen",
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
                if fs_resp.clicked() {
                    self.screen_fullscreen = true;
                }
            }
        } else {
            let center = stage_rect.center() - egui::vec2(0.0, 10.0);
            let mut shown: Vec<(String, Option<String>, i64, bool)> = Vec::new();
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
                shown.push((p.username.clone(), hash, p.user_id, p.speaking));
            }
            let n = shown.len().max(1);
            for (idx, (name, hash, user_id, speaking)) in shown.into_iter().enumerate() {
                let x = center.x + (idx as f32 - (n as f32 - 1.0) / 2.0) * 120.0;
                let c = egui::pos2(x, center.y);
                let ring = if speaking {
                    Color32::from_rgb(0xBA, 0xE6, 0xFD)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 70)
                };
                painter.circle_stroke(
                    c,
                    44.0,
                    Stroke::new(if speaking { 4.0_f32 } else { 2.0_f32 }, ring),
                );
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
                            Rect::from_center_size(c, egui::vec2(78.0, 78.0)),
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        drew = true;
                    }
                }
                if !drew {
                    painter.circle_filled(
                        c,
                        39.0,
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
                        c,
                        Align2::CENTER_CENTER,
                        initial,
                        FontId::proportional(26.0),
                        Color32::WHITE,
                    );
                }
                painter.text(
                    c + egui::vec2(0.0, 56.0),
                    Align2::CENTER_TOP,
                    name,
                    FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        }

        let controls_y = stage_rect.bottom() - 36.0;
        let buttons = [
            ("Share", if self.sharing { "Stop" } else { "Screen" }),
            ("Mute", if self.muted { "Unmute" } else { "Mute" }),
            ("Deafen", if self.deafened { "Undeaf" } else { "Deafen" }),
            ("Add", "+Add"),
            ("End", "End"),
        ];
        let mut actions = [false; 5];
        let btn_w = 72.0;
        let gap = 10.0;
        let total_w = buttons.len() as f32 * btn_w + (buttons.len() - 1) as f32 * gap;
        let mut x = stage_rect.center().x - total_w / 2.0 + btn_w * 0.5;
        for (i, (key, text)) in buttons.iter().enumerate() {
            let c = egui::pos2(x, controls_y);
            let r = Rect::from_center_size(c, egui::vec2(btn_w, 34.0));
            let resp = ui.interact(r, ui.id().with(("call_btn", i)), Sense::click());
            let fill = if *key == "End" {
                theme::DANGER
            } else if (*key == "Mute" && self.muted)
                || (*key == "Deafen" && self.deafened)
                || (*key == "Share" && self.sharing)
            {
                theme::ACCENT
            } else if resp.hovered() {
                Color32::from_rgba_unmultiplied(255, 255, 255, 60)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 35)
            };
            painter.rect_filled(r, 17.0, fill);
            painter.text(
                c,
                Align2::CENTER_CENTER,
                *text,
                FontId::proportional(13.0),
                Color32::WHITE,
            );
            if resp.clicked() {
                actions[i] = true;
            }
            x += btn_w + gap;
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
            self.show_add_people = !self.show_add_people;
        }
        if actions[4] {
            self.leave_voice();
        }

        if self.show_add_people && channel.kind == "group" {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(theme::SURFACE_CARD)
                .stroke(theme::border())
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.label(RichText::new("Add people to group").strong());
                    Self::form_field(ui, &mut self.add_member_query, "Search friend name", false);
                    let q = self.add_member_query.to_ascii_lowercase();
                    for friend in self
                        .friends
                        .clone()
                        .into_iter()
                        .filter(|f| f.status == FriendshipStatus::Accepted)
                    {
                        if !q.is_empty() && !friend.username.to_ascii_lowercase().contains(&q) {
                            continue;
                        }
                        if channel.members.iter().any(|m| m.user_id == friend.user_id) {
                            continue;
                        }
                        if ui.button(format!("Add {}", friend.username)).clicked() {
                            self.add_group_member(channel.id, friend.user_id);
                        }
                    }
                });
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
                    ui.label(RichText::new("Shared screen").strong());
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
