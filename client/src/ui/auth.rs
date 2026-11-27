//! Sleek Wireline auth screen with segmented Sign In / Create Account tabs.

use super::{AuthStep, AuthTab, SkyApp};
use crate::events::UiEvent;
use crate::theme;
use eframe::egui;
use egui::{Color32, Context, RichText, Stroke, Ui};
use skypeline_shared::{LoginRequest, RegisterRequest, ResendCodeRequest, VerifyCodeRequest};

impl SkyApp {
    fn auth_field(ui: &mut Ui, value: &mut String, hint: &str, password: bool) -> egui::Response {
        egui::Frame::new()
            .fill(theme::SURFACE_RAIL)
            .stroke(theme::input_border())
            .corner_radius(11.0)
            .inner_margin(egui::Margin::symmetric(14, 11))
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
            egui::Button::new(RichText::new(label).color(theme::CYAN_GLOW).small())
                .frame(false)
                .corner_radius(6.0),
        )
        .clicked()
    }

    pub(crate) fn ui_login(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_RAIL)
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                let page_rect = ui.available_rect_before_wrap();
                let card_width = page_rect.width().min(420.0_f32);
                let content_width = (card_width - 64.0_f32).max(180.0_f32);
                let estimated_card_height = 500.0_f32;
                let top_space = (page_rect.height() - estimated_card_height).max(0.0) * 0.48;
                let card_x = page_rect.center().x - card_width * 0.5;
                let content_rect = egui::Rect::from_min_size(
                    egui::pos2(card_x + 32.0, page_rect.min.y + top_space),
                    egui::vec2(content_width, (page_rect.height() - top_space).max(0.0)),
                );

                let fill_shape = ui.painter().add(egui::Shape::Noop);
                let stroke_shape = ui.painter().add(egui::Shape::Noop);
                let accent_bar = ui.painter().add(egui::Shape::Noop);

                let content = ui.allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    |ui| {
                        ui.add_space(30.0);
                        ui.spacing_mut().item_spacing.y = 8.0;

                        ui.vertical_centered(|ui| {
                            egui::Frame::new()
                                .fill(theme::ACCENT)
                                .corner_radius(12.0)
                                .inner_margin(egui::Margin::symmetric(14, 6))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("WIRELINE")
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    );
                                });
                        });
                        ui.add_space(10.0);

                        if self.auth_step == AuthStep::Credentials {
                            egui::Frame::new()
                                .fill(theme::SURFACE_RAIL)
                                .stroke(theme::border())
                                .corner_radius(12.0)
                                .inner_margin(egui::Margin::same(4))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let half = ((content_width - 14.0) * 0.5).max(80.0);
                                        for (tab, label) in [
                                            (AuthTab::Login, "Sign In"),
                                            (AuthTab::Register, "Create Account"),
                                        ] {
                                            let active = self.auth_tab == tab;
                                            if ui
                                                .add_enabled(
                                                    !self.busy,
                                                    egui::Button::new(
                                                        RichText::new(label).strong().color(
                                                            if active {
                                                                Color32::WHITE
                                                            } else {
                                                                theme::SECONDARY_TEXT
                                                            },
                                                        ),
                                                    )
                                                    .fill(if active {
                                                        theme::ACCENT
                                                    } else {
                                                        Color32::TRANSPARENT
                                                    })
                                                    .stroke(Stroke::NONE)
                                                    .corner_radius(9.0)
                                                    .min_size([half, 34.0].into()),
                                                )
                                                .clicked()
                                            {
                                                self.auth_tab = tab;
                                            }
                                        }
                                    });
                                });
                            ui.add_space(12.0);
                        }

                        match self.auth_step {
                            AuthStep::Credentials => match self.auth_tab {
                                AuthTab::Login => {
                                    ui.label(RichText::new("Welcome back").size(24.0).strong());
                                    ui.label(theme::muted("Sign in to your Wireline workspace."));
                                    ui.add_space(10.0);

                                    ui.label(theme::muted("Email").small());
                                    Self::auth_field(
                                        ui,
                                        &mut self.login_email,
                                        "you@example.com",
                                        false,
                                    );
                                    ui.add_space(6.0);
                                    ui.label(theme::muted("Password").small());
                                    let pw = Self::auth_field(
                                        ui,
                                        &mut self.login_password,
                                        "Password",
                                        true,
                                    );
                                    ui.add_space(14.0);

                                    let enter = pw.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let sign_in = egui::Button::new(
                                        RichText::new("Sign in").strong().color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(11.0)
                                    .min_size([content_width, 42.0].into());
                                    if (ui.add_enabled(!self.busy, sign_in).clicked() || enter)
                                        && !self.busy
                                    {
                                        self.submit_login();
                                    }
                                    ui.add_space(8.0);
                                    ui.vertical_centered(|ui| {
                                        if Self::auth_link(ui, "Need to verify your email / resend code?")
                                            && !self.busy
                                        {
                                            if !self.login_email.trim().is_empty() {
                                                self.verify_email =
                                                    self.login_email.trim().to_string();
                                            }
                                            self.auth_step = AuthStep::VerifyCode;
                                            self.notice.clear();
                                        }
                                    });
                                }
                                AuthTab::Register => {
                                    ui.label(
                                        RichText::new("Create your account").size(24.0).strong(),
                                    );
                                    ui.label(theme::muted(
                                        "We'll email a 6-digit confirmation code.",
                                    ));
                                    ui.add_space(8.0);

                                    ui.label(theme::muted("Email").small());
                                    Self::auth_field(
                                        ui,
                                        &mut self.register_email,
                                        "you@example.com",
                                        false,
                                    );
                                    ui.add_space(4.0);
                                    ui.label(theme::muted("Username").small());
                                    Self::auth_field(
                                        ui,
                                        &mut self.register_username,
                                        "Display name",
                                        false,
                                    );
                                    ui.add_space(4.0);
                                    ui.label(theme::muted("Password (8+ characters)").small());
                                    let pw = Self::auth_field(
                                        ui,
                                        &mut self.register_password,
                                        "Password",
                                        true,
                                    );
                                    ui.add_space(12.0);

                                    let enter = pw.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let create = egui::Button::new(
                                        RichText::new("Create account")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(theme::ACCENT)
                                    .corner_radius(11.0)
                                    .min_size([content_width, 42.0].into());
                                    if (ui.add_enabled(!self.busy, create).clicked() || enter)
                                        && !self.busy
                                    {
                                        self.submit_register();
                                    }
                                }
                            },
                            AuthStep::VerifyCode => {
                                ui.label(RichText::new("Confirm your email").size(24.0).strong());
                                ui.label(theme::muted(
                                    "Enter your email and the 6-digit code we sent you.",
                                ));
                                ui.add_space(10.0);
                                ui.label(theme::muted("Email").small());
                                Self::auth_field(
                                    ui,
                                    &mut self.verify_email,
                                    "you@example.com",
                                    false,
                                );
                                ui.add_space(6.0);
                                ui.label(theme::muted("6-digit code").small());
                                let code_field = Self::auth_field(
                                    ui,
                                    &mut self.verify_code,
                                    "6-digit code",
                                    false,
                                );
                                ui.add_space(12.0);
                                let enter = code_field.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let verify = egui::Button::new(
                                    RichText::new("Verify & sign in")
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(theme::ACCENT)
                                .corner_radius(11.0)
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
                                            if Self::auth_link(ui, "Back to sign in") && !self.busy
                                            {
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
                            ui.add_space(8.0);
                            self.ui_notice(ui);
                        }
                        ui.add_space(28.0);
                    },
                );

                let card_rect = content
                    .response
                    .rect
                    .expand2(egui::vec2(32.0, 0.0))
                    .expand(1.0);
                ui.painter().set(
                    fill_shape,
                    egui::Shape::rect_filled(card_rect, 18.0, theme::SURFACE_CARD),
                );
                ui.painter().set(
                    stroke_shape,
                    egui::Shape::rect_stroke(
                        card_rect,
                        18.0,
                        theme::border(),
                        egui::StrokeKind::Outside,
                    ),
                );
                let top_bar = egui::Rect::from_min_max(
                    card_rect.left_top() + egui::vec2(18.0, 0.0),
                    egui::pos2(card_rect.right() - 18.0, card_rect.top() + 3.0),
                );
                ui.painter().set(
                    accent_bar,
                    egui::Shape::rect_filled(top_bar, 2.0, theme::ACCENT),
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
