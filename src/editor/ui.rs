use iced::mouse;
use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Image as CanvasImage, Path, Program, Stroke,
};
use iced::widget::image::FilterMethod;
use iced::widget::pane_grid;
use iced::widget::{
    Space, button, center, column, container, image, mouse_area, pin, row, scrollable, stack, text,
};
use iced::{Alignment, Color, Element, Fill, Length, Padding, Point, Rectangle, Size, Theme};
use lucide_icons::Icon;

use super::animate::{AnimateAction, Workspace};
use super::components::{
    BODY_SIZE, CAPTION_SIZE, CONTROL_HEIGHT, CYAN, DIALOG_TITLE_SIZE, MUTED, OVERLINE_SIZE,
    PANEL_TITLE_SIZE, RED, ROW_HEIGHT, SECTION_GAP, TEXT, TIMECODE_SIZE, YELLOW, accent_bar,
    app_background, button as button_component, cast_icon, console_button_style,
    dialog as dialog_component, divider_vertical, dropdown as dropdown_component, empty_state,
    fill_portion, fitted_rect, floating_badge_style, ghost_button, hierarchy_icon_button, icon,
    icon_button, input as input_component, labeled_control, layer_tree_controls_style,
    layer_tree_row_style, mini_icon_button, nonempty_name, notice_style, panel, panel_body,
    panel_header, panel_section, panel_toolbar, panel_with_toolbar, parse_or, passive_icon,
    property_row, resizable_panes, section_header_button_style, section_heading, section_surface,
    sub_bar_style, surface_style, timecode, toggle as toggle_component, top_bar_style,
    tree_controls_style, tree_row_style,
};
use super::model::{
    CanvasTransformEdit, CastRoleDraft, EditorAction, ImageBindingDraft, InspectorField,
    InspectorSection, TransformField, TransformTool, VisualGeometryDraft,
};
use super::{
    ContextDropdown, EDITOR_PREVIEW_SIZE, Editor, ItemDialog, Message, PaneKind,
    PreviewCastSelection,
};
use crate::document::display_srd_name;
use crate::render::select_srd_image_render_preset;
use crate::scene::{CastClassification, Hierarchy, Layer, NodeRecord, SrCastKind};

pub fn view(editor: &Editor) -> Element<'_, Message> {
    let mut body = column![top_bar(editor), timeline_context_bar(editor)]
        .spacing(0)
        .width(Fill)
        .height(Fill);
    if let Some(error) = editor.preview_error.as_deref() {
        body = body.push(error_bar(error));
    } else if let Some(error) = editor.model.load_error() {
        body = body.push(error_bar(error));
    }
    let workspace_body = match editor.model.workspace() {
        Workspace::Design => design_workbench(editor),
        Workspace::Animate => animate_workbench(editor),
    };
    body = body.push(workspace_body);

    let mut application: Element<'_, Message> = container(body)
        .width(Fill)
        .height(Fill)
        .style(app_background)
        .into();
    if let Some(notice) = editor.notice.as_ref() {
        let overlay = container(action_notice(&notice.message))
            .width(Fill)
            .height(Fill)
            .padding(Padding::new(12.0).top(88.0))
            .align_x(Alignment::End)
            .align_y(Alignment::Start);
        application = stack![application, overlay].into();
    }
    if editor.quit_confirmation_open {
        stack![application, quit_confirmation(editor)].into()
    } else if editor.item_dialog.is_some() {
        stack![application, item_dialog_overlay(editor)].into()
    } else if editor.context_dropdown.is_some() {
        stack![application, context_dropdown_overlay(editor)].into()
    } else {
        application
    }
}

fn item_dialog_overlay(editor: &Editor) -> Element<'_, Message> {
    let content = match editor.item_dialog.as_ref() {
        Some(ItemDialog::RenameScene { value, .. }) => rename_dialog_content(
            "Rename scene",
            "The SRD scene name is written back with the scene.",
            "Scene name",
            value,
        ),
        Some(ItemDialog::RenameAnimationSet { value, .. }) => rename_dialog_content(
            "Rename animation set",
            "The new name is written to the selected ANMS record.",
            "Animation set name",
            value,
        ),
        Some(ItemDialog::DeleteScene { name, .. }) => delete_dialog_content(
            "Delete scene",
            name,
            "This removes the scene and its animation data. Undo restores it.",
        ),
        Some(ItemDialog::DeleteAnimationSet { name, .. }) => delete_dialog_content(
            "Delete animation set",
            name,
            "This removes the animation set. Undo restores it.",
        ),
        None => Space::new().into(),
    };
    dialog_component::modal(content, 440.0, Message::CancelItemDialog)
}

fn rename_dialog_content<'a>(
    title: &'a str,
    detail: &'a str,
    placeholder: &'a str,
    value: &'a str,
) -> Element<'a, Message> {
    column![
        dialog_component::title(Icon::Pencil, title, CYAN),
        dialog_component::description(detail),
        input_component::field(placeholder, value)
            .on_input(Message::RenameValueChanged)
            .on_submit(Message::CommitItemRename),
        row![
            Space::new().width(Fill),
            button_component::secondary("Cancel", Message::CancelItemDialog),
            button_component::action("Rename", Message::CommitItemRename, CYAN),
        ]
        .spacing(7),
    ]
    .spacing(12)
    .into()
}

fn delete_dialog_content<'a>(
    title: &'a str,
    name: &'a str,
    detail: &'a str,
) -> Element<'a, Message> {
    column![
        dialog_component::title(Icon::AlertTriangle, title, RED),
        text(name).size(BODY_SIZE).color(TEXT),
        dialog_component::description(detail),
        row![
            Space::new().width(Fill),
            button_component::secondary("Cancel", Message::CancelItemDialog),
            button_component::action("Delete", Message::ConfirmItemDelete, RED),
        ]
        .spacing(7),
    ]
    .spacing(12)
    .into()
}

fn quit_confirmation(editor: &Editor) -> Element<'_, Message> {
    let path = editor
        .model
        .project_path()
        .map_or_else(|| "this SRD".to_owned(), |path| path.display().to_string());
    let content = column![
        dialog_component::title(Icon::AlertTriangle, "Unsaved SRD changes", YELLOW),
        text(path).size(CAPTION_SIZE).color(MUTED),
        text("Quit now and the changes in this document will be lost.")
            .size(BODY_SIZE)
            .color(TEXT),
        row![
            Space::new().width(Fill),
            button_component::secondary("Cancel", Message::CancelClose),
            button_component::action("Quit without saving", Message::ConfirmDiscardAndClose, RED,),
        ]
        .spacing(7),
    ]
    .spacing(12);
    dialog_component::modal(content, 460.0, Message::CancelClose)
}

fn design_workbench(editor: &Editor) -> Element<'_, Message> {
    resizable_panes(&editor.design_panes, |_pane, pane, _maximized| {
        pane_grid::Content::new(match pane {
            PaneKind::Hierarchy => hierarchy_panel(editor),
            PaneKind::Composition => composition_panel(editor),
            PaneKind::Inspector => inspector_panel(editor),
            PaneKind::Assignment | PaneKind::DopeSheet => {
                unreachable!("Design pane grid only contains Hierarchy, Composition, and Inspector")
            }
        })
    })
}

fn animate_workbench(editor: &Editor) -> Element<'_, Message> {
    resizable_panes(&editor.animate_panes, |_pane, pane, _maximized| {
        pane_grid::Content::new(match pane {
            PaneKind::Hierarchy => hierarchy_panel(editor),
            PaneKind::Composition => composition_panel(editor),
            PaneKind::Inspector => inspector_panel(editor),
            PaneKind::Assignment => super::animate_ui::assignment_panel(editor),
            PaneKind::DopeSheet => super::animate_ui::dope_sheet_panel(editor),
        })
    })
}

fn top_bar(editor: &Editor) -> Element<'_, Message> {
    let brand = row![
        container(Space::new().width(3).height(24)).style(accent_bar),
        icon(Icon::Layers, 19, CYAN),
        text("SRD EDITOR").size(DIALOG_TITLE_SIZE).color(TEXT),
    ]
    .spacing(9)
    .align_y(Alignment::Center);

    let file_tools = row![
        icon_button(
            Icon::FolderOpen,
            "Open SRD · Ctrl/Cmd+O",
            Message::OpenFile,
            false
        ),
        icon_button(
            Icon::Save,
            "Save SRD · Ctrl/Cmd+S",
            Message::SaveRequested,
            false
        ),
        icon_button(
            Icon::SaveAll,
            "Save SRD as… · Ctrl/Cmd+Shift+S",
            Message::SaveAsRequested,
            false,
        ),
    ]
    .spacing(3);

    let undo = if editor.model.can_undo() {
        icon_button(
            Icon::Undo2,
            "Undo · Ctrl/Cmd+Z",
            Message::Model(EditorAction::Undo),
            false,
        )
    } else {
        passive_icon(Icon::Undo2, "Nothing to undo")
    };
    let redo = if editor.model.can_redo() {
        icon_button(
            Icon::Redo2,
            "Redo · Ctrl/Cmd+Shift+Z",
            Message::Model(EditorAction::Redo),
            false,
        )
    } else {
        passive_icon(Icon::Redo2, "Nothing to redo")
    };
    let edit_tools = row![undo, redo].spacing(3);

    let transform_tools = row![
        icon_button(
            Icon::MousePointer2,
            "Select hierarchy casts",
            Message::Model(EditorAction::SetTransformTool(TransformTool::Select)),
            editor.model.transform_tool() == TransformTool::Select,
        ),
        icon_button(
            Icon::Move3D,
            "Move selected cast on the canvas",
            Message::Model(EditorAction::SetTransformTool(TransformTool::Move)),
            editor.model.transform_tool() == TransformTool::Move,
        ),
        icon_button(
            Icon::RotateCw,
            "Rotate selected cast on the canvas",
            Message::Model(EditorAction::SetTransformTool(TransformTool::Rotate)),
            editor.model.transform_tool() == TransformTool::Rotate,
        ),
        icon_button(
            Icon::Scale3D,
            "Scale selected cast on the canvas",
            Message::Model(EditorAction::SetTransformTool(TransformTool::Scale)),
            editor.model.transform_tool() == TransformTool::Scale,
        ),
    ]
    .spacing(3);

    let edit_toggle = toggle_component::switch(editor.model.document_editing()).label("Edit");
    let edit_toggle = if editor.model.document().is_some() {
        edit_toggle.on_toggle(|_| Message::Model(EditorAction::ToggleDocumentEditing))
    } else {
        edit_toggle
    };

    let workspace_switch = row![
        workspace_tab(editor, Workspace::Design),
        workspace_tab(editor, Workspace::Animate),
    ]
    .spacing(2);
    let right_tools = row![edit_toggle].align_y(Alignment::Center);

    let content = row![
        brand,
        divider_vertical(),
        workspace_switch,
        divider_vertical(),
        file_tools,
        edit_tools,
        divider_vertical(),
        transform_tools,
        Space::new().width(Fill),
        right_tools,
    ]
    .spacing(9)
    .padding([7, 12])
    .align_y(Alignment::Center)
    .height(46);

    container(content).width(Fill).style(top_bar_style).into()
}

/// Same-window workspace switch. Design keeps the existing workbench; Animate
/// opens the motion console without leaving the OS window.
fn workspace_tab(editor: &Editor, workspace: Workspace) -> Element<'static, Message> {
    let selected = editor.model.workspace() == workspace;
    button(text(workspace.label()).size(PANEL_TITLE_SIZE))
        .on_press(Message::Model(EditorAction::SetWorkspace(workspace)))
        .padding([6, 12])
        .style(move |_theme, status| console_button_style(CYAN, selected, status))
        .into()
}

const CONTEXT_PICKER_WIDTH: f32 = 164.0;
const CONTEXT_MENU_TOP: f32 = 84.0;
const SCENE_MENU_LEFT: f32 = 70.0;
const ANIMATION_SET_MENU_LEFT: f32 = 323.0;

fn foreground_animation_set_picker(editor: &Editor) -> Element<'static, Message> {
    let label = editor
        .model
        .animate()
        .source_set()
        .and_then(|index| {
            editor
                .model
                .selected_scene()
                .and_then(|scene| scene.animation_sets.get(index))
        })
        .map_or_else(
            || "No animation set".to_owned(),
            |set| {
                let name = display_srd_name(&set.name);
                if name.is_empty() {
                    "Unnamed animation set".to_owned()
                } else {
                    name
                }
            },
        );
    context_picker_trigger(
        label,
        editor.context_dropdown == Some(ContextDropdown::AnimationSets),
        Message::ToggleAnimationSetDropdown,
    )
}

fn scene_context_picker(editor: &Editor) -> Element<'static, Message> {
    let label = editor.model.selected_scene().map_or_else(
        || "No scene".to_owned(),
        |scene| {
            let name = display_srd_name(&scene.name);
            if name.is_empty() {
                format!("Scene {}", editor.model.selected_scene_index() + 1)
            } else {
                name
            }
        },
    );
    context_picker_trigger(
        label,
        editor.context_dropdown == Some(ContextDropdown::Scenes),
        Message::ToggleSceneDropdown,
    )
}

fn context_picker_trigger(
    label: String,
    open: bool,
    message: Message,
) -> Element<'static, Message> {
    let muted = label.starts_with("No ");
    dropdown_component::trigger(
        compact_hierarchy_label(&label, 20),
        muted,
        open,
        message,
        CONTEXT_PICKER_WIDTH,
    )
}

fn context_dropdown_overlay(editor: &Editor) -> Element<'_, Message> {
    let (left, menu) = match editor.context_dropdown {
        Some(ContextDropdown::Scenes) => (SCENE_MENU_LEFT, scene_context_menu(editor)),
        Some(ContextDropdown::AnimationSets) => {
            (ANIMATION_SET_MENU_LEFT, animation_set_context_menu(editor))
        }
        None => return Space::new().into(),
    };
    mouse_area(
        pin(menu)
            .x(left)
            .y(CONTEXT_MENU_TOP)
            .width(Fill)
            .height(Fill),
    )
    .on_press(Message::CloseContextDropdown)
    .into()
}

fn scene_context_menu(editor: &Editor) -> Element<'_, Message> {
    let editing = editor.model.document_editing();
    let mut rows = column![].spacing(2);
    if editing {
        rows = rows.push(context_menu_add_entry(
            "Add scene",
            editor.model.can_add_scene(),
            Message::Model(EditorAction::AddScene),
        ));
    }
    if let Some(document) = editor.model.document()
        && !document.project.scenes.is_empty()
    {
        for (index, scene) in document.project.scenes.iter().enumerate() {
            let label = nonempty_name(&scene.name, format!("Scene {}", index + 1));
            let actions = editing.then(|| {
                (
                    Message::StartSceneRename(index),
                    Message::RequestDeleteScene(index),
                )
            });
            rows = rows.push(context_menu_item_row(
                label,
                index == editor.model.selected_scene_index(),
                Message::Model(EditorAction::SelectScene(index)),
                actions,
            ));
        }
    }
    context_menu_surface(rows)
}

fn animation_set_context_menu(editor: &Editor) -> Element<'_, Message> {
    let editing = editor.model.document_editing();
    let mut rows = column![].spacing(2);
    if editing {
        rows = rows.push(context_menu_add_entry(
            "Add animation set",
            editor.model.selected_scene().is_some(),
            Message::Model(EditorAction::Animate(AnimateAction::CreateSet)),
        ));
    }
    rows = rows.push(context_menu_item_row(
        "No animation set".to_owned(),
        editor.model.animate().source_set().is_none(),
        Message::Model(EditorAction::Animate(AnimateAction::LoadStoredSet(None))),
        None,
    ));
    if let Some(scene) = editor.model.selected_scene() {
        for (index, set) in scene.animation_sets.iter().enumerate() {
            let label = nonempty_name(&set.name, format!("Animation set {}", index + 1));
            let actions = editing.then(|| {
                (
                    Message::StartAnimationSetRename(index),
                    Message::RequestDeleteAnimationSet(index),
                )
            });
            rows = rows.push(context_menu_item_row(
                label,
                editor.model.animate().source_set() == Some(index),
                Message::Model(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(
                    index,
                )))),
                actions,
            ));
        }
    }
    context_menu_surface(rows)
}

fn context_menu_surface<'a>(rows: iced::widget::Column<'a, Message>) -> Element<'a, Message> {
    dropdown_component::menu(rows, 300.0, 2.0)
}

fn context_menu_add_entry(
    label: &'static str,
    enabled: bool,
    message: Message,
) -> Element<'static, Message> {
    dropdown_component::add_item(label, enabled, message)
}

fn context_menu_item_row(
    label: String,
    selected: bool,
    select_message: Message,
    actions: Option<(Message, Message)>,
) -> Element<'static, Message> {
    dropdown_component::item(
        compact_hierarchy_label(&label, 24),
        selected,
        select_message,
        actions,
    )
}

fn compact_transport(editor: &Editor) -> Element<'_, Message> {
    let (playing, frame) = compact_playback_state(&editor.model);
    if editor.model.selected_scene().is_none() {
        return row![
            text(timecode(frame))
                .size(TIMECODE_SIZE)
                .color(MUTED)
                .width(Length::Fixed(104.0)),
            passive_icon(Icon::SkipBack, "Add a scene to use transport"),
            passive_icon(Icon::StepBack, "Add a scene to use transport"),
            passive_icon(Icon::Play, "Add a scene to use transport"),
            passive_icon(Icon::StepForward, "Add a scene to use transport"),
            passive_icon(Icon::SkipForward, "Add a scene to use transport"),
        ]
        .spacing(3)
        .align_y(Alignment::Center)
        .into();
    }
    let play_icon = if playing { Icon::Pause } else { Icon::Play };
    row![
        text(timecode(frame))
            .size(TIMECODE_SIZE)
            .color(MUTED)
            .width(Length::Fixed(104.0)),
        icon_button(
            Icon::SkipBack,
            "First frame · Home",
            Message::Transport(super::Transport::JumpToStart),
            false,
        ),
        icon_button(
            Icon::StepBack,
            "Previous frame · ←",
            Message::Transport(super::Transport::StepFrame(-1)),
            false,
        ),
        icon_button(
            play_icon,
            "Play or pause · Space",
            Message::Transport(super::Transport::TogglePlaying),
            playing,
        ),
        icon_button(
            Icon::StepForward,
            "Next frame · →",
            Message::Transport(super::Transport::StepFrame(1)),
            false,
        ),
        icon_button(
            Icon::SkipForward,
            "Last frame · End",
            Message::Transport(super::Transport::JumpToEnd),
            false,
        ),
    ]
    .spacing(3)
    .align_y(Alignment::Center)
    .into()
}

fn compact_playback_state(model: &super::model::EditorModel) -> (bool, i32) {
    match model.workspace() {
        Workspace::Design => (model.playing(), model.frame()),
        Workspace::Animate => {
            let animate = model.animate();
            (animate.playing(), animate.frame())
        }
    }
}

fn timeline_context_bar(editor: &Editor) -> Element<'_, Message> {
    let context_controls = row![
        container(icon(Icon::Film, 14, CYAN)).width(Length::Fixed(14.0)),
        text("Scene")
            .size(CAPTION_SIZE)
            .color(MUTED)
            .width(Length::Fixed(34.0)),
        scene_context_picker(editor),
        divider_vertical(),
        container(icon(Icon::ListVideo, 14, CYAN)).width(Length::Fixed(14.0)),
        text("Animation")
            .size(CAPTION_SIZE)
            .color(MUTED)
            .width(Length::Fixed(54.0)),
        foreground_animation_set_picker(editor),
    ]
    .spacing(5)
    .align_y(Alignment::Center);
    // Equal flexible columns on both sides keep transport centered in the
    // window instead of merely centered in the space left by the pickers.
    let context = row![
        container(context_controls).width(Fill),
        compact_transport(editor),
        Space::new().width(Fill),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    container(context)
        .padding([4, 12])
        .height(38)
        .width(Fill)
        .style(sub_bar_style)
        .into()
}

fn action_notice(message: &str) -> Element<'_, Message> {
    container(
        row![
            icon(Icon::Info, 13, CYAN),
            text(message).size(BODY_SIZE).color(TEXT),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .max_width(560)
    .style(notice_style)
    .into()
}
fn error_bar(message: &str) -> Element<'_, Message> {
    container(
        row![
            icon(Icon::AlertTriangle, 13, RED),
            text(message).size(BODY_SIZE).color(TEXT)
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([5, 14])
    .width(Fill)
    .style(error_style)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CastTreeRow {
    node_index: usize,
    depth: usize,
    child_count: usize,
    parent_row: Option<usize>,
}

fn cast_display_name(node: &NodeRecord, node_index: usize) -> String {
    node.name
        .as_deref()
        .map(display_srd_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Cast {node_index}"))
}

fn compact_hierarchy_label(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    value
        .chars()
        .take(max_chars.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

fn mark_cast_descendants_visited(hierarchy: &Hierarchy, root: usize, visited: &mut [bool]) {
    let Some(children) = hierarchy.children.get(root) else {
        return;
    };
    for &child in children {
        let Some(was_visited) = visited.get_mut(child) else {
            continue;
        };
        if *was_visited {
            continue;
        }
        *was_visited = true;
        mark_cast_descendants_visited(hierarchy, child, visited);
    }
}

fn append_cast_subtree(
    hierarchy: &Hierarchy,
    root: usize,
    depth: usize,
    parent_row: Option<usize>,
    reveal_all: bool,
    is_expanded: &dyn Fn(usize) -> bool,
    visited: &mut [bool],
    rows: &mut Vec<CastTreeRow>,
) {
    let Some(was_visited) = visited.get_mut(root) else {
        return;
    };
    if *was_visited {
        return;
    }
    *was_visited = true;

    let children = hierarchy
        .children
        .get(root)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let row_index = rows.len();
    rows.push(CastTreeRow {
        node_index: root,
        depth,
        child_count: children.len(),
        parent_row,
    });
    if !reveal_all && !is_expanded(root) {
        mark_cast_descendants_visited(hierarchy, root, visited);
        return;
    }
    for &child in children {
        append_cast_subtree(
            hierarchy,
            child,
            depth + 1,
            Some(row_index),
            reveal_all,
            is_expanded,
            visited,
            rows,
        );
    }
}

fn cast_tree_rows(
    layer: &Layer,
    filter: &str,
    is_expanded: impl Fn(usize) -> bool,
) -> Vec<CastTreeRow> {
    let reveal_all = !filter.is_empty();
    let mut rows = Vec::new();
    let Ok(hierarchy) = layer.build_hierarchy() else {
        rows.extend(
            layer
                .nodes
                .iter()
                .enumerate()
                .map(|(node_index, _)| CastTreeRow {
                    node_index,
                    depth: 0,
                    child_count: 0,
                    parent_row: None,
                }),
        );
        return rows;
    };
    let mut visited = vec![false; layer.nodes.len()];
    for &root in &hierarchy.roots {
        append_cast_subtree(
            &hierarchy,
            root,
            0,
            None,
            reveal_all,
            &is_expanded,
            &mut visited,
            &mut rows,
        );
    }
    for node_index in 0..layer.nodes.len() {
        append_cast_subtree(
            &hierarchy,
            node_index,
            0,
            None,
            reveal_all,
            &is_expanded,
            &mut visited,
            &mut rows,
        );
    }
    if filter.is_empty() {
        return rows;
    }

    let mut visible = vec![false; rows.len()];
    for row_index in 0..rows.len() {
        let row = rows[row_index];
        if !cast_display_name(&layer.nodes[row.node_index], row.node_index)
            .to_ascii_lowercase()
            .contains(filter)
        {
            continue;
        }
        let mut ancestor = Some(row_index);
        while let Some(index) = ancestor {
            if visible[index] {
                break;
            }
            visible[index] = true;
            ancestor = rows[index].parent_row;
        }
    }
    rows.into_iter()
        .zip(visible)
        .filter_map(|(row, visible)| visible.then_some(row))
        .collect()
}

fn hierarchy_panel(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let header = panel_header(Icon::FolderTree, "Hierarchy", None);
    if model.selected_scene().is_none() {
        let empty = empty_state(
            Icon::FolderOpen,
            "No scenes",
            "Add the first scene to begin composing this SRD.",
        );
        let add_scene = button(
            row![
                icon(Icon::Plus, 13, CYAN),
                text("Add scene").size(BODY_SIZE).color(TEXT)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([7, 12])
        .style(move |_theme, status| console_button_style(CYAN, false, status));
        let add_scene = if model.document_editing() {
            add_scene.on_press(Message::Model(EditorAction::AddScene))
        } else {
            add_scene
        };
        let cue = column![empty, add_scene]
            .spacing(2)
            .align_x(Alignment::Center);
        return panel(header, center(cue).width(Fill).height(Fill));
    }
    let search = input_component::field("Filter hierarchy…", model.asset_search())
        .on_input(|value| Message::Model(EditorAction::SetAssetSearch(value)));
    let filter = model.asset_search().trim().to_ascii_lowercase();
    let scene_index = model.selected_scene_index();
    let mut tree = column![].spacing(2).width(Fill);

    let scene = model.selected_scene().expect("validated selected scene");
    for (layer_index, layer) in scene.layers.iter().enumerate() {
        let assignment = model
            .animate()
            .assignment_rows()
            .iter()
            .find(|entry| entry.layer == layer_index);
        if let Some(preview) = model.animate().preview() {
            if preview.layer != layer_index {
                continue;
            }
        } else if model.animation_active() && !assignment.is_some_and(|entry| entry.enabled) {
            continue;
        }
        let layer_name = nonempty_name(&layer.name, format!("Layer {layer_index}"));
        let layer_name_matches = layer_name.to_ascii_lowercase().contains(&filter);
        let layer_has_node_match = layer.nodes.iter().enumerate().any(|(node_index, node)| {
            cast_display_name(node, node_index)
                .to_ascii_lowercase()
                .contains(&filter)
        });
        if !filter.is_empty() && !layer_name_matches && !layer_has_node_match {
            continue;
        }

        let layer_expanded = model.layer_expanded(scene_index, layer_index);
        let layer_is_current = model.selected_layer_index() == layer_index;
        let layer_selected = layer_is_current && model.selected_node_index().is_none();
        let layer_hovered = editor.hovered_layer == Some((scene_index, layer_index));
        let visible = model.layer_visible(layer_index);
        let locked = model.layer_locked(layer_index);
        let soloed = model.layer_soloed(layer_index);
        let animation_status = if model.animation_active() {
            assignment.map_or_else(String::new, |entry| {
                if entry.animation_label == "—" {
                    "BASE".to_owned()
                } else {
                    compact_hierarchy_label(&entry.animation_label, 12)
                }
            })
        } else {
            String::new()
        };
        let layer_label = compact_hierarchy_label(&layer_name, 24);
        let layer_row = row![
            mini_icon_button(
                if layer_expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                Message::Model(EditorAction::ToggleLayerExpanded(scene_index, layer_index)),
            ),
            icon(Icon::Layers2, 13, if layer_selected { CYAN } else { MUTED }),
            text(layer_label)
                .size(BODY_SIZE)
                .color(if layer_selected { TEXT } else { MUTED }),
            Space::new().width(Fill),
            text(animation_status).size(OVERLINE_SIZE).color(
                if assignment.is_some_and(|entry| entry.enabled) {
                    CYAN
                } else {
                    MUTED
                }
            ),
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        let layer_label: Element<'_, Message> = mouse_area(
            container(layer_row)
                .width(Fill)
                .height(ROW_HEIGHT)
                .align_y(Alignment::Center)
                .padding([0, 7])
                .clip(true),
        )
        .on_press(Message::Model(EditorAction::SelectSceneLayer(
            scene_index,
            layer_index,
        )))
        .into();
        let eye_control = hierarchy_icon_button(
            if visible { Icon::Eye } else { Icon::EyeOff },
            "Toggle layer visibility",
            Message::Model(EditorAction::ToggleLayerVisible(layer_index)),
            !visible,
            layer_hovered || !visible,
        );
        let lock_control = hierarchy_icon_button(
            if locked { Icon::Lock } else { Icon::LockOpen },
            "Toggle layer transform lock",
            Message::Model(EditorAction::ToggleLayerLocked(layer_index)),
            locked,
            layer_hovered || locked,
        );
        let solo_control = hierarchy_icon_button(
            Icon::Focus,
            "Toggle layer solo",
            Message::Model(EditorAction::ToggleLayerSolo(layer_index)),
            soloed,
            layer_hovered || soloed,
        );
        let controls_cover = container(
            row![eye_control, lock_control, solo_control]
                .spacing(1)
                .align_y(Alignment::Center),
        )
        .padding([2, 3])
        .height(CONTROL_HEIGHT)
        .style(move |_| layer_tree_controls_style(layer_selected, layer_hovered));
        let full_row = container(
            row![layer_label, controls_cover]
                .width(Fill)
                .height(ROW_HEIGHT)
                .align_y(Alignment::Center),
        )
        .padding(Padding::ZERO.right(1))
        .width(Fill)
        .height(ROW_HEIGHT)
        .style(move |_| layer_tree_row_style(layer_selected, layer_hovered));
        let hover_target = (scene_index, layer_index);
        tree = tree.push(
            mouse_area(full_row)
                .on_enter(Message::LayerHovered(hover_target, true))
                .on_exit(Message::LayerHovered(hover_target, false)),
        );
        let selected_layer = layer_is_current;

        if !layer_expanded {
            continue;
        }
        for tree_row in cast_tree_rows(layer, &filter, |node| {
            model.cast_expanded(scene_index, layer_index, node)
        }) {
            let node_index = tree_row.node_index;
            let node = &layer.nodes[node_index];
            let selected_node = selected_layer && model.selected_node_index() == Some(node_index);
            let branch_expanded = model.cast_expanded(scene_index, layer_index, node_index);
            let branch_control: Element<'_, Message> = if tree_row.child_count > 0 {
                mini_icon_button(
                    if branch_expanded {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    },
                    Message::Model(EditorAction::ToggleCastExpanded(
                        scene_index,
                        layer_index,
                        node_index,
                    )),
                )
            } else {
                Space::new().width(15).into()
            };
            let cast = PreviewCastSelection {
                scene_index,
                layer_index,
                node_index,
            };
            let visible = model.cast_visible(scene_index, layer_index, node_index);
            let locked = model.cast_locked(scene_index, layer_index, node_index);
            let soloed = model.cast_soloed(scene_index, layer_index, node_index);
            let hovered = editor.hovered_cast == Some(cast);
            let cast_label: Element<'_, Message> = mouse_area(
                container(
                    row![
                        Space::new().width(6.0 + tree_row.depth as f32 * 14.0),
                        branch_control,
                        icon(
                            cast_icon(
                                layer
                                    .classify_cast(node_index)
                                    .map_or(SrCastKind::Null, |value| value.kind),
                            ),
                            12,
                            if selected_node { CYAN } else { MUTED },
                        ),
                        text(compact_hierarchy_label(
                            &cast_display_name(node, node_index),
                            28,
                        ))
                        .size(BODY_SIZE)
                        .color(if selected_node { TEXT } else { MUTED }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .width(Fill)
                .height(ROW_HEIGHT)
                .align_y(Alignment::Center)
                .padding([0, 7])
                .clip(true),
            )
            .on_press(Message::Model(EditorAction::SelectCast(
                scene_index,
                layer_index,
                node_index,
            )))
            .into();
            let eye_control = hierarchy_icon_button(
                if visible { Icon::Eye } else { Icon::EyeOff },
                "Toggle CAST visibility",
                Message::Model(EditorAction::ToggleCastVisible(
                    scene_index,
                    layer_index,
                    node_index,
                )),
                !visible,
                hovered || !visible,
            );
            let lock_control = hierarchy_icon_button(
                if locked { Icon::Lock } else { Icon::LockOpen },
                "Toggle CAST transform lock",
                Message::Model(EditorAction::ToggleCastLocked(
                    scene_index,
                    layer_index,
                    node_index,
                )),
                locked,
                hovered || locked,
            );
            let solo_control = hierarchy_icon_button(
                Icon::Focus,
                "Toggle CAST solo",
                Message::Model(EditorAction::ToggleCastSolo(
                    scene_index,
                    layer_index,
                    node_index,
                )),
                soloed,
                hovered || soloed,
            );
            let controls_cover = container(
                row![eye_control, lock_control, solo_control]
                    .spacing(1)
                    .align_y(Alignment::Center),
            )
            .padding([2, 3])
            .height(CONTROL_HEIGHT)
            .style(move |_| tree_controls_style(selected_node, hovered));
            let full_row = container(
                row![cast_label, controls_cover]
                    .width(Fill)
                    .height(ROW_HEIGHT)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::ZERO.right(1))
            .width(Fill)
            .height(ROW_HEIGHT)
            .style(move |_| tree_row_style(selected_node, hovered));
            tree = tree.push(
                mouse_area(full_row)
                    .on_enter(Message::CastHovered(cast, true))
                    .on_exit(Message::CastHovered(cast, false)),
            );
        }
    }

    let hierarchy: Element<'_, Message> = if scene.layers.is_empty() {
        center(empty_state(
            Icon::Layers2,
            "No layers",
            "This scene is ready for its first layer.",
        ))
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        scrollable(container(tree).padding([5, 6]))
            .height(Fill)
            .into()
    };
    panel_with_toolbar(header, panel_toolbar(search), hierarchy)
}

fn composition_panel(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let header = panel_header(
        Icon::MonitorPlay,
        "Composition",
        model
            .selected_scene()
            .map(|_| format!("{} / MainScene / embedded CAM", model.selected_scene_name())),
    );
    if model.selected_scene().is_none() {
        return panel(
            header,
            center(empty_state(
                Icon::MonitorPlay,
                "Nothing to render",
                "Add a scene to make the composition viewport available.",
            ))
            .width(Fill)
            .height(Fill),
        );
    }
    let [target_width, target_height] = EDITOR_PREVIEW_SIZE;
    let toolbar = row![
        Space::new().width(Fill),
        icon_button(
            Icon::Grid2X2,
            "Toggle grid · G",
            Message::Model(EditorAction::ToggleGrid),
            model.show_grid(),
        ),
        divider_vertical(),
        icon_button(
            Icon::ZoomOut,
            "Zoom out",
            Message::Model(EditorAction::ZoomBy(-10)),
            false,
        ),
        button(
            container(text(format!("{}%", model.zoom_percent())).size(BODY_SIZE))
                .height(CONTROL_HEIGHT)
                .align_y(Alignment::Center),
        )
        .on_press(Message::Model(EditorAction::SetZoom(100)))
        .padding([0, 7])
        .height(CONTROL_HEIGHT)
        .style(ghost_button),
        icon_button(
            Icon::ZoomIn,
            "Zoom in",
            Message::Model(EditorAction::ZoomBy(10)),
            false,
        ),
        icon_button(
            Icon::Maximize,
            "Fit composition",
            Message::Model(EditorAction::SetZoom(100)),
            false,
        ),
    ]
    .spacing(4)
    .height(CONTROL_HEIGHT)
    .align_y(Alignment::Center);

    let image_layer: Element<'_, Message> = if let Some(handle) = &editor.preview_image {
        Canvas::new(PreviewImageLayer {
            handle: handle.clone(),
            zoom_percent: model.zoom_percent(),
            target_size: [target_width, target_height],
        })
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        center(empty_state(
            Icon::MonitorPlay,
            "Nothing to render",
            "This scene does not have a rendered frame yet.",
        ))
        .width(Fill)
        .height(Fill)
        .into()
    };

    let inspector = model.inspector();
    let overlay = Canvas::new(ArtboardOverlay {
        show_grid: model.show_grid(),
        zoom_percent: model.zoom_percent(),
        target_size: [target_width, target_height],
        position: [
            parse_or(&inspector.position[0], 0.0),
            parse_or(&inspector.position[1], 0.0),
        ],
        scale: [
            parse_or(&inspector.scale[0], 1.0),
            parse_or(&inspector.scale[1], 1.0),
        ],
        rotation_z: inspector.rotation[2].parse().unwrap_or(0),
        selection_bounds: editor.preview_selection_bounds,
        tool: model.transform_tool(),
        editable: model.document_editing()
            && !model.selected_cast_locked()
            && !model.animation_active(),
    })
    .width(Fill)
    .height(Fill);

    let selection_badge: Element<'_, Message> = if model.selected_node_index().is_some() {
        let is_image = model.selected_cast_type_name() == "ImageCast";
        container(
            row![
                icon(
                    if is_image {
                        Icon::Image
                    } else {
                        Icon::MousePointer2
                    },
                    13,
                    CYAN
                ),
                text(format!(
                    "Selected {} · {}",
                    if is_image { "image" } else { "cast" },
                    model.selected_node_name(),
                ))
                .size(BODY_SIZE)
                .color(TEXT),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .style(floating_badge_style)
        .into()
    } else {
        Space::new().into()
    };

    let target_badge = container(
        text(format!("{target_width} × {target_height} target"))
            .size(BODY_SIZE)
            .color(MUTED),
    )
    .padding([5, 8])
    .style(floating_badge_style);

    let preview_badge: Element<'_, Message> = model
        .animate()
        .preview()
        .and_then(|preview| {
            let layer = model.selected_scene()?.layers.get(preview.layer)?;
            let (animation_index, animation) = layer.find_animation(&preview.animation_name)?;
            let layer_name = nonempty_name(&layer.name, format!("Layer {}", preview.layer));
            Some(
                container(
                    row![
                        icon(Icon::MonitorPlay, 13, CYAN),
                        text(format!(
                            "Previewing {} animation in {layer_name} layer",
                            super::animate_ui::animation_label(&animation.name, animation_index),
                        ))
                        .size(BODY_SIZE)
                        .color(TEXT),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([5, 8])
                .style(floating_badge_style)
                .into(),
            )
        })
        .unwrap_or_else(|| Space::new().into());

    let canvas_area = stack![
        image_layer,
        overlay,
        container(column![
            row![preview_badge, Space::new().width(Fill)].padding(12),
            Space::new().height(Fill),
            row![selection_badge, Space::new().width(Fill), target_badge].padding(12),
        ])
        .width(Fill)
        .height(Fill),
    ];

    panel_with_toolbar(header, panel_toolbar(toolbar), canvas_area)
}

fn inspector_panel(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let header = panel_header(Icon::SlidersHorizontal, "Inspector", None);
    if model.selected_layer().is_none() {
        return panel(
            header,
            center(empty_state(
                Icon::SlidersHorizontal,
                "Nothing to inspect",
                "Select a layer or CAST to inspect its properties.",
            ))
            .width(Fill)
            .height(Fill),
        );
    }
    let body: Element<'_, Message> = if model.selected_node_index().is_none() {
        column![
            panel_section(
                section_heading(Icon::Tag, "Element Identity", None),
                layer_identity(editor),
            ),
            layer_inspector(editor),
        ]
        .spacing(1)
        .into()
    } else {
        let mut content = column![
            panel_section(
                section_heading(Icon::Tag, "Element Identity", None),
                cast_identity(editor),
            ),
            transform_inspector(editor),
            cast_role_inspector(editor),
            structure_inspector(editor),
        ]
        .spacing(1);
        if let Some(advanced) = advanced_inspector(editor) {
            content = content.push(advanced);
        }
        if let Some(diagnostics) = cast_diagnostics_inspector(editor) {
            content = content.push(diagnostics);
        }
        if model.workspace() == Workspace::Animate
            && let Some(animation) = super::animate_ui::animation_inspector_section(editor)
        {
            content = content.push(animation);
        }
        content.into()
    };
    panel(
        header,
        panel_body(scrollable(container(body).padding([3, 2])).height(Fill)),
    )
}

fn layer_identity(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let read_only = model.layer_locked(model.selected_layer_index()) || !model.document_editing();
    column![
        element_identity_row(
            "Name",
            inspector_field_input(
                "Layer name",
                &model.inspector().layer_name,
                InspectorField::LayerName,
                read_only,
            ),
        ),
        element_identity_row(
            "Type",
            input_component::field("", "Layer").width(Fill).into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn cast_identity(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let read_only = model.selected_cast_locked()
        || model.layer_locked(model.selected_layer_index())
        || !model.document_editing();
    column![
        element_identity_row(
            "Name",
            inspector_field_input(
                "CAST name",
                &model.inspector().cast_name,
                InspectorField::CastName,
                read_only,
            ),
        ),
        element_identity_row(
            "Type",
            input_component::field("", model.selected_cast_type_name())
                .width(Fill)
                .into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn element_identity_row<'a>(label: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    row![
        text(label)
            .size(BODY_SIZE)
            .color(MUTED)
            .width(Length::Fixed(78.0)),
        control,
    ]
    .spacing(5)
    .height(ROW_HEIGHT)
    .align_y(Alignment::Center)
    .into()
}

fn inspector_section<'a>(
    editor: &'a Editor,
    section: InspectorSection,
    icon_value: Icon,
    label: &str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let collapsed = editor.model.inspector_section_collapsed(section);
    let header = button(
        row![
            icon(icon_value, 13, CYAN),
            text(label.to_owned()).size(BODY_SIZE).color(TEXT),
            Space::new().width(Fill),
            icon(
                if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                },
                12,
                MUTED,
            ),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .on_press(Message::Model(EditorAction::ToggleInspectorSection(
        section,
    )))
    .padding([0, 2])
    .height(if collapsed {
        Length::Shrink
    } else {
        Length::Fixed(ROW_HEIGHT)
    })
    .width(Fill)
    .style(|_, status| section_header_button_style(status));
    if collapsed {
        section_surface(header)
    } else {
        section_surface(column![header, body].spacing(SECTION_GAP))
    }
}

fn transform_inspector(editor: &Editor) -> Element<'_, Message> {
    let values = editor.model.inspector();
    let read_only = editor.model.selected_cast_locked()
        || editor
            .model
            .layer_locked(editor.model.selected_layer_index())
        || !editor.model.document_editing();
    let transform = column![
        animated_transform_group(
            editor,
            "Position",
            &values.position,
            [
                TransformField::PositionX,
                TransformField::PositionY,
                TransformField::PositionZ,
            ],
            [0, 1, 2],
            read_only,
        ),
        animated_transform_group(
            editor,
            "Rotation",
            &values.rotation,
            [
                TransformField::RotationX,
                TransformField::RotationY,
                TransformField::RotationZ,
            ],
            [3, 4, 5],
            read_only,
        ),
        animated_transform_group(
            editor,
            "Scale",
            &values.scale,
            [
                TransformField::ScaleX,
                TransformField::ScaleY,
                TransformField::ScaleZ,
            ],
            [6, 7, 8],
            read_only,
        ),
    ]
    .spacing(7);
    let color = column![
        animated_color_group(
            editor,
            "Multiply",
            &values.multiply_color,
            true,
            [9, 9, 9, 21],
            read_only,
        ),
        animated_color_group(
            editor,
            "Additive",
            &values.additive_color,
            false,
            [19, 19, 19, 22],
            read_only,
        ),
        animated_inspector_field_row(
            editor,
            "Visibility",
            "1",
            &values.visibility,
            InspectorField::Visibility,
            10,
            read_only,
        ),
    ]
    .spacing(7);
    column![
        inspector_section(
            editor,
            InspectorSection::Transform,
            Icon::Move3D,
            "Transform",
            transform.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::Compositing,
            Icon::Palette,
            "Compositing",
            color.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn layer_inspector(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let layer_index = model.selected_layer_index();
    let read_only = model.layer_locked(layer_index) || !model.document_editing();
    let (node_count, animation_count) = model
        .selected_layer()
        .map_or((0, 0), |layer| (layer.nodes.len(), layer.animations.len()));
    let metadata = column![
        inspector_field_row(
            "Flags",
            "0x0",
            &model.inspector().layer_flags,
            InspectorField::LayerFlags,
            read_only,
        ),
        property_row("Scene", model.selected_scene_name()),
        property_row("Layer index", layer_index.to_string()),
        property_row("CAST count", node_count.to_string()),
        property_row("Animation count", animation_count.to_string()),
    ]
    .spacing(8);
    inspector_section(
        editor,
        InspectorSection::LayerMetadata,
        Icon::Layers2,
        "Layer metadata",
        metadata.into(),
    )
}

fn cast_role_inspector(editor: &Editor) -> Element<'_, Message> {
    match &editor.model.inspector().role {
        CastRoleDraft::None | CastRoleDraft::Null => Space::new().height(0).into(),
        CastRoleDraft::Image(value) => image_cast_inspector(editor, value),
        CastRoleDraft::Text(value) => text_cast_inspector(editor, value),
        CastRoleDraft::Slice(value) => slice_cast_inspector(editor, value),
        CastRoleDraft::Reference(value) => reference_cast_inspector(editor, value),
        CastRoleDraft::Number(value) => number_cast_inspector(editor, value),
    }
}

fn inspector_read_only(editor: &Editor) -> bool {
    editor.model.selected_cast_locked()
        || editor
            .model
            .layer_locked(editor.model.selected_layer_index())
        || !editor.model.document_editing()
}

fn visual_geometry_fields<'a>(
    editor: &'a Editor,
    geometry: &'a VisualGeometryDraft,
    read_only: bool,
) -> Element<'a, Message> {
    let mut content = column![
        animated_inspector_vector2_row(
            editor,
            "Size",
            [&geometry.size[0], &geometry.size[1]],
            [InspectorField::PayloadWidth, InspectorField::PayloadHeight],
            [11, 12],
            read_only,
        ),
        inspector_vector2_row(
            "Custom origin",
            [&geometry.origin[0], &geometry.origin[1]],
            [InspectorField::OriginX, InspectorField::OriginY],
            read_only,
        ),
        inspector_field_row(
            "Origin mode",
            "0",
            &geometry.origin_mode,
            InspectorField::OriginMode,
            read_only,
        ),
        section_heading(Icon::Palette, "Vertex colours", None),
    ]
    .spacing(9);
    for vertex in 0..4 {
        content = content.push(animated_inspector_vertex_color_row(
            editor,
            vertex,
            &geometry.vertex_colors[vertex],
            [13, 15, 14, 16][vertex],
            read_only,
        ));
    }
    content.into()
}

fn image_binding_fields<'a>(
    editor: &'a Editor,
    binding: &'a ImageBindingDraft,
    read_only: bool,
    animated: bool,
) -> Element<'a, Message> {
    let primary: Element<'a, Message> = if animated {
        animated_inspector_field_row(
            editor,
            "Primary (CREF)",
            "-1",
            &binding.cref_index,
            InspectorField::CrefIndex,
            17,
            read_only,
        )
    } else {
        inspector_field_row(
            "Primary (CREF)",
            "-1",
            &binding.cref_index,
            InspectorField::CrefIndex,
            read_only,
        )
    };
    let secondary: Element<'a, Message> = if animated {
        animated_inspector_field_row(
            editor,
            "Secondary (CRE1)",
            "-1",
            &binding.cre1_index,
            InspectorField::Cre1Index,
            20,
            read_only,
        )
    } else {
        inspector_field_row(
            "Secondary (CRE1)",
            "-1",
            &binding.cre1_index,
            InspectorField::Cre1Index,
            read_only,
        )
    };
    column![
        primary,
        inspector_vector2_row(
            "Primary UV offset",
            [
                &binding.coordinate_offsets[0][0],
                &binding.coordinate_offsets[0][1],
            ],
            [
                InspectorField::CoordinateOffset(0, 0),
                InspectorField::CoordinateOffset(0, 1),
            ],
            read_only,
        ),
        secondary,
        inspector_vector2_row(
            "Secondary UV offset",
            [
                &binding.coordinate_offsets[1][0],
                &binding.coordinate_offsets[1][1],
            ],
            [
                InspectorField::CoordinateOffset(1, 0),
                InspectorField::CoordinateOffset(1, 1),
            ],
            read_only,
        ),
    ]
    .spacing(9)
    .into()
}

fn image_cast_inspector<'a>(
    editor: &'a Editor,
    value: &'a super::model::ImageCastDraft,
) -> Element<'a, Message> {
    let read_only = inspector_read_only(editor);
    let model = &editor.model;
    let image = model
        .selected_node_index()
        .and_then(|node| model.selected_layer()?.image_by_node.get(node)?.as_ref());
    let render_preset = image
        .and_then(|image| select_srd_image_render_preset(image.flags, -1, false))
        .map_or_else(|| "none".into(), |preset| preset.to_string());
    let mut material = column![
        image_binding_fields(editor, &value.binding, read_only, true),
        property_row("Render preset", render_preset),
    ]
    .spacing(9);
    if let Some(texture_index) = model.selected_image_texture_index() {
        material = material.push(property_row("Resolved texture", texture_index));
        if let Some(texture) = model
            .document()
            .and_then(|document| document.textures.textures.get(texture_index))
        {
            material = material
                .push(property_row(
                    "Asset",
                    nonempty_name(&texture.filename, format!("Texture {texture_index}")),
                ))
                .push(property_row(
                    "Texture size",
                    format!("{} × {}", texture.width, texture.height),
                ));
        }
    } else {
        material = material.push(property_row("Resolved texture", "none"));
    }
    column![
        inspector_section(
            editor,
            InspectorSection::ImageGeometry,
            Icon::Move3D,
            "Image geometry",
            visual_geometry_fields(editor, &value.geometry, read_only),
        ),
        inspector_section(
            editor,
            InspectorSection::ImageMaterial,
            Icon::Image,
            "Image material",
            material.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn text_cast_inspector<'a>(
    editor: &'a Editor,
    value: &'a super::model::TextCastDraft,
) -> Element<'a, Message> {
    const SUBSTITUTION_LABELS: [&str; 8] = [
        "$[0]", "$[1]", "$[2]", "$[3]", "$[4]", "$[5]", "$[6]", "$[7]",
    ];
    let read_only = inspector_read_only(editor);
    let mut content = column![
        inspector_field_row(
            "Text flags",
            "0x0",
            &value.flags,
            InspectorField::TextFlags,
            read_only,
        ),
        inspector_field_row(
            "Font index",
            "0",
            &value.font,
            InspectorField::TextFont,
            read_only,
        ),
    ]
    .spacing(9);
    if let Some(text_bytes) = editor.model.selected_text_content() {
        match std::str::from_utf8(text_bytes) {
            Ok(value) => {
                let mut input = input_component::field("Text content", value);
                if !read_only {
                    input = input
                        .on_input(|value| Message::Model(EditorAction::EditTextContent(value)));
                }
                content = content.push(labeled_control("Content", input));
            }
            Err(_) => {
                content = content.push(
                    text("The serialized TEXT bytes are non-UTF-8 and remain read-only.")
                        .size(CAPTION_SIZE)
                        .color(YELLOW),
                );
            }
        }
    }

    let enabled = editor.model.selected_runtime_text_input().is_some();
    let runtime_toggle =
        toggle_component::switch(enabled).label("Supply explicit $[0]…$[7] values");
    let runtime_toggle = if read_only {
        runtime_toggle
    } else {
        runtime_toggle.on_toggle(|_| Message::Model(EditorAction::ToggleRuntimeTextInput))
    };
    let mut substitutions = column![
        text("Preview-only values; not serialized into TEXT.")
            .size(CAPTION_SIZE)
            .color(MUTED),
        runtime_toggle,
    ]
    .spacing(8);
    if let Some(input) = editor.model.selected_runtime_text_input() {
        for (slot, bytes) in input.substitutions.iter().enumerate() {
            let current = std::str::from_utf8(bytes).unwrap_or("<non-UTF-8>");
            let mut substitution = input_component::field("Runtime value", current);
            if !read_only {
                substitution = substitution.on_input(move |value| {
                    Message::Model(EditorAction::SetRuntimeTextSubstitution(slot, value))
                });
            }
            substitutions =
                substitutions.push(labeled_control(SUBSTITUTION_LABELS[slot], substitution));
        }
    }
    column![
        inspector_section(
            editor,
            InspectorSection::TextContent,
            Icon::TextCursorInput,
            "Text",
            content.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::TextBox,
            Icon::Move3D,
            "Text box",
            visual_geometry_fields(editor, &value.box_geometry, read_only),
        ),
        inspector_section(
            editor,
            InspectorSection::TextPreviewOverrides,
            Icon::TextCursorInput,
            "Preview substitutions",
            substitutions.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn slice_cast_inspector<'a>(
    editor: &'a Editor,
    value: &'a super::model::SliceCastDraft,
) -> Element<'a, Message> {
    let read_only = inspector_read_only(editor);
    let slice = editor.model.selected_node_index().and_then(|node| {
        editor
            .model
            .selected_layer()?
            .csli_by_node
            .get(node)?
            .as_ref()
    });
    let mut grid = column![visual_geometry_fields(editor, &value.grid, read_only)].spacing(9);
    if let Some(slice) = slice {
        grid = grid
            .push(property_row(
                "Grid",
                format!("{} × {}", slice.columns, slice.rows),
            ))
            .push(property_row(
                "Explicit widths",
                slice.explicit_width_cell_count,
            ))
            .push(property_row(
                "Explicit heights",
                slice.explicit_height_cell_count,
            ));
    }
    let cells = column![
        property_row(
            "Serialized cells",
            slice.map_or(0, |value| value.cells.len()),
        ),
        text("Cell geometry and colour belong to SLIC records, not an ImageCast component.")
            .size(CAPTION_SIZE)
            .color(MUTED),
    ]
    .spacing(8);
    let atlas = column![
        property_row(
            "Primary references",
            slice.map_or(0, |value| value.crefs.len()),
        ),
        text("Each cell selects its own CREF image/rectangle pair.")
            .size(CAPTION_SIZE)
            .color(MUTED),
    ]
    .spacing(8);
    column![
        inspector_section(
            editor,
            InspectorSection::SliceGrid,
            Icon::Grid2X2,
            "Slice grid",
            grid.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::SliceCells,
            Icon::Grid2X2,
            "Slice cells",
            cells.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::SliceAtlas,
            Icon::Image,
            "Cell atlas",
            atlas.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn reference_cast_inspector<'a>(
    editor: &'a Editor,
    value: &'a super::model::ReferenceCastDraft,
) -> Element<'a, Message> {
    let read_only = inspector_read_only(editor);
    let reference = editor.model.selected_node_index().and_then(|node| {
        editor
            .model
            .selected_layer()?
            .reference_by_node
            .get(node)?
            .as_ref()
    });
    let target = reference.and_then(|reference| {
        editor
            .model
            .document()?
            .project
            .resolve_reference(reference)
    });
    let resolution = target.map_or_else(
        || "Unresolved".to_owned(),
        |target| format!("SCN {} / LAYR {}", target.scene_index, target.layer_index),
    );
    let cast_count = target
        .and_then(|target| {
            editor
                .model
                .document()?
                .project
                .scenes
                .get(target.scene_index)?
                .layers
                .get(target.layer_index)
                .map(|layer| layer.nodes.len())
        })
        .unwrap_or(0);
    let target_fields = column![
        inspector_field_row(
            "Source scene",
            "Scene",
            &value.source,
            InspectorField::ReferenceSource,
            read_only,
        ),
        inspector_field_row(
            "Source layer",
            "Layer",
            &value.layer,
            InspectorField::ReferenceLayer,
            read_only,
        ),
        property_row("Resolution", resolution),
    ]
    .spacing(9);
    let animation = column![
        inspector_field_row(
            "Animation",
            "Animation",
            &value.animation,
            InspectorField::ReferenceAnimation,
            read_only,
        ),
        inspector_field_row(
            "Enabled",
            "1",
            &value.enabled,
            InspectorField::ReferenceEnabled,
            read_only,
        ),
        animated_inspector_field_row(
            editor,
            "Default / keyed frame",
            "0",
            &value.frame,
            InspectorField::ReferenceFrame,
            23,
            read_only,
        ),
    ]
    .spacing(9);
    let instance = column![
        property_row("Instantiated CASTs", cast_count),
        text("The runtime owns an independent copied layer; it does not alias the target.")
            .size(CAPTION_SIZE)
            .color(MUTED),
    ]
    .spacing(8);
    column![
        inspector_section(
            editor,
            InspectorSection::ReferenceTarget,
            Icon::ExternalLink,
            "Reference target",
            target_fields.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::ReferenceAnimation,
            Icon::Film,
            "Child animation",
            animation.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::ReferenceInstance,
            Icon::Layers2,
            "Runtime instance",
            instance.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn number_cast_inspector<'a>(
    editor: &'a Editor,
    value: &'a super::model::NumberCastDraft,
) -> Element<'a, Message> {
    let read_only = inspector_read_only(editor);
    let number = editor.model.selected_node_index().and_then(|node| {
        editor
            .model
            .selected_layer()?
            .number_by_node
            .get(node)?
            .as_ref()
    });
    let values = column![
        inspector_field_row(
            "Initial integer",
            "0",
            &value.integer,
            InspectorField::NumberInteger,
            read_only,
        ),
        inspector_field_row(
            "Initial fraction",
            "0",
            &value.fraction,
            InspectorField::NumberFraction,
            read_only,
        ),
        text("Number content is setter-driven and has no ANIM target.")
            .size(CAPTION_SIZE)
            .color(MUTED),
    ]
    .spacing(9);
    let format = column![
        inspector_field_row(
            "Format flags",
            "0x0",
            &value.format,
            InspectorField::NumberFormat,
            read_only,
        ),
        property_row(
            "Special glyph slots",
            number.map_or(0, |value| value.fields_8d_94.len()),
        ),
    ]
    .spacing(9);
    let atlas = column![
        property_row(
            "Primary glyphs",
            number.map_or(0, |value| value.crefs.len())
        ),
        property_row(
            "Secondary glyphs",
            number.map_or(0, |value| value.cre1s.len())
        ),
    ]
    .spacing(8);
    column![
        inspector_section(
            editor,
            InspectorSection::NumberValue,
            Icon::Hash,
            "Number value",
            values.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::NumberFormat,
            Icon::Hash,
            "Number format",
            format.into(),
        ),
        inspector_section(
            editor,
            InspectorSection::NumberLayout,
            Icon::Move3D,
            "Glyph layout",
            visual_geometry_fields(editor, &value.layout, read_only),
        ),
        inspector_section(
            editor,
            InspectorSection::NumberAtlas,
            Icon::Image,
            "Glyph atlas",
            atlas.into(),
        ),
    ]
    .spacing(5)
    .into()
}

fn structure_inspector(editor: &Editor) -> Element<'_, Message> {
    let model = &editor.model;
    let summary = model.selected_cast_structure();
    let mut body = column![
        property_row("NODE index", model.selected_node_index().unwrap_or(0)),
        property_row(
            "Parent",
            summary
                .and_then(|value| value.parent_index)
                .map_or_else(|| "root".into(), |value| format!("NODE {value}")),
        ),
        property_row(
            "Direct children",
            summary.map_or(0, |value| value.child_count)
        ),
        property_row(
            "Render-layer offset",
            summary.map_or(0, |value| value.render_layer_offset),
        ),
    ]
    .spacing(8);
    if let Some(cell) = summary.and_then(|value| value.parent_slice_cell) {
        body = body.push(property_row("Parent slice cell", cell));
    }
    if model.selected_cast_kind() == Some(SrCastKind::Null) {
        body = body.push(
            text("Structural CAST: no direct geometry; descendants inherit this state.")
                .size(CAPTION_SIZE)
                .color(MUTED),
        );
    }
    inspector_section(
        editor,
        InspectorSection::Structure,
        Icon::FolderTree,
        "Structure",
        body.into(),
    )
}

fn advanced_inspector(editor: &Editor) -> Option<Element<'_, Message>> {
    let model = &editor.model;
    let node = model.selected_node()?;
    let classification = model.selected_cast_classification()?;
    let read_only = inspector_read_only(editor);
    let serialized_type = classification
        .serialized_type
        .map_or_else(|| "missing".into(), |value| value.to_string());
    let full_flags = node
        .type_flags
        .map_or_else(|| "missing".into(), |value| format!("0x{value:08X}"));
    let mut body = column![
        property_row("NODE type", serialized_type),
        property_row("NODE type flags", full_flags),
    ]
    .spacing(9);
    if let Some(geometry) = model.inspector().role.geometry() {
        body = body.push(inspector_field_row(
            "Record flags",
            "0x0",
            &geometry.flags,
            InspectorField::PayloadFlags,
            read_only,
        ));
    }
    if let CastRoleDraft::Text(value) = &model.inspector().role {
        body = body
            .push(section_heading(
                Icon::Image,
                "Serialized CIMG channels",
                None,
            ))
            .push(image_binding_fields(
                editor,
                &value.source_binding,
                read_only,
                false,
            ));
    }
    Some(inspector_section(
        editor,
        InspectorSection::Advanced,
        Icon::Info,
        "Advanced source data",
        body.into(),
    ))
}

fn cast_diagnostics_inspector(editor: &Editor) -> Option<Element<'_, Message>> {
    let classification = editor.model.selected_cast_classification()?;
    let messages = cast_diagnostic_messages(editor, classification);
    if messages.is_empty() {
        return None;
    }
    let mut body = column![].spacing(7);
    for message in messages {
        body = body.push(text(message).size(CAPTION_SIZE).color(YELLOW));
    }
    Some(inspector_section(
        editor,
        InspectorSection::Diagnostics,
        Icon::AlertTriangle,
        "Diagnostics",
        body.into(),
    ))
}

fn cast_diagnostic_messages(editor: &Editor, classification: CastClassification) -> Vec<String> {
    let mut messages = Vec::new();
    if let Some(serialized_type) = classification.unsupported_serialized_type() {
        messages.push(format!(
            "Unsupported NODE type {serialized_type}; the game instantiates NullCast."
        ));
    }
    if classification.missing_expected_payload()
        && let Some(expected) = classification.expected_payload()
    {
        messages.push(format!(
            "{} expects a linked {} record, but none is present.",
            classification.kind.display_name(),
            expected.record_name(),
        ));
    }
    let extras = classification
        .extra_payloads()
        .iter()
        .map(|payload| payload.record_name())
        .collect::<Vec<_>>();
    if !extras.is_empty() {
        messages.push(format!(
            "Inactive payload records: {}. They do not change the runtime CAST kind.",
            extras.join(", "),
        ));
    }
    if classification.inactive_text_payload {
        messages.push(
            "TEXT is present, but CIMG flag 0x100 is clear; the runtime creates ImageCast.".into(),
        );
    }
    if classification.kind == SrCastKind::Reference
        && let Some(node) = editor.model.selected_node_index()
        && let Some(reference) = editor
            .model
            .selected_layer()
            .and_then(|layer| layer.reference_by_node.get(node))
            .and_then(Option::as_ref)
        && editor
            .model
            .document()
            .and_then(|document| document.project.resolve_reference(reference))
            .is_none()
    {
        messages.push("The CRFD scene/layer target does not resolve in this project.".into());
    }
    messages
}
#[derive(Debug, Clone)]
struct PreviewImageLayer {
    handle: image::Handle,
    zoom_percent: u16,
    target_size: [u32; 2],
}

impl Program<Message> for PreviewImageLayer {
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
        let artboard = fitted_rect(bounds.size(), self.target_size, 26.0, self.zoom_percent);
        frame.draw_image(
            artboard,
            CanvasImage::new(self.handle.clone()).filter_method(FilterMethod::Linear),
        );
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy)]
struct ArtboardOverlay {
    show_grid: bool,
    zoom_percent: u16,
    target_size: [u32; 2],
    position: [f32; 2],
    scale: [f32; 2],
    rotation_z: i32,
    selection_bounds: Option<[u32; 4]>,
    tool: TransformTool,
    editable: bool,
}

#[derive(Debug, Default)]
struct ArtboardState {
    drag: Option<ArtboardDrag>,
}

#[derive(Debug, Clone, Copy)]
struct ArtboardDrag {
    start: Point,
    current: Point,
}

impl ArtboardOverlay {
    fn artboard(&self, bounds: Rectangle) -> Rectangle {
        fitted_rect(bounds.size(), self.target_size, 26.0, self.zoom_percent)
    }

    fn selection_rect(&self, artboard: Rectangle) -> Option<Rectangle> {
        self.selection_bounds
            .map(|bounds| rendered_selection_rect(artboard, self.target_size, bounds))
    }

    fn dragged_selection_rect(
        &self,
        state: &ArtboardState,
        artboard: Rectangle,
    ) -> Option<Rectangle> {
        let mut selected = self.selection_rect(artboard)?;
        let Some(drag) = state.drag else {
            return Some(selected);
        };
        let delta_x = drag.current.x - drag.start.x;
        let delta_y = drag.current.y - drag.start.y;
        match self.tool {
            TransformTool::Move => {
                selected.x += delta_x;
                selected.y += delta_y;
            }
            TransformTool::Scale => {
                let center = selected.center();
                let factor = (1.0 + (delta_x - delta_y) * 0.005).clamp(0.01, 100.0);
                selected.width *= factor;
                selected.height *= factor;
                selected.x = center.x - selected.width / 2.0;
                selected.y = center.y - selected.height / 2.0;
            }
            TransformTool::Rotate | TransformTool::Select => {}
        }
        Some(selected)
    }

    fn drag_values(&self, state: &ArtboardState, artboard: Rectangle) -> ([f32; 2], [f32; 2], i32) {
        let mut position = self.position;
        let mut scale = self.scale;
        let mut rotation_z = self.rotation_z;
        let Some(drag) = state.drag else {
            return (position, scale, rotation_z);
        };
        let delta_x = drag.current.x - drag.start.x;
        let delta_y = drag.current.y - drag.start.y;
        match self.tool {
            TransformTool::Move => {
                position[0] +=
                    delta_x * self.target_size[0].max(1) as f32 / artboard.width.max(1.0);
                position[1] -=
                    delta_y * self.target_size[1].max(1) as f32 / artboard.height.max(1.0);
            }
            TransformTool::Rotate => {
                let center = self.selection_rect(artboard).unwrap_or(artboard).center();
                let start = [drag.start.x - center.x, drag.start.y - center.y];
                let current = [drag.current.x - center.x, drag.current.y - center.y];
                let mut radians =
                    if start[0].hypot(start[1]) >= 4.0 && current[0].hypot(current[1]) >= 4.0 {
                        current[1].atan2(current[0]) - start[1].atan2(start[0])
                    } else {
                        delta_x * 0.01
                    };
                if radians > std::f32::consts::PI {
                    radians -= std::f32::consts::TAU;
                } else if radians < -std::f32::consts::PI {
                    radians += std::f32::consts::TAU;
                }
                let units = f64::from(self.rotation_z)
                    + f64::from(radians) * 65_536.0 / std::f64::consts::TAU;
                rotation_z = units
                    .round()
                    .clamp(f64::from(i32::MIN), f64::from(i32::MAX))
                    as i32;
            }
            TransformTool::Scale => {
                let factor = (1.0 + (delta_x - delta_y) * 0.005).clamp(0.01, 100.0);
                scale[0] *= factor;
                scale[1] *= factor;
            }
            TransformTool::Select => {}
        }
        (position, scale, rotation_z)
    }
}

impl Program<Message> for ArtboardOverlay {
    type State = ArtboardState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let canvas::Event::Mouse(mouse::Event::CursorMoved { position }) = event
            && let Some(drag) = &mut state.drag
        {
            drag.current = Point::new(position.x - bounds.x, position.y - bounds.y);
            return Some(canvas::Action::request_redraw().and_capture());
        }

        if let canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event
            && state.drag.is_some()
        {
            let artboard = self.artboard(bounds);
            let (position, scale, rotation_z) = self.drag_values(state, artboard);
            state.drag = None;
            let edit = match self.tool {
                TransformTool::Move => CanvasTransformEdit::Move(position),
                TransformTool::Rotate => CanvasTransformEdit::RotateZ(rotation_z),
                TransformTool::Scale => CanvasTransformEdit::Scale(scale),
                TransformTool::Select => return Some(canvas::Action::capture()),
            };
            return Some(
                canvas::Action::publish(Message::Model(EditorAction::CommitCanvasTransform(edit)))
                    .and_capture(),
            );
        }

        if !cursor.is_over(bounds) {
            return None;
        }
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && self.editable
            && matches!(
                self.tool,
                TransformTool::Move | TransformTool::Rotate | TransformTool::Scale
            )
        {
            let point = cursor.position_in(bounds)?;
            let selected = self.selection_rect(self.artboard(bounds))?;
            if selected.contains(point) {
                state.drag = Some(ArtboardDrag {
                    start: point,
                    current: point,
                });
                return Some(canvas::Action::request_redraw().and_capture());
            }
        }
        if let canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let y = match delta {
                mouse::ScrollDelta::Lines { y, .. } => *y,
                mouse::ScrollDelta::Pixels { y, .. } => *y / 24.0,
            };
            if y.abs() > f32::EPSILON {
                return Some(
                    canvas::Action::publish(Message::Model(EditorAction::ZoomBy(if y > 0.0 {
                        10
                    } else {
                        -10
                    })))
                    .and_capture(),
                );
            }
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let artboard = self.artboard(bounds);
        frame.stroke(
            &Path::rectangle(artboard.position(), artboard.size()),
            Stroke::default()
                .with_color(Color::from_rgba(0.63, 0.63, 0.67, 0.82))
                .with_width(1.0),
        );
        if self.show_grid {
            for index in 1..12 {
                let x = artboard.x + artboard.width * index as f32 / 12.0;
                frame.stroke(
                    &Path::line(
                        Point::new(x, artboard.y),
                        Point::new(x, artboard.y + artboard.height),
                    ),
                    Stroke::default()
                        .with_color(Color::from_rgba(0.63, 0.63, 0.67, 0.08))
                        .with_width(1.0),
                );
            }
            for index in 1..8 {
                let y = artboard.y + artboard.height * index as f32 / 8.0;
                frame.stroke(
                    &Path::line(
                        Point::new(artboard.x, y),
                        Point::new(artboard.x + artboard.width, y),
                    ),
                    Stroke::default()
                        .with_color(Color::from_rgba(0.63, 0.63, 0.67, 0.08))
                        .with_width(1.0),
                );
            }
        }
        if let Some(selected) = self.dragged_selection_rect(state, artboard) {
            let (_, _, rotation_z) = self.drag_values(state, artboard);
            let center = selected.center();
            frame.stroke(
                &Path::rectangle(selected.position(), selected.size()),
                Stroke::default().with_color(CYAN).with_width(1.25),
            );
            for point in [
                selected.position(),
                Point::new(selected.x + selected.width, selected.y),
                Point::new(selected.x, selected.y + selected.height),
                Point::new(selected.x + selected.width, selected.y + selected.height),
            ] {
                frame.fill_rectangle(
                    Point::new(point.x - 3.0, point.y - 3.0),
                    Size::new(6.0, 6.0),
                    CYAN,
                );
            }
            frame.stroke(
                &Path::line(
                    Point::new(center.x - 6.0, center.y),
                    Point::new(center.x + 6.0, center.y),
                ),
                Stroke::default().with_color(CYAN).with_width(1.0),
            );
            frame.stroke(
                &Path::line(
                    Point::new(center.x, center.y - 6.0),
                    Point::new(center.x, center.y + 6.0),
                ),
                Stroke::default().with_color(CYAN).with_width(1.0),
            );
            let radians = rotation_z as f32 * std::f32::consts::TAU / 65_536.0;
            frame.stroke(
                &Path::line(
                    center,
                    Point::new(
                        center.x + radians.cos() * 22.0,
                        center.y + radians.sin() * 22.0,
                    ),
                ),
                Stroke::default().with_color(YELLOW).with_width(1.0),
            );
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if !self.editable
            || !matches!(
                self.tool,
                TransformTool::Move | TransformTool::Rotate | TransformTool::Scale
            )
        {
            return mouse::Interaction::default();
        }
        let Some(point) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        let Some(selected) = self.selection_rect(self.artboard(bounds)) else {
            return mouse::Interaction::default();
        };
        if !selected.contains(point) {
            return mouse::Interaction::default();
        }
        match self.tool {
            TransformTool::Move => mouse::Interaction::Move,
            TransformTool::Rotate => mouse::Interaction::Crosshair,
            TransformTool::Scale => mouse::Interaction::ResizingDiagonallyDown,

            TransformTool::Select => mouse::Interaction::default(),
        }
    }
}

fn rendered_selection_rect(
    artboard: Rectangle,
    target_size: [u32; 2],
    bounds: [u32; 4],
) -> Rectangle {
    let target_width = target_size[0].max(1) as f32;
    let target_height = target_size[1].max(1) as f32;
    Rectangle::new(
        Point::new(
            artboard.x + bounds[0] as f32 / target_width * artboard.width,
            artboard.y + bounds[1] as f32 / target_height * artboard.height,
        ),
        Size::new(
            (bounds[2] as f32 / target_width * artboard.width).max(1.0),
            (bounds[3] as f32 / target_height * artboard.height).max(1.0),
        ),
    )
}
fn animated_transform_group<'a>(
    editor: &'a Editor,
    label: &'a str,
    values: &'a [String; 3],
    fields: [TransformField; 3],
    targets: [u16; 3],
    read_only: bool,
) -> Element<'a, Message> {
    let mut content = column![text(label).size(BODY_SIZE).color(MUTED)]
        .spacing(3)
        .width(Fill);
    for index in 0..3 {
        let field = fields[index];
        let mut input = input_component::field("0", &values[index]);
        if !read_only {
            input = input
                .on_input(move |value| Message::Model(EditorAction::EditTransform(field, value)));
        }
        content = content.push(
            row![
                text(["X", "Y", "Z"][index])
                    .size(CAPTION_SIZE)
                    .color(MUTED)
                    .width(18),
                input.width(Fill),
                super::animate_ui::inspector_property_controls(editor, targets[index]),
            ]
            .spacing(5)
            .height(ROW_HEIGHT)
            .align_y(Alignment::Center),
        );
    }
    content.into()
}

fn animated_color_group<'a>(
    editor: &'a Editor,
    label: &'a str,
    values: &'a [String; 4],
    multiply: bool,
    targets: [u16; 4],
    read_only: bool,
) -> Element<'a, Message> {
    let mut content = column![text(label).size(BODY_SIZE).color(MUTED)]
        .spacing(3)
        .width(Fill);
    for index in 0..4 {
        let field = if multiply {
            InspectorField::MultiplyColor(index)
        } else {
            InspectorField::AdditiveColor(index)
        };
        let mut input = input_component::field("0", &values[index]);
        if !read_only {
            input = input.on_input(move |value| {
                Message::Model(EditorAction::EditInspectorField(field, value))
            });
        }
        content = content.push(
            row![
                text(["R", "G", "B", "A"][index])
                    .size(CAPTION_SIZE)
                    .color(MUTED)
                    .width(18),
                input.width(Fill),
                super::animate_ui::inspector_property_controls(editor, targets[index]),
            ]
            .spacing(5)
            .height(ROW_HEIGHT)
            .align_y(Alignment::Center),
        );
    }
    content.into()
}

fn animated_inspector_field_row<'a>(
    editor: &'a Editor,
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    field: InspectorField,
    target: u16,
    read_only: bool,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(BODY_SIZE)
            .color(MUTED)
            .width(Length::Fixed(78.0)),
        inspector_field_input(placeholder, value, field, read_only),
        super::animate_ui::inspector_property_controls(editor, target),
    ]
    .spacing(5)
    .height(ROW_HEIGHT)
    .align_y(Alignment::Center)
    .into()
}

fn inspector_field_row<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    field: InspectorField,
    read_only: bool,
) -> Element<'a, Message> {
    labeled_control(
        label,
        inspector_field_input(placeholder, value, field, read_only),
    )
}

fn inspector_field_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    field: InspectorField,
    read_only: bool,
) -> Element<'a, Message> {
    let mut input = input_component::field(placeholder, value);
    if !read_only {
        input = input
            .on_input(move |value| Message::Model(EditorAction::EditInspectorField(field, value)));
    }
    input.into()
}

fn animated_inspector_vector2_row<'a>(
    editor: &'a Editor,
    label: &'a str,
    values: [&'a str; 2],
    fields: [InspectorField; 2],
    targets: [u16; 2],
    read_only: bool,
) -> Element<'a, Message> {
    let mut content = column![text(label).size(BODY_SIZE).color(MUTED)].spacing(3);
    for index in 0..2 {
        content = content.push(
            row![
                text(["X", "Y"][index])
                    .size(CAPTION_SIZE)
                    .color(MUTED)
                    .width(18),
                inspector_field_input("0", values[index], fields[index], read_only),
                super::animate_ui::inspector_property_controls(editor, targets[index]),
            ]
            .spacing(5)
            .height(ROW_HEIGHT)
            .align_y(Alignment::Center),
        );
    }
    content.into()
}

fn inspector_vector2_row<'a>(
    label: &'a str,
    values: [&'a str; 2],
    fields: [InspectorField; 2],
    read_only: bool,
) -> Element<'a, Message> {
    column![
        text(label).size(BODY_SIZE).color(MUTED),
        row![
            inspector_axis_input("X", values[0], fields[0], read_only),
            inspector_axis_input("Y", values[1], fields[1], read_only),
        ]
        .spacing(5),
    ]
    .spacing(4)
    .into()
}

fn animated_inspector_vertex_color_row<'a>(
    editor: &'a Editor,
    vertex: usize,
    values: &'a [String; 4],
    target: u16,
    read_only: bool,
) -> Element<'a, Message> {
    column![
        text(format!("Vertex {vertex}"))
            .size(CAPTION_SIZE)
            .color(MUTED),
        row![
            inspector_axis_input(
                "R",
                &values[0],
                InspectorField::VertexColor(vertex, 0),
                read_only,
            ),
            inspector_axis_input(
                "G",
                &values[1],
                InspectorField::VertexColor(vertex, 1),
                read_only,
            ),
            inspector_axis_input(
                "B",
                &values[2],
                InspectorField::VertexColor(vertex, 2),
                read_only,
            ),
            inspector_axis_input(
                "A",
                &values[3],
                InspectorField::VertexColor(vertex, 3),
                read_only,
            ),
            super::animate_ui::inspector_property_controls(editor, target),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    ]
    .spacing(3)
    .into()
}

fn inspector_axis_input<'a>(
    axis: &'static str,
    value: &'a str,
    field: InspectorField,
    read_only: bool,
) -> Element<'a, Message> {
    column![
        text(axis).size(OVERLINE_SIZE).color(MUTED),
        inspector_field_input("0", value, field, read_only),
    ]
    .spacing(2)
    .width(fill_portion(1))
    .into()
}

fn error_style(_: &Theme) -> container::Style {
    surface_style(Color::from_rgb(0.24, 0.065, 0.09), RED, 0.0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Transport;

    fn overlay(tool: TransformTool) -> ArtboardOverlay {
        ArtboardOverlay {
            show_grid: false,
            zoom_percent: 100,
            target_size: [1920, 1080],
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation_z: 0,
            selection_bounds: Some([760, 440, 400, 200]),
            tool,
            editable: true,
        }
    }

    #[test]
    fn canvas_drag_math_maps_move_rotate_and_scale_to_serialized_components() {
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 600.0));

        let move_overlay = overlay(TransformTool::Move);
        let artboard = move_overlay.artboard(bounds);
        let move_state = ArtboardState {
            drag: Some(ArtboardDrag {
                start: Point::new(100.0, 100.0),
                current: Point::new(150.0, 125.0),
            }),
        };
        let (position, scale, rotation) = move_overlay.drag_values(&move_state, artboard);
        assert!((position[0] - 50.0 * 1920.0 / artboard.width).abs() < 0.001);
        assert!((position[1] + 25.0 * 1080.0 / artboard.height).abs() < 0.001);
        assert_eq!(scale, [1.0, 1.0]);
        assert_eq!(rotation, 0);

        let scale_overlay = overlay(TransformTool::Scale);
        let (position, scale, rotation) = scale_overlay.drag_values(&move_state, artboard);
        assert_eq!(position, [0.0, 0.0]);
        assert_eq!(scale, [1.125, 1.125]);
        assert_eq!(rotation, 0);

        let rotate_overlay = overlay(TransformTool::Rotate);
        let center = rotate_overlay
            .selection_rect(artboard)
            .expect("test overlay has rendered bounds")
            .center();
        let rotate_state = ArtboardState {
            drag: Some(ArtboardDrag {
                start: Point::new(center.x + 50.0, center.y),
                current: Point::new(center.x, center.y + 50.0),
            }),
        };
        let (position, scale, rotation) = rotate_overlay.drag_values(&rotate_state, artboard);
        assert_eq!(position, [0.0, 0.0]);
        assert_eq!(scale, [1.0, 1.0]);
        assert_eq!(rotation, 16_384);
    }

    #[test]
    fn canvas_events_capture_drag_and_publish_one_committed_edit() {
        let overlay = overlay(TransformTool::Move);
        let bounds = Rectangle::new(Point::new(40.0, 60.0), Size::new(1000.0, 600.0));
        let artboard = overlay.artboard(bounds);
        let start = overlay
            .selection_rect(artboard)
            .expect("test overlay has rendered bounds")
            .center();
        let start_global = Point::new(start.x + bounds.x, start.y + bounds.y);
        let end_global = Point::new(start_global.x + 50.0, start_global.y + 25.0);
        let mut state = ArtboardState::default();

        let pressed = overlay
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                mouse::Cursor::Available(start_global),
            )
            .expect("pressing the selection starts a captured drag");
        assert_eq!(pressed.into_inner().2, iced::event::Status::Captured);
        assert_eq!(state.drag.unwrap().start, start);

        let moved = overlay
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::CursorMoved {
                    position: end_global,
                }),
                bounds,
                mouse::Cursor::Available(end_global),
            )
            .expect("moving an active drag requests a captured redraw");
        assert_eq!(moved.into_inner().2, iced::event::Status::Captured);

        let released = overlay
            .update(
                &mut state,
                &canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                bounds,
                mouse::Cursor::Available(end_global),
            )
            .expect("releasing an active drag publishes its edit");
        let (message, _, status) = released.into_inner();
        assert_eq!(status, iced::event::Status::Captured);
        assert!(state.drag.is_none());
        match message {
            Some(Message::Model(EditorAction::CommitCanvasTransform(
                CanvasTransformEdit::Move(position),
            ))) => {
                assert!((position[0] - 50.0 * 1920.0 / artboard.width).abs() < 0.001);
                assert!((position[1] + 25.0 * 1080.0 / artboard.height).abs() < 0.001);
            }
            _ => panic!("canvas drag did not publish its committed move"),
        }
    }
    #[test]
    fn preview_zoom_scales_the_fitted_image_bounds() {
        let bounds = Size::new(1000.0, 600.0);
        let fitted = fitted_rect(bounds, [1920, 1080], 26.0, 100);
        let zoomed_out = fitted_rect(bounds, [1920, 1080], 26.0, 50);
        let zoomed_in = fitted_rect(bounds, [1920, 1080], 26.0, 200);
        let expanded = fitted_rect(Size::new(1600.0, 1000.0), [1920, 1080], 26.0, 100);

        assert_eq!(zoomed_out.center(), fitted.center());
        assert_eq!(zoomed_in.center(), fitted.center());
        assert_eq!(zoomed_out.width, fitted.width * 0.5);
        assert_eq!(zoomed_out.height, fitted.height * 0.5);
        assert_eq!(zoomed_in.width, fitted.width * 2.0);
        assert_eq!(zoomed_in.height, fitted.height * 2.0);
        assert!(expanded.width > fitted.width);
        assert!(expanded.height > fitted.height);
    }

    #[test]
    fn cast_tree_rows_follow_recovered_parent_and_sibling_order() {
        let Some(path) =
            crate::test_support::game_data_path("surfboard/advertise/chu_ui_advertise_00_v10.srd")
        else {
            return;
        };
        let document = crate::document::EditorDocument::load(path).unwrap();
        let project = &document.project;
        let ((scene_index, layer_index), hierarchy) = project
            .scenes
            .iter()
            .enumerate()
            .flat_map(|(scene_index, scene)| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .filter_map(move |(layer_index, layer)| {
                        let hierarchy = layer.build_hierarchy().ok()?;
                        hierarchy
                            .children
                            .iter()
                            .any(|children| !children.is_empty())
                            .then_some(((scene_index, layer_index), hierarchy))
                    })
            })
            .next()
            .expect("fixture has a parented CAST hierarchy");
        let layer = &project.scenes[scene_index].layers[layer_index];
        let rows = cast_tree_rows(layer, "", |_| true);
        for (parent_index, children) in hierarchy.children.iter().enumerate() {
            let Some(parent_position) = rows.iter().position(|row| row.node_index == parent_index)
            else {
                continue;
            };
            for child in children {
                let child_position = rows
                    .iter()
                    .position(|row| row.node_index == *child)
                    .expect("child remains in tree");
                assert!(parent_position < child_position);
                assert_eq!(rows[child_position].depth, rows[parent_position].depth + 1);
            }
        }

        let collapsed_parent = hierarchy
            .children
            .iter()
            .position(|children| !children.is_empty())
            .unwrap();
        let hidden_children = &hierarchy.children[collapsed_parent];
        let collapsed = cast_tree_rows(layer, "", |node| node != collapsed_parent);
        assert!(
            hidden_children
                .iter()
                .all(|child| !collapsed.iter().any(|row| row.node_index == *child)),
            "collapsed structural descendants must not be promoted to root rows"
        );
    }
    #[test]
    fn compact_transport_tracks_the_visible_workspace_and_routes_every_intent() {
        use crate::editor::animate::AnimateAction;
        use crate::editor::model::{EditorAction, EditorModel};

        let Some(path) =
            crate::test_support::game_data_path("surfboard/advertise/chu_ui_advertise_00_v10.srd")
        else {
            return;
        };
        assert!(path.is_file());
        let mut model = EditorModel::with_workspace(Some(path), Workspace::Design);
        model.update(EditorAction::Animate(AnimateAction::SetFrame(7)));
        model.update(EditorAction::Animate(AnimateAction::TogglePlaying));
        assert_eq!(compact_playback_state(&model), (false, 0));

        model.update(EditorAction::SetWorkspace(Workspace::Animate));
        assert_eq!(compact_playback_state(&model), (true, 7));

        assert!(matches!(
            Transport::TogglePlaying.action(Workspace::Animate),
            EditorAction::Animate(AnimateAction::TogglePlaying)
        ));
        assert!(matches!(
            Transport::StepFrame(-1).action(Workspace::Animate),
            EditorAction::Animate(AnimateAction::StepFrame(-1))
        ));
        assert!(matches!(
            Transport::JumpToStart.action(Workspace::Animate),
            EditorAction::Animate(AnimateAction::JumpToStart)
        ));
        assert!(matches!(
            Transport::JumpToEnd.action(Workspace::Animate),
            EditorAction::Animate(AnimateAction::JumpToEnd)
        ));
    }
}
