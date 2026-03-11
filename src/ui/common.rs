use crate::ui::theme::{panel_container, TEXT_DIM};
use crate::app::Message;
use iced::widget::{column, container, text};
use iced::{Element, Length};

pub fn panel_card<'a, T>(title: &'a str, content: T) -> container::Container<'a, Message>
where
    T: Into<Element<'a, Message>>,
{
    panel_container(
        column![
            text(title).size(11).font(font_bold()).color(TEXT_DIM),
            content.into(),
        ]
        .spacing(10),
    )
    .width(Length::Fill)
}

pub fn info_block<'a, V>(label: &'a str, value: V) -> Element<'a, Message>
where
    V: Into<String>,
{
    let value = value.into();
    column![
        text(label).size(9).color(TEXT_DIM),
        text(value)
            .size(14)
            .font(font_bold())
            .color(iced::Color::WHITE),
    ]
    .spacing(2)
    .width(Length::FillPortion(1))
    .into()
}

pub fn font_bold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..iced::Font::DEFAULT
    }
}

pub fn font_mono() -> iced::Font {
    iced::Font::with_name("Noto Sans Mono")
}
