//! Shared graphite/zinc presentation system for both editor workspaces.
//!
//! Surfaces use a compact zinc scale. Bright neutral is reserved for selection
//! and transport state; semantic colour is limited to explicit warnings,
//! errors, and animation-state signals.

use iced::widget::{Space, button, column, container, pane_grid, pick_list, row, text};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Point, Rectangle, Shadow, Size,
    Theme,
};
use lucide_icons::Icon;

use super::super::Message;
use crate::document::display_srd_name;
use crate::scene::SrCastKind;

use super::tokens::*;

pub fn surface_style(
    background: Color,
    border: Color,
    width: f32,
    radius: f32,
) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(background)),
        border: Border {
            color: border,
            width,
            radius: radius.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn workbench_grid_style(theme: &Theme) -> pane_grid::Style {
    let mut style = pane_grid::default(theme);
    style.hovered_split = pane_grid::Line {
        color: MUTED,
        width: 2.0,
    };
    style.picked_split = pane_grid::Line {
        color: TEXT,
        width: 2.0,
    };
    style
}

pub fn app_background(_: &Theme) -> container::Style {
    container::Style {
        background: Some(APP_BG.into()),
        text_color: Some(TEXT),
        ..container::Style::default()
    }
}

pub fn console_well_style(_: &Theme) -> container::Style {
    surface_style(CONSOLE_WELL, CONSOLE_LINE, 1.0, 3.0)
}

pub fn top_bar_style(_: &Theme) -> container::Style {
    surface_style(TOP_BAR_BG, BORDER, 0.0, 0.0)
}

pub fn settings_style(_: &Theme) -> container::Style {
    surface_style(PANEL_ALT, CYAN_SOFT, 0.0, 0.0)
}

pub fn sub_bar_style(_: &Theme) -> container::Style {
    surface_style(PANEL_ALT, BORDER, 0.0, 0.0)
}

pub fn floating_badge_style(_: &Theme) -> container::Style {
    surface_style(INPUT_BG, BORDER, 1.0, 4.0)
}

pub fn badge_style(_: &Theme) -> container::Style {
    surface_style(PANEL_ALT, BORDER, 1.0, 99.0)
}

pub fn tooltip_style(_: &Theme) -> container::Style {
    surface_style(INPUT_BG, BORDER, 1.0, 4.0)
}

pub fn notice_style(_: &Theme) -> container::Style {
    surface_style(CYAN_SOFT, CYAN, 1.0, 4.0)
}

pub fn accent_bar(_: &Theme) -> container::Style {
    surface_style(CYAN, CYAN, 0.0, 2.0)
}

pub fn divider_style(_: &Theme) -> container::Style {
    surface_style(BORDER, BORDER, 0.0, 0.0)
}

fn tree_row_background(selected: bool, hovered: bool) -> Color {
    if selected {
        CYAN_SOFT
    } else if hovered {
        Color::from_rgb(0.12, 0.12, 0.132)
    } else {
        PANEL_BG
    }
}

pub fn tree_row_style(selected: bool, hovered: bool) -> container::Style {
    surface_style(
        if selected || hovered {
            tree_row_background(selected, hovered)
        } else {
            Color::TRANSPARENT
        },
        if selected { BORDER } else { Color::TRANSPARENT },
        if selected { 1.0 } else { 0.0 },
        4.0,
    )
}

/// Keeps an assignment picker visually inside the full hierarchy-style row
/// highlight while retaining a local border when the picker itself is active.
pub fn assignment_pick_list_style(
    selected: bool,
    row_hovered: bool,
    status: pick_list::Status,
) -> pick_list::Style {
    let field_hovered = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    pick_list::Style {
        text_color: TEXT,
        placeholder_color: MUTED,
        handle_color: if field_hovered { CYAN } else { MUTED },
        background: Background::Color(tree_row_background(selected, row_hovered)),
        border: Border {
            color: if field_hovered {
                CYAN
            } else {
                Color::TRANSPARENT
            },
            width: 1.0,
            radius: 2.0.into(),
        },
    }
}

fn layer_tree_row_background(selected: bool, hovered: bool) -> Color {
    if selected {
        PANEL_ALT
    } else if hovered {
        Color::from_rgb(0.12, 0.12, 0.132)
    } else {
        PANEL_BG
    }
}

/// Layers use a restrained neutral selection; CAST selection stays cyan and
/// visually dominant through `tree_row_style`.
pub fn layer_tree_row_style(selected: bool, hovered: bool) -> container::Style {
    surface_style(
        if selected || hovered {
            layer_tree_row_background(selected, hovered)
        } else {
            Color::TRANSPARENT
        },
        if selected { BORDER } else { Color::TRANSPARENT },
        if selected { 1.0 } else { 0.0 },
        4.0,
    )
}

/// Opaque covers for the hierarchy row actions. Their fills match the visible
/// row state exactly, while the normal state uses the panel beneath the row.
/// This keeps the controls visually seamless and masks clipped label text.
pub fn tree_controls_style(selected: bool, hovered: bool) -> container::Style {
    surface_style(
        tree_row_background(selected, hovered),
        Color::TRANSPARENT,
        0.0,
        0.0,
    )
}

pub fn layer_tree_controls_style(selected: bool, hovered: bool) -> container::Style {
    surface_style(
        layer_tree_row_background(selected, hovered),
        Color::TRANSPARENT,
        0.0,
        0.0,
    )
}

pub fn icon_button_style(selected: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if selected {
            CYAN
        } else if hovered {
            PANEL_ALT
        } else {
            Color::TRANSPARENT
        })),
        text_color: if selected { Color::BLACK } else { TEXT },
        border: Border {
            color: if selected { CYAN } else { Color::TRANSPARENT },
            width: if selected { 1.0 } else { 0.0 },
            radius: 4.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn context_picker_style(open: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if open || hovered {
            PANEL_ALT
        } else {
            INPUT_BG
        })),
        text_color: TEXT,
        border: Border {
            color: if open { CYAN } else { BORDER },
            width: 1.0,
            radius: 3.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn context_menu_style(_: &Theme) -> container::Style {
    surface_style(INPUT_BG, BORDER, 1.0, 4.0)
}

pub fn context_menu_item_style(selected: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if selected {
            CYAN_SOFT
        } else if hovered {
            PANEL_ALT
        } else {
            Color::TRANSPARENT
        })),
        text_color: if selected { TEXT } else { MUTED },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 3.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn context_menu_action_style(danger: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let color = if danger { RED } else { MUTED };
    button::Style {
        background: Some(Background::Color(if hovered {
            PANEL_ALT
        } else {
            Color::TRANSPARENT
        })),
        text_color: color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 3.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Fixed-size hierarchy affordance. Active state changes only the glyph;
/// backgrounds never turn bright, so revealing controls cannot resize or
/// visually punch holes through the selected row.
pub fn hierarchy_icon_button_style(active: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered {
            PANEL_ALT
        } else {
            Color::TRANSPARENT
        })),
        text_color: if active { CYAN } else { MUTED },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 3.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Collapsible Inspector headings never acquire button chrome. Their chevron
/// communicates interactivity without changing the section hierarchy on hover.
pub fn section_header_button_style(_: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: TEXT,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}
/// Logical property controls shared by Inspector widgets and Dope Sheet canvas.
#[derive(Debug, Clone, Copy)]
pub enum AnimationTrackControl {
    Stopwatch,
    Keyframe,
}

/// Track controls use one 24×24 Lucide-derived vector coordinate system.
/// Stateful variants only remove the timer hand or fill the diamond path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationTrackGlyph {
    ChevronLeft,
    Stopwatch { hand: bool },
    Diamond { filled: bool },
    ChevronRight,
}

#[derive(Debug, Clone, Copy)]
pub struct AnimationTrackVisual {
    pub glyph: AnimationTrackGlyph,
    pub color: Color,
}

pub fn animation_track_visual(
    control: AnimationTrackControl,
    active: bool,
    enabled: bool,
) -> AnimationTrackVisual {
    let glyph = match (control, active) {
        (AnimationTrackControl::Stopwatch, true) => AnimationTrackGlyph::Stopwatch { hand: true },
        (AnimationTrackControl::Stopwatch, false) => AnimationTrackGlyph::Stopwatch { hand: false },
        (AnimationTrackControl::Keyframe, true) => AnimationTrackGlyph::Diamond { filled: true },
        (AnimationTrackControl::Keyframe, false) => AnimationTrackGlyph::Diamond { filled: false },
    };
    let color = match (active, enabled) {
        (true, true) => CYAN,
        (true, false) => Color { a: 0.45, ..CYAN },
        (false, true) => CONSOLE_TEXT,
        (false, false) => CONSOLE_FAINT,
    };
    AnimationTrackVisual { glyph, color }
}

/// Property-track affordance shared with the Dope Sheet canvas. Active state
/// is cyan glyph-only; disabled active controls retain a dimmed cyan signal.
pub fn animation_track_button_style(
    active: bool,
    enabled: bool,
    status: button::Status,
) -> button::Style {
    let hovered = enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered {
            Color {
                a: 0.10,
                ..CONSOLE_TEXT
            }
        } else {
            Color::TRANSPARENT
        })),
        text_color: animation_track_visual(AnimationTrackControl::Stopwatch, active, enabled).color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 2.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    icon_button_style(false, status)
}

/// Flat instrument control for the Animate console. `accent` drives selected
/// state so bright-neutral playback and amber motion authoring stay visibly
/// distinct without inventing new control shapes.
pub fn console_button_style(
    accent: Color,
    selected: bool,
    status: button::Status,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if selected {
            Color { a: 0.16, ..accent }
        } else if hovered {
            CONSOLE_RAIL
        } else {
            Color::TRANSPARENT
        })),
        text_color: if selected { accent } else { CONSOLE_TEXT },
        border: Border {
            color: if selected {
                accent
            } else {
                Color { a: 0.0, ..accent }
            },
            width: 1.0,
            radius: 2.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn fill_portion(value: u16) -> Length {
    Length::FillPortion(value)
}

pub fn icon(value: Icon, size: u32, color: Color) -> iced::widget::Text<'static> {
    iced::widget::Text::from(value).size(size).color(color)
}

pub fn property_row<'a>(label: &'a str, value: impl ToString) -> Element<'a, Message> {
    row![
        text(label)
            .size(BODY_SIZE)
            .color(MUTED)
            .width(fill_portion(2)),
        text(value.to_string())
            .size(BODY_SIZE)
            .color(TEXT)
            .width(fill_portion(3)),
    ]
    .spacing(8)
    .into()
}

pub fn labeled_control<'a>(
    label: &'a str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![text(label).size(BODY_SIZE).color(MUTED), control.into()]
        .spacing(4)
        .into()
}

pub fn divider_vertical() -> Element<'static, Message> {
    container(Space::new().width(1).height(20))
        .style(divider_style)
        .into()
}

pub fn badge(value: impl ToString) -> Element<'static, Message> {
    container(text(value.to_string()).size(CAPTION_SIZE).color(MUTED))
        .padding([2, 5])
        .style(badge_style)
        .into()
}

/// Square console tag used for DIRECT / INHERITED / state marks.
pub fn console_tag(label: impl ToString, color: Color) -> Element<'static, Message> {
    container(text(label.to_string()).size(OVERLINE_SIZE).color(color))
        .padding([1, 4])
        .style(move |_: &Theme| surface_style(Color { a: 0.12, ..color }, color, 1.0, 2.0))
        .into()
}

pub fn empty_state<'a>(icon_value: Icon, title: &'a str, detail: &'a str) -> Element<'a, Message> {
    container(
        column![
            icon(icon_value, 23, MUTED),
            text(title).size(BODY_SIZE).color(TEXT),
            text(detail).size(CAPTION_SIZE).color(MUTED),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .padding(18)
    .width(Fill)
    .center_x(Fill)
    .into()
}

/// Console empty state: left-aligned instruction, never a centred card.
pub fn console_empty_state<'a>(title: &'a str, detail: &'a str) -> Element<'a, Message> {
    container(
        column![
            text(title).size(BODY_SIZE).color(CONSOLE_TEXT),
            text(detail).size(CAPTION_SIZE).color(CONSOLE_MUTED),
        ]
        .spacing(4),
    )
    .padding([10, 10])
    .width(Fill)
    .into()
}

pub fn nonempty_name(raw: &[u8], fallback: String) -> String {
    let name = display_srd_name(raw);
    if name.is_empty() { fallback } else { name }
}

pub fn parse_or(value: &str, fallback: f32) -> f32 {
    value.parse().unwrap_or(fallback)
}

pub fn timecode(frame: i32) -> String {
    let frame = frame.max(0);
    let total_seconds = frame / 60;
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        total_seconds / 3600,
        (total_seconds / 60) % 60,
        total_seconds % 60,
        frame % 60,
    )
}

pub fn fitted_rect(bounds: Size, target: [u32; 2], padding: f32, zoom_percent: u16) -> Rectangle {
    let available = Size::new(
        (bounds.width - padding * 2.0).max(1.0),
        (bounds.height - padding * 2.0).max(1.0),
    );
    let aspect = target[0].max(1) as f32 / target[1].max(1) as f32;
    let mut width = available.width;
    let mut height = width / aspect;
    if height > available.height {
        height = available.height;
        width = height * aspect;
    }
    let zoom = f32::from(zoom_percent) / 100.0;
    width *= zoom;
    height *= zoom;
    Rectangle::new(
        Point::new((bounds.width - width) / 2.0, (bounds.height - height) / 2.0),
        Size::new(width, height),
    )
}

pub fn cast_icon(kind: SrCastKind) -> Icon {
    match kind {
        SrCastKind::Null => Icon::Group,
        SrCastKind::Image => Icon::Image,
        SrCastKind::Text => Icon::TextCursorInput,
        SrCastKind::Slice => Icon::Grid2X2,
        SrCastKind::Reference => Icon::ExternalLink,
        SrCastKind::Number => Icon::Hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_animation_track_control_is_cyan_without_selected_chrome() {
        let style = animation_track_button_style(true, true, button::Status::Active);
        assert_eq!(style.text_color, CYAN);
        assert_eq!(
            style.background,
            Some(Background::Color(Color::TRANSPARENT))
        );
        assert_eq!(style.border.color, Color::TRANSPARENT);
        assert_eq!(style.border.width, 0.0);
    }

    #[test]
    fn animation_track_visuals_distinguish_shape_activity_and_enablement() {
        let running = animation_track_visual(AnimationTrackControl::Stopwatch, true, true);
        assert_eq!(running.glyph, AnimationTrackGlyph::Stopwatch { hand: true });
        assert_eq!(running.color, CYAN);

        let stopped = animation_track_visual(AnimationTrackControl::Stopwatch, false, true);
        assert_eq!(
            stopped.glyph,
            AnimationTrackGlyph::Stopwatch { hand: false }
        );
        assert_eq!(stopped.color, TEXT);

        let keyed = animation_track_visual(AnimationTrackControl::Keyframe, true, false);
        assert_eq!(keyed.glyph, AnimationTrackGlyph::Diamond { filled: true });
        assert_eq!(keyed.color, Color { a: 0.45, ..CYAN });

        let unkeyed = animation_track_visual(AnimationTrackControl::Keyframe, false, false);
        assert_eq!(
            unkeyed.glyph,
            AnimationTrackGlyph::Diamond { filled: false }
        );
        assert_eq!(unkeyed.color, CONSOLE_FAINT);
    }

    #[test]
    fn disabled_animation_track_control_has_no_hover_state() {
        let idle = animation_track_button_style(true, false, button::Status::Active);
        let hovered = animation_track_button_style(true, false, button::Status::Hovered);
        assert_eq!(idle.background, hovered.background);
        assert_eq!(idle.text_color, hovered.text_color);
    }

    #[test]
    fn inspector_section_headers_never_acquire_hover_chrome() {
        let idle = section_header_button_style(button::Status::Active);
        for status in [
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            let style = section_header_button_style(status);
            assert_eq!(style.background, idle.background);
            assert_eq!(style.text_color, idle.text_color);
            assert_eq!(style.border.color, Color::TRANSPARENT);
            assert_eq!(style.border.width, 0.0);
        }
    }

    #[test]
    fn hierarchy_control_covers_match_every_row_state() {
        for (selected, hovered) in [(false, false), (false, true), (true, false), (true, true)] {
            let cast = tree_controls_style(selected, hovered);
            assert_eq!(
                cast.background,
                Some(Background::Color(tree_row_background(selected, hovered)))
            );
            assert_eq!(cast.border.color, Color::TRANSPARENT);
            assert_eq!(cast.border.width, 0.0);

            let layer = layer_tree_controls_style(selected, hovered);
            assert_eq!(
                layer.background,
                Some(Background::Color(layer_tree_row_background(
                    selected, hovered
                )))
            );
            assert_eq!(layer.border.color, Color::TRANSPARENT);
            assert_eq!(layer.border.width, 0.0);
        }

        let hovered_button = hierarchy_icon_button_style(false, button::Status::Hovered);
        assert_eq!(
            hovered_button.background,
            Some(Background::Color(PANEL_ALT)),
            "the individual button keeps its established hover fill"
        );
        assert_ne!(
            hovered_button.background,
            tree_controls_style(false, true).background,
            "the button hover must remain visible above the row-matched cover"
        );
        assert_ne!(
            hovered_button.background,
            layer_tree_controls_style(false, true).background,
            "the button hover must remain visible above the layer-row cover"
        );
    }

    #[test]
    fn assignment_picker_preserves_the_full_tree_row_fill() {
        for (selected, hovered) in [(false, false), (false, true), (true, false), (true, true)] {
            let style = assignment_pick_list_style(selected, hovered, pick_list::Status::Active);
            assert_eq!(
                style.background,
                Background::Color(tree_row_background(selected, hovered))
            );
        }
    }
}
