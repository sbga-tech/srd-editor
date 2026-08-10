use iced::widget::canvas::{Canvas, Frame, Geometry, LineCap, LineJoin, Path, Program, Stroke};
use iced::widget::{button, container, text, tooltip};
use iced::{Color, Element, Length, Point, Rectangle, Theme, mouse};
use lucide_icons::Icon;

use super::super::Message;
use super::style::{
    AnimationTrackControl, AnimationTrackGlyph, animation_track_button_style,
    animation_track_visual, console_button_style, console_well_style, ghost_button,
    hierarchy_icon_button_style, icon, icon_button_style, tooltip_style,
};
use super::tokens::{BODY_SIZE, CAPTION_SIZE, CONTROL_HEIGHT, CYAN, MUTED, TEXT};

fn square_icon(icon_value: Icon, size: u32, color: Color) -> Element<'static, Message> {
    container(icon(icon_value, size, color))
        .width(CONTROL_HEIGHT)
        .height(CONTROL_HEIGHT)
        .center(CONTROL_HEIGHT)
        .into()
}

pub(crate) const ANIMATION_TRACK_GLYPH_SIZE: f32 = 16.0;
const LUCIDE_VIEWBOX_SIZE: f32 = 24.0;

#[derive(Debug, Clone, Copy)]
struct AnimationTrackGlyphCanvas {
    glyph: AnimationTrackGlyph,
    color: Color,
}

/// Draws the original Lucide timer, diamond, and chevron paths in one shared
/// 24×24 coordinate system. Stateful variants only omit the timer hand or fill
/// the diamond; their outer geometry and optical scale remain unchanged.
pub(crate) fn draw_animation_track_glyph(
    frame: &mut Frame,
    glyph: AnimationTrackGlyph,
    center: Point,
    size: f32,
    color: Color,
) {
    let scale = size / LUCIDE_VIEWBOX_SIZE;
    let origin = Point::new(center.x - size / 2.0, center.y - size / 2.0);
    let point = |x: f32, y: f32| Point::new(origin.x + x * scale, origin.y + y * scale);
    let stroke = Stroke::default()
        .with_color(color)
        .with_width(2.0 * scale)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round);

    let path = match glyph {
        AnimationTrackGlyph::ChevronLeft => Path::new(|builder| {
            builder.move_to(point(15.0, 18.0));
            builder.line_to(point(9.0, 12.0));
            builder.line_to(point(15.0, 6.0));
        }),
        AnimationTrackGlyph::Stopwatch { hand } => Path::new(|builder| {
            builder.move_to(point(10.0, 2.0));
            builder.line_to(point(14.0, 2.0));
            if hand {
                builder.move_to(point(12.0, 14.0));
                builder.line_to(point(15.0, 11.0));
            }
            builder.circle(point(12.0, 14.0), 8.0 * scale);
            builder.close();
        }),
        AnimationTrackGlyph::Diamond { .. } => Path::new(|builder| {
            let radius = 2.41 * scale;
            builder.move_to(point(2.70, 10.30));
            builder.arc_to(point(1.0, 12.0), point(2.70, 13.71), radius);
            builder.line_to(point(10.29, 21.30));
            builder.arc_to(point(12.0, 23.0), point(13.70, 21.30), radius);
            builder.line_to(point(21.29, 13.71));
            builder.arc_to(point(23.0, 12.0), point(21.29, 10.30), radius);
            builder.line_to(point(13.70, 2.71));
            builder.arc_to(point(12.0, 1.0), point(10.29, 2.71), radius);
            builder.close();
        }),
        AnimationTrackGlyph::ChevronRight => Path::new(|builder| {
            builder.move_to(point(9.0, 18.0));
            builder.line_to(point(15.0, 12.0));
            builder.line_to(point(9.0, 6.0));
        }),
    };

    if matches!(glyph, AnimationTrackGlyph::Diamond { filled: true }) {
        frame.fill(&path, color);
    }
    frame.stroke(&path, stroke);
}

impl Program<Message> for AnimationTrackGlyphCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        draw_animation_track_glyph(
            &mut frame,
            self.glyph,
            Point::new(bounds.width / 2.0, bounds.height / 2.0),
            ANIMATION_TRACK_GLYPH_SIZE,
            self.color,
        );
        vec![frame.into_geometry()]
    }
}

pub(crate) fn animation_track_glyph(
    glyph: AnimationTrackGlyph,
    color: Color,
) -> Element<'static, Message> {
    Canvas::new(AnimationTrackGlyphCanvas { glyph, color })
        .width(CONTROL_HEIGHT)
        .height(CONTROL_HEIGHT)
        .into()
}

pub fn icon_button(
    icon_value: Icon,
    help: &'static str,
    message: Message,
    selected: bool,
) -> Element<'static, Message> {
    let control = button(square_icon(
        icon_value,
        14,
        if selected { Color::BLACK } else { TEXT },
    ))
    .on_press(message)
    .padding(0)
    .width(CONTROL_HEIGHT)
    .height(CONTROL_HEIGHT)
    .style(move |_theme, status| icon_button_style(selected, status));
    tooltip(
        control,
        container(text(help).size(BODY_SIZE).color(TEXT))
            .padding([5, 7])
            .style(tooltip_style),
        tooltip::Position::Bottom,
    )
    .gap(7)
    .into()
}

pub fn console_icon_button(
    icon_value: Icon,
    help: &'static str,
    message: Message,
    selected: bool,
    accent: Color,
) -> Element<'static, Message> {
    let control = button(square_icon(
        icon_value,
        14,
        if selected { accent } else { TEXT },
    ))
    .on_press(message)
    .padding(0)
    .width(CONTROL_HEIGHT)
    .height(CONTROL_HEIGHT)
    .style(move |_theme, status| console_button_style(accent, selected, status));
    tooltip(
        control,
        container(text(help).size(CAPTION_SIZE).color(TEXT))
            .padding([4, 6])
            .style(console_well_style),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into()
}

pub fn animation_track_icon_button(
    glyph: AnimationTrackGlyph,
    help: &'static str,
    message: Message,
    active: bool,
) -> Element<'static, Message> {
    let control = button(animation_track_glyph(
        glyph,
        if active { CYAN } else { TEXT },
    ))
    .on_press(message)
    .padding(0)
    .width(CONTROL_HEIGHT)
    .height(CONTROL_HEIGHT)
    .style(move |_theme, status| animation_track_button_style(active, true, status));
    tooltip(
        control,
        container(text(help).size(CAPTION_SIZE).color(TEXT))
            .padding([4, 6])
            .style(console_well_style),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into()
}

pub fn animation_track_control_button(
    kind: AnimationTrackControl,
    help: &'static str,
    message: Option<Message>,
    active: bool,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let visual = animation_track_visual(kind, active, enabled);
    let control = button(animation_track_glyph(visual.glyph, visual.color))
        .padding(0)
        .width(CONTROL_HEIGHT)
        .height(CONTROL_HEIGHT)
        .style(move |_theme, status| animation_track_button_style(active, enabled, status));
    let control = if let Some(message) = message {
        control.on_press(message)
    } else {
        control
    };
    tooltip(
        control,
        container(text(help).size(CAPTION_SIZE).color(TEXT))
            .padding([4, 6])
            .style(console_well_style),
        tooltip::Position::Bottom,
    )
    .gap(6)
    .into()
}

pub fn passive_icon(icon_value: Icon, help: &'static str) -> Element<'static, Message> {
    tooltip(
        container(icon(icon_value, 14, MUTED))
            .width(CONTROL_HEIGHT)
            .height(CONTROL_HEIGHT)
            .center(CONTROL_HEIGHT),
        container(text(help).size(BODY_SIZE).color(TEXT))
            .padding([5, 7])
            .style(tooltip_style),
        tooltip::Position::Bottom,
    )
    .gap(7)
    .into()
}

pub fn mini_icon_button(icon_value: Icon, message: Message) -> Element<'static, Message> {
    button(icon(icon_value, 12, MUTED))
        .on_press(message)
        .padding(2)
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0))
        .style(ghost_button)
        .into()
}

pub fn hierarchy_icon_button(
    icon_value: Icon,
    help: &'static str,
    message: Message,
    active: bool,
    revealed: bool,
) -> Element<'static, Message> {
    let color = if active {
        CYAN
    } else if revealed {
        MUTED
    } else {
        Color::TRANSPARENT
    };
    let control = button(icon(icon_value, 13, color))
        .on_press(message)
        .padding(4)
        .width(24)
        .height(24)
        .style(move |_theme, status| hierarchy_icon_button_style(active, status));
    tooltip(
        control,
        container(text(help).size(CAPTION_SIZE).color(TEXT))
            .padding([4, 6])
            .style(tooltip_style),
        tooltip::Position::Bottom,
    )
    .gap(5)
    .into()
}

pub fn action(label: &'static str, message: Message, accent: Color) -> Element<'static, Message> {
    button(text(label).size(BODY_SIZE))
        .on_press(message)
        .padding([5, 10])
        .height(CONTROL_HEIGHT)
        .style(move |_theme, status| console_button_style(accent, true, status))
        .into()
}

pub fn secondary(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).size(BODY_SIZE))
        .on_press(message)
        .padding([5, 10])
        .height(CONTROL_HEIGHT)
        .style(ghost_button)
        .into()
}
