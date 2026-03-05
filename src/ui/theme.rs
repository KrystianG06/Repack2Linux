use iced::gradient::Linear;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Gradient, Shadow, Theme, Vector};

// --- CYBER-GLASS 2.0 COLORS ---
pub const DEEP_DARK: Color = Color::from_rgb(0.04, 0.04, 0.07);
pub const ACCENT_BLUE: Color = Color::from_rgb(0.16, 0.56, 0.93);
pub const ACCENT_CYAN: Color = Color::from_rgb(0.36, 0.72, 0.98);
pub const ACCENT_RED: Color = Color::from_rgb(0.92, 0.24, 0.30);
pub const ACCENT_GRAY: Color = Color::from_rgb(0.62, 0.68, 0.74);
pub const GLASS_BG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.03);
#[allow(dead_code)]
pub const GLASS_BG_LITE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const GLASS_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const GRADIENT_TOP: Color = Color::from_rgb(0.02, 0.02, 0.05);
pub const GRADIENT_BOTTOM: Color = Color::from_rgb(0.07, 0.08, 0.12);
pub const TEXT_DIM: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.65);
pub const PANEL_RADIUS: f32 = 20.0;
pub const CARD_RADIUS: f32 = 12.0;
pub const PANEL_BG: Color = Color::from_rgba(0.08, 0.09, 0.12, 0.95);
pub const PANEL_BORDER: Color = Color::from_rgba(0.35, 0.45, 0.65, 0.25);

const BUTTON_RADIUS: f32 = 14.0;

fn tint_color(color: Color, delta: f32) -> Color {
    fn clamp_value(value: f32) -> f32 {
        if value < 0.0 {
            0.0
        } else if value > 1.0 {
            1.0
        } else {
            value
        }
    }

    Color::from_rgb(
        clamp_value(color.r + delta),
        clamp_value(color.g + delta),
        clamp_value(color.b + delta),
    )
}

fn button_gradient(accent: Color, status: button::Status) -> Background {
    let offset = match status {
        button::Status::Pressed => -0.18,
        button::Status::Hovered => 0.12,
        _ => 0.0,
    };

    let top_color = tint_color(accent, offset);
    let bottom_color = tint_color(accent, offset - 0.08);

    Background::Gradient(Gradient::Linear(
        Linear::new(std::f32::consts::FRAC_PI_2)
            .add_stop(0.0, top_color)
            .add_stop(1.0, bottom_color),
    ))
}

fn button_shadow(status: button::Status) -> Shadow {
    let alpha = if status == button::Status::Pressed {
        0.15
    } else {
        0.35
    };
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, 5.0),
        blur_radius: 16.0,
    }
}

pub fn brand_button_style(_theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let accent = if active { ACCENT_CYAN } else { ACCENT_BLUE };
    let mut style = button::Style::default();
    style.border = Border {
        radius: BUTTON_RADIUS.into(),
        width: 0.8,
        color: accent,
    };
    style.background = Some(button_gradient(accent, status));
    style.shadow = button_shadow(status);
    style.text_color = Color::WHITE;
    style
}

pub fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style::default();
    style.background = Some(button_gradient(ACCENT_RED, status));
    style.border = Border {
        color: ACCENT_RED.into(),
        width: 1.0,
        radius: BUTTON_RADIUS.into(),
    };
    style.shadow = button_shadow(status);
    style.text_color = Color::WHITE;
    style
}

pub fn glass_container<'a, T>(content: T) -> container::Container<'a, crate::Message>
where
    T: Into<iced::Element<'a, crate::Message>>,
{
    container(content)
        .padding(24)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(GLASS_BG)),
            border: Border {
                color: GLASS_BORDER,
                width: 1.0,
                radius: PANEL_RADIUS.into(),
            },
            ..Default::default()
        })
}

pub fn panel_container<'a, T>(content: T) -> container::Container<'a, crate::Message>
where
    T: Into<iced::Element<'a, crate::Message>>,
{
    container(content)
        .padding(18)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(PANEL_BG)),
            border: Border {
                color: PANEL_BORDER,
                width: 1.0,
                radius: CARD_RADIUS.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        })
}

pub fn root_background<'a, T>(content: T) -> container::Container<'a, crate::Message>
where
    T: Into<iced::Element<'a, crate::Message>>,
{
    container(content).style(|_: &Theme| container::Style {
        background: Some(Background::Gradient(Gradient::Linear(
            iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                .add_stop(0.0, GRADIENT_BOTTOM)
                .add_stop(1.0, GRADIENT_TOP),
        ))),
        ..Default::default()
    })
}
