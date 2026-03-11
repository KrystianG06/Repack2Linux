use iced::widget::{button, column, container, row, stack, text, Space};
use iced::{font, Alignment, Background, Border, Color, Element, Length, Theme, Vector, Shadow};
use crate::app::{RepackApp, Tab, Message, ExportStatus};
use crate::ui;
use crate::ui::theme::{root_background, TEXT_DIM};

const APP_VERSION: &str = "1.3.0";

impl RepackApp {
    pub fn subscription(&self) -> iced::Subscription<Message> {
        if matches!(self.export_status, ExportStatus::Running(_))
            || (self.show_welcome_overlay && self.cfg.welcome_animation_enabled)
        {
            iced::time::every(std::time::Duration::from_millis(300)).map(|_| Message::Tick)
        } else {
            iced::Subscription::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = ui::theme::sidebar_container(
            column![
                text("R2L").size(38).font(font_bold()).color(ui::theme::ACCENT_BLUE),
                text(self.tr("sidebar_tagline"))
                    .size(10)
                    .color(TEXT_DIM)
                    .font(font_bold()),
                Space::with_height(60),
                sidebar_btn(
                    format!("  {}", self.tr("factory")),
                    Tab::Factory,
                    self.current_tab == Tab::Factory
                ),
                sidebar_btn(
                    format!("  {}", self.tr("tools")),
                    Tab::Tools,
                    self.current_tab == Tab::Tools
                ),
                sidebar_btn(
                    format!("  {}", self.tr("settings")),
                    Tab::Settings,
                    self.current_tab == Tab::Settings
                ),
                Space::with_height(Length::Fill),
                container(column![
                    row![
                        container(Space::with_width(8))
                            .style(|_: &Theme| container::Style {
                                background: Some(Background::Color(Color::from_rgb(0.0, 1.0, 0.4))),
                                border: Border {
                                    radius: 4.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .width(8)
                            .height(8),
                        Space::with_width(8),
                        text(self.tr("cloud_database"))
                            .size(10)
                            .font(font_bold())
                            .color(ui::theme::ACCENT_BLUE),
                    ]
                    .align_y(Alignment::Center),
                    text(format!(
                        "{} {}",
                        self.preset_count,
                        self.tr("presets_loaded")
                    ))
                    .size(9)
                    .color(TEXT_DIM),
                ])
                .padding(10),
                container(text(format!("v{}", APP_VERSION)).size(9).color(TEXT_DIM)).padding(10)
            ]
            .spacing(16)
            .align_x(Alignment::Start),
        )
        .width(220)
        .height(Length::Fill);

        let tab_content: Element<'_, Message> = match self.current_tab {
            Tab::Factory => ui::factory::view_factory(self),
            Tab::Tools => ui::tools::view_tools(self),
            Tab::Settings => ui::settings::view_settings(self),
        };

        let content_with_update: Element<'_, Message> =
            if let Some(version) = &self.available_update {
                let update_row = container(
                    row![
                        text(format!(
                            "{} v{} - {}",
                            self.tr("update_available_prefix"),
                            version,
                            self.tr("update_available_suffix")
                        ))
                        .size(12)
                        .font(font_bold())
                        .color(Color::from_rgb(0.22, 0.17, 0.02)),
                        Space::with_width(Length::Fill),
                        button(text(self.tr("update_download_btn")).size(11))
                            .on_press(Message::OpenReleasesPage)
                            .style(|t, s| ui::theme::primary_button_style(t, s)),
                        button(text(self.tr("update_hide_btn")).size(11))
                            .on_press(Message::DismissUpdateBanner)
                            .style(|t, s| ui::theme::secondary_button_style(t, s)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(12)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.99, 0.85, 0.28, 0.9))),
                    border: Border {
                        radius: 10.0.into(),
                        width: 1.0,
                        color: Color::from_rgb(0.85, 0.66, 0.1),
                    },
                    ..Default::default()
                });

                column![update_row, Space::with_height(10), tab_content].into()
            } else {
                tab_content
            };

        let main_content = container(content_with_update)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(32);

        let content = row![sidebar, main_content].spacing(0);

        let mut root: Element<'_, Message> = root_background(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                }),
        )
        .into();

        if self.show_export_modal {
            root = stack![
                root,
                button(
                    container(Space::with_width(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(
                                0.0, 0.0, 0.0, 0.7
                            ))),
                            ..Default::default()
                        }),
                )
                .on_press(Message::ModalBackdropClicked)
                .style(|_, _| button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                }),
                container(ui::factory::view_export_dialog(self))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            ]
            .into();
        }

        if self.show_welcome_overlay {
            root = stack![
                root,
                button(
                    container(Space::with_width(Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(
                                0.0, 0.0, 0.0, 0.78
                            ))),
                            ..Default::default()
                        }),
                )
                .on_press(Message::ModalBackdropClicked)
                .style(|_, _| button::Style {
                    background: Some(Background::Color(Color::TRANSPARENT)),
                    ..Default::default()
                }),
                container(view_welcome_overlay(self))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            ]
            .into();
        }

        root
    }
}

pub fn view_welcome_overlay(app: &RepackApp) -> Element<'_, Message> {
    let glow = if app.cfg.welcome_animation_enabled {
        (app.animation_tick % 4) as f32 * 0.04
    } else {
        0.0
    };
    let accent = Color::from_rgb(0.36 + glow, 0.72, 0.98);

    container(
        column![
            text(app.tr("welcome_title"))
                .size(30)
                .font(font_bold())
                .color(accent),
            text(app.tr("welcome_subtitle")).size(13).color(TEXT_DIM),
            Space::with_height(16),
            text(app.tr("welcome_points")).size(12).color(Color::WHITE),
            Space::with_height(18),
            button(
                container(text(app.tr("welcome_start")).size(14).font(font_bold()))
                    .padding(14)
                    .center_x(Length::Fill),
            )
            .on_press(Message::DismissWelcomePressed)
            .style(|t, s| ui::theme::brand_button_style(t, s, false))
            .width(Length::Fill),
        ]
        .spacing(8),
    )
    .padding(28)
    .width(560)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.08, 0.09, 0.13, 0.96))),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.36, 0.72, 0.98, 0.45),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    })
    .into()
}

pub fn sidebar_btn(
    label: String,
    tab: Tab,
    active: bool,
) -> Element<'static, Message> {
    let indicator: Element<'static, Message> = if active {
        container(Space::with_width(4))
            .height(20)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(ui::theme::ACCENT_BLUE)),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::with_width(0).into()
    };

    button(
        container(
            row![indicator, text(label).size(13).font(font_bold())]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding(16)
        .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .on_press(Message::TabChanged(tab))
    .style(move |_t, s| {
        let mut style = button::Style::default();
        match s {
            button::Status::Hovered => {
                style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05)));
                style.text_color = Color::WHITE;
            }
            _ => {
                style.text_color = if active { Color::WHITE } else { TEXT_DIM };
            }
        }
        style
    })
    .into()
}

pub fn font_bold() -> font::Font {
    font::Font {
        weight: font::Weight::Bold,
        ..font::Font::DEFAULT
    }
}
