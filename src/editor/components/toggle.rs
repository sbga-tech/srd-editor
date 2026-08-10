use iced::widget::{Checkbox, Toggler, checkbox, toggler};
use iced::{Background, Border, Color, Theme};

use super::super::Message;
use super::tokens::{BODY_SIZE, BORDER, CONTROL_RADIUS, CYAN, INPUT_BG, MUTED, TEXT, TOGGLE_SIZE};

pub fn switch<'a>(is_toggled: bool) -> Toggler<'a, Message> {
    toggler(is_toggled)
        .size(TOGGLE_SIZE)
        .text_size(BODY_SIZE)
        .spacing(7)
        .style(switch_style)
}

pub fn check<'a>(label: &'a str, is_checked: bool) -> Checkbox<'a, Message> {
    checkbox(is_checked)
        .label(label)
        .size(TOGGLE_SIZE)
        .text_size(BODY_SIZE)
        .spacing(7)
        .style(check_style)
}

fn switch_style(_: &Theme, status: toggler::Status) -> toggler::Style {
    let (is_toggled, hovered, disabled) = match status {
        toggler::Status::Active { is_toggled } => (is_toggled, false, false),
        toggler::Status::Hovered { is_toggled } => (is_toggled, true, false),
        toggler::Status::Disabled { is_toggled } => (is_toggled, false, true),
    };
    toggler::Style {
        background: Background::Color(if is_toggled { CYAN } else { INPUT_BG }),
        background_border_width: 1.0,
        background_border_color: if hovered { TEXT } else { BORDER },
        foreground: Background::Color(if is_toggled { Color::BLACK } else { MUTED }),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(if disabled { MUTED } else { TEXT }),
        border_radius: Some(CONTROL_RADIUS.into()),
        padding_ratio: 0.18,
    }
}

fn check_style(_: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (is_checked, hovered, disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };
    checkbox::Style {
        background: Background::Color(if is_checked { CYAN } else { INPUT_BG }),
        icon_color: Color::BLACK,
        border: Border {
            color: if hovered { TEXT } else { BORDER },
            width: 1.0,
            radius: CONTROL_RADIUS.into(),
        },
        text_color: Some(if disabled { MUTED } else { TEXT }),
    }
}
