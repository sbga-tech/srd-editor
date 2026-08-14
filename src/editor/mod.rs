pub mod animate;
mod animate_ui;
pub mod channels;
mod components;
pub mod model;
mod ui;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::widget::{image, pane_grid};
use iced::{Element, Event, Subscription, Task, Theme, event, keyboard, time, window};

use self::animate::{AnimateAction, Workspace};
use self::model::{EditorAction, EditorModel};
use crate::document::display_srd_name;
use crate::renderer::{
    PreviewCastSelection, PreviewCoordinator, PreviewFrame, PreviewHighlightBounds,
    PreviewLayerRequest, PreviewProfile, PreviewRequest,
};

pub(super) const EDITOR_PREVIEW_PROFILE: PreviewProfile = PreviewProfile::AdvertiseLogoMain;
pub(super) const EDITOR_PREVIEW_SIZE: [u32; 2] = [1920, 1080];
/// Environment override used by the screenshot harness to open directly in the
/// Animate workspace. It is deliberately not exposed as a user setting.
const WORKSPACE_ENV: &str = "SRD_EDITOR_WORKSPACE";
const ACTION_NOTICE_DURATION: Duration = Duration::from_secs(4);
const ACTION_NOTICE_TICK: Duration = Duration::from_millis(100);

fn startup_document_path() -> Option<PathBuf> {
    startup_document_path_from(std::env::args_os())
}

fn startup_document_path_from(
    arguments: impl IntoIterator<Item = std::ffi::OsString>,
) -> Option<PathBuf> {
    arguments.into_iter().nth(1).map(PathBuf::from)
}

const PANE_LAYOUT_FILE: &str = "pane-layouts-v3.txt";
const MIN_PANE_RATIO: f32 = 0.12;
const MAX_PANE_RATIO: f32 = 0.88;
const DEFAULT_DESIGN_PANE_RATIOS: [f32; 2] = [0.18, 0.67];
const DEFAULT_ANIMATE_PANE_RATIOS: [f32; 4] = [0.17, 0.69, 0.58, 0.65];

#[derive(Default)]
struct PersistedPaneRatios {
    design: Option<[f32; 2]>,
    animate: Option<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneKind {
    Hierarchy,
    Composition,
    Inspector,
    Assignment,
    DopeSheet,
}

/// Transport intents that both workspaces answer, routed to whichever one is
/// on screen so a keyboard shortcut never drives the hidden workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    TogglePlaying,
    StepFrame(i32),
    JumpToStart,
    JumpToEnd,
}

impl Transport {
    fn action(self, workspace: Workspace) -> EditorAction {
        match (workspace, self) {
            (Workspace::Design, Self::TogglePlaying) => EditorAction::TogglePlaying,
            (Workspace::Design, Self::StepFrame(delta)) => EditorAction::StepFrame(delta),
            (Workspace::Design, Self::JumpToStart) => EditorAction::JumpToStart,
            (Workspace::Design, Self::JumpToEnd) => EditorAction::JumpToEnd,
            (Workspace::Animate, Self::TogglePlaying) => {
                EditorAction::Animate(AnimateAction::TogglePlaying)
            }
            (Workspace::Animate, Self::StepFrame(delta)) => {
                EditorAction::Animate(AnimateAction::StepFrame(delta))
            }
            (Workspace::Animate, Self::JumpToStart) => {
                EditorAction::Animate(AnimateAction::JumpToStart)
            }
            (Workspace::Animate, Self::JumpToEnd) => {
                EditorAction::Animate(AnimateAction::JumpToEnd)
            }
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) struct ActionNotice {
    pub(crate) message: String,
    expires_at: Instant,
}

impl ActionNotice {
    fn shown_at(message: impl Into<String>, now: Instant) -> Self {
        Self {
            message: message.into(),
            expires_at: now + ACTION_NOTICE_DURATION,
        }
    }

    fn is_expired_at(&self, now: Instant) -> bool {
        now >= self.expires_at
    }
}

fn update_hover<T: Copy + Eq>(current: &mut Option<T>, target: T, hovered: bool) {
    if hovered {
        *current = Some(target);
    } else if *current == Some(target) {
        *current = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextDropdown {
    Scenes,
    AnimationSets,
}

#[derive(Debug, Clone)]
pub(super) enum ItemDialog {
    RenameScene {
        index: usize,
        value: String,
    },
    DeleteScene {
        index: usize,
        name: String,
    },
    RenameAnimationSet {
        scene_index: usize,
        index: usize,
        value: String,
    },
    DeleteAnimationSet {
        scene_index: usize,
        index: usize,
        name: String,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    Model(EditorAction),
    Transport(Transport),
    PaneResized(pane_grid::ResizeEvent),
    CastHovered(PreviewCastSelection, bool),
    LayerHovered((usize, usize), bool),
    AssignmentLayerHovered((usize, usize), bool),
    SaveRequested,
    SaveAsRequested,
    OpenFile,
    FilePicked(Option<PathBuf>),
    SaveAsFilePicked(Option<PathBuf>),
    ToggleSceneDropdown,
    ToggleAnimationSetDropdown,
    CloseContextDropdown,
    StartSceneRename(usize),
    StartAnimationSetRename(usize),
    RenameValueChanged(String),
    CommitItemRename,
    RequestDeleteScene(usize),
    RequestDeleteAnimationSet(usize),
    ConfirmItemDelete,
    CancelItemDialog,
    ActionNoticeTick(Instant),
    CaptureScreenshot,
    ScreenshotCaptured(window::Screenshot),
    PreviewAllocated {
        request_id: u64,
        size: [u32; 2],
        selection_bounds: Option<[u32; 4]>,
        highlight_bounds: Vec<PreviewHighlightBounds>,
        result: Result<image::Allocation, image::Error>,
    },
    CloseRequested,
    ConfirmDiscardAndClose,
    CancelClose,
}
pub struct Editor {
    pub(crate) design_panes: pane_grid::State<PaneKind>,
    pub(crate) animate_panes: pane_grid::State<PaneKind>,
    pub(crate) notice: Option<ActionNotice>,
    pub(crate) hovered_cast: Option<PreviewCastSelection>,
    pub(crate) hovered_layer: Option<(usize, usize)>,
    pub(crate) hovered_assignment_layer: Option<(usize, usize)>,
    pub(crate) screenshot_path: Option<PathBuf>,
    pub(crate) screenshot_ready_at: Option<Instant>,
    pub(crate) model: EditorModel,
    pub(crate) preview: PreviewCoordinator,
    pub(crate) preview_image: Option<image::Handle>,
    pub(crate) preview_allocation: Option<image::Allocation>,
    pub(crate) preview_request_id: u64,
    pub(crate) preview_presented_id: u64,
    pub(crate) preview_size: [u32; 2],
    pub(crate) preview_error: Option<String>,
    pub(crate) preview_selection_bounds: Option<[u32; 4]>,
    pub(crate) preview_highlight_bounds: Vec<PreviewHighlightBounds>,
    pub(crate) context_dropdown: Option<ContextDropdown>,
    pub(crate) item_dialog: Option<ItemDialog>,
    pub(crate) quit_confirmation_open: bool,
}

impl Editor {
    fn new() -> (Self, Task<Message>) {
        let workspace = Workspace::from_env_value(std::env::var(WORKSPACE_ENV).ok().as_deref());
        let mut model = EditorModel::with_workspace(startup_document_path(), workspace);
        if workspace == Workspace::Animate
            && model
                .selected_scene()
                .is_some_and(|scene| !scene.animation_sets.is_empty())
        {
            model.update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))));
            let scene_index = model.selected_scene_index();
            let typed_cast = model.selected_scene().and_then(|scene| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .find_map(|(layer_index, layer)| {
                        (0..layer.nodes.len()).find_map(|node_index| {
                            let known_payload = layer
                                .image_by_node
                                .get(node_index)
                                .is_some_and(Option::is_some)
                                || layer
                                    .csli_by_node
                                    .get(node_index)
                                    .is_some_and(Option::is_some)
                                || layer
                                    .number_by_node
                                    .get(node_index)
                                    .is_some_and(Option::is_some)
                                || layer
                                    .reference_by_node
                                    .get(node_index)
                                    .is_some_and(Option::is_some);
                            known_payload.then_some((layer_index, node_index))
                        })
                    })
            });
            if let Some((layer_index, node_index)) = typed_cast {
                model.update(EditorAction::SelectCast(
                    scene_index,
                    layer_index,
                    node_index,
                ));
            }
        }
        let screenshot_path = std::env::var_os("SRD_EDITOR_SCREENSHOT").map(PathBuf::from);
        let screenshot_ready_at = screenshot_path
            .as_ref()
            .map(|_| Instant::now() + Duration::from_secs(1));
        let (design_panes, animate_panes) = restored_pane_layouts();
        let mut editor = Self {
            design_panes,
            animate_panes,
            hovered_cast: None,
            hovered_layer: None,
            hovered_assignment_layer: None,
            notice: None,
            screenshot_path,
            screenshot_ready_at,
            model,
            preview: PreviewCoordinator::system_default(),
            preview_image: None,
            preview_allocation: None,
            preview_request_id: 0,
            preview_presented_id: 0,
            preview_size: [0, 0],
            preview_error: None,
            preview_selection_bounds: None,
            preview_highlight_bounds: Vec::new(),
            context_dropdown: None,
            item_dialog: None,
            quit_confirmation_open: false,
        };
        let preview = editor.refresh_preview();
        (editor, preview)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Model(action) => {
                self.context_dropdown = None;
                let change = self.model.update(action);
                if let Some(notice) = self.model.take_action_notice() {
                    self.show_notice(notice);
                }
                if change.preview {
                    return self.refresh_preview();
                }
            }
            Message::Transport(transport) => {
                let action = transport.action(self.model.workspace());
                return self.update(Message::Model(action));
            }
            Message::PaneResized(event) => {
                self.active_panes_mut().resize(event.split, event.ratio);
                self.persist_pane_layouts();
            }
            Message::CastHovered(cast, hovered) => {
                update_hover(&mut self.hovered_cast, cast, hovered);
            }
            Message::LayerHovered(layer, hovered) => {
                update_hover(&mut self.hovered_layer, layer, hovered);
            }
            Message::AssignmentLayerHovered(layer, hovered) => {
                update_hover(&mut self.hovered_assignment_layer, layer, hovered);
            }
            Message::SaveRequested => {
                return self.save_document();
            }
            Message::SaveAsRequested => {
                return self.request_save_as();
            }
            Message::OpenFile => {
                self.notice = None;
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Surfride project", &["srd"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    Message::FilePicked,
                );
            }
            Message::FilePicked(Some(path)) => {
                if self.model.load_document(path) {
                    self.show_notice("Project loaded");
                    return self.refresh_preview();
                } else if let Some(error) = self.model.load_error().map(str::to_owned) {
                    self.show_notice(error);
                }
            }
            Message::FilePicked(None) => {}
            Message::SaveAsFilePicked(Some(path)) => match self.model.save_as(&path) {
                Ok(report) => self.show_notice(format!(
                    "Saved {} ({} bytes)",
                    report.path.display(),
                    report.bytes_written
                )),
                Err(error) => self.show_notice(error),
            },
            Message::SaveAsFilePicked(None) => {}
            Message::ToggleSceneDropdown => {
                self.context_dropdown = if self.context_dropdown == Some(ContextDropdown::Scenes) {
                    None
                } else {
                    Some(ContextDropdown::Scenes)
                };
            }
            Message::ToggleAnimationSetDropdown => {
                self.context_dropdown =
                    if self.context_dropdown == Some(ContextDropdown::AnimationSets) {
                        None
                    } else {
                        Some(ContextDropdown::AnimationSets)
                    };
            }
            Message::CloseContextDropdown => self.context_dropdown = None,
            Message::StartSceneRename(index) => {
                self.context_dropdown = None;
                if !self.model.document_editing() {
                    return Task::none();
                }
                let Some(value) = self
                    .model
                    .document()
                    .and_then(|document| document.project.scenes.get(index))
                    .map(|scene| display_srd_name(&scene.name))
                else {
                    return Task::none();
                };
                let change = self.model.update(EditorAction::SelectScene(index));
                self.item_dialog = Some(ItemDialog::RenameScene { index, value });
                if change.preview {
                    return self.refresh_preview();
                }
            }
            Message::StartAnimationSetRename(index) => {
                self.context_dropdown = None;
                if !self.model.document_editing() {
                    return Task::none();
                }
                let scene_index = self.model.selected_scene_index();
                let Some(value) = self
                    .model
                    .selected_scene()
                    .and_then(|scene| scene.animation_sets.get(index))
                    .map(|set| display_srd_name(&set.name))
                else {
                    return Task::none();
                };
                self.item_dialog = Some(ItemDialog::RenameAnimationSet {
                    scene_index,
                    index,
                    value,
                });
            }
            Message::RenameValueChanged(value) => match &mut self.item_dialog {
                Some(ItemDialog::RenameScene { value: current, .. })
                | Some(ItemDialog::RenameAnimationSet { value: current, .. }) => {
                    *current = value;
                }
                _ => {}
            },
            Message::CommitItemRename => {
                let Some(dialog) = self.item_dialog.clone() else {
                    return Task::none();
                };
                let preview = match dialog {
                    ItemDialog::RenameScene { index, value } => {
                        let selected = self.model.update(EditorAction::SelectScene(index));
                        let renamed = self.model.update(EditorAction::RenameScene(value));
                        selected.preview || renamed.preview
                    }
                    ItemDialog::RenameAnimationSet {
                        scene_index,
                        index,
                        value,
                    } => {
                        let selected = self.model.update(EditorAction::SelectScene(scene_index));
                        let loaded =
                            self.model
                                .update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(
                                    index,
                                ))));
                        let renamed = self
                            .model
                            .update(EditorAction::Animate(AnimateAction::SetName(value)));
                        selected.preview || loaded.preview || renamed.preview
                    }
                    ItemDialog::DeleteScene { .. } | ItemDialog::DeleteAnimationSet { .. } => {
                        return Task::none();
                    }
                };
                if let Some(notice) = self.model.take_action_notice() {
                    self.show_notice(notice);
                } else {
                    self.item_dialog = None;
                }
                if preview {
                    return self.refresh_preview();
                }
            }
            Message::RequestDeleteScene(index) => {
                self.context_dropdown = None;
                if !self.model.document_editing() {
                    return Task::none();
                }
                let Some(name) = self
                    .model
                    .document()
                    .and_then(|document| document.project.scenes.get(index))
                    .map(|scene| {
                        let name = display_srd_name(&scene.name);
                        if name.is_empty() {
                            format!("Scene {}", index + 1)
                        } else {
                            name
                        }
                    })
                else {
                    return Task::none();
                };
                self.item_dialog = Some(ItemDialog::DeleteScene { index, name });
            }
            Message::RequestDeleteAnimationSet(index) => {
                self.context_dropdown = None;
                if !self.model.document_editing() {
                    return Task::none();
                }
                let scene_index = self.model.selected_scene_index();
                let Some(name) = self
                    .model
                    .selected_scene()
                    .and_then(|scene| scene.animation_sets.get(index))
                    .map(|set| {
                        let name = display_srd_name(&set.name);
                        if name.is_empty() {
                            format!("Animation set {}", index + 1)
                        } else {
                            name
                        }
                    })
                else {
                    return Task::none();
                };
                self.item_dialog = Some(ItemDialog::DeleteAnimationSet {
                    scene_index,
                    index,
                    name,
                });
            }
            Message::ConfirmItemDelete => {
                let Some(dialog) = self.item_dialog.take() else {
                    return Task::none();
                };
                let preview = match dialog {
                    ItemDialog::DeleteScene { index, .. } => {
                        let selected = self.model.update(EditorAction::SelectScene(index));
                        let deleted = self.model.update(EditorAction::DeleteScene);
                        selected.preview || deleted.preview
                    }
                    ItemDialog::DeleteAnimationSet {
                        scene_index, index, ..
                    } => {
                        let selected = self.model.update(EditorAction::SelectScene(scene_index));
                        let loaded =
                            self.model
                                .update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(
                                    index,
                                ))));
                        let deleted = self
                            .model
                            .update(EditorAction::Animate(AnimateAction::DeleteSet));
                        selected.preview || loaded.preview || deleted.preview
                    }
                    ItemDialog::RenameScene { .. } | ItemDialog::RenameAnimationSet { .. } => {
                        return Task::none();
                    }
                };
                if let Some(notice) = self.model.take_action_notice() {
                    self.show_notice(notice);
                }
                if preview {
                    return self.refresh_preview();
                }
            }
            Message::CancelItemDialog => self.item_dialog = None,
            Message::ActionNoticeTick(now) => self.expire_notice_at(now),
            Message::CloseRequested => {
                if self.model.is_dirty() {
                    self.model.stop_playback();
                    self.quit_confirmation_open = true;
                } else {
                    return window::latest().and_then(window::close);
                }
            }
            Message::ConfirmDiscardAndClose => {
                return window::latest().and_then(window::close);
            }
            Message::CancelClose => {
                self.quit_confirmation_open = false;
            }
            Message::CaptureScreenshot => {
                if self
                    .screenshot_ready_at
                    .is_some_and(|ready_at| Instant::now() < ready_at)
                {
                    return Task::none();
                }
                return window::latest()
                    .and_then(window::screenshot)
                    .map(Message::ScreenshotCaptured);
            }
            Message::ScreenshotCaptured(screenshot) => {
                if let Some(path) = self.screenshot_path.take()
                    && let Err(error) = save_screenshot(&path, &screenshot)
                {
                    self.show_notice(error);
                    self.screenshot_path = Some(path);
                }
            }
            Message::PreviewAllocated {
                request_id,
                size,
                selection_bounds,
                highlight_bounds,
                result,
            } => {
                let allocation_failed_for_current_request = request_id == self.preview_request_id
                    && request_id > self.preview_presented_id
                    && result.is_err();
                if self.finish_preview_allocation(
                    request_id,
                    size,
                    selection_bounds,
                    highlight_bounds,
                    result,
                ) {
                    // Every-frame playback: request exactly one successor only
                    // after the active request allocated successfully.
                    return self.advance_playback();
                }
                if allocation_failed_for_current_request {
                    self.model.stop_playback();
                }
            }
        }
        Task::none()
    }
    fn show_notice(&mut self, message: impl Into<String>) {
        self.show_notice_at(message, Instant::now());
    }

    fn show_notice_at(&mut self, message: impl Into<String>, now: Instant) {
        self.notice = Some(ActionNotice::shown_at(message, now));
    }

    fn expire_notice_at(&mut self, now: Instant) {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| notice.is_expired_at(now))
        {
            self.notice = None;
        }
    }

    fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }

    /// Playback is driven entirely by completed preview allocations, so there
    /// is no wall-clock frame subscription that could drop frames.
    fn subscription(&self) -> Subscription<Message> {
        let screenshot = if self.screenshot_path.is_some() {
            time::every(Duration::from_secs(5)).map(|_| Message::CaptureScreenshot)
        } else {
            Subscription::none()
        };
        let notice = if self.notice.is_some() {
            time::every(ACTION_NOTICE_TICK).map(Message::ActionNoticeTick)
        } else {
            Subscription::none()
        };
        Subscription::batch([screenshot, notice, event::listen_with(shortcut)])
    }

    fn save_document(&mut self) -> Task<Message> {
        if self.model.project_path().is_none() {
            return self.request_save_as();
        }
        match self.model.save() {
            Ok(report) => self.show_notice(format!(
                "Saved {} ({} bytes)",
                report.path.display(),
                report.bytes_written
            )),
            Err(error) => self.show_notice(error),
        }
        Task::none()
    }

    fn request_save_as(&mut self) -> Task<Message> {
        self.notice = None;
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .add_filter("Surfride project", &["srd"])
                    .save_file()
                    .await
                    .map(|file| file.path().to_owned())
            },
            Message::SaveAsFilePicked,
        )
    }

    /// Requests the next playback frame once the previous allocation landed.
    /// Both workspaces advance this way: there is no timer that could outrun
    /// the renderer and skip frames.
    fn advance_playback(&mut self) -> Task<Message> {
        let action = match self.model.workspace() {
            Workspace::Design if self.model.playing() => EditorAction::Tick,
            Workspace::Animate if self.model.animate().playing() => {
                EditorAction::Animate(AnimateAction::FrameCompleted)
            }
            _ => return Task::none(),
        };
        if self.model.update(action).preview {
            return self.refresh_preview();
        }
        Task::none()
    }

    fn refresh_preview(&mut self) -> Task<Message> {
        self.preview_error = None;
        self.preview_request_id = self.preview_request_id.wrapping_add(1);
        let request_id = self.preview_request_id;
        if self.model.document().is_none() || self.model.selected_scene().is_none() {
            self.clear_preview(request_id);
            self.model.stop_playback();
            return Task::none();
        }
        let document = self
            .model
            .document()
            .expect("validated document with selected scene");
        match self.preview.render(PreviewRequest {
            foreground: PreviewLayerRequest {
                document,
                document_revision: self.model.document_revision(),
                scene_index: self.model.selected_scene_index(),
                animation_assignment: self.model.preview_assignment(),
                frame: self.model.preview_frame(),
                hidden_layers: self.model.hidden_layers(),
                solo_layers: self.model.solo_layers(),
                hidden_casts: self.model.hidden_casts(),
                solo_casts: self.model.solo_casts(),
                runtime_text_inputs: self.model.runtime_text_inputs(),
            },
            background: None,
            profile: EDITOR_PREVIEW_PROFILE,
            selection: self
                .model
                .selected_node_index()
                .map(|node_index| PreviewCastSelection {
                    scene_index: self.model.selected_scene_index(),
                    layer_index: self.model.selected_layer_index(),
                    node_index,
                }),
            highlights: self.model.preview_highlights(),
        }) {
            Ok(frame) => Self::allocate_preview(request_id, frame),
            Err(error) => {
                self.preview_error = Some(error);
                self.model.stop_playback();
                Task::none()
            }
        }
    }

    fn allocate_preview(request_id: u64, frame: PreviewFrame) -> Task<Message> {
        let size = [frame.width, frame.height];
        let selection_bounds = frame.selection_bounds;
        let highlight_bounds = frame.highlight_bounds;
        let handle = image::Handle::from_rgba(frame.width, frame.height, frame.rgba);
        image::allocate(handle).map(move |result| Message::PreviewAllocated {
            request_id,
            size,
            selection_bounds,
            highlight_bounds: highlight_bounds.clone(),
            result,
        })
    }

    /// Updates the displayed image and reports whether this was the current
    /// request's first successful allocation. Only that event can clock
    /// every-frame playback.
    fn finish_preview_allocation(
        &mut self,
        request_id: u64,
        size: [u32; 2],
        selection_bounds: Option<[u32; 4]>,
        highlight_bounds: Vec<PreviewHighlightBounds>,
        result: Result<image::Allocation, image::Error>,
    ) -> bool {
        let current = request_id == self.preview_request_id;
        if request_id <= self.preview_presented_id {
            return false;
        }
        match result {
            Ok(allocation) => {
                self.preview_image = Some(allocation.handle().clone());
                self.preview_allocation = Some(allocation);
                self.preview_size = size;
                self.preview_selection_bounds = selection_bounds;
                self.preview_highlight_bounds = highlight_bounds;
                self.preview_presented_id = request_id;
                if current {
                    self.preview_error = None;
                }
                current
            }
            Err(error) if current => {
                self.preview_error = Some(format!("failed to allocate preview image: {error}"));
                false
            }
            Err(_) => false,
        }
    }

    fn clear_preview(&mut self, request_id: u64) {
        self.preview_image = None;
        self.preview_allocation = None;
        self.preview_size = [0, 0];
        self.preview_selection_bounds = None;
        self.preview_highlight_bounds.clear();
        self.preview_presented_id = request_id;
    }

    fn active_panes_mut(&mut self) -> &mut pane_grid::State<PaneKind> {
        match self.model.workspace() {
            Workspace::Design => &mut self.design_panes,
            Workspace::Animate => &mut self.animate_panes,
        }
    }

    fn persist_pane_layouts(&self) {
        let Some(design) = pane_ratio_sequence::<2>(&self.design_panes) else {
            return;
        };
        let Some(animate) = pane_ratio_sequence::<4>(&self.animate_panes) else {
            return;
        };
        persist_pane_ratios(design, animate);
    }
}

fn restored_pane_layouts() -> (pane_grid::State<PaneKind>, pane_grid::State<PaneKind>) {
    let saved = load_pane_ratios();
    (
        pane_layout(
            Workspace::Design,
            saved.design.unwrap_or(DEFAULT_DESIGN_PANE_RATIOS),
        ),
        pane_layout(
            Workspace::Animate,
            saved.animate.unwrap_or(DEFAULT_ANIMATE_PANE_RATIOS),
        ),
    )
}

#[cfg(test)]
fn default_design_pane_layout() -> pane_grid::State<PaneKind> {
    pane_layout(Workspace::Design, DEFAULT_DESIGN_PANE_RATIOS)
}

#[cfg(test)]
fn default_animate_pane_layout() -> pane_grid::State<PaneKind> {
    pane_layout(Workspace::Animate, DEFAULT_ANIMATE_PANE_RATIOS)
}

fn pane_layout(workspace: Workspace, ratios: impl AsRef<[f32]>) -> pane_grid::State<PaneKind> {
    use pane_grid::{Axis, Configuration};
    let ratios = ratios.as_ref();
    let configuration = match workspace {
        Workspace::Design => Configuration::Split {
            axis: Axis::Vertical,
            ratio: ratios[0],
            a: Box::new(Configuration::Pane(PaneKind::Hierarchy)),
            b: Box::new(Configuration::Split {
                axis: Axis::Vertical,
                ratio: ratios[1],
                a: Box::new(Configuration::Pane(PaneKind::Composition)),
                b: Box::new(Configuration::Pane(PaneKind::Inspector)),
            }),
        },
        Workspace::Animate => Configuration::Split {
            axis: Axis::Vertical,
            ratio: ratios[0],
            a: Box::new(Configuration::Pane(PaneKind::Hierarchy)),
            b: Box::new(Configuration::Split {
                axis: Axis::Vertical,
                ratio: ratios[1],
                a: Box::new(Configuration::Split {
                    axis: Axis::Horizontal,
                    ratio: ratios[2],
                    a: Box::new(Configuration::Pane(PaneKind::Composition)),
                    b: Box::new(Configuration::Pane(PaneKind::DopeSheet)),
                }),
                b: Box::new(Configuration::Split {
                    axis: Axis::Horizontal,
                    ratio: ratios[3],
                    a: Box::new(Configuration::Pane(PaneKind::Inspector)),
                    b: Box::new(Configuration::Pane(PaneKind::Assignment)),
                }),
            }),
        },
    };
    pane_grid::State::with_configuration(configuration)
}

fn pane_ratio_sequence<const N: usize>(panes: &pane_grid::State<PaneKind>) -> Option<[f32; N]> {
    fn collect(node: &pane_grid::Node, ratios: &mut Vec<f32>) {
        if let pane_grid::Node::Split { ratio, a, b, .. } = node {
            ratios.push(*ratio);
            collect(a, ratios);
            collect(b, ratios);
        }
    }

    let mut ratios = Vec::with_capacity(N);
    collect(panes.layout(), &mut ratios);
    ratios.try_into().ok()
}

fn pane_layout_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let mut directory = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(target_os = "macos")]
    let mut directory =
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut directory = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    directory.push("srd-editor");
    directory.push(PANE_LAYOUT_FILE);
    Some(directory)
}

fn parse_ratio_sequence<const N: usize>(value: &str) -> Option<[f32; N]> {
    let mut ratios = [0.0; N];
    let mut count = 0;
    for field in value.split(',') {
        if count == N {
            return None;
        }
        let ratio = field.trim().parse::<f32>().ok()?;
        if !ratio.is_finite() {
            return None;
        }
        ratios[count] = ratio.clamp(MIN_PANE_RATIO, MAX_PANE_RATIO);
        count += 1;
    }
    (count == N).then_some(ratios)
}

fn load_pane_ratios() -> PersistedPaneRatios {
    let Some(path) = pane_layout_config_path() else {
        return PersistedPaneRatios::default();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return PersistedPaneRatios::default();
    };
    let mut ratios = PersistedPaneRatios::default();
    for line in contents.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        match name.trim() {
            "design" => ratios.design = parse_ratio_sequence(value),
            "animate" => ratios.animate = parse_ratio_sequence(value),
            _ => {}
        }
    }
    ratios
}

fn format_ratio_sequence(ratios: &[f32]) -> String {
    ratios
        .iter()
        .map(|ratio| format!("{ratio:.5}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn persist_pane_ratios(design: [f32; 2], animate: [f32; 4]) {
    let Some(path) = pane_layout_config_path() else {
        return;
    };
    let Some(directory) = path.parent() else {
        return;
    };
    if fs::create_dir_all(directory).is_err() {
        return;
    }
    let contents = format!(
        "version=3\ndesign={}\nanimate={}\n",
        format_ratio_sequence(&design),
        format_ratio_sequence(&animate)
    );
    let _ = fs::write(path, contents);
}

fn shortcut(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    if matches!(&event, Event::Window(window::Event::CloseRequested)) {
        return Some(Message::CloseRequested);
    }
    if status == event::Status::Captured {
        return None;
    }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    match key.as_ref() {
        keyboard::Key::Character("o") if modifiers.command() => Some(Message::OpenFile),
        keyboard::Key::Character("s") if modifiers.command() && modifiers.shift() => {
            Some(Message::SaveAsRequested)
        }
        keyboard::Key::Character("s") if modifiers.command() => Some(Message::SaveRequested),
        keyboard::Key::Character("z") if modifiers.command() && modifiers.shift() => {
            Some(Message::Model(EditorAction::Redo))
        }
        keyboard::Key::Character("z") if modifiers.command() => {
            Some(Message::Model(EditorAction::Undo))
        }
        keyboard::Key::Character("y") if modifiers.command() => {
            Some(Message::Model(EditorAction::Redo))
        }
        keyboard::Key::Character("g") => Some(Message::Model(EditorAction::ToggleGrid)),
        keyboard::Key::Named(keyboard::key::Named::Space) => {
            Some(Message::Transport(Transport::TogglePlaying))
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
            Some(Message::Transport(Transport::StepFrame(-1)))
        }
        keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
            Some(Message::Transport(Transport::StepFrame(1)))
        }
        keyboard::Key::Named(keyboard::key::Named::Home) => {
            Some(Message::Transport(Transport::JumpToStart))
        }
        keyboard::Key::Named(keyboard::key::Named::End) => {
            Some(Message::Transport(Transport::JumpToEnd))
        }
        _ => None,
    }
}

fn title(editor: &Editor) -> String {
    let path = editor.model.project_path().map_or_else(
        || editor.model.project_file_name(),
        |path| path.display().to_string(),
    );
    let dirty_marker = if editor.model.is_dirty() { " *" } else { "" };
    let mode_marker = if editor.model.document().is_some() && !editor.model.document_editing() {
        " [READONLY]"
    } else {
        ""
    };
    format!("{path}{dirty_marker}{mode_marker} — SRD Editor")
}

fn save_screenshot(path: &std::path::Path, screenshot: &window::Screenshot) -> Result<(), String> {
    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let mut encoder = png::Encoder::new(file, screenshot.size.width, screenshot.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to initialize screenshot PNG: {error}"))?;
    writer
        .write_image_data(&screenshot.rgba)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    writer
        .finish()
        .map_err(|error| format!("failed to finalize {}: {error}", path.display()))
}

fn theme(_: &Editor) -> Theme {
    Theme::custom(
        "SRD Zinc",
        iced::theme::Palette {
            background: iced::Color::from_rgb8(9, 9, 11),
            text: iced::Color::from_rgb8(250, 250, 250),
            primary: iced::Color::from_rgb8(212, 212, 216),
            success: iced::Color::from_rgb8(134, 239, 172),
            danger: iced::Color::from_rgb8(248, 113, 113),
            warning: iced::Color::from_rgb8(251, 191, 36),
        },
    )
}

pub fn run() -> iced::Result {
    iced::application(Editor::new, Editor::update, Editor::view)
        .title(title)
        .subscription(Editor::subscription)
        .theme(theme)
        .window(window::Settings {
            size: iced::Size::new(1600.0, 1000.0),
            min_size: Some(iced::Size::new(1100.0, 720.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        })
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .run()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet, VecDeque};
    use std::sync::mpsc::{self, Sender};

    use crate::renderer::PreviewRenderer;
    use crate::renderer::{FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput};

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct LayerSnapshot {
        path: Option<PathBuf>,
        document_revision: u64,
        scene_index: usize,
        animation_assignment: Option<crate::scene::AnimationSetDefinition>,
        frame: i32,
        hidden_layers: BTreeSet<usize>,
        runtime_text_inputs: BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct RequestSnapshot {
        foreground: LayerSnapshot,
        background: Option<LayerSnapshot>,
        profile: crate::renderer::PreviewProfile,
        selection: Option<PreviewCastSelection>,
        highlights: Vec<crate::renderer::PreviewHighlightRequest>,
    }
    struct RecordingRenderer {
        requests: Sender<RequestSnapshot>,
        results: VecDeque<Result<PreviewFrame, String>>,
    }
    impl PreviewRenderer for RecordingRenderer {
        fn name(&self) -> &str {
            "recording preview"
        }

        fn omissions(&self) -> &str {
            "test renderer"
        }

        fn render(&mut self, request: PreviewRequest<'_>) -> Result<PreviewFrame, String> {
            fn snapshot(layer: &PreviewLayerRequest<'_>) -> LayerSnapshot {
                LayerSnapshot {
                    path: layer.document.path().map(|path| path.to_path_buf()),
                    document_revision: layer.document_revision,
                    scene_index: layer.scene_index,
                    animation_assignment: layer.animation_assignment.cloned(),
                    frame: layer.frame,
                    hidden_layers: layer.hidden_layers.clone(),
                    runtime_text_inputs: layer.runtime_text_inputs.clone(),
                }
            }

            self.requests
                .send(RequestSnapshot {
                    foreground: snapshot(&request.foreground),
                    background: request.background.as_ref().map(snapshot),
                    profile: request.profile,
                    selection: request.selection,
                    highlights: request.highlights.to_vec(),
                })
                .expect("recording preview receiver remains live");
            self.results
                .pop_front()
                .unwrap_or_else(|| Err("recording renderer has no queued result".into()))
        }
    }

    fn test_editor(model: EditorModel, renderer: RecordingRenderer) -> Editor {
        let design_panes = default_design_pane_layout();
        let animate_panes = default_animate_pane_layout();
        Editor {
            design_panes,
            animate_panes,
            hovered_cast: None,
            hovered_layer: None,
            hovered_assignment_layer: None,
            notice: None,
            screenshot_path: None,
            model,
            preview: PreviewCoordinator::new(renderer),
            preview_image: None,
            screenshot_ready_at: None,
            preview_allocation: None,
            preview_request_id: 0,
            preview_presented_id: 0,
            preview_size: [0, 0],
            preview_error: None,
            preview_selection_bounds: None,
            preview_highlight_bounds: Vec::new(),
            context_dropdown: None,
            item_dialog: None,
            quit_confirmation_open: false,
        }
    }

    #[test]
    fn preview_submission_starts_only_after_the_first_scene_exists() {
        let model = EditorModel::new(None);
        let (sender, receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: sender,
            results: VecDeque::from([Err("expected empty-scene renderer stop".into())]),
        };
        let mut editor = test_editor(model, renderer);

        drop(editor.refresh_preview());
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));

        editor.model.update(EditorAction::AddScene);
        drop(editor.refresh_preview());
        let request = receiver.try_recv().expect("first scene submits preview");
        assert_eq!(request.foreground.path, None);
        assert_eq!(request.foreground.scene_index, 0);
    }

    #[test]
    fn empty_scene_dropdown_adds_then_edits_its_first_item() {
        let model = EditorModel::new(None);
        let (sender, _receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: sender,
            results: VecDeque::from([
                Err("expected add preview stop".into()),
                Err("expected rename preview stop".into()),
            ]),
        };
        let mut editor = test_editor(model, renderer);

        drop(editor.update(Message::ToggleSceneDropdown));
        assert_eq!(editor.context_dropdown, Some(ContextDropdown::Scenes));
        drop(ui::view(&editor));
        drop(editor.update(Message::Model(EditorAction::AddScene)));
        assert!(editor.context_dropdown.is_none());
        assert_eq!(editor.model.selected_scene_name(), "Scene 1");

        drop(editor.update(Message::StartSceneRename(0)));
        assert!(matches!(
            editor.item_dialog,
            Some(ItemDialog::RenameScene { index: 0, .. })
        ));
        drop(editor.update(Message::RenameValueChanged("Gameplay".into())));
        drop(editor.update(Message::CommitItemRename));
        assert!(editor.item_dialog.is_none());
        assert_eq!(editor.model.selected_scene_name(), "Gameplay");

        drop(editor.update(Message::RequestDeleteScene(0)));
        assert!(matches!(
            editor.item_dialog,
            Some(ItemDialog::DeleteScene { index: 0, .. })
        ));
        drop(editor.update(Message::ConfirmItemDelete));
        assert!(editor.item_dialog.is_none());
        assert!(editor.model.selected_scene().is_none());
        assert!(!editor.model.is_dirty());
    }

    #[test]
    fn animation_set_dropdown_adds_renames_and_deletes_items() {
        let mut model = EditorModel::new(None);
        model.update(EditorAction::AddScene);
        model.take_action_notice();
        let (sender, _receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: sender,
            results: VecDeque::from([
                Err("expected create-set preview stop".into()),
                Err("expected rename-set preview stop".into()),
                Err("expected delete-set preview stop".into()),
            ]),
        };
        let mut editor = test_editor(model, renderer);

        drop(editor.update(Message::ToggleAnimationSetDropdown));
        assert_eq!(
            editor.context_dropdown,
            Some(ContextDropdown::AnimationSets)
        );
        drop(editor.update(Message::Model(EditorAction::Animate(
            AnimateAction::CreateSet,
        ))));
        assert!(editor.context_dropdown.is_none());
        assert_eq!(
            editor.model.selected_scene().unwrap().animation_sets.len(),
            1
        );

        drop(editor.update(Message::StartAnimationSetRename(0)));
        assert!(matches!(
            editor.item_dialog,
            Some(ItemDialog::RenameAnimationSet { index: 0, .. })
        ));
        drop(editor.update(Message::RenameValueChanged("Intro".into())));
        drop(editor.update(Message::CommitItemRename));
        assert!(editor.item_dialog.is_none());
        assert_eq!(
            editor.model.selected_scene().unwrap().animation_sets[0].name,
            b"Intro"
        );

        drop(editor.update(Message::RequestDeleteAnimationSet(0)));
        assert!(matches!(
            editor.item_dialog,
            Some(ItemDialog::DeleteAnimationSet { index: 0, .. })
        ));
        drop(editor.update(Message::ConfirmItemDelete));
        assert!(editor.item_dialog.is_none());
        assert!(
            editor
                .model
                .selected_scene()
                .unwrap()
                .animation_sets
                .is_empty()
        );
    }

    #[test]
    fn persisted_pane_ratios_require_a_complete_finite_sequence_and_clamp_bounds() {
        assert_eq!(
            parse_ratio_sequence::<3>("0.01,0.50,2.00"),
            Some([MIN_PANE_RATIO, 0.50, MAX_PANE_RATIO])
        );
        assert_eq!(parse_ratio_sequence::<3>("0.5,not-a-number,0.7"), None);
        assert_eq!(parse_ratio_sequence::<3>("0.5,0.7"), None);
        assert_eq!(parse_ratio_sequence::<3>("0.5,0.7,0.8,0.9"), None);
    }

    #[test]
    fn startup_without_an_srd_argument_has_no_document() {
        assert_eq!(
            startup_document_path_from([std::ffi::OsString::from("srd-editor")]),
            None
        );
    }

    #[test]
    fn startup_uses_the_explicit_srd_argument() {
        let path = PathBuf::from("project.srd");
        assert_eq!(
            startup_document_path_from([
                std::ffi::OsString::from("srd-editor"),
                path.clone().into_os_string(),
            ]),
            Some(path)
        );
    }

    fn sample_srd(relative: &str) -> Option<PathBuf> {
        crate::test_support::game_data_path(PathBuf::from("surfboard").join(relative))
    }

    fn temporary_srd_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "srd-editor-preview-{label}-{}-{}.srd",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn select_first_text_cast(model: &mut EditorModel) -> (usize, usize, usize) {
        let selection =
            model
                .document()
                .expect("test SRD is loaded")
                .project
                .scenes
                .iter()
                .enumerate()
                .find_map(|(scene_index, scene)| {
                    scene
                        .layers
                        .iter()
                        .enumerate()
                        .find_map(|(layer_index, layer)| {
                            layer.image_by_node.iter().enumerate().find_map(
                                |(node_index, image)| {
                                    image
                                        .as_ref()
                                        .is_some_and(|image| image.creates_text_cast())
                                        .then_some((scene_index, layer_index, node_index))
                                },
                            )
                        })
                })
                .expect("fixture contains an SrTextCast");
        model.update(EditorAction::SelectScene(selection.0));
        model.update(EditorAction::SelectSceneLayer(selection.0, selection.1));
        model.update(EditorAction::SelectNode(selection.2));
        selection
    }

    #[test]
    fn refresh_preview_uses_fixed_profile_and_foreground_editor_state() {
        let Some(foreground_path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        assert!(foreground_path.is_file());

        let mut model = EditorModel::new(Some(foreground_path.clone()));
        let (_, selected_layer, _) = select_first_text_cast(&mut model);
        model.update(EditorAction::ToggleRuntimeTextInput);
        model.update(EditorAction::SetRuntimeTextSubstitution(2, "CARD".into()));
        model.update(EditorAction::ToggleLayerVisible(selected_layer));
        let expected_foreground_revision = model.document_revision();

        let (request_sender, request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::from([Ok(PreviewFrame {
                width: 2,
                height: 1,
                rgba: vec![1, 2, 3, 4, 5, 6, 7, 8],
                selection_bounds: Some([0, 0, 2, 1]),
                highlight_bounds: Vec::new(),
            })]),
        };
        let mut editor = test_editor(model, renderer);
        drop(editor.refresh_preview());

        assert_eq!(editor.preview_size, [0, 0]);
        assert!(editor.preview_image.is_none());
        assert!(editor.preview_allocation.is_none());
        assert_eq!(editor.preview_request_id, 1);
        assert!(editor.preview_error.is_none());
        let request = request_receiver.recv().unwrap();
        assert_eq!(request.profile, EDITOR_PREVIEW_PROFILE);
        assert_eq!(request.foreground.animation_assignment, None);
        assert!(request.highlights.is_empty());
        assert_eq!(request.foreground.path, Some(foreground_path));
        assert_eq!(
            request.foreground.document_revision,
            expected_foreground_revision
        );
        assert!(request.foreground.hidden_layers.contains(&selected_layer));
        assert_eq!(request.foreground.runtime_text_inputs.len(), 1);
        assert!(
            request
                .foreground
                .runtime_text_inputs
                .values()
                .any(|input| { input.substitutions[2] == b"CARD" })
        );
        assert!(request.background.is_none());
    }

    #[test]
    fn refresh_preview_keeps_completed_frame_during_errors_and_allocation() {
        let path = temporary_srd_path("stale-preview");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let model = EditorModel::new(Some(path.clone()));
        let (request_sender, _request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::from([
                Err("synthetic preview failure".into()),
                Ok(PreviewFrame {
                    width: 2,
                    height: 1,
                    rgba: vec![0; 8],
                    selection_bounds: None,
                    highlight_bounds: Vec::new(),
                }),
            ]),
        };
        let mut editor = test_editor(model, renderer);
        let retained = image::Handle::from_rgba(1, 1, vec![9, 8, 7, 6]);
        editor.preview_image = Some(retained.clone());
        editor.preview_size = [1, 1];

        drop(editor.refresh_preview());
        assert_eq!(editor.preview_image.as_ref(), Some(&retained));
        assert_eq!(editor.preview_size, [1, 1]);
        assert_eq!(
            editor.preview_error.as_deref(),
            Some("synthetic preview failure")
        );

        drop(editor.refresh_preview());
        assert_eq!(editor.preview_image.as_ref(), Some(&retained));
        assert_eq!(editor.preview_size, [1, 1]);
        assert!(editor.preview_error.is_none());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn workspace_env_override_only_accepts_animate() {
        assert_eq!(Workspace::from_env_value(None), Workspace::Design);
        assert_eq!(Workspace::from_env_value(Some("")), Workspace::Design);
        assert_eq!(Workspace::from_env_value(Some("design")), Workspace::Design);
        assert_eq!(
            Workspace::from_env_value(Some("Animate")),
            Workspace::Animate
        );
        assert_eq!(
            Workspace::from_env_value(Some(" ANIMATE ")),
            Workspace::Animate
        );
    }

    #[test]
    fn animate_workspace_submits_the_working_assignment_and_highlights() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        assert!(path.is_file());
        let mut model = EditorModel::with_workspace(Some(path), Workspace::Animate);
        let stored = model
            .selected_scene()
            .map(|scene| scene.animation_sets.len())
            .unwrap_or(0);
        assert!(stored > 0, "fixture provides at least one stored ANMS");
        model.update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))));

        let (request_sender, request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::from([Ok(PreviewFrame {
                width: 1,
                height: 1,
                rgba: vec![0; 4],
                selection_bounds: None,
                highlight_bounds: Vec::new(),
            })]),
        };
        let mut editor = test_editor(model, renderer);
        drop(editor.refresh_preview());

        let request = request_receiver.recv().unwrap();
        let assignment = request
            .foreground
            .animation_assignment
            .expect("Animate submits the working assignment");
        let scene_layers = editor
            .model
            .selected_scene()
            .expect("fixture scene")
            .layers
            .len();
        assert_eq!(assignment.slots.len(), scene_layers);
        assert!(
            !editor
                .model
                .animate()
                .assignment_is_scratch(editor.model.selected_scene())
        );
    }

    #[test]
    fn playback_advances_only_after_the_previous_allocation_completes() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        let mut model = EditorModel::with_workspace(Some(path), Workspace::Animate);
        model.update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))));
        model.update(EditorAction::Animate(AnimateAction::TogglePlaying));
        assert!(model.animate().playing());
        let start = model.animate().frame();

        let (request_sender, _request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::from([Ok(PreviewFrame {
                width: 1,
                height: 1,
                rgba: vec![0; 4],
                selection_bounds: None,
                highlight_bounds: Vec::new(),
            })]),
        };
        let mut editor = test_editor(model, renderer);
        // No wall-clock subscription drives playback, so the frame only moves
        // when a completed allocation asks for the next one.
        assert_eq!(editor.model.animate().frame(), start);
        drop(editor.advance_playback());
        assert_eq!(editor.model.animate().frame(), start + 1);
    }

    #[test]
    fn stale_or_failed_allocations_never_advance_or_retry_playback() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        let mut model = EditorModel::with_workspace(Some(path), Workspace::Animate);
        model.update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))));
        model.update(EditorAction::Animate(AnimateAction::TogglePlaying));
        let start = model.animate().frame();

        let (request_sender, request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::new(),
        };
        let mut editor = test_editor(model, renderer);
        editor.preview_request_id = 2;

        // An older request may finish after a newer one was issued. Its
        // failure cannot clock the active transport or schedule a retry.
        drop(editor.update(Message::PreviewAllocated {
            request_id: 1,
            size: [1, 1],
            selection_bounds: None,
            highlight_bounds: Vec::new(),
            result: Err(image::Error::Unsupported),
        }));
        assert_eq!(editor.model.animate().frame(), start);
        assert!(editor.model.animate().playing());
        assert_eq!(editor.preview_request_id, 2);
        assert!(request_receiver.try_recv().is_err());

        // The active request failing has no completion to advance from, so it
        // stops playback instead of creating another failing request.
        drop(editor.update(Message::PreviewAllocated {
            request_id: 2,
            size: [1, 1],
            selection_bounds: None,
            highlight_bounds: Vec::new(),
            result: Err(image::Error::Unsupported),
        }));
        assert_eq!(editor.model.animate().frame(), start);
        assert!(!editor.model.animate().playing());
        assert_eq!(editor.preview_request_id, 2);
        assert!(request_receiver.try_recv().is_err());
    }

    #[test]
    fn play_does_not_start_without_a_document_to_render() {
        let (request_sender, request_receiver) = mpsc::channel();
        let renderer = RecordingRenderer {
            requests: request_sender,
            results: VecDeque::new(),
        };
        let model = EditorModel::with_workspace(None, Workspace::Animate);
        let mut editor = test_editor(model, renderer);

        drop(editor.update(Message::Transport(Transport::TogglePlaying)));
        assert!(!editor.model.animate().playing());
        assert_eq!(editor.preview_request_id, 0);
        assert!(request_receiver.try_recv().is_err());
    }
    #[test]
    fn animate_layout_places_assignment_below_inspector() {
        fn collect_leaves(
            state: &pane_grid::State<PaneKind>,
            node: &pane_grid::Node,
            leaves: &mut Vec<PaneKind>,
        ) {
            match node {
                pane_grid::Node::Split { a, b, .. } => {
                    collect_leaves(state, a, leaves);
                    collect_leaves(state, b, leaves);
                }
                pane_grid::Node::Pane(pane) => {
                    leaves.push(*state.get(*pane).expect("layout pane has state"));
                }
            }
        }

        let state = default_animate_pane_layout();
        let mut leaves = Vec::new();
        collect_leaves(&state, state.layout(), &mut leaves);
        assert_eq!(
            leaves,
            [
                PaneKind::Hierarchy,
                PaneKind::Composition,
                PaneKind::DopeSheet,
                PaneKind::Inspector,
                PaneKind::Assignment,
            ]
        );
        assert_eq!(
            pane_ratio_sequence::<4>(&state),
            Some(DEFAULT_ANIMATE_PANE_RATIOS)
        );
    }

    #[test]
    fn title_preserves_full_path_and_marks_read_only_and_dirty_states() {
        let path = temporary_srd_path("title-state");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let model = EditorModel::new(Some(path.clone()));
        let (request_sender, _request_receiver) = mpsc::channel();
        let mut editor = test_editor(
            model,
            RecordingRenderer {
                requests: request_sender,
                results: VecDeque::new(),
            },
        );

        assert_eq!(
            title(&editor),
            format!("{} [READONLY] — SRD Editor", path.display())
        );

        editor.model.update(EditorAction::ToggleDocumentEditing);
        assert!(
            editor
                .model
                .update(EditorAction::EditTransform(
                    crate::editor::model::TransformField::PositionX,
                    "12".into(),
                ))
                .preview
        );
        assert_eq!(title(&editor), format!("{} * — SRD Editor", path.display()));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn close_request_opens_confirmation_only_for_unsaved_changes() {
        let path = temporary_srd_path("close-guard");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let model = EditorModel::new(Some(path.clone()));
        let (request_sender, _request_receiver) = mpsc::channel();
        let mut editor = test_editor(
            model,
            RecordingRenderer {
                requests: request_sender,
                results: VecDeque::new(),
            },
        );
        editor.model.update(EditorAction::ToggleDocumentEditing);
        editor.model.update(EditorAction::EditTransform(
            crate::editor::model::TransformField::PositionX,
            "12".into(),
        ));

        drop(editor.update(Message::CloseRequested));
        assert!(editor.quit_confirmation_open);
        drop(editor.update(Message::CancelClose));
        assert!(!editor.quit_confirmation_open);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn action_notices_replace_and_expire_from_their_own_deadline() {
        let (request_sender, _request_receiver) = mpsc::channel();
        let mut editor = test_editor(
            EditorModel::default(),
            RecordingRenderer {
                requests: request_sender,
                results: VecDeque::new(),
            },
        );
        let start = Instant::now();

        editor.show_notice_at("first", start);
        editor.expire_notice_at(start + ACTION_NOTICE_DURATION - Duration::from_millis(1));
        assert_eq!(
            editor.notice.as_ref().map(|notice| notice.message.as_str()),
            Some("first")
        );

        let replacement_time = start + Duration::from_secs(3);
        editor.show_notice_at("replacement", replacement_time);
        editor.expire_notice_at(start + ACTION_NOTICE_DURATION);
        assert_eq!(
            editor.notice.as_ref().map(|notice| notice.message.as_str()),
            Some("replacement")
        );
        editor.expire_notice_at(replacement_time + ACTION_NOTICE_DURATION);
        assert!(editor.notice.is_none());
    }

    #[test]
    fn successful_edit_mode_toggles_never_surface_an_action_notice() {
        let path = temporary_srd_path("silent-edit-toggle");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let (request_sender, _request_receiver) = mpsc::channel();
        let mut editor = test_editor(
            EditorModel::new(Some(path.clone())),
            RecordingRenderer {
                requests: request_sender,
                results: VecDeque::new(),
            },
        );

        drop(editor.update(Message::Model(EditorAction::ToggleDocumentEditing)));
        assert!(editor.model.document_editing());
        assert!(editor.notice.is_none());
        drop(editor.update(Message::Model(EditorAction::ToggleDocumentEditing)));
        assert!(!editor.model.document_editing());
        assert!(editor.notice.is_none());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stale_hover_exits_never_clear_the_new_target() {
        let first = (0, 0);
        let second = (0, 1);
        let mut hovered = None;

        update_hover(&mut hovered, first, true);
        update_hover(&mut hovered, second, true);
        update_hover(&mut hovered, first, false);
        assert_eq!(hovered, Some(second));

        update_hover(&mut hovered, second, false);
        assert_eq!(hovered, None);
        update_hover(&mut hovered, second, true);
        assert_eq!(hovered, Some(second));
    }
}
