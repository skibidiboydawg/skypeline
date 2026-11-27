//! Authentication UI (sign-in, registration, and 6-digit email verification).

use super::{AuthStep, AuthTab, SkyApp};
use crate::events::UiEvent;
use crate::theme;
use eframe::egui;
use egui::{Context, RichText, Ui};
use skypeline_shared::{LoginRequest, RegisterRequest, ResendCodeRequest, VerifyCodeRequest};

impl SkyApp {
    fn auth_field(ui: &mut Ui, value: &mut String, hint: &str, password: bool) -> egui::Response {
        egui::Frame::new()
            .fill(theme::SURFACE_SIDEBAR)
            .stroke(theme::input_border())
            .corner_radius(10.0)
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

    fn auth_link(ui: &mut Ui, label: &str) -> bool {
        ui.add(
            egui::Button::new(RichText::new(label).color(theme::ACCENT).small())
                .frame(false)
                .corner_radius(6.0),
        )
        .clicked()
    }

    pub(crate) fn ui_login(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_SIDEBAR)
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                let page_rect = ui.available_rect_before_wrap();
                let card_width = page_rect.width().min(400.0_f32);
                let content_width = (card_width - 64.0_f32).max(160.0_f32);
                let estimated_card_height = 480.0_f32;
                let top_space = (page_rect.height() - estimated_card_height).max(0.0) * 0.5;
                let card_x = page_rect.center().x - card_width * 0.5;
                let content_rect = egui::Rect::from_min_size(
                    egui::pos2(card_x + 32.0, page_rect.min.y + top_space),
                    egui::vec2(content_width, (page_rect.height() - top_space).max(0.0)),
                );

                let fill_shape = ui.painter().add(egui::Shape::Noop);
                let stroke_shape = ui.painter().add(egui::Shape::Noop);
                let content = ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.add_space(36.0);
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.vertical(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("WIRELINE")
                                        .size(13.0)
                                        .strong()
                                        .color(theme::ACCENT),
                                );
                            });
                            ui.add_space(12.0);

                            match self.auth_step {
                                AuthStep::Credentials => match self.auth_tab {
                                    AuthTab::Login => {
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new("Welcome back").size(26.0).strong(),
                                            );
                                        });
                                        ui.add_space(4.0);
                                        ui.vertical_centered(|ui| {
                                            ui.label(theme::muted(
                                                "Sign in to continue to your workspace.",
                                            ));
                                        });
                                        ui.add_space(16.0);

                                        ui.label(theme::muted("Email"));
                                        Self::auth_field(
                                            ui,
                                            &mut self.login_email,
                                            "you@example.com",
                                            false,
                                        );
                                        ui.add_space(12.0);
                                        ui.label(theme::muted("Password"));
                                        let pw = Self::auth_field(
                                            ui,
                                            &mut self.login_password,
                                            "Password",
                                            true,
                                        );
                                        ui.add_space(16.0);

                                        let enter = pw.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        let sign_in = egui::Button::new(
                                            RichText::new("Sign in")
                                                .strong()
                                                .color(theme::PRIMARY_TEXT),
                                        )
                                        .fill(theme::ACCENT)
                                        .corner_radius(9.0)
                                        .min_size([content_width, 40.0].into());
                                        ui.vertical_centered(|ui| {
                                            if (ui.add_enabled(!self.busy, sign_in).clicked()
                                                || enter)
                                                && !self.busy
                                            {
                                                self.submit_login();
                                            }
                                        });
                                        ui.add_space(10.0);
                                        ui.vertical_centered(|ui| {
                                            if Self::auth_link(ui, "New here? Create account →")
                                                && !self.busy
                                            {
                                                self.auth_tab = AuthTab::Register;
                                            }
                                        });
                                    }
                                    AuthTab::Register => {
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                RichText::new("Create your account")
                                                    .size(26.0)
                                                    .strong(),
                                            );
                                        });
                                        ui.add_space(4.0);
                                        ui.vertical_centered(|ui| {
                                            ui.label(theme::muted(
                                                "We'll email a confirmation code before sign-in.",
                                            ));
                                        });
                                        ui.add_space(16.0);

                                        ui.label(theme::muted("Email"));
                                        Self::auth_field(
                                            ui,
                                            &mut self.register_email,
                                            "you@example.com",
                                            false,
                                        );
                                        ui.add_space(10.0);
                                        ui.label(theme::muted("Username"));
                                        Self::auth_field(
                                            ui,
                                            &mut self.register_username,
                                            "Display name",
                                            false,
                                        );
                                        ui.add_space(10.0);
                                        ui.label(theme::muted("Password (8+ characters)"));
                                        Self::auth_field(
                                            ui,
                                            &mut self.register_password,
                                            "Password",
                                            true,
                                        );
                                        ui.add_space(16.0);

                                        let create = egui::Button::new(
                                            RichText::new("Create account")
                                                .strong()
                                                .color(theme::PRIMARY_TEXT),
                                        )
                                        .fill(theme::ACCENT)
                                        .corner_radius(9.0)
                                        .min_size([content_width, 40.0].into());
                                        ui.vertical_centered(|ui| {
                                            if ui.add_enabled(!self.busy, create).clicked() {
                                                self.submit_register();
                                            }
                                        });
                                        ui.add_space(10.0);
                                        ui.vertical_centered(|ui| {
                                            if Self::auth_link(
                                                ui,
                                                "Already have an account? Sign in",
                                            ) && !self.busy
                                            {
                                                self.auth_tab = AuthTab::Login;
                                            }
                                        });
                                    }
                                },
                                AuthStep::VerifyCode => {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new("Confirm your email").size(24.0).strong(),
                                        );
                                    });
                                    ui.add_space(4.0);
                                    ui.label(theme::muted(format!(
                                        "Enter the confirmation code sent to {}",
                                        self.verify_email
                                    )));
                                    ui.add_space(12.0);
                                    Self::auth_field(
                                        ui,
                                        &mut self.verify_code,
                                        "6-digit code",
                                        false,
                                    );
                                    ui.add_space(16.0);
                                    let verify = egui::Button::new(
                                        RichText::new("Verify & sign in")
                                            .strong()
                                            .color(theme::PRIMARY_TEXT),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(9.0)
                                    .min_size([content_width, 40.0].into());
                                    ui.vertical_centered(|ui| {
                                        if ui.add_enabled(!self.busy, verify).clicked() {
                                            self.submit_verify();
                                        }
                                    });
                                    ui.add_space(10.0);
                                    ui.horizontal(|ui| {
                                        if Self::auth_link(ui, "Resend code") && !self.busy {
                                            self.resend_code();
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if Self::auth_link(ui, "Back to sign in")
                                                    && !self.busy
                                                {
                                                    self.auth_step = AuthStep::Credentials;
                                                    self.auth_tab = AuthTab::Login;
                                                    self.verify_code.clear();
                                                    self.notice =
                                                        "Sign in with your confirmed account."
                                                            .into();
                                                }
                                            },
                                        );
                                    });
                                }
                            }

                            if !self.notice.is_empty() {
                                ui.add_space(10.0);
                                self.ui_notice(ui);
                            }
                        });
                        ui.add_space(36.0);
                    },
                );
                let card_rect = content
                    .response
                    .rect
                    .expand2(egui::vec2(32.0, 0.0))
                    .expand(1.0);
                ui.painter().set(
                    fill_shape,
                    egui::Shape::rect_filled(card_rect, 16.0, theme::SURFACE_CARD),
                );
                ui.painter().set(
                    stroke_shape,
                    egui::Shape::rect_stroke(
                        card_rect,
                        16.0,
                        theme::border(),
                        egui::StrokeKind::Outside,
                    ),
                );
            });
    }

    fn submit_login(&mut self) {
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
