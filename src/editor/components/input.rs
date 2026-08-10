use iced::widget::{TextInput, text_input};
use iced::{Background, Border, Theme};

use super::super::Message;
use super::tokens::{
    BODY_SIZE, BORDER, CONTROL_PAD_X, CONTROL_PAD_Y, CONTROL_RADIUS, CYAN, CYAN_SOFT, INPUT_BG,
    MUTED, TEXT,
};

pub fn field<'a>(placeholder: &str, value: &str) -> TextInput<'a, Message> {
    text_input(placeholder, value)
        .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
        .size(BODY_SIZE)
        .style(field_style)
}

fn field_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    let hovered = matches!(status, text_input::Status::Hovered);
    let disabled = matches!(status, text_input::Status::Disabled);
    text_input::Style {
        background: Background::Color(INPUT_BG),
        border: Border {
            color: if focused {
                CYAN
            } else if hovered {
                MUTED
            } else {
                BORDER
            },
            width: 1.0,
            radius: CONTROL_RADIUS.into(),
        },
        icon: if disabled { MUTED } else { TEXT },
        placeholder: MUTED,
        value: if disabled { MUTED } else { TEXT },
        selection: CYAN_SOFT,
    }
}
