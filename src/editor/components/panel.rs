use iced::widget::{Space, column, container, pane_grid, row, rule, text};
use iced::{Alignment, Element, Fill, Theme};
use lucide_icons::Icon;

use super::super::{Message, PaneKind};
use super::style::{badge, icon, surface_style, workbench_grid_style};
use super::tokens::{
    BODY_SIZE, CYAN, MUTED, PANEL_BODY_BG, PANEL_PAD_X, PANEL_TITLE_BG, PANEL_TITLE_HEIGHT,
    PANEL_TITLE_SIZE, SECTION_GAP, TEXT, TOOLBAR_BG, TOOLBAR_HEIGHT,
};

pub fn panel<'a>(
    header: Element<'a, Message>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(column![header, body.into()].height(Fill))
        .width(Fill)
        .height(Fill)
        .style(panel_body_style)
        .into()
}

pub fn panel_with_toolbar<'a>(
    header: Element<'a, Message>,
    toolbar: Element<'a, Message>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    panel(
        header,
        column![toolbar, body.into()].height(Fill).width(Fill),
    )
}

pub fn panel_header<'a>(
    icon_value: Icon,
    label: &'a str,
    context: Option<String>,
) -> Element<'a, Message> {
    let mut content = row![
        icon(icon_value, 14, CYAN),
        text(label).size(PANEL_TITLE_SIZE).color(TEXT),
        Space::new().width(Fill),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    if let Some(context) = context {
        content = content.push(text(context).size(BODY_SIZE).color(MUTED));
    }
    container(content)
        .padding([7.0, PANEL_PAD_X])
        .height(PANEL_TITLE_HEIGHT)
        .width(Fill)
        .style(panel_title_style)
        .into()
}

pub fn panel_toolbar<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([4, 8])
        .height(TOOLBAR_HEIGHT)
        .width(Fill)
        .align_y(Alignment::Center)
        .style(panel_toolbar_style)
        .into()
}

pub fn panel_body<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .style(panel_body_style)
        .into()
}

pub fn panel_section<'a>(
    heading: Element<'a, Message>,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    section_surface(column![heading, body.into()].spacing(SECTION_GAP))
}

pub fn section_surface<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![
        container(content)
            .padding([8, 9])
            .width(Fill)
            .style(panel_body_style),
        rule::horizontal(1),
    ]
    .spacing(0)
    .width(Fill)
    .into()
}

pub fn section_heading<'a>(
    icon_value: Icon,
    label: &'a str,
    tag: Option<&'a str>,
) -> Element<'a, Message> {
    let mut content = row![
        icon(icon_value, 13, CYAN),
        text(label).size(BODY_SIZE).color(TEXT),
        Space::new().width(Fill),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(tag) = tag {
        content = content.push(badge(tag));
    }
    content.into()
}

pub fn resizable_panes<'a>(
    state: &'a pane_grid::State<PaneKind>,
    view: impl Fn(pane_grid::Pane, &'a PaneKind, bool) -> pane_grid::Content<'a, Message> + 'a,
) -> Element<'a, Message> {
    pane_grid::PaneGrid::new(state, view)
        .spacing(3)
        .min_size(170)
        .style(workbench_grid_style)
        .on_resize(10, Message::PaneResized)
        .into()
}

fn panel_title_style(_: &Theme) -> container::Style {
    surface_style(PANEL_TITLE_BG, super::tokens::BORDER, 0.0, 0.0)
}

fn panel_toolbar_style(_: &Theme) -> container::Style {
    surface_style(TOOLBAR_BG, super::tokens::BORDER, 0.0, 0.0)
}

fn panel_body_style(_: &Theme) -> container::Style {
    surface_style(PANEL_BODY_BG, super::tokens::BORDER, 0.0, 0.0)
}
