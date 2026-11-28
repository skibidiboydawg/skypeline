//! Clean Discord-style authentication screen (#313338 card on #1e1f22 canvas).

use super::{AuthStep, AuthTab, SkyApp};
use crate::events::UiEvent;
use crate::theme;
use eframe::egui;
use egui::{Color32, Context, RichText, Stroke, Ui};
use skypeline_shared::{LoginRequest, RegisterRequest, ResendCodeRequest, VerifyCodeRequest};

impl SkyApp {
    fn auth_field(ui: &mut Ui, value: &mut String, hint: &str, password: bool) -> egui::Response {
        egui::Frame::new()
            .fill(theme::BG_TERTIARY)
            .stroke(Stroke::NONE)
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(
                    egui::TextEdit::singleline(value)
                        .frame(false)
                        .password(password)
                        .hint_text(hint)
                        .desired_width(ui.available_width()),
                )
            })
            .inner
    }

    fn auth_label(ui: &mut Ui, text: &str) {
        ui.label(theme::semibold(text, 11.5).color(theme::TEXT_SECONDARY));
    }

    fn auth_link(ui: &mut Ui, label: &str) -> bool {
        ui.add(
            egui::Button::new(RichText::new(label).color(theme::TEXT_LINK).size(13.0))
                .frame(false),
        )
        .clicked()
    }

    pub(crate) fn ui_login(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_TERTIARY)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                let page_rect = ui.available_rect_before_wrap();
                let card_width = page_rect.width().min(440.0_f32);
                let content_width = (card_width - 64.0_f32).max(200.0_f32);
                let estimated_card_height = 420.0_f32;
                let top_space = (page_rect.height() - estimated_card_height).max(0.0) * 0.45;
                let card_x = page_rect.center().x - card_width * 0.5;
                let content_rect = egui::Rect::from_min_size(
                    egui::pos2(card_x + 32.0, page_rect.min.y + top_space),
                    egui::vec2(content_width, (page_rect.height() - top_space).max(0.0)),
                );

                let fill_shape = ui.painter().add(egui::Shape::Noop);

                let content = ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.add_space(32.0);
                        ui.spacing_mut().item_spacing.y = 8.0;

                        match self.auth_step {
                            AuthStep::Credentials => match self.auth_tab {
                                AuthTab::Login => {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            theme::semibold("Welcome back!", 22.0)
                                                .color(theme::TEXT_PRIMARY),
                                        );
                                        ui.label(
                                            RichText::new("We're so excited to see you again!")
                                                .size(14.0)
                                                .color(theme::TEXT_SECONDARY),
                                        );
                                    });
                                    ui.add_space(16.0);

                                    Self::auth_label(ui, "EMAIL");
                                    Self::auth_field(
                                        ui,
                                        &mut self.login_email,
                                        "you@example.com",
                                        false,
                                    );
                                    ui.add_space(8.0);
                                    Self::auth_label(ui, "PASSWORD");
                                    let pw = Self::auth_field(
                                        ui,
                                        &mut self.login_password,
                                        "",
                                        true,
                                    );
                                    ui.add_space(16.0);

                                    let enter = pw.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let sign_in = egui::Button::new(
                                        theme::semibold("Log In", 14.5).color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(4.0)
                                    .min_size([content_width, 42.0].into());
                                    if (ui.add_enabled(!self.busy, sign_in).clicked() || enter)
                                        && !self.busy
                                    {
                                        self.submit_login();
                                    }
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Need an account?")
                                                .size(13.0)
                                                .color(theme::TEXT_MUTED),
                                        );
                                        if Self::auth_link(ui, "Register") && !self.busy {
                                            self.auth_tab = AuthTab::Register;
                                            self.notice.clear();
                                        }
                                    });
                                }
                                AuthTab::Register => {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            theme::semibold("Create an account", 22.0)
                                                .color(theme::TEXT_PRIMARY),
                                        );
                                    });
                                    ui.add_space(12.0);

                                    Self::auth_label(ui, "EMAIL");
                                    Self::auth_field(
                                        ui,
                                        &mut self.register_email,
                                        "you@example.com",
                                        false,
                                    );
                                    ui.add_space(6.0);
                                    Self::auth_label(ui, "USERNAME");
                                    Self::auth_field(
                                        ui,
                                        &mut self.register_username,
                                        "Username",
                                        false,
                                    );
                                    ui.add_space(6.0);
                                    Self::auth_label(ui, "PASSWORD");
                                    let pw = Self::auth_field(
                                        ui,
                                        &mut self.register_password,
                                        "8+ characters",
                                        true,
                                    );
                                    ui.add_space(16.0);

                                    let enter = pw.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let create = egui::Button::new(
                                        theme::semibold("Continue", 14.5).color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(4.0)
                                    .min_size([content_width, 42.0].into());
                                    if (ui.add_enabled(!self.busy, create).clicked() || enter)
                                        && !self.busy
                                    {
                                        self.submit_register();
                                    }
                                    ui.add_space(8.0);
                                    if Self::auth_link(ui, "Already have an account?")
                                        && !self.busy
                                    {
                                        self.auth_tab = AuthTab::Login;
                                        self.notice.clear();
                                    }
                                }
                            },
                            AuthStep::VerifyCode => {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        theme::semibold("Confirm your email", 22.0)
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new("Enter the 6-digit verification code.")
                                            .size(14.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                });
                                ui.add_space(14.0);
                                Self::auth_label(ui, "EMAIL");
                                Self::auth_field(
                                    ui,
                                    &mut self.verify_email,
                                    "you@example.com",
                                    false,
                                );
                                ui.add_space(6.0);
                                Self::auth_label(ui, "VERIFICATION CODE");
                                let code_field = Self::auth_field(
                                    ui,
                                    &mut self.verify_code,
                                    "6-digit code",
                                    false,
                                );
                                ui.add_space(14.0);
                                let enter = code_field.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let verify = egui::Button::new(
                                    theme::semibold("Verify & Log In", 14.5).color(Color32::WHITE),
                                )
                                .fill(theme::ACCENT)
                                .corner_radius(4.0)
                                .min_size([content_width, 42.0].into());
                                if (ui.add_enabled(!self.busy, verify).clicked() || enter)
                                    && !self.busy
                                {
                                    self.submit_verify();
                                }
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if Self::auth_link(ui, "Resend code") && !self.busy {
                                        self.resend_code();
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if Self::auth_link(ui, "Back to Log In") && !self.busy {
                                                self.auth_step = AuthStep::Credentials;
                                                self.auth_tab = AuthTab::Login;
                                                self.verify_code.clear();
                                                self.notice.clear();
                                            }
                                        },
                                    );
                                });
                            }
                        }

                        if !self.notice.is_empty() {
                            ui.add_space(6.0);
                            self.ui_notice(ui);
                        }
                        ui.add_space(28.0);
                    },
                );

                let card_rect = content.response.rect.expand2(egui::vec2(32.0, 0.0));
                ui.painter().set(
                    fill_shape,
                    egui::Shape::rect_filled(card_rect, 6.0, theme::BG_PRIMARY),
                );
            });
    }

    pub(crate) fn submit_login(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = LoginRequest {
            email: self.login_email.clone(),
            password: self.login_password.clone(),
        };
        self.busy = true;
        self.notice = "Signing in...".into();
        self.rt.spawn(async move {
            match api.login(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::SignedIn(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn submit_register(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = RegisterRequest {
            email: self.register_email.clone(),
            username: self.register_username.clone(),
            password: self.register_password.clone(),
        };
        self.busy = true;
        self.notice = "Creating account...".into();
        self.rt.spawn(async move {
            match api.register(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::RegisterPending(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn submit_verify(&mut self) {
        if self.busy {
            return;
        }
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = VerifyCodeRequest {
            email: self.verify_email.clone(),
            code: self.verify_code.clone(),
        };
        self.busy = true;
        self.notice = "Confirming code...".into();
        self.rt.spawn(async move {
            match api.verify_code(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::SignedIn(response));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }

    fn resend_code(&mut self) {
        let api = self.api();
        let tx = self.events_tx.clone();
        let request = ResendCodeRequest {
            email: self.verify_email.clone(),
        };
        self.busy = true;
        self.notice = "Sending a new code...".into();
        self.rt.spawn(async move {
            match api.resend_code(&request).await {
                Ok(response) => {
                    let _ = tx.send(UiEvent::Status(response.message));
                }
                Err(error) => {
                    let _ = tx.send(UiEvent::Error(error));
                }
            }
        });
    }
}
