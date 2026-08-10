use iced::widget::{center, container, mouse_area, row, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme};
use lucide_icons::Icon;

use super::super::Message;
use super::style::{icon, settings_style, surface_style};
use super::tokens::{DIALOG_TITLE_SIZE, MUTED, TEXT};

pub fn modal<'a>(
    content: impl Into<Element<'a, Message>>,
    width: f32,
    dismiss: Message,
) -> Element<'a, Message> {
    let dialog = container(content)
        .width(Length::Fixed(width))
        .padding(18)
        .style(settings_style);
    mouse_area(
        container(center(dialog))
            .width(Fill)
            .height(Fill)
            .style(backdrop_style),
    )
    .on_press(dismiss)
    .into()
}

pub fn title<'a>(icon_value: Icon, label: &'a str, color: Color) -> Element<'a, Message> {
    row![
        icon(icon_value, 17, color),
        text(label).size(DIALOG_TITLE_SIZE).color(TEXT),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn description(value: &str) -> Element<'_, Message> {
    text(value)
        .size(super::tokens::BODY_SIZE)
        .color(MUTED)
        .into()
}

fn backdrop_style(_: &Theme) -> container::Style {
    surface_style(
        Color::from_rgba(0.015, 0.015, 0.020, 0.78),
        Color::TRANSPARENT,
        0.0,
        0.0,
    )
}
