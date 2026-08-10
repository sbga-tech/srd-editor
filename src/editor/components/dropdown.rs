use std::borrow::Borrow;

use iced::widget::{
    Column, PickList, Space, button, container, overlay::menu, pick_list, row, scrollable, text,
};
use iced::{Alignment, Background, Border, Element, Fill, Length, Pixels, Shadow, Theme};
use lucide_icons::Icon;

use super::super::Message;
use super::style::{
    context_menu_action_style, context_menu_item_style, context_menu_style, context_picker_style,
    icon, surface_style,
};
use super::tokens::{
    BODY_SIZE, BORDER, CAPTION_SIZE, CONTROL_HEIGHT, CONTROL_PAD_X, CONTROL_PAD_Y, CONTROL_RADIUS,
    CYAN, DROPDOWN_ITEM_HEIGHT, DROPDOWN_TEXT_LINE_HEIGHT, INPUT_BG, MUTED, RED, TEXT,
};

/// Canonical compact single-select control. The menu inherits the trigger's
/// exact text metrics and padding, so every option is exactly as tall as the
/// closed control. Selection is communicated by background only—never a
/// checkmark, which would imply multi-selection.
pub fn single_select<'a, T, L, V>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    single_select_base(options, selected, on_selected).style(pick_list_style)
}

/// Compact single-select metrics with caller-supplied trigger styling.
pub fn single_select_base<'a, T, L, V>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
{
    pick_list(options, selected, on_selected)
        .text_size(BODY_SIZE)
        .text_line_height(Pixels(DROPDOWN_TEXT_LINE_HEIGHT))
        .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
        .menu_style(pick_list_menu_style)
}

pub fn pick_list_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let active = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    pick_list::Style {
        text_color: TEXT,
        placeholder_color: MUTED,
        handle_color: if active { CYAN } else { MUTED },
        background: Background::Color(super::tokens::INPUT_BG),
        border: Border {
            color: if active { CYAN } else { super::tokens::BORDER },
            width: 1.0,
            radius: super::tokens::CONTROL_RADIUS.into(),
        },
    }
}

pub fn pick_list_menu_style(_: &Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(super::tokens::INPUT_BG),
        border: Border {
            color: super::tokens::BORDER,
            width: 1.0,
            radius: super::tokens::CONTROL_RADIUS.into(),
        },
        text_color: super::tokens::TEXT,
        selected_text_color: super::tokens::TEXT,
        selected_background: Background::Color(super::tokens::CYAN_SOFT),
        shadow: Shadow::default(),
    }
}

pub fn trigger(
    label: String,
    muted: bool,
    open: bool,
    message: Message,
    width: f32,
) -> Element<'static, Message> {
    button(
        row![
            text(label)
                .size(BODY_SIZE)
                .color(if muted { MUTED } else { TEXT }),
            Space::new().width(Fill),
            icon(
                if open {
                    Icon::ChevronUp
                } else {
                    Icon::ChevronDown
                },
                12,
                MUTED,
            ),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .on_press(message)
    .padding([5, 7])
    .width(Length::Fixed(width))
    .height(CONTROL_HEIGHT)
    .style(move |_theme, status| context_picker_style(open, status))
    .into()
}

pub fn read_only_select(label: String, width: Length) -> Element<'static, Message> {
    container(
        row![
            text(label).size(BODY_SIZE).color(MUTED),
            Space::new().width(Fill),
            icon(Icon::ChevronDown, 12, MUTED),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .padding([0.0, CONTROL_PAD_X])
    .width(width)
    .height(CONTROL_HEIGHT)
    .align_y(Alignment::Center)
    .style(|_| surface_style(INPUT_BG, BORDER, 1.0, CONTROL_RADIUS))
    .clip(true)
    .into()
}

pub fn menu<'a>(rows: Column<'a, Message>, width: f32, row_gap: f32) -> Element<'a, Message> {
    const VISIBLE_ROWS: f32 = 10.0;
    let max_height = DROPDOWN_ITEM_HEIGHT * VISIBLE_ROWS + row_gap * (VISIBLE_ROWS - 1.0) + 6.0;
    container(scrollable(rows).height(Length::Shrink))
        .padding(3)
        .width(Length::Fixed(width))
        .max_height(max_height)
        .style(context_menu_style)
        .into()
}

pub fn add_item(label: &'static str, enabled: bool, message: Message) -> Element<'static, Message> {
    let content = row![
        icon(Icon::Plus, 12, if enabled { CYAN } else { MUTED }),
        text(label)
            .size(BODY_SIZE)
            .color(if enabled { CYAN } else { MUTED }),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    if enabled {
        button(content)
            .on_press(message)
            .padding([5, 7])
            .width(Fill)
            .height(DROPDOWN_ITEM_HEIGHT)
            .style(move |_theme, status| context_menu_item_style(false, status))
            .into()
    } else {
        container(content)
            .padding([5, 7])
            .width(Fill)
            .height(DROPDOWN_ITEM_HEIGHT)
            .align_y(Alignment::Center)
            .into()
    }
}

pub fn item(
    label: String,
    selected: bool,
    select_message: Message,
    actions: Option<(Message, Message)>,
) -> Element<'static, Message> {
    let select = button(text(label).size(BODY_SIZE))
        .on_press(select_message)
        .padding([5, 7])
        .width(Fill)
        .height(DROPDOWN_ITEM_HEIGHT)
        .style(move |_theme, status| context_menu_item_style(selected, status));
    let actions: Element<'static, Message> = if let Some((rename, delete)) = actions {
        row![
            action_button(Icon::Pencil, rename, false),
            action_button(Icon::X, delete, true),
        ]
        .spacing(1)
        .into()
    } else {
        Space::new()
            .width(Length::Fixed(DROPDOWN_ITEM_HEIGHT * 2.0 + 1.0))
            .into()
    };
    row![select, actions]
        .spacing(1)
        .height(DROPDOWN_ITEM_HEIGHT)
        .align_y(Alignment::Center)
        .into()
}

fn action_button(icon_value: Icon, message: Message, danger: bool) -> Element<'static, Message> {
    button(icon(icon_value, 12, if danger { RED } else { MUTED }))
        .on_press(message)
        .padding(5)
        .width(DROPDOWN_ITEM_HEIGHT)
        .height(DROPDOWN_ITEM_HEIGHT)
        .style(move |_theme, status| context_menu_action_style(danger, status))
        .into()
}

pub fn empty_item(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(CAPTION_SIZE).color(MUTED))
        .padding([5, 7])
        .width(Fill)
        .height(DROPDOWN_ITEM_HEIGHT)
        .align_y(Alignment::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_and_option_rows_have_identical_compact_height() {
        assert_eq!(DROPDOWN_ITEM_HEIGHT, CONTROL_HEIGHT);
        assert_eq!(
            DROPDOWN_TEXT_LINE_HEIGHT + CONTROL_PAD_Y * 2.0,
            CONTROL_HEIGHT,
        );
    }
}
