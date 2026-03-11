use iced::gradient::Linear;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Gradient, Shadow, Theme, Vector};

// --- SUNSET PALETTE (Midnight Variant) ---
pub const SUNSET_DARK: Color = Color::from_rgb(0.00, 0.00, 0.01); 
pub const SUNSET_BLUE: Color = Color::from_rgb(0.02, 0.05, 0.12); 
pub const SUNSET_ACCENT: Color = Color::from_rgb(0.04, 0.12, 0.25); 
pub const SUNSET_PINK: Color = Color::from_rgb(0.20, 0.04, 0.10); // Dark Wine/Maroon

pub const ACCENT_BLUE: Color = Color::from_rgb(0.15, 0.55, 0.95);
pub const ACCENT_RED: Color = Color::from_rgb(1.00, 0.40, 0.50);
pub const ACCENT_GRAY: Color = Color::from_rgb(0.35, 0.40, 0.45);
pub const GLASS_BG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.05);
#[allow(dead_code)]
pub const GLASS_BG_LITE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
pub const GLASS_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const TEXT_DIM: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.60);
pub const PANEL_RADIUS: f32 = 20.0;
pub const CARD_RADIUS: f32 = 14.0;
pub const PANEL_BG: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.35); // Transparent Glass Card
pub const PANEL_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.12); // Brighter border for glass effect

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
        button::Status::Pressed => -0.15,
        button::Status::Hovered => 0.12,
        _ => 0.0,
    };

    let top_color = tint_color(accent, offset + 0.04);
    let mid_color = tint_color(accent, offset);
    let bottom_color = tint_color(accent, offset - 0.12);

    Background::Gradient(Gradient::Linear(
        Linear::new(std::f32::consts::FRAC_PI_2)
            .add_stop(0.0, top_color)
            .add_stop(0.5, mid_color)
            .add_stop(1.0, bottom_color),
    ))
}

fn button_shadow(status: button::Status) -> Shadow {
    let (alpha, offset_y, blur) = match status {
        button::Status::Pressed => (0.15, 2.0, 8.0),
        button::Status::Hovered => (0.35, 6.0, 16.0),
        _ => (0.25, 4.0, 12.0),
    };
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, offset_y),
        blur_radius: blur,
    }
}

pub fn brand_button_style(_theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let accent = if active { ACCENT_RED } else { ACCENT_BLUE };
    let mut style = button::Style::default();
    style.border = Border {
        radius: BUTTON_RADIUS.into(),
        width: 1.0,
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
    };
    style.background = Some(button_gradient(accent, status));
    style.shadow = button_shadow(status);
    style.text_color = Color::WHITE;
    style
}

pub fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    brand_button_style(_theme, status, true)
}

pub fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    brand_button_style(_theme, status, false)
}

pub fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::Style::default();
    style.background = Some(button_gradient(ACCENT_RED, status));
    style.border = Border {
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.1),
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

pub fn sidebar_container<'a, T>(content: T) -> container::Container<'a, crate::Message>
where
    T: Into<iced::Element<'a, crate::Message>>,
{
    container(content)
        .padding(24)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.4))),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
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
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
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
            iced::gradient::Linear::new(std::f32::consts::FRAC_PI_4) // 45 degrees
                .add_stop(0.0, SUNSET_DARK)
                .add_stop(0.6, SUNSET_BLUE)
                .add_stop(0.9, SUNSET_ACCENT)
                .add_stop(1.0, SUNSET_PINK),
        ))),
        ..Default::default()
    })
}
