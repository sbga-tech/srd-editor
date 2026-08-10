//! Animate-specific panes. Shared workspace shells live in `ui.rs`; this module
//! owns assignment, property authoring, and the batched dope-sheet canvas.

use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Program, Stroke, Text as CanvasText,
};
use iced::widget::{Space, button, column, container, mouse_area, row, rule, scrollable, text};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Size, Theme, mouse};
use lucide_icons::Icon;

use super::animate::{
    AnimateAction, AssignmentRow, DOPE_MIN_TIMELINE_WIDTH, DOPE_ROW_HEIGHT, DopeRow, DopeSheet,
    TrackAddress, Workspace, is_authorable_track_target,
};
use super::channels::{self, ChannelValue, KeySemantics};
use super::components::{
    ANIMATION_TRACK_GLYPH_SIZE, AnimationTrackControl, AnimationTrackGlyph, BODY_SIZE,
    CONSOLE_FAINT, CONSOLE_LINE, CONSOLE_MUTED, CONSOLE_TEXT, CONSOLE_WELL, CONTROL_HEIGHT, CYAN,
    OVERLINE_SIZE, RED, ROW_HEIGHT, TIMECODE_SIZE, animation_track_control_button,
    animation_track_glyph, animation_track_icon_button, animation_track_visual,
    assignment_pick_list_style, button as button_component, console_button_style,
    console_empty_state, console_icon_button, console_tag, draw_animation_track_glyph,
    dropdown as dropdown_component, input as input_component, panel, panel_header, panel_section,
    panel_toolbar, panel_with_toolbar, section_heading, timecode, toggle as toggle_component,
    tree_row_style,
};
use super::model::{EditorAction, EditorModel};
use super::{Editor, Message};
use crate::document::display_srd_name;

const LABEL_COLUMN: f32 = 284.0;
const MIN_LABEL_COLUMN: f32 = 220.0;
const DIVIDER_HIT_RADIUS: f32 = 5.0;
const ROW_CONTROL_SIZE: f32 = 18.0;
const ROW_CONTROL_MARGIN: f32 = 4.0;
const RULER_HEIGHT: f32 = 22.0;
const ASSIGNMENT_ROW_INSET: u16 = 8;
const ASSIGNMENT_ROW_GAP: f32 = 6.0;
const ASSIGNMENT_TOGGLE_WIDTH: f32 = 26.0;

fn compact_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    value
        .chars()
        .take(max_chars.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

pub(super) fn animation_label(raw: &[u8], index: usize) -> String {
    let name = nonempty_animation_name(raw, format!("Animation {}", index + 1));
    format!("({:02}) {name}", index + 1)
}

fn animate(action: AnimateAction) -> Message {
    Message::Model(EditorAction::Animate(action))
}

// ------------------------------------------------------------- assignments

pub(super) fn assignment_panel(editor: &Editor) -> Element<'_, Message> {
    let state = editor.model.animate();
    let rows = state.assignment_rows();
    let scratch = state.source_set().is_none();

    let set_choices: Vec<AnimationSetChoice> = std::iter::once(AnimationSetChoice {
        index: None,
        label: "No animation set".into(),
    })
    .chain(
        editor
            .model
            .selected_scene()
            .into_iter()
            .flat_map(|scene| scene.animation_sets.iter().enumerate())
            .map(|(index, set)| AnimationSetChoice {
                index: Some(index),
                label: nonempty_animation_name(&set.name, format!("Animation set {}", index + 1)),
            }),
    )
    .collect();
    let selected_set = set_choices
        .iter()
        .find(|choice| choice.index == state.source_set())
        .cloned();
    let set_selector = dropdown_component::single_select(set_choices, selected_set, |choice| {
        animate(AnimateAction::LoadStoredSet(choice.index))
    })
    .placeholder("Select animation set")
    .width(Fill);

    let has_stored_set = state.source_set().is_some();
    let create_set: Element<'_, Message> = if state.editing() {
        console_icon_button(
            Icon::Plus,
            "Create animation set",
            animate(AnimateAction::CreateSet),
            false,
            CYAN,
        )
    } else {
        inactive_property_icon(Icon::Plus)
    };
    let mut set_actions = row![create_set].spacing(2);
    if has_stored_set {
        if state.editing() {
            set_actions = set_actions
                .push(console_icon_button(
                    Icon::Copy,
                    "Duplicate loaded animation set",
                    animate(AnimateAction::DuplicateSet),
                    false,
                    CONSOLE_TEXT,
                ))
                .push(console_icon_button(
                    Icon::Trash2,
                    "Delete loaded animation set",
                    animate(AnimateAction::DeleteSet),
                    false,
                    CONSOLE_MUTED,
                ));
        } else {
            set_actions = set_actions
                .push(inactive_property_icon(Icon::Copy))
                .push(inactive_property_icon(Icon::Trash2));
        }
    }
    let mut animation_set_body = column![
        row![set_selector, set_actions]
            .spacing(4)
            .align_y(Alignment::Center)
    ]
    .spacing(7);
    if has_stored_set {
        animation_set_body = animation_set_body
            .push(console_field(
                "NAME",
                &state.drafts().set_name,
                Fill,
                state.editing(),
                AnimateAction::SetName,
            ))
            .push(
                row![
                    console_field(
                        "START FRAME",
                        &state.drafts().set_start,
                        Fill,
                        state.editing(),
                        AnimateAction::SetStartFrame,
                    ),
                    console_field(
                        "DURATION",
                        &state.drafts().set_duration,
                        Fill,
                        state.editing(),
                        AnimateAction::SetDuration,
                    ),
                ]
                .spacing(7),
            );
    }
    let animation_set = panel_section(
        section_heading(Icon::Layers2, "Animation Set", None),
        animation_set_body,
    );

    let header_content: Element<'_, Message> = if scratch {
        row![
            text("LAYER")
                .size(OVERLINE_SIZE)
                .color(CONSOLE_FAINT)
                .width(Length::FillPortion(2)),
            rule::vertical(1),
            text("BOUND ANIMATION")
                .size(OVERLINE_SIZE)
                .color(CONSOLE_FAINT)
                .width(Length::FillPortion(3)),
        ]
        .spacing(ASSIGNMENT_ROW_GAP)
        .height(ROW_HEIGHT)
        .align_y(Alignment::Center)
        .into()
    } else {
        row![
            text("ON")
                .size(OVERLINE_SIZE)
                .color(CONSOLE_FAINT)
                .width(Length::Fixed(ASSIGNMENT_TOGGLE_WIDTH)),
            text("LAYER")
                .size(OVERLINE_SIZE)
                .color(CONSOLE_FAINT)
                .width(Length::FillPortion(2)),
            rule::vertical(1),
            text("BOUND ANIMATION")
                .size(OVERLINE_SIZE)
                .color(CONSOLE_FAINT)
                .width(Length::FillPortion(3)),
        ]
        .spacing(ASSIGNMENT_ROW_GAP)
        .height(ROW_HEIGHT)
        .align_y(Alignment::Center)
        .into()
    };
    let table_header = container(header_content)
        .padding([0, ASSIGNMENT_ROW_INSET])
        .height(ROW_HEIGHT)
        .width(Fill)
        .align_y(Alignment::Center);
    let mut table = column![].spacing(1).width(Fill);
    if rows.is_empty() {
        table = table.push(console_empty_state(
            "No assignment slots",
            "Load a scene with layers; every slot maps one-to-one to a layer.",
        ));
    }
    for entry in rows {
        table = table.push(assignment_row(editor, entry));
    }
    let bindings = column![table_header, table].spacing(1);

    let mut animation_content = column![animation_management(editor)].spacing(7).width(Fill);
    if let Some(pending) = state.pending_delete() {
        animation_content =
            animation_content
                .push(rule::horizontal(1))
                .push(delete_confirmation_strip(
                    &pending.animation_name,
                    pending.referencing_sets,
                ));
    }
    animation_content = animation_content.push(rule::horizontal(1)).push(bindings);
    let animation = panel_section(
        section_heading(Icon::Library, "Animation", None),
        animation_content,
    );

    panel(
        panel_header(Icon::Layers2, "Animation Assignment", None),
        scrollable(column![animation_set, animation].spacing(0)).height(Fill),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnimationSetChoice {
    index: Option<usize>,
    label: String,
}

impl std::fmt::Display for AnimationSetChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn animation_management(editor: &Editor) -> Element<'_, Message> {
    let state = editor.model.animate();
    let layer_index = state.selected_layer();
    let layer = editor
        .model
        .selected_scene()
        .and_then(|scene| scene.layers.get(layer_index));
    let choices: Vec<AnimationChoice> = layer
        .into_iter()
        .flat_map(|layer| layer.animations.iter().enumerate())
        .map(|(index, animation)| AnimationChoice {
            index: Some(index),
            label: animation_label(&animation.name, index),
        })
        .collect();
    let selected_choice = choices
        .iter()
        .find(|choice| choice.index == state.selected_animation())
        .cloned();
    let picker: Element<'_, Message> = if choices.is_empty() {
        dropdown_component::empty_item("No animations on this layer")
    } else {
        dropdown_component::single_select(choices, selected_choice, move |choice| {
            animate(AnimateAction::SelectAnimation(
                layer_index,
                choice
                    .index
                    .expect("library choices always name an animation"),
            ))
        })
        .placeholder("Select animation")
        .width(Fill)
        .into()
    };
    let authoring_actions = if state.editing() {
        row![
            console_icon_button(
                Icon::Plus,
                "Add an animation to the selected layer",
                animate(AnimateAction::CreateAnimation),
                false,
                CYAN,
            ),
            console_icon_button(
                Icon::Copy,
                "Duplicate the selected animation",
                animate(AnimateAction::DuplicateAnimation),
                false,
                CONSOLE_TEXT,
            ),
            console_icon_button(
                Icon::Trash2,
                "Delete the selected animation",
                animate(AnimateAction::DeleteAnimation),
                false,
                CONSOLE_MUTED,
            ),
        ]
        .spacing(2)
    } else {
        row![
            inactive_property_icon(Icon::Plus),
            inactive_property_icon(Icon::Copy),
            inactive_property_icon(Icon::Trash2),
        ]
        .spacing(2)
    };
    let previewing_selected = state.preview().is_some_and(|preview| {
        preview.layer == layer_index
            && state.selected_animation().is_some_and(|animation_index| {
                layer
                    .and_then(|layer| layer.animations.get(animation_index))
                    .is_some_and(|animation| animation.name == preview.animation_name)
            })
    });
    let library_selector = row![
        picker,
        authoring_actions,
        console_icon_button(
            Icon::MonitorPlay,
            if previewing_selected {
                "Stop previewing the selected animation"
            } else {
                "Preview the selected animation"
            },
            animate(AnimateAction::ToggleAnimationPreview),
            previewing_selected,
            CYAN,
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let content = column![library_selector].spacing(7);
    if state.selected_animation().is_some() {
        content
            .push(
                column![
                    console_field(
                        "NAME",
                        &state.drafts().animation_name,
                        Fill,
                        state.editing(),
                        AnimateAction::SetAnimationName,
                    ),
                    row![
                        console_field(
                            "DURATION",
                            &state.drafts().animation_duration,
                            Fill,
                            state.editing(),
                            AnimateAction::SetAnimationDuration,
                        ),
                        console_field(
                            "FLAGS",
                            &state.drafts().animation_flags,
                            Fill,
                            state.editing(),
                            AnimateAction::SetAnimationFlags,
                        ),
                    ]
                    .spacing(7),
                ]
                .spacing(7),
            )
            .into()
    } else {
        content.into()
    }
}

fn delete_confirmation_strip<'a>(name: &'a str, referencing_sets: usize) -> Element<'a, Message> {
    column![
        section_heading(Icon::AlertTriangle, "Confirm Animation Deletion", None),
        row![
            text(format!(
                "Delete {name}? Clears bindings in {referencing_sets} stored set{}.",
                if referencing_sets == 1 { "" } else { "s" }
            ))
            .size(BODY_SIZE)
            .color(CONSOLE_TEXT),
            Space::new().width(Fill),
            button_component::secondary("Cancel", animate(AnimateAction::CancelDelete)),
            button_component::action("Delete", animate(AnimateAction::ConfirmDelete), RED),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    ]
    .spacing(6)
    .into()
}

fn nonempty_animation_name(raw: &[u8], fallback: String) -> String {
    let name = display_srd_name(raw);
    if name.is_empty() { fallback } else { name }
}

fn assignment_row<'a>(editor: &'a Editor, entry: &'a AssignmentRow) -> Element<'a, Message> {
    let layer = entry.layer;
    let selected = editor.model.animate().selected_layer() == layer;
    let hover_target = (editor.model.selected_scene_index(), layer);
    let hovered = editor.hovered_assignment_layer == Some(hover_target);
    let choices: Vec<AnimationChoice> = std::iter::once(AnimationChoice {
        index: None,
        label: "None".into(),
    })
    .chain(
        editor
            .model
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer))
            .into_iter()
            .flat_map(|layer| layer.animations.iter().enumerate())
            .map(|(index, animation)| AnimationChoice {
                index: Some(index),
                label: animation_label(&animation.name, index),
            }),
    )
    .collect();
    let selected_choice = choices
        .iter()
        .find(|choice| choice.index == entry.animation)
        .cloned();
    let selected_label = selected_choice
        .as_ref()
        .map_or_else(|| "None".to_owned(), |choice| choice.label.clone());
    let enabled = entry.enabled;
    let scratch = editor.model.animate().source_set().is_none();
    let assignment_enabled = scratch || editor.model.document_editing();
    let assignment: Element<'_, Message> = if assignment_enabled {
        dropdown_component::single_select_base(choices, selected_choice, move |choice| {
            animate(AnimateAction::AssignSlotAnimation(layer, choice.index))
        })
        .width(Length::FillPortion(3))
        .style(move |_theme, status| assignment_pick_list_style(selected, hovered, status))
        .into()
    } else {
        dropdown_component::read_only_select(selected_label, Length::FillPortion(3))
    };
    let layer_control: Element<'_, Message> = container(
        text(compact_label(&entry.name, 40))
            .size(BODY_SIZE)
            .color(if enabled { CONSOLE_TEXT } else { CONSOLE_FAINT }),
    )
    .width(Length::FillPortion(2))
    .height(ROW_HEIGHT)
    .align_y(Alignment::Center)
    .clip(true)
    .into();
    let content: Element<'_, Message> = if scratch {
        row![layer_control, rule::vertical(1), assignment]
            .spacing(ASSIGNMENT_ROW_GAP)
            .height(ROW_HEIGHT)
            .align_y(Alignment::Center)
            .into()
    } else {
        let slot_toggle = toggle_component::check("", entry.enabled);
        let slot_toggle = if editor.model.document_editing() {
            slot_toggle.on_toggle(move |_| animate(AnimateAction::ToggleSlotEnabled(layer)))
        } else {
            slot_toggle
        };
        row![
            container(slot_toggle).width(Length::Fixed(ASSIGNMENT_TOGGLE_WIDTH)),
            layer_control,
            rule::vertical(1),
            assignment,
        ]
        .spacing(ASSIGNMENT_ROW_GAP)
        .height(ROW_HEIGHT)
        .align_y(Alignment::Center)
        .into()
    };

    mouse_area(
        container(content)
            .padding([0, ASSIGNMENT_ROW_INSET])
            .height(ROW_HEIGHT)
            .width(Fill)
            .align_y(Alignment::Center)
            .style(move |_| tree_row_style(selected, hovered)),
    )
    .on_press(animate(AnimateAction::SelectLayer(layer)))
    .on_enter(Message::AssignmentLayerHovered(hover_target, true))
    .on_exit(Message::AssignmentLayerHovered(hover_target, false))
    .into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnimationChoice {
    index: Option<usize>,
    label: String,
}

impl std::fmt::Display for AnimationChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

// --------------------------------------------------------------- inspector

/// Edit, layer lock, and CAST-hierarchy lock form one authoring gate.
fn track_authoring_enabled(
    model: &EditorModel,
    scene: usize,
    layer: usize,
    node: Option<usize>,
) -> bool {
    model.document_editing()
        && !model.layer_locked(layer)
        && node.is_none_or(|node| !model.cast_locked(scene, layer, node))
}

/// Premiere-style animation affordances embedded directly beside a shared
/// Inspector field. MOT records stay internal; the user sees only the
/// property stopwatch and the key at the current playhead.
pub(super) fn inspector_property_controls(editor: &Editor, target: u16) -> Element<'_, Message> {
    if editor.model.workspace() != Workspace::Animate {
        return inactive_property_controls();
    }
    let state = editor.model.animate();
    let scene_index = editor.model.selected_scene_index();
    let layer_index = editor.model.selected_layer_index();
    let Some(node_index) = editor.model.selected_node_index() else {
        return inactive_property_controls();
    };
    let animation_index = state
        .assignment_rows()
        .iter()
        .find(|row| row.layer == layer_index)
        .and_then(|row| row.animation)
        .or_else(|| {
            (state.selected_layer() == layer_index)
                .then(|| state.selected_animation())
                .flatten()
        });
    let Some(animation_index) = animation_index else {
        return inactive_property_controls();
    };
    let Some(layer) = editor
        .model
        .selected_scene()
        .and_then(|scene| scene.layers.get(layer_index))
    else {
        return inactive_property_controls();
    };
    if !is_authorable_track_target(layer, node_index, target) {
        return inactive_property_controls();
    }
    let authoring_enabled =
        track_authoring_enabled(&editor.model, scene_index, layer_index, Some(node_index));
    let existing = layer
        .animations
        .get(animation_index)
        .into_iter()
        .flat_map(|animation| animation.motions.iter().enumerate())
        .filter(|(_, motion)| usize::try_from(motion.target).ok() == Some(node_index))
        .find_map(|(motion_index, motion)| {
            motion
                .tracks
                .iter()
                .enumerate()
                .find(|(_, track)| track.target == target)
                .map(|(track_index, track)| {
                    (
                        TrackAddress::new(layer_index, animation_index, motion_index, track_index),
                        track,
                    )
                })
        });
    let (previous, stopwatch, diamond, next): (
        Element<'_, Message>,
        Element<'_, Message>,
        Element<'_, Message>,
        Element<'_, Message>,
    ) = if let Some((address, track)) = existing {
        let (range_start, range_end) = editor.model.animate_play_range();
        let mut has_previous = false;
        let mut has_next = false;
        let mut key_present = false;
        for index in 0..channels::key_count(&track.keys) {
            let Some(frame) = channels::key_frame(&track.keys, index) else {
                continue;
            };
            key_present |= frame == state.frame();
            has_previous |= frame >= range_start && frame < state.frame();
            has_next |= frame <= range_end && frame > state.frame();
        }
        let previous = if has_previous {
            animation_track_icon_button(
                AnimationTrackGlyph::ChevronLeft,
                "Previous keyframe on this property",
                animate(AnimateAction::JumpToPreviousTrackKey(address)),
                false,
            )
        } else {
            inactive_animation_track_icon(AnimationTrackGlyph::ChevronLeft)
        };
        let stopwatch = animation_track_control_button(
            AnimationTrackControl::Stopwatch,
            "Remove animation from this property",
            authoring_enabled.then(|| animate(AnimateAction::ToggleTrackAnimation(address))),
            true,
        );
        let diamond = animation_track_control_button(
            AnimationTrackControl::Keyframe,
            "Add or remove a key at the playhead",
            authoring_enabled
                .then(|| animate(AnimateAction::ToggleKeyAtFrame(address, state.frame()))),
            key_present,
        );
        let next = if has_next {
            animation_track_icon_button(
                AnimationTrackGlyph::ChevronRight,
                "Next keyframe on this property",
                animate(AnimateAction::JumpToNextTrackKey(address)),
                false,
            )
        } else {
            inactive_animation_track_icon(AnimationTrackGlyph::ChevronRight)
        };
        (previous, stopwatch, diamond, next)
    } else {
        let stopwatch = animation_track_control_button(
            AnimationTrackControl::Stopwatch,
            "Animate this property",
            authoring_enabled.then(|| {
                animate(AnimateAction::AddProperty {
                    layer: layer_index,
                    animation: animation_index,
                    node: node_index,
                    target,
                })
            }),
            false,
        );
        (
            inactive_animation_track_icon(AnimationTrackGlyph::ChevronLeft),
            stopwatch,
            animation_track_control_button(
                AnimationTrackControl::Keyframe,
                "Add or remove a key at the playhead",
                None,
                false,
            ),
            inactive_animation_track_icon(AnimationTrackGlyph::ChevronRight),
        )
    };
    row![previous, stopwatch, diamond, next]
        .spacing(1)
        .align_y(Alignment::Center)
        .into()
}

fn inactive_property_controls() -> Element<'static, Message> {
    row![
        inactive_animation_track_icon(AnimationTrackGlyph::ChevronLeft),
        animation_track_control_button(
            AnimationTrackControl::Stopwatch,
            "Animation control unavailable",
            None,
            false,
        ),
        animation_track_control_button(
            AnimationTrackControl::Keyframe,
            "Keyframe control unavailable",
            None,
            false,
        ),
        inactive_animation_track_icon(AnimationTrackGlyph::ChevronRight),
    ]
    .spacing(1)
    .align_y(Alignment::Center)
    .into()
}

fn inactive_property_icon(icon_value: Icon) -> Element<'static, Message> {
    container(super::components::icon(icon_value, 14, CONSOLE_FAINT))
        .width(CONTROL_HEIGHT)
        .height(CONTROL_HEIGHT)
        .center(CONTROL_HEIGHT)
        .into()
}

fn inactive_animation_track_icon(glyph: AnimationTrackGlyph) -> Element<'static, Message> {
    animation_track_glyph(glyph, CONSOLE_FAINT)
}

/// Only focused key metadata remains below the component Inspector. Property
/// creation and values live on the component rows above.
pub(super) fn animation_inspector_section(editor: &Editor) -> Option<Element<'_, Message>> {
    focused_key_sections(editor)
        .map(|body| panel_section(section_heading(Icon::Diamond, "Selected Key", None), body))
}

fn focused_key_sections(editor: &Editor) -> Option<Element<'_, Message>> {
    let state = editor.model.animate();
    let (motion_index, track_index) = state.selected_track()?;
    let (animation_index, animation) = editor.model.animate_active_animation()?;
    let track = animation
        .motions
        .get(motion_index)?
        .tracks
        .get(track_index)?;
    let value_kind = channels::value_kind(track.target);
    let address = TrackAddress::new(
        state.selected_layer(),
        animation_index,
        motion_index,
        track_index,
    );
    let key_index = state.selected_key()?;
    let view = channels::key_view(&track.keys, key_index)?;

    let mut body = column![
        row![
            console_tag(format!("KEY F{}", view.frame), CONSOLE_MUTED),
            text(channels::channel_name(track.target))
                .size(BODY_SIZE)
                .color(CONSOLE_TEXT),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        console_field(
            "FRAME",
            &state.drafts().key_frame,
            Length::Fixed(84.0),
            state.editing(),
            AnimateAction::SetKeyFrame,
        ),
    ]
    .spacing(6)
    .width(Fill);

    match value_kind {
        ChannelValue::Rgba => {
            let bytes = &state.drafts().key_rgba;
            body = body.push(
                row![
                    console_field(
                        "R",
                        &bytes[0],
                        Length::Fixed(56.0),
                        state.editing(),
                        |value| { AnimateAction::SetKeyByte(0, value) }
                    ),
                    console_field(
                        "G",
                        &bytes[1],
                        Length::Fixed(56.0),
                        state.editing(),
                        |value| { AnimateAction::SetKeyByte(1, value) }
                    ),
                    console_field(
                        "B",
                        &bytes[2],
                        Length::Fixed(56.0),
                        state.editing(),
                        |value| { AnimateAction::SetKeyByte(2, value) }
                    ),
                    console_field(
                        "A",
                        &bytes[3],
                        Length::Fixed(56.0),
                        state.editing(),
                        |value| { AnimateAction::SetKeyByte(3, value) }
                    ),
                ]
                .spacing(4),
            );
        }
        ChannelValue::Selector => {
            body = body.push(console_field(
                "SELECTOR INDEX",
                &state.drafts().key_value,
                Fill,
                state.editing(),
                AnimateAction::SetKeyValue,
            ));
        }
        ChannelValue::Float | ChannelValue::Integer => {
            body = body.push(console_field(
                if value_kind == ChannelValue::Float {
                    "VALUE · f32"
                } else {
                    "VALUE · i32"
                },
                &state.drafts().key_value,
                Fill,
                state.editing(),
                AnimateAction::SetKeyValue,
            ));
        }
    }

    let available = channels::available_semantics(track.format);
    let mut semantics_row = row![
        text("INTERPOLATION")
            .size(OVERLINE_SIZE)
            .color(CONSOLE_FAINT)
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    for semantics in KeySemantics::ALL {
        if !available.contains(&semantics) {
            continue;
        }
        let selected = view.semantics == semantics;
        let control = button(text(semantics.label()).size(BODY_SIZE))
            .height(ROW_HEIGHT)
            .padding([0, 8])
            .style(move |_theme, status| console_button_style(CONSOLE_TEXT, selected, status));
        let control = if state.editing() {
            control.on_press(animate(AnimateAction::SetKeySemantics(semantics)))
        } else {
            control
        };
        semantics_row = semantics_row.push(control);
    }
    body = body.push(semantics_row);
    if view.semantics.uses_slopes() {
        body = body.push(
            row![
                console_field(
                    "SLOPE IN",
                    &state.drafts().slope_in,
                    Length::Fixed(84.0),
                    state.editing(),
                    |value| AnimateAction::SetKeySlope(true, value),
                ),
                console_field(
                    "SLOPE OUT",
                    &state.drafts().slope_out,
                    Length::Fixed(84.0),
                    state.editing(),
                    |value| AnimateAction::SetKeySlope(false, value),
                ),
            ]
            .spacing(6),
        );
    }
    let delete_key = button(text("Delete key").size(BODY_SIZE))
        .height(ROW_HEIGHT)
        .padding([0, 8])
        .style(move |_theme, status| console_button_style(CONSOLE_MUTED, false, status));
    let delete_key = if state.editing() {
        delete_key.on_press(animate(AnimateAction::DeleteKey(address, key_index)))
    } else {
        delete_key
    };
    body = body.push(delete_key);
    Some(body.into())
}

fn console_field<'a>(
    label: &'a str,
    value: &str,
    width: Length,
    editing: bool,
    action: impl Fn(String) -> AnimateAction + 'a,
) -> Element<'a, Message> {
    let input = input_component::field("", value);
    let input = if editing {
        input.on_input(move |value| animate(action(value)))
    } else {
        input
    };
    column![text(label).size(OVERLINE_SIZE).color(CONSOLE_FAINT), input]
        .spacing(2)
        .width(width)
        .into()
}

// -------------------------------------------------------------- dope sheet

pub(super) fn dope_sheet_panel(editor: &Editor) -> Element<'_, Message> {
    let state = editor.model.animate();
    let sheet = state.sheet();
    let play_range = editor.model.animate_play_range();
    let runtime_range = editor.model.animate_runtime_range();
    let selected_track = selected_track_address(state);
    let controls = row![
        text(timecode(state.frame()))
            .size(TIMECODE_SIZE)
            .color(CONSOLE_TEXT),
        Space::new().width(Fill),
        console_icon_button(
            Icon::ZoomOut,
            "Zoom out",
            animate(AnimateAction::ZoomBy(-0.25)),
            false,
            CONSOLE_TEXT,
        ),
        console_icon_button(
            Icon::ZoomIn,
            "Zoom in",
            animate(AnimateAction::ZoomBy(0.25)),
            false,
            CONSOLE_TEXT,
        ),
        console_icon_button(
            Icon::MoveLeft,
            "Pan back",
            animate(AnimateAction::PanBy(-20.0)),
            false,
            CONSOLE_TEXT,
        ),
        console_icon_button(
            Icon::MoveRight,
            "Pan forward",
            animate(AnimateAction::PanBy(20.0)),
            false,
            CONSOLE_TEXT,
        ),
    ]
    .spacing(4)
    .height(CONTROL_HEIGHT)
    .align_y(Alignment::Center);

    let has_active_animation = state.preview().is_some()
        || state
            .assignment_rows()
            .iter()
            .any(|row| row.enabled && row.animation.is_some());
    let (empty_title, empty_instruction) = if has_active_animation {
        (
            "No animated properties",
            "The active animation has no supported property tracks to show.",
        )
    } else {
        (
            "No active animation",
            "Bind an animation to a layer in Animation Assignment to show its properties here.",
        )
    };
    let body: Element<'_, Message> = if sheet.visible.is_empty() {
        container(console_empty_state(empty_title, empty_instruction))
            .width(Fill)
            .height(Fill)
            .into()
    } else {
        Canvas::new(DopeSheetCanvas {
            scene: editor.model.selected_scene_index(),
            model: &editor.model,
            sheet,
            frame: state.frame(),
            zoom: state.zoom(),
            pan: state.pan(),
            scroll: state.scroll(),
            play_range,
            runtime_range,
            selected_track,
            selected_key: state.selected_key(),
        })
        .width(Fill)
        .height(Fill)
        .into()
    };

    panel_with_toolbar(
        panel_header(Icon::ChartNoAxesGantt, "Dope Sheet", None),
        panel_toolbar(controls),
        body,
    )
}

fn selected_track_address(state: &super::animate::AnimateState) -> Option<TrackAddress> {
    state
        .selected_track()
        .zip(state.selected_animation())
        .map(|((motion, track), animation)| {
            TrackAddress::new(state.selected_layer(), animation, motion, track)
        })
}

/// Batched dope-sheet renderer. Rows and keys are drawn straight from the
/// model's flat arrays and clipped to the visible window, so cost tracks the
/// visible area rather than the total node or key count.
struct DopeSheetCanvas<'a> {
    scene: usize,
    model: &'a EditorModel,
    sheet: &'a DopeSheet,
    frame: i32,
    zoom: f32,
    pan: f32,
    scroll: f32,
    play_range: (i32, i32),
    runtime_range: (i32, i32),
    selected_track: Option<TrackAddress>,
    selected_key: Option<usize>,
}

#[derive(Debug)]
struct DopeSheetState {
    drag: Option<KeyDrag>,
    label_width: f32,
    resizing_labels: bool,
}

impl Default for DopeSheetState {
    fn default() -> Self {
        Self {
            drag: None,
            label_width: LABEL_COLUMN,
            resizing_labels: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyDrag {
    address: TrackAddress,
    row: usize,
    key: usize,
    frame: i32,
}

/// Index of the same key after `set_key_frame` changes its frame then stably
/// sorts the vector. Equal-frame keys retain their original order.
fn relocated_key_index(keys: &[super::animate::DopeKey], key: usize, frame: i32) -> usize {
    keys.iter()
        .enumerate()
        .filter(|(index, other)| {
            *index != key && (other.frame < frame || (other.frame == frame && *index < key))
        })
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DopeRowControl {
    Previous,
    Animation,
    Key,
    Next,
}

const DOPE_ROW_CONTROLS: [DopeRowControl; 4] = [
    DopeRowControl::Previous,
    DopeRowControl::Animation,
    DopeRowControl::Key,
    DopeRowControl::Next,
];

fn dope_row_controls_start(label_width: f32) -> f32 {
    label_width - ROW_CONTROL_MARGIN - ROW_CONTROL_SIZE * DOPE_ROW_CONTROLS.len() as f32
}

fn dope_row_control_x(label_width: f32, control: DopeRowControl) -> f32 {
    let index = DOPE_ROW_CONTROLS
        .iter()
        .position(|candidate| *candidate == control)
        .unwrap_or(0);
    dope_row_controls_start(label_width) + index as f32 * ROW_CONTROL_SIZE
}

fn dope_row_control_at(x: f32, label_width: f32) -> Option<DopeRowControl> {
    let start = dope_row_controls_start(label_width);
    if x < start || x >= label_width - ROW_CONTROL_MARGIN {
        return None;
    }
    DOPE_ROW_CONTROLS
        .get(((x - start) / ROW_CONTROL_SIZE) as usize)
        .copied()
}

impl DopeSheetCanvas<'_> {
    fn row_authoring_enabled(&self, row: &DopeRow) -> bool {
        track_authoring_enabled(self.model, self.scene, row.layer, row.node)
    }

    fn row_control_enabled(
        &self,
        row_index: usize,
        row: &DopeRow,
        control: DopeRowControl,
    ) -> bool {
        match control {
            DopeRowControl::Previous => self.sheet.row_keys(row_index).iter().any(|key| {
                key.frame >= self.play_range.0
                    && key.frame <= self.play_range.1
                    && key.frame < self.frame
            }),
            DopeRowControl::Animation | DopeRowControl::Key => self.row_authoring_enabled(row),
            DopeRowControl::Next => self.sheet.row_keys(row_index).iter().any(|key| {
                key.frame >= self.play_range.0
                    && key.frame <= self.play_range.1
                    && key.frame > self.frame
            }),
        }
    }

    fn label_width(&self, state: &DopeSheetState, bounds: &Rectangle) -> f32 {
        let maximum = (bounds.width - DOPE_MIN_TIMELINE_WIDTH).max(MIN_LABEL_COLUMN);
        state.label_width.clamp(MIN_LABEL_COLUMN, maximum)
    }

    fn frame_to_x(&self, bounds: &Rectangle, label_width: f32, frame: f32) -> f32 {
        bounds.x + label_width + (frame - self.pan) * self.zoom
    }

    fn x_to_frame(&self, bounds: &Rectangle, label_width: f32, x: f32) -> i32 {
        (((x - bounds.x - label_width) / self.zoom.max(0.01)) + self.pan).round() as i32
    }

    /// First and last frame currently visible in the track area.
    fn visible_frames(&self, bounds: &Rectangle, label_width: f32) -> (f32, f32) {
        let span = (bounds.width - label_width).max(1.0) / self.zoom.max(0.01);
        (self.pan, self.pan + span)
    }

    /// Offsets into `sheet.visible` that intersect the vertical viewport.
    fn visible_range(&self, bounds: &Rectangle) -> (usize, usize) {
        let first = (self.scroll / DOPE_ROW_HEIGHT).floor().max(0.0) as usize;
        let count = ((bounds.height - RULER_HEIGHT) / DOPE_ROW_HEIGHT).ceil() as usize + 1;
        (
            first.min(self.sheet.visible.len()),
            (first + count).min(self.sheet.visible.len()),
        )
    }

    fn row_at(&self, bounds: &Rectangle, y: f32) -> Option<usize> {
        let local = y - bounds.y - RULER_HEIGHT + self.scroll;
        if local < 0.0 {
            return None;
        }
        let slot = (local / DOPE_ROW_HEIGHT) as usize;
        self.sheet.visible.get(slot).map(|row| *row as usize)
    }
}

impl Program<Message> for DopeSheetCanvas<'_> {
    type State = DopeSheetState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let label_width = self.label_width(state, &bounds);
        if let canvas::Event::Mouse(mouse::Event::CursorMoved { position }) = event
            && state.resizing_labels
        {
            let maximum = (bounds.width - DOPE_MIN_TIMELINE_WIDTH).max(MIN_LABEL_COLUMN);
            state.label_width = (position.x - bounds.x).clamp(MIN_LABEL_COLUMN, maximum);
            return Some(canvas::Action::request_redraw().and_capture());
        }
        if matches!(
            event,
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. })
        ) && state.drag.as_ref().is_some_and(|drag| {
            self.sheet
                .rows
                .get(drag.row)
                .is_none_or(|row| !self.row_authoring_enabled(row))
        }) {
            state.drag = None;
            return Some(canvas::Action::request_redraw().and_capture());
        }
        if let canvas::Event::Mouse(mouse::Event::CursorMoved { position }) = event
            && let Some(drag) = &mut state.drag
        {
            let frame = self.x_to_frame(&bounds, label_width, position.x);
            if frame != drag.frame {
                let key = drag.key;
                drag.key = relocated_key_index(self.sheet.row_keys(drag.row), key, frame);
                drag.frame = frame;
                return Some(
                    canvas::Action::publish(animate(AnimateAction::MoveKey(
                        drag.address,
                        key,
                        frame,
                    )))
                    .and_capture(),
                );
            }
            return Some(canvas::Action::request_redraw().and_capture());
        }
        if let canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
            let finished_drag = state.resizing_labels || state.drag.is_some();
            state.resizing_labels = false;
            state.drag = None;
            if finished_drag {
                return Some(canvas::Action::request_redraw().and_capture());
            }
        }

        let point = cursor.position_in(bounds)?;
        let absolute = Point::new(bounds.x + point.x, bounds.y + point.y);

        if let canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let (pixel_x, pixel_y) = match delta {
                mouse::ScrollDelta::Lines { x, y } => (*x * 36.0, *y * 36.0),
                mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
            };
            if pixel_x.abs() > f32::EPSILON {
                return Some(
                    canvas::Action::publish(animate(AnimateAction::PanBy(
                        -pixel_x / self.zoom.max(0.01),
                    )))
                    .and_capture(),
                );
            }
            if pixel_y.abs() > f32::EPSILON {
                return Some(
                    canvas::Action::publish(animate(AnimateAction::ScrollBy(
                        -pixel_y,
                        (bounds.height - RULER_HEIGHT).max(0.0),
                    )))
                    .and_capture(),
                );
            }
        }

        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if (point.x - label_width).abs() <= DIVIDER_HIT_RADIUS {
                state.resizing_labels = true;
                state.drag = None;
                return Some(canvas::Action::request_redraw().and_capture());
            }
            // Ruler: scrub the playhead.
            if point.y < RULER_HEIGHT && point.x > label_width {
                let frame = self.x_to_frame(&bounds, label_width, absolute.x);
                return Some(
                    canvas::Action::publish(animate(AnimateAction::SetFrame(frame))).and_capture(),
                );
            }
            let row_index = self.row_at(&bounds, absolute.y)?;
            let row = self.sheet.rows.get(row_index)?;
            if let (Some(address), Some(control)) =
                (row.track, dope_row_control_at(point.x, label_width))
            {
                if !self.row_control_enabled(row_index, row, control) {
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                let message = match control {
                    DopeRowControl::Previous => {
                        animate(AnimateAction::JumpToPreviousTrackKey(address))
                    }
                    DopeRowControl::Animation => {
                        animate(AnimateAction::ToggleTrackAnimation(address))
                    }
                    DopeRowControl::Key => {
                        animate(AnimateAction::ToggleKeyAtFrame(address, self.frame))
                    }
                    DopeRowControl::Next => animate(AnimateAction::JumpToNextTrackKey(address)),
                };
                return Some(canvas::Action::publish(message).and_capture());
            }
            // The chevron collapses a CAST; its breadcrumb selects that concrete
            // CAST, while a channel label selects its cross-layer address.
            if point.x <= label_width {
                let collapse_width = 18.0 + f32::from(row.depth) * 10.0;
                let message = if row.has_children && point.x <= collapse_width {
                    animate(AnimateAction::ToggleRowCollapsed(row_index))
                } else if let Some(address) = row.track {
                    animate(AnimateAction::SelectTrack(address))
                } else if let Some(node) = row.node {
                    Message::Model(EditorAction::SelectCast(self.scene, row.layer, node))
                } else {
                    animate(AnimateAction::SelectLayer(row.layer))
                };
                return Some(canvas::Action::publish(message).and_capture());
            }
            let frame = self.x_to_frame(&bounds, label_width, absolute.x);
            let Some(address) = row.track else {
                return Some(
                    canvas::Action::publish(animate(AnimateAction::SetFrame(frame))).and_capture(),
                );
            };
            let hit = self
                .sheet
                .row_keys(row_index)
                .iter()
                .enumerate()
                .find(|(_, key)| {
                    (self.frame_to_x(&bounds, label_width, key.frame as f32) - absolute.x).abs()
                        <= 5.0
                });
            let authoring_enabled = self.row_authoring_enabled(row);
            return match hit {
                Some((offset, key)) => {
                    if authoring_enabled {
                        state.drag = Some(KeyDrag {
                            address,
                            row: row_index,
                            key: offset,
                            frame: key.frame,
                        });
                    }
                    Some(
                        canvas::Action::publish(animate(AnimateAction::SelectKey(address, offset)))
                            .and_capture(),
                    )
                }
                None if authoring_enabled => Some(
                    canvas::Action::publish(animate(AnimateAction::ToggleKeyAtFrame(
                        address, frame,
                    )))
                    .and_capture(),
                ),
                None => Some(
                    canvas::Action::publish(animate(AnimateAction::SetFrame(frame))).and_capture(),
                ),
            };
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let label_width = self.label_width(state, &bounds);
        let (visible_start, visible_end) = self.visible_frames(&bounds, label_width);
        let cursor = cursor.position_in(bounds);

        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            Size::new(label_width, RULER_HEIGHT),
            CONSOLE_WELL,
        );
        frame.fill_text(CanvasText {
            content: "PROPERTY / CHANNEL".into(),
            position: Point::new(8.0, 5.0),
            color: CONSOLE_MUTED,
            size: OVERLINE_SIZE.into(),
            ..CanvasText::default()
        });
        frame.fill_text(CanvasText {
            content: "FRAME →".into(),
            position: Point::new(label_width - 47.0, 5.0),
            color: CYAN,
            size: OVERLINE_SIZE.into(),
            ..CanvasText::default()
        });

        // Out-of-range shading: everything past the runtime range is dimmed,
        // because the runtime never evaluates those frames.
        let (runtime_start, runtime_end) = self.runtime_range;
        let runtime_x0 = (self.frame_to_x(&bounds, label_width, runtime_start as f32) - bounds.x)
            .max(label_width);
        let runtime_x1 = (self.frame_to_x(&bounds, label_width, runtime_end as f32) - bounds.x)
            .min(bounds.width);
        if runtime_x0 > label_width {
            frame.fill_rectangle(
                Point::new(label_width, 0.0),
                Size::new(runtime_x0 - label_width, bounds.height),
                Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            );
        }
        if runtime_x1 < bounds.width {
            frame.fill_rectangle(
                Point::new(runtime_x1, 0.0),
                Size::new(bounds.width - runtime_x1, bounds.height),
                Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            );
        }

        // Work-area band across the ruler. Cyan is reserved for active timing state.
        let (play_start, play_end) = self.play_range;
        let play_x0 =
            (self.frame_to_x(&bounds, label_width, play_start as f32) - bounds.x).max(label_width);
        let play_x1 =
            (self.frame_to_x(&bounds, label_width, play_end as f32) - bounds.x).min(bounds.width);
        if play_x1 > play_x0 {
            frame.fill_rectangle(
                Point::new(play_x0, 0.0),
                Size::new(play_x1 - play_x0, RULER_HEIGHT),
                Color { a: 0.12, ..CYAN },
            );
        }

        // Ruler ticks: stepped so labels stay readable at any zoom.
        let step = tick_step(self.zoom);
        let mut tick = ((visible_start / step as f32).floor() as i32) * step;
        while (tick as f32) <= visible_end {
            let x = self.frame_to_x(&bounds, label_width, tick as f32) - bounds.x;
            if x >= label_width && x + 32.0 <= bounds.width {
                frame.fill_rectangle(
                    Point::new(x, 0.0),
                    Size::new(1.0, RULER_HEIGHT),
                    CONSOLE_LINE,
                );
                frame.fill_text(CanvasText {
                    content: format!("{tick}f"),
                    position: Point::new(x + 3.0, 4.0),
                    color: CONSOLE_FAINT,
                    size: OVERLINE_SIZE.into(),
                    ..CanvasText::default()
                });
            }
            tick += step;
        }

        frame.fill_rectangle(
            Point::new(0.0, RULER_HEIGHT - 1.0),
            Size::new(bounds.width, 1.0),
            CONSOLE_LINE,
        );

        // Rows: only the vertical window is touched.
        let (first_row, last_row) = self.visible_range(&bounds);
        for slot in first_row..last_row {
            let row_index = self.sheet.visible[slot] as usize;
            let Some(row) = self.sheet.rows.get(row_index) else {
                continue;
            };
            let y = RULER_HEIGHT + slot as f32 * DOPE_ROW_HEIGHT - self.scroll;
            if y < RULER_HEIGHT || y + DOPE_ROW_HEIGHT > bounds.height {
                continue;
            }
            let selected = row.track.is_some() && row.track == self.selected_track;
            if selected {
                frame.fill_rectangle(
                    Point::new(0.0, y),
                    Size::new(bounds.width, DOPE_ROW_HEIGHT),
                    Color { a: 0.10, ..CYAN },
                );
            } else if slot % 2 == 1 {
                frame.fill_rectangle(
                    Point::new(0.0, y),
                    Size::new(bounds.width, DOPE_ROW_HEIGHT),
                    Color::from_rgba(1.0, 1.0, 1.0, 0.015),
                );
            }
            frame.fill_rectangle(
                Point::new(0.0, y + DOPE_ROW_HEIGHT - 1.0),
                Size::new(bounds.width, 1.0),
                Color {
                    a: 0.5,
                    ..CONSOLE_LINE
                },
            );

            let indent = 6.0 + f32::from(row.depth) * 10.0;
            let label_color = if selected {
                CYAN
            } else if row.has_children {
                CONSOLE_TEXT
            } else {
                CONSOLE_MUTED
            };
            if row.has_children {
                frame.fill_text(CanvasText {
                    content: if row.collapsed {
                        "▸".into()
                    } else {
                        "▾".into()
                    },
                    position: Point::new(indent - 4.0, y + 4.0),
                    color: CONSOLE_FAINT,
                    size: OVERLINE_SIZE.into(),
                    ..CanvasText::default()
                });
            }
            let track_controls = row.track.is_some();
            let label_right = if track_controls {
                dope_row_controls_start(label_width) - 4.0
            } else {
                label_width - 32.0
            };
            let label_text_width = (label_right - indent - 8.0).max(24.0);
            frame.fill_text(CanvasText {
                content: compact_path_label(&row.label, (label_text_width / 6.2).floor() as usize),
                position: Point::new(indent + 8.0, y + 4.0),
                color: label_color,
                size: BODY_SIZE.into(),
                max_width: label_text_width,
                ..CanvasText::default()
            });
            if track_controls {
                let key_present = self
                    .sheet
                    .row_keys(row_index)
                    .iter()
                    .any(|key| key.frame == self.frame);
                for control in DOPE_ROW_CONTROLS {
                    let x = dope_row_control_x(label_width, control);
                    let enabled = self.row_control_enabled(row_index, row, control);
                    let (glyph, color) = match control {
                        DopeRowControl::Previous => (
                            AnimationTrackGlyph::ChevronLeft,
                            if enabled { CONSOLE_TEXT } else { CONSOLE_FAINT },
                        ),
                        DopeRowControl::Animation => {
                            let visual = animation_track_visual(
                                AnimationTrackControl::Stopwatch,
                                true,
                                enabled,
                            );
                            (visual.glyph, visual.color)
                        }
                        DopeRowControl::Key => {
                            let visual = animation_track_visual(
                                AnimationTrackControl::Keyframe,
                                key_present,
                                enabled,
                            );
                            (visual.glyph, visual.color)
                        }
                        DopeRowControl::Next => (
                            AnimationTrackGlyph::ChevronRight,
                            if enabled { CONSOLE_TEXT } else { CONSOLE_FAINT },
                        ),
                    };
                    let hovered = enabled
                        && cursor.is_some_and(|point| {
                            point.y >= y
                                && point.y < y + DOPE_ROW_HEIGHT
                                && dope_row_control_at(point.x, label_width) == Some(control)
                        });
                    if hovered {
                        frame.fill_rectangle(
                            Point::new(x, y + 2.0),
                            Size::new(ROW_CONTROL_SIZE, DOPE_ROW_HEIGHT - 4.0),
                            Color {
                                a: 0.10,
                                ..CONSOLE_TEXT
                            },
                        );
                    }
                    draw_animation_track_glyph(
                        &mut frame,
                        glyph,
                        Point::new(x + ROW_CONTROL_SIZE / 2.0, y + DOPE_ROW_HEIGHT / 2.0),
                        ANIMATION_TRACK_GLYPH_SIZE,
                        color,
                    );
                }
            } else {
                frame.fill_text(CanvasText {
                    content: row.layer_badge.clone(),
                    position: Point::new(label_width - 28.0, y + 4.0),
                    color: CONSOLE_FAINT,
                    size: OVERLINE_SIZE.into(),
                    ..CanvasText::default()
                });
            }

            let center = y + DOPE_ROW_HEIGHT / 2.0;
            for (offset, key) in self.sheet.row_keys(row_index).iter().enumerate() {
                let x = self.frame_to_x(&bounds, label_width, key.frame as f32) - bounds.x;
                if x < label_width || x > bounds.width {
                    continue;
                }
                let is_selected = selected && self.selected_key == Some(offset);
                let color = Color {
                    a: if key.out_of_range { 0.42 } else { 0.86 },
                    ..if is_selected { CYAN } else { CONSOLE_TEXT }
                };
                let radius = if is_selected { 5.0 } else { 3.5 };
                let diamond = Path::new(|builder| {
                    builder.move_to(Point::new(x, center - radius));
                    builder.line_to(Point::new(x + radius, center));
                    builder.line_to(Point::new(x, center + radius));
                    builder.line_to(Point::new(x - radius, center));
                    builder.close();
                });
                frame.fill(&diamond, color);
                if is_selected {
                    frame.stroke(
                        &diamond,
                        Stroke::default().with_color(CONSOLE_TEXT).with_width(1.0),
                    );
                }
            }
        }

        // Label gutter separator and playhead.
        frame.fill_rectangle(
            Point::new(label_width - 1.0, 0.0),
            Size::new(2.0, bounds.height),
            CONSOLE_LINE,
        );
        frame.fill_rectangle(
            Point::new(label_width - 1.5, 7.0),
            Size::new(3.0, 8.0),
            Color { a: 0.65, ..CYAN },
        );
        let playhead = self.frame_to_x(&bounds, label_width, self.frame as f32) - bounds.x;
        if playhead >= label_width && playhead <= bounds.width {
            frame.fill_rectangle(
                Point::new(playhead, 0.0),
                Size::new(1.0, bounds.height),
                CYAN,
            );
            frame.fill_rectangle(Point::new(playhead - 3.0, 0.0), Size::new(7.0, 4.0), CYAN);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let label_width = self.label_width(state, &bounds);
        if state.resizing_labels {
            return mouse::Interaction::ResizingHorizontally;
        }
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let Some(point) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        if (point.x - label_width).abs() <= DIVIDER_HIT_RADIUS {
            return mouse::Interaction::ResizingHorizontally;
        }
        if let Some(control) = dope_row_control_at(point.x, label_width)
            && let Some(row_index) = self.row_at(&bounds, bounds.y + point.y)
            && let Some(row) = self.sheet.rows.get(row_index)
            && row.track.is_some()
            && self.row_control_enabled(row_index, row, control)
        {
            return mouse::Interaction::Pointer;
        }
        if point.x > label_width {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

fn compact_path_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let leaf = value.rsplit(" / ").next().unwrap_or(value);
    let abbreviated = format!("… / {leaf}");
    if abbreviated.chars().count() <= max_chars {
        abbreviated
    } else {
        compact_label(leaf, max_chars)
    }
}

/// Ruler step chosen so labels never collide at the current zoom.
fn tick_step(zoom: f32) -> i32 {
    for step in [1, 2, 5, 10, 20, 30, 60, 120, 300, 600, 1800, 3600] {
        if step as f32 * zoom >= 46.0 {
            return step;
        }
    }
    7200
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruler_step_grows_as_zoom_shrinks() {
        assert_eq!(tick_step(64.0), 1);
        assert_eq!(tick_step(6.0), 10);
        assert!(tick_step(0.5) >= 120);
    }

    #[test]
    fn narrow_assignment_labels_are_ellipsized_without_overflow() {
        assert_eq!(compact_label("short", 17), "short");
        assert_eq!(
            compact_label("L_Advertise_common_black", 17),
            "L_Advertise_comm…"
        );
    }

    #[test]
    fn animation_choices_put_the_padded_index_first() {
        assert_eq!(animation_label(b"Move", 0), "(01) Move");
        assert_eq!(animation_label(b"", 11), "(12) Animation 12");
    }

    #[test]
    fn dope_paths_stay_on_one_line_and_preserve_the_leaf() {
        assert_eq!(
            compact_path_label("Root / LongMiddle / VisibleLeaf", 16),
            "… / VisibleLeaf"
        );
        assert_eq!(compact_path_label("VeryLongVisibleLeaf", 9), "VeryLong…");
    }
    #[test]
    fn every_dope_track_control_has_a_distinct_row_hit_target() {
        for control in DOPE_ROW_CONTROLS {
            let center = dope_row_control_x(LABEL_COLUMN, control) + ROW_CONTROL_SIZE / 2.0;
            assert_eq!(dope_row_control_at(center, LABEL_COLUMN), Some(control));
        }
        assert_eq!(
            dope_row_control_at(dope_row_controls_start(LABEL_COLUMN) - 1.0, LABEL_COLUMN),
            None
        );
        assert_eq!(
            dope_row_control_at(LABEL_COLUMN - ROW_CONTROL_MARGIN, LABEL_COLUMN),
            None
        );
    }

    #[test]
    fn dope_sheet_label_timeline_divider_drags_with_bounded_width() {
        let sheet = DopeSheet::default();
        let model = EditorModel::default();
        let canvas = DopeSheetCanvas {
            scene: 0,
            model: &model,
            sheet: &sheet,
            frame: 0,
            zoom: 6.0,
            pan: 0.0,
            scroll: 0.0,
            play_range: (0, 120),
            runtime_range: (0, 120),
            selected_track: None,
            selected_key: None,
        };
        let bounds = Rectangle::new(Point::new(10.0, 20.0), Size::new(640.0, 300.0));
        let mut state = DopeSheetState::default();

        let press = canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let cursor = mouse::Cursor::Available(Point::new(10.0 + LABEL_COLUMN, 30.0));
        assert!(canvas.update(&mut state, &press, bounds, cursor).is_some());
        assert!(state.resizing_labels);

        let move_to_360 = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(370.0, 30.0),
        });
        assert!(
            canvas
                .update(&mut state, &move_to_360, bounds, cursor)
                .is_some()
        );
        assert_eq!(state.label_width, 360.0);
        assert_eq!(canvas.frame_to_x(&bounds, state.label_width, 0.0), 370.0);

        let move_past_maximum = canvas::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(1_000.0, 30.0),
        });
        let _ = canvas.update(&mut state, &move_past_maximum, bounds, cursor);
        assert_eq!(state.label_width, 460.0);

        let release = canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(
            canvas
                .update(&mut state, &release, bounds, cursor)
                .is_some()
        );
        assert!(!state.resizing_labels);
        assert_eq!(
            canvas.mouse_interaction(
                &state,
                bounds,
                mouse::Cursor::Available(Point::new(470.0, 30.0)),
            ),
            mouse::Interaction::ResizingHorizontally
        );
    }

    #[test]
    fn crossing_a_neighbour_relocates_the_held_key_index() {
        let keys = [
            crate::editor::animate::DopeKey {
                row: 0,
                frame: 0,
                index: 0,
                semantics: KeySemantics::Linear,
                out_of_range: false,
            },
            crate::editor::animate::DopeKey {
                row: 0,
                frame: 45,
                index: 1,
                semantics: KeySemantics::Linear,
                out_of_range: false,
            },
        ];

        assert_eq!(relocated_key_index(&keys, 0, 50), 1);
    }
}
