use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::animation::{KeyData, Track};
use crate::animation_persistence::projects_equal_for_persistence;
use crate::document::{EditorDocument, SaveReport, TransformComponent, display_srd_name};
use crate::image::SrdTextureBindingSource;
use crate::reference_runtime::{ProjectRuntime, ReferenceLayerParent};
use crate::renderer::PreviewCastSelection;
use crate::renderer::{FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput};
use crate::scene::{
    CastClassification, Layer, Project, RawTransform, ReferenceTarget, Scene, SrCastKind,
};
use crate::transform::SpatialTransform;

use super::animate::{AnimateAction, AnimateHistory, AnimateState, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanvasTransformEdit {
    Move([f32; 2]),
    RotateZ(i32),
    Scale([f32; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformField {
    PositionX,
    PositionY,
    PositionZ,
    RotationX,
    RotationY,
    RotationZ,
    ScaleX,
    ScaleY,
    ScaleZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastStructureSummary {
    pub parent_index: Option<usize>,
    pub child_count: usize,
    pub render_layer_offset: u8,
    pub parent_slice_cell: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorField {
    LayerName,
    CastName,
    MultiplyColor(usize),
    AdditiveColor(usize),
    PayloadWidth,
    PayloadHeight,
    OriginX,
    OriginY,
    OriginMode,
    VertexColor(usize, usize),
    CrefIndex,
    Cre1Index,
    CoordinateOffset(usize, usize),
    NumberInteger,
    NumberFraction,
    ReferenceSource,
    ReferenceLayer,
    ReferenceAnimation,
    ReferenceFrame,
    TextFont,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorToggle {
    LayerActive,
    CastActive,
    CastVisible,
    PayloadFlipU,
    PayloadFlipV,
    PayloadPointSampling,
    TextLayoutBypass,
    NumberForcePlus,
    NumberGrouping,
    NumberPadInteger,
    NumberFractionalDigits,
    NumberPadFraction,
    NumberDigitSpacingAfterDecimal,
    ReferenceAnimationEnabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadRenderSelector {
    Preset3,
    Preset4,
    Preset5,
    Preset9,
}

impl PayloadRenderSelector {
    pub const ALL: [Self; 4] = [Self::Preset3, Self::Preset4, Self::Preset5, Self::Preset9];

    const fn bits(self) -> u32 {
        match self {
            Self::Preset3 => 0,
            Self::Preset4 => 1,
            Self::Preset5 => 2,
            Self::Preset9 => 3,
        }
    }

    pub const fn from_flags(flags: u32) -> Option<Self> {
        match flags & 0x0F {
            0 => Some(Self::Preset3),
            1 => Some(Self::Preset4),
            2 => Some(Self::Preset5),
            3 => Some(Self::Preset9),
            _ => None,
        }
    }
}

impl std::fmt::Display for PayloadRenderSelector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Preset3 => "Preset 3 · standard",
            Self::Preset4 => "Preset 4",
            Self::Preset5 => "Preset 5",
            Self::Preset9 => "Preset 9",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UvVertexOrder {
    TopLeftFirst,
    TopRightFirst,
    BottomRightFirst,
    BottomLeftFirst,
}

impl UvVertexOrder {
    pub const ALL: [Self; 4] = [
        Self::TopLeftFirst,
        Self::TopRightFirst,
        Self::BottomRightFirst,
        Self::BottomLeftFirst,
    ];

    const fn bits(self) -> u32 {
        match self {
            Self::TopLeftFirst => 0,
            Self::TopRightFirst => 0x40,
            Self::BottomRightFirst => 0x80,
            Self::BottomLeftFirst => 0xC0,
        }
    }

    pub const fn from_flags(flags: u32) -> Self {
        match flags & 0xC0 {
            0x40 => Self::TopRightFirst,
            0x80 => Self::BottomRightFirst,
            0xC0 => Self::BottomLeftFirst,
            _ => Self::TopLeftFirst,
        }
    }
}

impl std::fmt::Display for UvVertexOrder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::TopLeftFirst => "TL · BL · TR · BR",
            Self::TopRightFirst => "TR · TL · BR · BL",
            Self::BottomRightFirst => "BR · TR · BL · TL",
            Self::BottomLeftFirst => "BL · BR · TL · TR",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadSpecialPreset {
    None,
    Preset20,
    Preset21,
}

impl PayloadSpecialPreset {
    pub const ALL: [Self; 3] = [Self::None, Self::Preset20, Self::Preset21];

    const fn bits(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Preset20 => 0x200,
            Self::Preset21 => 0x400,
        }
    }

    pub const fn from_flags(flags: u32) -> Option<Self> {
        match flags & 0x600 {
            0 => Some(Self::None),
            0x200 => Some(Self::Preset20),
            0x400 => Some(Self::Preset21),
            _ => None,
        }
    }
}

impl std::fmt::Display for PayloadSpecialPreset {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::None => "None",
            Self::Preset20 => "Preset 20",
            Self::Preset21 => "Preset 21",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

impl HorizontalAlignment {
    pub const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    const fn bits(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Center => 0x04,
            Self::Right => 0x08,
        }
    }

    pub const fn from_flags(flags: u32) -> Option<Self> {
        match flags & 0x0C {
            0 => Some(Self::Left),
            0x04 => Some(Self::Center),
            0x08 => Some(Self::Right),
            _ => None,
        }
    }
}

impl std::fmt::Display for HorizontalAlignment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Middle,
    Bottom,
}

impl VerticalAlignment {
    pub const ALL: [Self; 3] = [Self::Top, Self::Middle, Self::Bottom];

    const fn bits(self) -> u32 {
        match self {
            Self::Top => 0,
            Self::Middle => 0x10,
            Self::Bottom => 0x20,
        }
    }

    pub const fn from_flags(flags: u32) -> Option<Self> {
        match flags & 0x30 {
            0 => Some(Self::Top),
            0x10 => Some(Self::Middle),
            0x20 => Some(Self::Bottom),
            _ => None,
        }
    }
}

impl std::fmt::Display for VerticalAlignment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Top => "Top",
            Self::Middle => "Middle",
            Self::Bottom => "Bottom",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorChoice {
    PayloadRenderSelector(PayloadRenderSelector),
    UvVertexOrder(UvVertexOrder),
    PayloadSpecialPreset(PayloadSpecialPreset),
    TextHorizontalAlignment(HorizontalAlignment),
    TextVerticalAlignment(VerticalAlignment),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualGeometryDraft {
    pub size: [String; 2],
    pub origin: [String; 2],
    pub flags: u32,
    pub origin_mode: String,
    pub vertex_colors: [[String; 4]; 4],
}

impl Default for VisualGeometryDraft {
    fn default() -> Self {
        Self {
            size: std::array::from_fn(|_| "0".into()),
            origin: std::array::from_fn(|_| "0".into()),
            flags: 0,
            origin_mode: "0".into(),
            vertex_colors: std::array::from_fn(|_| std::array::from_fn(|_| "255".into())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageBindingDraft {
    pub cref_index: String,
    pub cre1_index: String,
    pub coordinate_offsets: [[String; 2]; 2],
}

impl Default for ImageBindingDraft {
    fn default() -> Self {
        Self {
            cref_index: "-1".into(),
            cre1_index: "-1".into(),
            coordinate_offsets: std::array::from_fn(|_| std::array::from_fn(|_| "0".into())),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageCastDraft {
    pub geometry: VisualGeometryDraft,
    pub binding: ImageBindingDraft,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextCastDraft {
    pub box_geometry: VisualGeometryDraft,
    pub source_binding: ImageBindingDraft,
    pub flags: Option<u32>,
    pub font: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SliceCastDraft {
    pub grid: VisualGeometryDraft,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberCastDraft {
    pub layout: VisualGeometryDraft,
    pub integer: String,
    pub fraction: String,
    pub format_flags: u32,
    pub alignment_flags: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReferenceCastDraft {
    pub source: String,
    pub layer: String,
    pub animation: String,
    pub enabled: bool,
    pub frame: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum CastRoleDraft {
    #[default]
    None,
    Null,
    Image(ImageCastDraft),
    Text(TextCastDraft),
    Slice(SliceCastDraft),
    Reference(ReferenceCastDraft),
    Number(NumberCastDraft),
}

impl CastRoleDraft {
    pub(super) fn geometry(&self) -> Option<&VisualGeometryDraft> {
        match self {
            Self::Image(value) => Some(&value.geometry),
            Self::Text(value) => Some(&value.box_geometry),
            Self::Slice(value) => Some(&value.grid),
            Self::Number(value) => Some(&value.layout),
            Self::None | Self::Null | Self::Reference(_) => None,
        }
    }

    fn geometry_mut(&mut self) -> Option<&mut VisualGeometryDraft> {
        match self {
            Self::Image(value) => Some(&mut value.geometry),
            Self::Text(value) => Some(&mut value.box_geometry),
            Self::Slice(value) => Some(&mut value.grid),
            Self::Number(value) => Some(&mut value.layout),
            Self::None | Self::Null | Self::Reference(_) => None,
        }
    }

    fn image_binding_mut(&mut self) -> Option<&mut ImageBindingDraft> {
        match self {
            Self::Image(value) => Some(&mut value.binding),
            Self::Text(value) => Some(&mut value.source_binding),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspectorDraft {
    pub layer_name: String,
    pub cast_name: String,
    pub layer_flags: u32,
    pub position: [String; 3],
    pub rotation: [String; 3],
    pub scale: [String; 3],
    pub multiply_color: [String; 4],
    pub additive_color: [String; 4],
    pub visibility: bool,
    pub role: CastRoleDraft,
}

impl Default for InspectorDraft {
    fn default() -> Self {
        Self {
            layer_name: String::new(),
            cast_name: String::new(),
            layer_flags: 0,
            position: std::array::from_fn(|_| "0".into()),
            rotation: std::array::from_fn(|_| "0".into()),
            scale: std::array::from_fn(|_| "1".into()),
            multiply_color: std::array::from_fn(|_| "255".into()),
            additive_color: std::array::from_fn(|_| "0".into()),
            visibility: true,
            role: CastRoleDraft::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum InspectorValue {
    Bytes(Vec<u8>),
    U8(u8),
    I16(i16),
    I32(i32),
    F32(f32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineTrack {
    pub label: String,
    pub keyframes: Vec<i32>,
    pub child: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InspectorSection {
    LayerMetadata,
    Transform,
    Compositing,
    Structure,
    ImageGeometry,
    ImageMaterial,
    TextContent,
    TextBox,
    TextPreviewOverrides,
    SliceGrid,
    SliceCells,
    SliceAtlas,
    ReferenceTarget,
    ReferenceAnimation,
    ReferenceInstance,
    NumberValue,
    NumberFormat,
    NumberLayout,
    NumberAtlas,
    Advanced,
    Diagnostics,
}

#[derive(Debug, Clone)]
pub enum EditorAction {
    SetAssetSearch(String),
    AddScene,
    DeleteScene,
    RenameScene(String),
    ToggleAssetDetails,
    ToggleSceneExpanded(usize),
    ToggleLayerExpanded(usize, usize),
    ToggleCastExpanded(usize, usize, usize),
    SelectScene(usize),
    SelectSceneLayer(usize, usize),
    SelectCast(usize, usize, usize),
    SelectNode(usize),
    SetTransformTool(TransformTool),
    EditTransform(TransformField, String),
    EditInspectorField(InspectorField, String),
    SetInspectorToggle(InspectorToggle, bool),
    SetInspectorChoice(InspectorChoice),
    CommitCanvasTransform(CanvasTransformEdit),
    EditTextContent(String),
    ToggleRuntimeTextInput,
    SetRuntimeTextSubstitution(usize, String),
    ToggleLayerVisible(usize),
    ToggleLayerLocked(usize),
    ToggleLayerSolo(usize),
    ToggleCastVisible(usize, usize, usize),
    ToggleCastLocked(usize, usize, usize),
    ToggleCastSolo(usize, usize, usize),
    ToggleGrid,
    ToggleDocumentEditing,
    ToggleInspectorSection(InspectorSection),
    ZoomBy(i16),
    SetZoom(u16),
    TogglePlaying,
    StepFrame(i32),
    SetFrame(i32),
    JumpToStart,
    JumpToEnd,
    Tick,
    Undo,
    Redo,
    ShowRuntimeHelp,
    /// Switches the same-window workspace. Routed through the Animate guard
    /// when explicit Edit mode holds unapplied changes.
    SetWorkspace(Workspace),
    Animate(AnimateAction),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelChange {
    pub preview: bool,
}

impl ModelChange {
    pub(super) const PREVIEW: Self = Self { preview: true };
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HistoryScope {
    None,
    Project,
    Full,
}

/// A bounded undo entry. SRD state and editor-only overrides travel together
/// so undo restores the model the user can actually observe, while dirty state
/// remains derived solely from the serializable project.
#[derive(Debug, Clone)]
struct HistoryState {
    project: Project,
    scene_origins: Vec<Option<usize>>,
    inspector: InspectorDraft,
    selected_scene: usize,
    selected_layer: usize,
    selected_node: Option<usize>,
    frame: i32,
    hidden_layers: Vec<BTreeSet<usize>>,
    locked_layers: Vec<BTreeSet<usize>>,
    solo_layers: Vec<BTreeSet<usize>>,
    runtime_text_inputs: BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
    hidden_casts: BTreeSet<PreviewCastSelection>,
    locked_casts: BTreeSet<PreviewCastSelection>,
    solo_casts: BTreeSet<PreviewCastSelection>,
    animate: AnimateHistory,
}

pub struct EditorModel {
    document: Option<EditorDocument>,
    load_error: Option<String>,
    document_revision: u64,
    asset_search: String,
    asset_details_open: bool,
    expanded_scenes: BTreeSet<usize>,
    expanded_layers: BTreeSet<(usize, usize)>,
    expanded_casts: BTreeSet<(usize, usize, usize)>,
    selected_scene: usize,
    selected_layer: usize,
    selected_node: Option<usize>,
    hidden_layers: Vec<BTreeSet<usize>>,
    locked_layers: Vec<BTreeSet<usize>>,
    solo_layers: Vec<BTreeSet<usize>>,
    runtime_text_inputs: BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
    hidden_casts: BTreeSet<PreviewCastSelection>,
    locked_casts: BTreeSet<PreviewCastSelection>,
    solo_casts: BTreeSet<PreviewCastSelection>,
    transform_tool: TransformTool,
    inspector: InspectorDraft,
    timeline_tracks: Vec<TimelineTrack>,
    frame: i32,
    playing: bool,
    zoom_percent: u16,
    show_grid: bool,
    collapsed_inspector_sections: BTreeSet<InspectorSection>,
    undo_history: VecDeque<HistoryState>,
    redo_history: VecDeque<HistoryState>,
    action_notice: Option<String>,
    pub(super) workspace: Workspace,
    pub(super) animate: AnimateState,
}

impl Default for EditorModel {
    fn default() -> Self {
        Self {
            document: None,
            load_error: None,
            document_revision: 0,
            asset_search: String::new(),
            asset_details_open: true,
            expanded_scenes: BTreeSet::new(),
            expanded_layers: BTreeSet::new(),
            expanded_casts: BTreeSet::new(),
            selected_scene: 0,
            selected_layer: 0,
            selected_node: None,
            hidden_layers: Vec::new(),
            locked_layers: Vec::new(),
            solo_layers: Vec::new(),
            runtime_text_inputs: BTreeMap::new(),
            hidden_casts: BTreeSet::new(),
            locked_casts: BTreeSet::new(),
            solo_casts: BTreeSet::new(),
            transform_tool: TransformTool::Select,
            inspector: InspectorDraft::default(),
            timeline_tracks: Vec::new(),
            frame: 0,
            playing: false,
            zoom_percent: 100,
            show_grid: true,
            collapsed_inspector_sections: [
                InspectorSection::Compositing,
                InspectorSection::Structure,
                InspectorSection::ImageMaterial,
                InspectorSection::TextPreviewOverrides,
                InspectorSection::SliceCells,
                InspectorSection::SliceAtlas,
                InspectorSection::ReferenceInstance,
                InspectorSection::NumberFormat,
                InspectorSection::NumberLayout,
                InspectorSection::NumberAtlas,
                InspectorSection::Advanced,
            ]
            .into_iter()
            .collect(),
            undo_history: VecDeque::new(),
            redo_history: VecDeque::new(),
            action_notice: None,
            workspace: Workspace::Design,
            animate: AnimateState::default(),
        }
    }
}

impl EditorModel {
    pub fn new(document_path: Option<PathBuf>) -> Self {
        Self::with_workspace(document_path, Workspace::Design)
    }

    /// Constructs a model with an explicit starting workspace. The shell uses
    /// this for `SRD_EDITOR_WORKSPACE`; tests use it to stay env-independent.
    pub fn with_workspace(document_path: Option<PathBuf>, workspace: Workspace) -> Self {
        let mut model = Self {
            workspace,
            ..Self::default()
        };
        if let Some(path) = document_path {
            model.load_document(path);
        } else {
            model.install_document(EditorDocument::empty());
            model.animate_enter_edit();
        }
        model.animate_sync();
        model
    }

    pub fn load_document(&mut self, path: impl AsRef<Path>) -> bool {
        if self.is_dirty() {
            self.load_error =
                Some("save or undo the current SRD edits before opening another document".into());
            return false;
        }
        match EditorDocument::load(path) {
            Ok(document) => {
                self.install_document(document);
                true
            }
            Err(error) => {
                self.load_error = Some(error);
                false
            }
        }
    }

    pub fn update(&mut self, action: EditorAction) -> ModelChange {
        let scope = self.history_scope(&action);
        let before = (scope != HistoryScope::None)
            .then(|| self.history_state())
            .flatten();
        let change = self.apply_action(action);
        if let Some(before) = before {
            let changed = match scope {
                HistoryScope::None => false,
                HistoryScope::Project => self.document.as_ref().is_some_and(|document| {
                    !projects_equal_for_persistence(&before.project, &document.project)
                }),
                HistoryScope::Full => !self.history_matches(&before),
            };
            if changed {
                push_bounded(&mut self.undo_history, before);
                self.redo_history.clear();
            }
        }
        change
    }

    /// Classifies every editor intent at the single update boundary. Keeping
    /// this match exhaustive makes a newly added action choose its undo
    /// contract instead of silently bypassing history.
    fn history_scope(&self, action: &EditorAction) -> HistoryScope {
        match action {
            EditorAction::AddScene
            | EditorAction::DeleteScene
            | EditorAction::RenameScene(_)
            | EditorAction::EditTransform(..)
            | EditorAction::EditInspectorField(..)
            | EditorAction::SetInspectorToggle(..)
            | EditorAction::SetInspectorChoice(..)
            | EditorAction::CommitCanvasTransform(..)
            | EditorAction::EditTextContent(..) => HistoryScope::Project,
            EditorAction::ToggleRuntimeTextInput
            | EditorAction::SetRuntimeTextSubstitution(..)
            | EditorAction::ToggleLayerVisible(..)
            | EditorAction::ToggleLayerLocked(..)
            | EditorAction::ToggleLayerSolo(..)
            | EditorAction::ToggleCastVisible(..)
            | EditorAction::ToggleCastLocked(..)
            | EditorAction::ToggleCastSolo(..) => HistoryScope::Full,
            EditorAction::Animate(action) => {
                if matches!(
                    action,
                    AnimateAction::SetName(_)
                        | AnimateAction::SetStartFrame(_)
                        | AnimateAction::SetDuration(_)
                        | AnimateAction::ToggleSlotEnabled(_)
                        | AnimateAction::AssignSlotAnimation(..)
                ) {
                    HistoryScope::Full
                } else if action.mutates_document() {
                    HistoryScope::Project
                } else {
                    HistoryScope::None
                }
            }
            EditorAction::SetAssetSearch(_)
            | EditorAction::ToggleAssetDetails
            | EditorAction::ToggleSceneExpanded(_)
            | EditorAction::ToggleLayerExpanded(..)
            | EditorAction::ToggleCastExpanded(..)
            | EditorAction::SelectScene(_)
            | EditorAction::SelectSceneLayer(..)
            | EditorAction::SelectCast(..)
            | EditorAction::SelectNode(_)
            | EditorAction::SetTransformTool(_)
            | EditorAction::ToggleGrid
            | EditorAction::ToggleDocumentEditing
            | EditorAction::ToggleInspectorSection(_)
            | EditorAction::ZoomBy(_)
            | EditorAction::SetZoom(_)
            | EditorAction::TogglePlaying
            | EditorAction::StepFrame(_)
            | EditorAction::SetFrame(_)
            | EditorAction::JumpToStart
            | EditorAction::JumpToEnd
            | EditorAction::Tick
            | EditorAction::Undo
            | EditorAction::Redo
            | EditorAction::ShowRuntimeHelp
            | EditorAction::SetWorkspace(_) => HistoryScope::None,
        }
    }

    fn history_matches(&self, state: &HistoryState) -> bool {
        self.document.as_ref().is_some_and(|document| {
            projects_equal_for_persistence(&document.project, &state.project)
                && document.scene_origins() == state.scene_origins
        }) && self.inspector == state.inspector
            && self.selected_scene == state.selected_scene
            && self.selected_layer == state.selected_layer
            && self.selected_node == state.selected_node
            && self.frame == state.frame
            && self.hidden_layers == state.hidden_layers
            && self.locked_layers == state.locked_layers
            && self.solo_layers == state.solo_layers
            && self.runtime_text_inputs == state.runtime_text_inputs
            && self.hidden_casts == state.hidden_casts
            && self.locked_casts == state.locked_casts
            && self.solo_casts == state.solo_casts
            && self.animate_history() == state.animate
    }

    fn apply_action(&mut self, action: EditorAction) -> ModelChange {
        if !self.document_editing()
            && matches!(
                &action,
                EditorAction::AddScene
                    | EditorAction::DeleteScene
                    | EditorAction::RenameScene(_)
                    | EditorAction::EditTransform(..)
                    | EditorAction::EditInspectorField(..)
                    | EditorAction::SetInspectorToggle(..)
                    | EditorAction::SetInspectorChoice(..)
                    | EditorAction::CommitCanvasTransform(..)
                    | EditorAction::EditTextContent(..)
                    | EditorAction::Undo
                    | EditorAction::Redo
            )
        {
            self.action_notice = Some("Enable Edit before changing the SRD document.".into());
            return ModelChange::default();
        }
        match action {
            EditorAction::SetAssetSearch(value) => self.asset_search = value,
            EditorAction::AddScene => {
                if self.add_scene() {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::DeleteScene => {
                if self.delete_scene() {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::RenameScene(value) => {
                if self.rename_scene(value) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleAssetDetails => self.asset_details_open = !self.asset_details_open,
            EditorAction::ToggleSceneExpanded(index) => {
                toggle_set(&mut self.expanded_scenes, index)
            }
            EditorAction::ToggleLayerExpanded(scene, layer) => {
                toggle_set(&mut self.expanded_layers, (scene, layer));
            }
            EditorAction::ToggleCastExpanded(scene, layer, node) => {
                toggle_set(&mut self.expanded_casts, (scene, layer, node));
            }
            EditorAction::SetWorkspace(workspace) => {
                if self.workspace == workspace {
                    return ModelChange::default();
                }
                self.animate.end_preview();
                self.workspace = workspace;
                self.animate_sync();
                return ModelChange::PREVIEW;
            }
            EditorAction::Animate(action) => return self.update_animate(action),
            EditorAction::SelectScene(index) => {
                return self.select_scene_for_animate(index);
            }
            EditorAction::SelectSceneLayer(scene, layer) => {
                let scene_changed = self.select_scene(scene);
                let layer_changed = self.select_layer(layer);
                if scene_changed || layer_changed {
                    self.animate_sync();
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SelectCast(scene, layer, node) => {
                let scene_changed = self.select_scene(scene);
                let layer_changed = self.select_layer(layer);
                let node_changed = self.select_node(node);
                if scene_changed || layer_changed || node_changed {
                    self.animate_sync();
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SelectNode(index) => {
                if self.select_node(index) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SetTransformTool(tool) => {
                self.transform_tool = tool;
                self.action_notice = Some(
                    match tool {
                        TransformTool::Select => "Select mode: choose casts in the hierarchy.",
                        TransformTool::Move => {
                            "Move mode: drag the selected cast in Composition or edit Position."
                        }
                        TransformTool::Rotate => {
                            "Rotate mode: drag around the selected cast or edit Rotation Z."
                        }
                        TransformTool::Scale => "Scale mode: drag the selected cast or edit Scale.",
                    }
                    .into(),
                );
            }
            EditorAction::EditTransform(field, value) => {
                if self.edit_transform(field, value) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::EditInspectorField(field, value) => {
                if self.edit_inspector_field(field, value) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SetInspectorToggle(toggle, enabled) => {
                if self.set_inspector_toggle(toggle, enabled) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SetInspectorChoice(choice) => {
                if self.set_inspector_choice(choice) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::CommitCanvasTransform(edit) => {
                if self.commit_canvas_transform(edit) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::EditTextContent(value) => {
                if self.edit_text_content(value) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleRuntimeTextInput => {
                if let Some(key) = self.selected_runtime_text_key() {
                    if self.runtime_text_inputs.remove(&key).is_none() {
                        self.runtime_text_inputs
                            .insert(key, FennelSrdRuntimeTextInput::default());
                    }
                    return ModelChange::PREVIEW;
                }
                self.action_notice = Some(
                    "Select a parsed SrTextCast before configuring host substitutions.".into(),
                );
            }
            EditorAction::SetRuntimeTextSubstitution(slot, value) => {
                if slot >= 8 {
                    self.action_notice = Some(
                        "Fennel exposes exactly eight host substitution slots: $[0] through $[7]."
                            .into(),
                    );
                } else if let Some(key) = self.selected_runtime_text_key() {
                    let bytes = value.into_bytes();
                    let changed = self
                        .runtime_text_inputs
                        .get(&key)
                        .is_some_and(|input| input.substitutions[slot] != bytes);
                    if changed {
                        self.runtime_text_inputs
                            .get_mut(&key)
                            .expect("checked runtime text input")
                            .substitutions[slot] = bytes;
                        return ModelChange::PREVIEW;
                    }
                    if !self.runtime_text_inputs.contains_key(&key) {
                        self.action_notice = Some(
                            "Enable manual runtime text input before editing substitutions.".into(),
                        );
                    }
                }
            }
            EditorAction::ToggleLayerVisible(layer) => {
                if self.hidden_layers.get(self.selected_scene).is_some() {
                    let hidden = self
                        .hidden_layers
                        .get_mut(self.selected_scene)
                        .expect("checked selected scene");
                    toggle_set(hidden, layer);
                    // Manual hide is an independent filter, but the assignment
                    // table shows it, so the rows must be rebuilt.
                    self.animate_sync();
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleLayerLocked(layer) => {
                if self.locked_layers.get(self.selected_scene).is_some() {
                    let locked = self
                        .locked_layers
                        .get_mut(self.selected_scene)
                        .expect("checked selected scene");
                    toggle_set(locked, layer);
                }
            }
            EditorAction::ToggleLayerSolo(layer) => {
                if self.solo_layers.get(self.selected_scene).is_some() {
                    let solo = self
                        .solo_layers
                        .get_mut(self.selected_scene)
                        .expect("checked selected scene");
                    toggle_set(solo, layer);
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleCastVisible(scene, layer, node) => {
                let selection = PreviewCastSelection {
                    scene_index: scene,
                    layer_index: layer,
                    node_index: node,
                };
                if self.valid_cast_selection(selection) {
                    toggle_set(&mut self.hidden_casts, selection);
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleCastLocked(scene, layer, node) => {
                let selection = PreviewCastSelection {
                    scene_index: scene,
                    layer_index: layer,
                    node_index: node,
                };
                if self.valid_cast_selection(selection) {
                    toggle_set(&mut self.locked_casts, selection);
                }
            }
            EditorAction::ToggleCastSolo(scene, layer, node) => {
                let selection = PreviewCastSelection {
                    scene_index: scene,
                    layer_index: layer,
                    node_index: node,
                };
                if self.valid_cast_selection(selection) {
                    toggle_set(&mut self.solo_casts, selection);
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ToggleGrid => self.show_grid = !self.show_grid,
            EditorAction::ToggleDocumentEditing => {
                if self.document.is_none() {
                    self.action_notice = Some("Open an SRD before enabling Edit.".into());
                } else if self.animate.editing {
                    self.animate_done();
                } else {
                    self.animate_enter_edit();
                }
            }
            EditorAction::ToggleInspectorSection(section) => {
                toggle_set(&mut self.collapsed_inspector_sections, section);
            }
            EditorAction::ZoomBy(delta) => {
                self.zoom_percent =
                    (i32::from(self.zoom_percent) + i32::from(delta)).clamp(25, 400) as u16;
            }
            EditorAction::SetZoom(value) => self.zoom_percent = value.clamp(25, 400),
            EditorAction::TogglePlaying => {
                self.playing = self.selected_scene().is_some() && !self.playing;
                if self.playing {
                    // Playback advances on completed preview allocations, so
                    // starting it must issue the first request.
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::StepFrame(delta) => {
                if self.set_frame(self.frame.saturating_add(delta)) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::SetFrame(frame) => {
                if self.set_frame(frame) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::JumpToStart => {
                if self.set_frame(self.frame_range().0) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::JumpToEnd => {
                if self.set_frame(self.frame_range().1) {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::Tick => {
                if self.playing {
                    let (start, end) = self.frame_range();
                    let next = if self.frame >= end {
                        start
                    } else {
                        self.frame + 1
                    };
                    if self.set_frame(next) {
                        return ModelChange::PREVIEW;
                    }
                }
            }
            EditorAction::Undo => {
                if self.undo() {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::Redo => {
                if self.redo() {
                    return ModelChange::PREVIEW;
                }
            }
            EditorAction::ShowRuntimeHelp => {
                self.action_notice = Some(
                    "Runtime path: ObjectManager GameObject wrapper → owned Impl → embedded projView::SrPlayer → player-owned runtime scenes/layers/casts; AIR SrResource retains the serialized project and DDS dependencies."
                        .into(),
                );
            }
        }
        ModelChange::default()
    }

    /// Scene selection is non-destructive; document edits remain in place.
    pub(super) fn select_scene_for_animate(&mut self, index: usize) -> ModelChange {
        if index == self.selected_scene {
            return ModelChange::default();
        }
        if self.select_scene(index) {
            self.animate_sync();
            return ModelChange::PREVIEW;
        }
        ModelChange::default()
    }

    pub(super) fn document_mut(&mut self) -> Option<&mut EditorDocument> {
        self.document.as_mut()
    }

    pub(super) fn set_action_notice(&mut self, notice: &str) {
        self.action_notice = Some(notice.to_owned());
    }

    pub fn document(&self) -> Option<&EditorDocument> {
        self.document.as_ref()
    }

    pub fn is_dirty(&self) -> bool {
        self.document.as_ref().is_some_and(EditorDocument::is_dirty)
    }

    pub fn document_editing(&self) -> bool {
        self.document.is_some() && self.animate.editing
    }

    pub fn inspector_section_collapsed(&self, section: InspectorSection) -> bool {
        self.collapsed_inspector_sections.contains(&section)
    }

    pub fn can_undo(&self) -> bool {
        self.document_editing() && !self.undo_history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        self.document_editing() && !self.redo_history.is_empty()
    }

    pub fn take_action_notice(&mut self) -> Option<String> {
        self.action_notice.take()
    }

    /// Saves without changing the document-level Edit toggle. A successful
    /// write becomes the new edit-session baseline.
    pub fn save(&mut self) -> Result<SaveReport, String> {
        let report = self
            .document
            .as_mut()
            .ok_or_else(|| "no SRD document is open".to_owned())
            .and_then(EditorDocument::save)?;
        self.bump_revision();
        Ok(report)
    }

    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<SaveReport, String> {
        let report = match self.document.as_mut() {
            Some(document) => document.save_as(path),
            None => Err("no SRD document is open".to_owned()),
        }?;
        self.bump_revision();
        Ok(report)
    }

    pub fn document_revision(&self) -> u64 {
        self.document_revision
    }

    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub fn asset_search(&self) -> &str {
        &self.asset_search
    }

    pub fn asset_details_open(&self) -> bool {
        self.asset_details_open
    }

    pub fn scene_expanded(&self, scene: usize) -> bool {
        self.expanded_scenes.contains(&scene)
    }

    pub fn layer_expanded(&self, scene: usize, layer: usize) -> bool {
        self.expanded_layers.contains(&(scene, layer))
    }
    pub fn cast_expanded(&self, scene: usize, layer: usize, node: usize) -> bool {
        self.expanded_casts.contains(&(scene, layer, node))
    }

    pub fn selected_scene_index(&self) -> usize {
        self.selected_scene
    }

    pub fn selected_animation_set_index(&self) -> Option<usize> {
        self.animate.source_set()
    }

    pub fn selected_layer_index(&self) -> usize {
        self.selected_layer
    }

    pub fn selected_node_index(&self) -> Option<usize> {
        self.selected_node
    }

    pub fn selected_scene(&self) -> Option<&crate::scene::Scene> {
        self.document
            .as_ref()?
            .project
            .scenes
            .get(self.selected_scene)
    }

    pub fn selected_layer(&self) -> Option<&Layer> {
        self.selected_scene()?.layers.get(self.selected_layer)
    }

    pub fn selected_node(&self) -> Option<&crate::scene::NodeRecord> {
        self.selected_layer()?.nodes.get(self.selected_node?)
    }

    pub fn selected_animation_set(&self) -> Option<&crate::scene::AnimationSetDefinition> {
        self.selected_scene()?
            .animation_sets
            .get(self.animate.source_set()?)
    }

    pub fn selected_scene_name(&self) -> String {
        self.selected_scene()
            .map(|scene| display_srd_name(&scene.name))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "No scene".into())
    }

    pub fn selected_layer_name(&self) -> String {
        self.selected_layer()
            .map(|layer| display_srd_name(&layer.name))
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "No layer".into())
    }

    pub fn selected_node_name(&self) -> String {
        let Some(index) = self.selected_node else {
            return "No cast".into();
        };
        self.selected_node()
            .and_then(|node| node.name.as_deref())
            .map(display_srd_name)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("Cast {index}"))
    }

    pub fn transform_tool(&self) -> TransformTool {
        self.transform_tool
    }

    pub fn inspector(&self) -> &InspectorDraft {
        &self.inspector
    }

    pub fn timeline_tracks(&self) -> &[TimelineTrack] {
        &self.timeline_tracks
    }

    pub fn selected_transform_is_2d(&self) -> bool {
        let Some(node) = self.selected_node else {
            return false;
        };
        matches!(
            self.selected_layer()
                .and_then(|layer| layer.transforms.get(node)),
            Some(RawTransform::Trs2(_))
        )
    }

    pub fn runtime_text_inputs(
        &self,
    ) -> &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput> {
        &self.runtime_text_inputs
    }

    pub fn selected_runtime_text_input(&self) -> Option<&FennelSrdRuntimeTextInput> {
        self.runtime_text_inputs
            .get(&self.selected_runtime_text_key()?)
    }

    pub fn selected_is_text_cast(&self) -> bool {
        self.selected_cast_kind() == Some(SrCastKind::Text)
    }

    pub fn selected_cast_classification(&self) -> Option<CastClassification> {
        let node = self.selected_node?;
        self.selected_layer()?.classify_cast(node)
    }

    pub fn selected_cast_kind(&self) -> Option<SrCastKind> {
        self.selected_cast_classification()
            .map(|classification| classification.kind)
    }

    pub fn selected_cast_structure(&self) -> Option<CastStructureSummary> {
        let node_index = self.selected_node?;
        let layer = self.selected_layer()?;
        let node = layer.nodes.get(node_index)?;
        let mut child_count = 0usize;
        let mut child = node.first_child_index;
        for _ in 0..layer.nodes.len() {
            if child < 0 {
                break;
            }
            let child_index = usize::try_from(child).ok()?;
            child_count += 1;
            child = layer.nodes.get(child_index)?.next_sibling_index;
        }
        Some(CastStructureSummary {
            parent_index: cast_parent_index(layer, node_index),
            child_count,
            render_layer_offset: node.render_layer_offset(),
            parent_slice_cell: node.parent_csli_cell_index,
        })
    }

    pub fn frame(&self) -> i32 {
        self.frame
    }

    pub fn frame_range(&self) -> (i32, i32) {
        self.selected_animation_set().map_or((0, 0), |set| {
            (
                set.start_frame,
                set.runtime_duration.max(set.start_frame + 1),
            )
        })
    }

    pub fn animation_active(&self) -> bool {
        self.animate.source_set().is_some()
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Stops both transports when the shell cannot produce another preview
    /// completion for the active request.
    pub(super) fn stop_playback(&mut self) {
        self.playing = false;
        self.animate.playing = false;
    }

    pub fn zoom_percent(&self) -> u16 {
        self.zoom_percent
    }

    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    pub fn hidden_layers(&self) -> &BTreeSet<usize> {
        self.hidden_layers
            .get(self.selected_scene)
            .unwrap_or(empty_usize_set())
    }

    pub fn layer_visible(&self, layer: usize) -> bool {
        !self.hidden_layers().contains(&layer)
    }

    pub fn solo_layers(&self) -> &BTreeSet<usize> {
        self.solo_layers
            .get(self.selected_scene)
            .unwrap_or(empty_usize_set())
    }

    pub fn layer_soloed(&self, layer: usize) -> bool {
        self.solo_layers().contains(&layer)
    }

    pub fn selected_image_texture_index(&self) -> Option<usize> {
        let node_index = self.selected_node?;
        if self.selected_cast_kind() != Some(SrCastKind::Image) {
            return None;
        }
        let image = self
            .selected_layer()?
            .image_by_node
            .get(node_index)?
            .as_ref()?;
        debug_assert!(!image.creates_text_cast());
        image
            .resolve_texture_slots(
                &image.initial_runtime_state(),
                &self.document.as_ref()?.textures,
                crate::image::ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                [false; 2],
            )
            .ok()?
            .slots
            .into_iter()
            .find_map(|slot| match slot {
                Some(SrdTextureBindingSource::TextureList(index)) => Some(index),
                _ => None,
            })
    }

    pub fn selected_text_content(&self) -> Option<&[u8]> {
        let node_index = self.selected_node?;
        if self.selected_cast_kind() != Some(SrCastKind::Text) {
            return None;
        }
        self.selected_layer()?
            .image_by_node
            .get(node_index)?
            .as_ref()?
            .text
            .as_ref()
            .map(|text| text.text.as_slice())
    }
    pub fn layer_locked(&self, layer: usize) -> bool {
        self.locked_layers
            .get(self.selected_scene)
            .is_some_and(|locked| locked.contains(&layer))
    }

    pub fn selected_layer_locked(&self) -> bool {
        self.layer_locked(self.selected_layer)
    }

    pub fn hidden_casts(&self) -> &BTreeSet<PreviewCastSelection> {
        &self.hidden_casts
    }

    pub fn solo_casts(&self) -> &BTreeSet<PreviewCastSelection> {
        &self.solo_casts
    }

    pub fn cast_visible(&self, scene: usize, layer: usize, node: usize) -> bool {
        !self.cast_or_ancestor_in(&self.hidden_casts, scene, layer, node)
    }

    pub fn cast_locked(&self, scene: usize, layer: usize, node: usize) -> bool {
        self.cast_or_ancestor_in(&self.locked_casts, scene, layer, node)
    }

    pub fn cast_soloed(&self, scene: usize, layer: usize, node: usize) -> bool {
        self.cast_visible(scene, layer, node)
            && self.cast_or_ancestor_in(&self.solo_casts, scene, layer, node)
    }

    pub fn selected_cast_locked(&self) -> bool {
        self.selected_node
            .is_some_and(|node| self.cast_locked(self.selected_scene, self.selected_layer, node))
    }

    pub fn project_file_name(&self) -> String {
        self.document
            .as_ref()
            .and_then(EditorDocument::path)
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled.srd".into())
    }

    pub fn project_path(&self) -> Option<&Path> {
        self.document.as_ref().and_then(EditorDocument::path)
    }

    pub fn can_add_scene(&self) -> bool {
        self.document_editing()
            && self
                .document
                .as_ref()
                .is_some_and(|document| document.project.scenes.len() < u32::MAX as usize)
    }

    pub fn selected_cast_type_name(&self) -> &'static str {
        self.selected_cast_kind()
            .map_or("No CAST", SrCastKind::display_name)
    }

    fn install_document(&mut self, document: EditorDocument) {
        let scene_count = document.project.scenes.len();
        let expanded_casts = document
            .project
            .scenes
            .iter()
            .enumerate()
            .flat_map(|(scene_index, scene)| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .flat_map(move |(layer_index, layer)| {
                        (0..layer.nodes.len())
                            .map(move |node_index| (scene_index, layer_index, node_index))
                    })
            })
            .collect();
        self.hidden_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.locked_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.solo_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.hidden_casts.clear();
        self.locked_casts.clear();
        self.solo_casts.clear();
        self.document = Some(document);
        self.load_error = None;
        self.undo_history.clear();
        self.redo_history.clear();
        self.runtime_text_inputs.clear();
        self.action_notice = None;
        self.inspector = InspectorDraft::default();
        self.document_revision = self.document_revision.wrapping_add(1);
        self.selected_scene = 0;
        self.selected_layer = 0;
        self.selected_node = self
            .selected_layer()
            .and_then(|layer| (!layer.nodes.is_empty()).then_some(0));
        self.expanded_scenes.clear();
        self.expanded_layers.clear();
        self.expanded_casts = expanded_casts;
        if self.selected_scene().is_some() {
            self.expanded_scenes.insert(0);
        }
        if self.selected_layer().is_some() {
            self.expanded_layers.insert((0, 0));
        }
        self.animate = AnimateState::default();
        self.frame = self.frame_range().0;
        self.sync_inspector();
        self.rebuild_timeline_tracks();
        self.animate_sync();
    }

    fn add_scene(&mut self) -> bool {
        if !self.can_add_scene() {
            self.action_notice = Some("Enable Edit before adding a scene.".into());
            return false;
        }
        let document = self.document.as_ref().expect("validated editable document");
        let mut ordinal = document.project.scenes.len().saturating_add(1);
        let name = loop {
            let candidate = format!("Scene {ordinal}");
            if !document
                .project
                .scenes
                .iter()
                .any(|scene| scene.name == candidate.as_bytes())
            {
                break candidate;
            }
            ordinal = ordinal.saturating_add(1);
        };
        let scene = Scene {
            name: name.as_bytes().to_vec(),
            declared_layer_count: 0,
            declared_animation_set_count: 0,
            width: 1920.0,
            height: 1080.0,
            layers: Vec::new(),
            animation_sets: Vec::new(),
        };
        let index = match self
            .document
            .as_mut()
            .expect("validated editable document")
            .push_scene(scene)
        {
            Ok(index) => index,
            Err(error) => {
                self.action_notice = Some(error);
                return false;
            }
        };
        self.reset_after_scene_structure(index);
        self.bump_revision();
        self.action_notice = Some(format!("Added {name}."));
        true
    }

    fn delete_scene(&mut self) -> bool {
        let index = self.selected_scene;
        let Some(name) = self.selected_scene().map(|scene| {
            let name = display_srd_name(&scene.name);
            if name.is_empty() {
                format!("Scene {}", index + 1)
            } else {
                name
            }
        }) else {
            return false;
        };
        let removed = self
            .document
            .as_mut()
            .and_then(|document| document.remove_scene(index));
        if removed.is_none() {
            return false;
        }
        self.reset_after_scene_structure(index);
        self.bump_revision();
        self.action_notice = Some(format!("Deleted {name}. Undo restores it."));
        true
    }

    fn rename_scene(&mut self, value: String) -> bool {
        let result = self.document.as_mut().map(|document| {
            document.rename_scene(self.selected_scene, value.trim().as_bytes().to_vec())
        });
        match result {
            Some(Ok(true)) => {
                self.bump_revision();
                true
            }
            Some(Ok(false)) | None => false,
            Some(Err(error)) => {
                self.action_notice = Some(error);
                false
            }
        }
    }

    fn reset_after_scene_structure(&mut self, selected_scene: usize) {
        let scene_count = self
            .document
            .as_ref()
            .map_or(0, |document| document.project.scenes.len());
        self.selected_scene = selected_scene.min(scene_count.saturating_sub(1));
        self.selected_layer = 0;
        self.selected_node = self
            .selected_layer()
            .and_then(|layer| (!layer.nodes.is_empty()).then_some(0));
        self.hidden_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.locked_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.solo_layers = (0..scene_count).map(|_| BTreeSet::new()).collect();
        self.hidden_casts.clear();
        self.locked_casts.clear();
        self.solo_casts.clear();
        self.runtime_text_inputs.clear();
        self.expanded_scenes.clear();
        self.expanded_layers.clear();
        self.expanded_casts = self
            .document
            .as_ref()
            .into_iter()
            .flat_map(|document| document.project.scenes.iter().enumerate())
            .flat_map(|(scene_index, scene)| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .flat_map(move |(layer_index, layer)| {
                        (0..layer.nodes.len())
                            .map(move |node_index| (scene_index, layer_index, node_index))
                    })
            })
            .collect();
        if self.selected_scene().is_some() {
            self.expanded_scenes.insert(self.selected_scene);
        }
        if self.selected_layer().is_some() {
            self.expanded_layers
                .insert((self.selected_scene, self.selected_layer));
        }
        let editing = self.animate.editing;
        self.animate = AnimateState::default();
        self.animate.editing = editing;
        self.frame = self.frame_range().0;
        self.playing = false;
        self.sync_inspector();
        self.rebuild_timeline_tracks();
        self.animate_sync();
    }

    pub(super) fn select_scene(&mut self, index: usize) -> bool {
        let Some(document) = &self.document else {
            return false;
        };
        if index >= document.project.scenes.len() || index == self.selected_scene {
            return false;
        }
        self.selected_scene = index;
        self.animate.source_set = None;
        self.selected_layer = 0;
        self.selected_node = self
            .selected_layer()
            .and_then(|layer| (!layer.nodes.is_empty()).then_some(0));
        self.expanded_scenes.insert(index);
        if self.selected_layer().is_some() {
            self.expanded_layers.insert((index, 0));
        }
        self.frame = self.frame_range().0;
        self.sync_inspector();
        self.rebuild_timeline_tracks();
        true
    }

    fn select_layer(&mut self, index: usize) -> bool {
        if self
            .selected_scene()
            .is_none_or(|scene| index >= scene.layers.len())
        {
            return false;
        }
        let changed = index != self.selected_layer || self.selected_node.is_some();
        if !changed {
            return false;
        }
        self.selected_layer = index;
        self.selected_node = None;
        self.expanded_layers.insert((self.selected_scene, index));
        self.sync_inspector();
        self.rebuild_timeline_tracks();
        true
    }

    fn select_node(&mut self, index: usize) -> bool {
        if self
            .selected_layer()
            .is_none_or(|layer| index >= layer.nodes.len())
            || self.selected_node == Some(index)
        {
            return false;
        }
        self.selected_node = Some(index);
        self.sync_inspector();
        self.rebuild_timeline_tracks();
        true
    }

    fn selected_runtime_text_key(&self) -> Option<FennelRuntimeTextCastKey> {
        let node_index = self.selected_node?;
        let creates_text_cast = self
            .selected_layer()?
            .image_by_node
            .get(node_index)?
            .as_ref()?
            .creates_text_cast();
        creates_text_cast.then_some(FennelRuntimeTextCastKey {
            owner: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: self.selected_scene,
                layer_index: self.selected_layer,
            }),
            node_index,
        })
    }
    fn edit_text_content(&mut self, value: String) -> bool {
        if self.selected_layer_locked() || self.selected_cast_locked() {
            self.action_notice = Some(
                "Unlock the selected layer and CAST hierarchy before editing its text.".into(),
            );
            return false;
        }
        if !self.document_editing() {
            self.action_notice = Some("Enable Edit before changing text content.".into());
            return false;
        }
        let bytes = value.into_bytes();
        if bytes.len() > 2047 {
            self.action_notice =
                Some("Text content cannot exceed the runtime 2047-byte limit.".into());
            return false;
        }
        let Some(node_index) = self.selected_node else {
            self.action_notice = Some("Select a TextCast before editing text content.".into());
            return false;
        };
        let current = self
            .selected_layer()
            .and_then(|layer| layer.image_by_node.get(node_index))
            .and_then(Option::as_ref)
            .and_then(|image| image.text.as_ref())
            .map(|text| text.text.as_slice());
        let Some(current) = current else {
            self.action_notice = Some("Selected cast has no serialized TEXT payload.".into());
            return false;
        };
        if current == bytes {
            return false;
        }

        let text = self
            .document
            .as_mut()
            .and_then(|document| document.project.scenes.get_mut(self.selected_scene))
            .and_then(|scene| scene.layers.get_mut(self.selected_layer))
            .and_then(|layer| layer.image_by_node.get_mut(node_index))
            .and_then(Option::as_mut)
            .and_then(|image| image.text.as_mut())
            .expect("validated selected TEXT payload");
        text.text = bytes;
        self.bump_revision();
        true
    }

    fn edit_inspector_field(&mut self, field: InspectorField, value: String) -> bool {
        if !self.inspector_mutation_allowed() {
            return false;
        }

        let Some(draft) = self.inspector_draft_mut(field) else {
            self.action_notice =
                Some("This Inspector field does not belong to the selected CAST kind.".into());
            return false;
        };
        *draft = value.clone();
        let Some(parsed) = parse_inspector_value(field, &value) else {
            return false;
        };
        if self.workspace == Workspace::Animate
            && let Some((target, value)) = self.animated_inspector_value(field, &parsed)
        {
            return self.animate_edit_inspector_value(target, value);
        }
        let Some(current) = self.inspector_value(field) else {
            self.action_notice = Some(
                "The selected CAST has no serialized property for this Inspector field.".into(),
            );
            return false;
        };
        if current == parsed {
            return false;
        }

        if self.set_inspector_value(field, parsed).is_none() {
            self.action_notice =
                Some("The selected CAST cannot store this Inspector value.".into());
            return false;
        }
        self.bump_revision();
        true
    }

    fn inspector_mutation_allowed(&mut self) -> bool {
        if self.selected_layer_locked() || self.selected_cast_locked() {
            self.action_notice = Some(
                "Unlock the selected layer and CAST hierarchy before editing Inspector fields."
                    .into(),
            );
            return false;
        }
        if !self.document_editing() {
            self.action_notice = Some("Enable Edit before changing Inspector values.".into());
            return false;
        }
        true
    }

    fn set_inspector_toggle(&mut self, toggle: InspectorToggle, enabled: bool) -> bool {
        if !self.inspector_mutation_allowed() {
            return false;
        }

        if toggle == InspectorToggle::CastVisible && self.workspace == Workspace::Animate {
            if self.inspector.visibility == enabled {
                return false;
            }
            let changed = self.animate_edit_inspector_value(10, u32::from(enabled).to_string());
            if changed {
                self.sync_inspector();
            }
            return changed;
        }

        let scene_index = self.selected_scene;
        let layer_index = self.selected_layer;
        let node = self.selected_node;
        let changed = (|| -> Option<bool> {
            let layer = self
                .document
                .as_mut()?
                .project
                .scenes
                .get_mut(scene_index)?
                .layers
                .get_mut(layer_index)?;
            if toggle == InspectorToggle::LayerActive {
                return Some(set_masked_flag(&mut layer.flags, 0x100, enabled));
            }

            let node = node?;
            let kind = layer.classify_cast(node)?.kind;
            match toggle {
                InspectorToggle::LayerActive => unreachable!(),
                InspectorToggle::CastActive => {
                    let flags = layer.nodes.get_mut(node)?.type_flags.as_mut()?;
                    Some(set_masked_flag(flags, 0x100, enabled))
                }
                InspectorToggle::CastVisible => {
                    let visibility = &mut payload_transform_mut(layer, node)?.visibility_word;
                    let current = *visibility != 0;
                    if current == enabled {
                        Some(false)
                    } else {
                        *visibility = u32::from(enabled);
                        Some(true)
                    }
                }
                InspectorToggle::PayloadFlipU => Some(set_masked_flag(
                    payload_flags_mut(layer, node, kind)?,
                    0x10,
                    enabled,
                )),
                InspectorToggle::PayloadFlipV => Some(set_masked_flag(
                    payload_flags_mut(layer, node, kind)?,
                    0x20,
                    enabled,
                )),
                InspectorToggle::PayloadPointSampling => Some(set_masked_flag(
                    payload_flags_mut(layer, node, kind)?,
                    0x0100_0000,
                    enabled,
                )),
                InspectorToggle::TextLayoutBypass => {
                    (kind == SrCastKind::Text).then_some(())?;
                    let flags = layer
                        .image_by_node
                        .get_mut(node)?
                        .as_mut()?
                        .text
                        .as_mut()?
                        .field_78
                        .as_mut()?;
                    Some(set_masked_flag(flags, 0x01, enabled))
                }
                InspectorToggle::NumberForcePlus
                | InspectorToggle::NumberGrouping
                | InspectorToggle::NumberPadInteger
                | InspectorToggle::NumberFractionalDigits
                | InspectorToggle::NumberPadFraction
                | InspectorToggle::NumberDigitSpacingAfterDecimal => {
                    (kind == SrCastKind::Number).then_some(())?;
                    let flags = &mut layer.number_by_node.get_mut(node)?.as_mut()?.format_flags;
                    let mask = match toggle {
                        InspectorToggle::NumberForcePlus => 0x01,
                        InspectorToggle::NumberGrouping => 0x02,
                        InspectorToggle::NumberPadInteger => 0x04,
                        InspectorToggle::NumberFractionalDigits => 0x08,
                        InspectorToggle::NumberPadFraction => 0x10,
                        InspectorToggle::NumberDigitSpacingAfterDecimal => 0x20,
                        _ => unreachable!(),
                    };
                    Some(set_masked_flag(flags, mask, enabled))
                }
                InspectorToggle::ReferenceAnimationEnabled => {
                    (kind == SrCastKind::Reference).then_some(())?;
                    let value = &mut layer
                        .reference_by_node
                        .get_mut(node)?
                        .as_mut()?
                        .animation_enabled;
                    let current = *value != 0;
                    if current == enabled {
                        Some(false)
                    } else {
                        *value = i32::from(enabled) as u32;
                        Some(true)
                    }
                }
            }
        })();

        let Some(changed) = changed else {
            self.action_notice =
                Some("This semantic control is unavailable for the selected CAST.".into());
            return false;
        };
        if !changed {
            return false;
        }
        self.bump_revision();
        self.sync_inspector();
        true
    }

    fn set_inspector_choice(&mut self, choice: InspectorChoice) -> bool {
        if !self.inspector_mutation_allowed() {
            return false;
        }

        let scene_index = self.selected_scene;
        let layer_index = self.selected_layer;
        let node = self.selected_node;
        let changed = (|| -> Option<bool> {
            let layer = self
                .document
                .as_mut()?
                .project
                .scenes
                .get_mut(scene_index)?
                .layers
                .get_mut(layer_index)?;
            let node = node?;
            let kind = layer.classify_cast(node)?.kind;
            match choice {
                InspectorChoice::PayloadRenderSelector(value) => Some(set_masked_value(
                    payload_flags_mut(layer, node, kind)?,
                    0x0F,
                    value.bits(),
                )),
                InspectorChoice::UvVertexOrder(value) => Some(set_masked_value(
                    payload_flags_mut(layer, node, kind)?,
                    0xC0,
                    value.bits(),
                )),
                InspectorChoice::PayloadSpecialPreset(value) => Some(set_masked_value(
                    payload_flags_mut(layer, node, kind)?,
                    0x600,
                    value.bits(),
                )),
                InspectorChoice::TextHorizontalAlignment(value) => {
                    (kind == SrCastKind::Text).then_some(())?;
                    let flags = layer
                        .image_by_node
                        .get_mut(node)?
                        .as_mut()?
                        .text
                        .as_mut()?
                        .field_78
                        .as_mut()?;
                    Some(set_masked_value(flags, 0x0C, value.bits()))
                }
                InspectorChoice::TextVerticalAlignment(value) => {
                    (kind == SrCastKind::Text).then_some(())?;
                    let flags = layer
                        .image_by_node
                        .get_mut(node)?
                        .as_mut()?
                        .text
                        .as_mut()?
                        .field_78
                        .as_mut()?;
                    Some(set_masked_value(flags, 0x30, value.bits()))
                }
            }
        })();

        let Some(changed) = changed else {
            self.action_notice =
                Some("This semantic control is unavailable for the selected CAST.".into());
            return false;
        };
        if !changed {
            return false;
        }
        self.bump_revision();
        self.sync_inspector();
        true
    }

    fn animated_inspector_value(
        &self,
        field: InspectorField,
        value: &InspectorValue,
    ) -> Option<(u16, String)> {
        match (field, value) {
            (InspectorField::MultiplyColor(component @ 0..=2), InspectorValue::U8(_)) => {
                let mut color = [0u8; 3];
                for (destination, value) in color.iter_mut().zip(&self.inspector.multiply_color) {
                    *destination = value.parse().ok()?;
                }
                let _ = component;
                Some((
                    9,
                    format!("#{:02X}{:02X}{:02X}00", color[2], color[1], color[0]),
                ))
            }
            (InspectorField::MultiplyColor(3), InspectorValue::U8(value)) => {
                Some((21, (f32::from(*value) / 255.0).to_string()))
            }
            (InspectorField::AdditiveColor(component @ 0..=2), InspectorValue::U8(_)) => {
                let mut color = [0u8; 3];
                for (destination, value) in color.iter_mut().zip(&self.inspector.additive_color) {
                    *destination = value.parse().ok()?;
                }
                let _ = component;
                Some((
                    19,
                    format!("#{:02X}{:02X}{:02X}00", color[2], color[1], color[0]),
                ))
            }
            (InspectorField::AdditiveColor(3), InspectorValue::U8(value)) => {
                Some((22, (f32::from(*value) / 255.0).to_string()))
            }
            (InspectorField::PayloadWidth, InspectorValue::F32(value)) => {
                Some((11, value.to_string()))
            }
            (InspectorField::PayloadHeight, InspectorValue::F32(value)) => {
                Some((12, value.to_string()))
            }
            (InspectorField::VertexColor(vertex, _), InspectorValue::U8(_)) => {
                let target = [13, 15, 14, 16].get(vertex).copied()?;
                let geometry = self.inspector.role.geometry()?;
                let mut color = [0u8; 4];
                for (destination, value) in
                    color.iter_mut().zip(geometry.vertex_colors.get(vertex)?)
                {
                    *destination = value.parse().ok()?;
                }
                Some((
                    target,
                    format!(
                        "#{:02X}{:02X}{:02X}{:02X}",
                        color[0], color[1], color[2], color[3]
                    ),
                ))
            }
            (InspectorField::CrefIndex, InspectorValue::I16(value)) => {
                Some((17, value.to_string()))
            }
            (InspectorField::Cre1Index, InspectorValue::I16(value)) => {
                Some((20, value.to_string()))
            }
            (InspectorField::ReferenceFrame, InspectorValue::F32(value)) => {
                Some((23, value.to_string()))
            }
            _ => None,
        }
    }

    fn inspector_draft_mut(&mut self, field: InspectorField) -> Option<&mut String> {
        match field {
            InspectorField::LayerName => Some(&mut self.inspector.layer_name),
            InspectorField::CastName => Some(&mut self.inspector.cast_name),
            InspectorField::MultiplyColor(component) => {
                self.inspector.multiply_color.get_mut(component)
            }
            InspectorField::AdditiveColor(component) => {
                self.inspector.additive_color.get_mut(component)
            }
            InspectorField::PayloadWidth => self.inspector.role.geometry_mut()?.size.get_mut(0),
            InspectorField::PayloadHeight => self.inspector.role.geometry_mut()?.size.get_mut(1),
            InspectorField::OriginX => self.inspector.role.geometry_mut()?.origin.get_mut(0),
            InspectorField::OriginY => self.inspector.role.geometry_mut()?.origin.get_mut(1),
            InspectorField::OriginMode => {
                Some(&mut self.inspector.role.geometry_mut()?.origin_mode)
            }
            InspectorField::VertexColor(vertex, component) => self
                .inspector
                .role
                .geometry_mut()?
                .vertex_colors
                .get_mut(vertex)?
                .get_mut(component),
            InspectorField::CrefIndex => {
                Some(&mut self.inspector.role.image_binding_mut()?.cref_index)
            }
            InspectorField::Cre1Index => {
                Some(&mut self.inspector.role.image_binding_mut()?.cre1_index)
            }
            InspectorField::CoordinateOffset(channel, component) => self
                .inspector
                .role
                .image_binding_mut()?
                .coordinate_offsets
                .get_mut(channel)?
                .get_mut(component),
            InspectorField::NumberInteger => match &mut self.inspector.role {
                CastRoleDraft::Number(value) => Some(&mut value.integer),
                _ => None,
            },
            InspectorField::NumberFraction => match &mut self.inspector.role {
                CastRoleDraft::Number(value) => Some(&mut value.fraction),
                _ => None,
            },
            InspectorField::ReferenceSource => match &mut self.inspector.role {
                CastRoleDraft::Reference(value) => Some(&mut value.source),
                _ => None,
            },
            InspectorField::ReferenceLayer => match &mut self.inspector.role {
                CastRoleDraft::Reference(value) => Some(&mut value.layer),
                _ => None,
            },
            InspectorField::ReferenceAnimation => match &mut self.inspector.role {
                CastRoleDraft::Reference(value) => Some(&mut value.animation),
                _ => None,
            },
            InspectorField::ReferenceFrame => match &mut self.inspector.role {
                CastRoleDraft::Reference(value) => Some(&mut value.frame),
                _ => None,
            },
            InspectorField::TextFont => match &mut self.inspector.role {
                CastRoleDraft::Text(value) => Some(&mut value.font),
                _ => None,
            },
        }
    }

    fn inspector_value(&self, field: InspectorField) -> Option<InspectorValue> {
        let layer = self.selected_layer()?;
        if field == InspectorField::LayerName {
            return Some(InspectorValue::Bytes(layer.name.clone()));
        }
        let node = self.selected_node?;
        let kind = layer.classify_cast(node)?.kind;
        Some(match field {
            InspectorField::LayerName => unreachable!(),
            InspectorField::CastName => {
                InspectorValue::Bytes(layer.nodes.get(node)?.name.as_ref()?.clone())
            }
            InspectorField::MultiplyColor(component) => {
                InspectorValue::U8(layer.transforms.get(node)?.spatial().multiply_color[component])
            }
            InspectorField::AdditiveColor(component) => {
                InspectorValue::U8(layer.transforms.get(node)?.spatial().additive_color[component])
            }
            InspectorField::PayloadWidth => {
                InspectorValue::F32(payload_size(layer, node, kind)?[0])
            }
            InspectorField::PayloadHeight => {
                InspectorValue::F32(payload_size(layer, node, kind)?[1])
            }
            InspectorField::OriginX => InspectorValue::F32(payload_origin(layer, node, kind)?[0]),
            InspectorField::OriginY => InspectorValue::F32(payload_origin(layer, node, kind)?[1]),
            InspectorField::OriginMode => {
                InspectorValue::U8(payload_origin_mode(layer, node, kind)?)
            }
            InspectorField::VertexColor(vertex, component) => {
                InspectorValue::U8(payload_vertex_colors(layer, node, kind)?[vertex][component])
            }
            InspectorField::CrefIndex => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                InspectorValue::I16(layer.image_by_node.get(node)?.as_ref()?.cref_index)
            }
            InspectorField::Cre1Index => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                InspectorValue::I16(layer.image_by_node.get(node)?.as_ref()?.cre1_index)
            }
            InspectorField::CoordinateOffset(channel, component) => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                InspectorValue::F32(
                    layer.image_by_node.get(node)?.as_ref()?.coordinate_offsets[channel][component],
                )
            }
            InspectorField::NumberInteger => {
                (kind == SrCastKind::Number).then_some(())?;
                InspectorValue::I32(layer.number_by_node.get(node)?.as_ref()?.initial_integer)
            }
            InspectorField::NumberFraction => {
                (kind == SrCastKind::Number).then_some(())?;
                InspectorValue::F32(layer.number_by_node.get(node)?.as_ref()?.initial_fraction)
            }
            InspectorField::ReferenceSource
            | InspectorField::ReferenceLayer
            | InspectorField::ReferenceAnimation
            | InspectorField::ReferenceFrame => {
                (kind == SrCastKind::Reference).then_some(())?;
                let reference = layer.reference_by_node.get(node)?.as_ref()?;
                match field {
                    InspectorField::ReferenceSource => {
                        InspectorValue::Bytes(reference.source_name.clone())
                    }
                    InspectorField::ReferenceLayer => {
                        InspectorValue::Bytes(reference.layer_name.clone())
                    }
                    InspectorField::ReferenceAnimation => {
                        InspectorValue::Bytes(reference.animation_name.clone())
                    }
                    InspectorField::ReferenceFrame => InspectorValue::F32(reference.default_frame),
                    _ => unreachable!(),
                }
            }
            InspectorField::TextFont => {
                (kind == SrCastKind::Text).then_some(())?;
                let text = layer.image_by_node.get(node)?.as_ref()?.text.as_ref()?;
                InspectorValue::I32(text.font_index?)
            }
        })
    }

    fn set_inspector_value(&mut self, field: InspectorField, value: InspectorValue) -> Option<()> {
        let scene = self.selected_scene;
        let layer_index = self.selected_layer;
        let node = self.selected_node;
        let layer = self
            .document
            .as_mut()?
            .project
            .scenes
            .get_mut(scene)?
            .layers
            .get_mut(layer_index)?;
        if field == InspectorField::LayerName {
            let InspectorValue::Bytes(value) = value else {
                return None;
            };
            layer.name = value;
            return Some(());
        }
        let node = node?;
        let kind = layer.classify_cast(node)?.kind;
        match (field, value) {
            (InspectorField::CastName, InspectorValue::Bytes(value)) => {
                let name = layer.nodes.get_mut(node)?.name.as_mut()?;
                *name = value;
            }
            (InspectorField::MultiplyColor(component), InspectorValue::U8(value)) => {
                payload_transform_mut(layer, node)?.multiply_color[component] = value;
            }
            (InspectorField::AdditiveColor(component), InspectorValue::U8(value)) => {
                payload_transform_mut(layer, node)?.additive_color[component] = value;
            }
            (InspectorField::PayloadWidth, InspectorValue::F32(value)) => {
                *payload_size_mut(layer, node, kind, 0)? = value;
            }
            (InspectorField::PayloadHeight, InspectorValue::F32(value)) => {
                *payload_size_mut(layer, node, kind, 1)? = value;
            }
            (InspectorField::OriginX, InspectorValue::F32(value)) => {
                *payload_origin_mut(layer, node, kind, 0)? = value;
            }
            (InspectorField::OriginY, InspectorValue::F32(value)) => {
                *payload_origin_mut(layer, node, kind, 1)? = value;
            }
            (InspectorField::OriginMode, InspectorValue::U8(value)) => {
                *payload_origin_mode_mut(layer, node, kind)? = value;
            }
            (InspectorField::VertexColor(vertex, component), InspectorValue::U8(value)) => {
                *payload_vertex_color_mut(layer, node, kind, vertex, component)? = value;
            }
            (InspectorField::CrefIndex, InspectorValue::I16(value)) => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                layer.image_by_node.get_mut(node)?.as_mut()?.cref_index = value;
            }
            (InspectorField::Cre1Index, InspectorValue::I16(value)) => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                layer.image_by_node.get_mut(node)?.as_mut()?.cre1_index = value;
            }
            (InspectorField::CoordinateOffset(channel, component), InspectorValue::F32(value)) => {
                matches!(kind, SrCastKind::Image | SrCastKind::Text).then_some(())?;
                layer
                    .image_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .coordinate_offsets[channel][component] = value;
            }
            (InspectorField::NumberInteger, InspectorValue::I32(value)) => {
                (kind == SrCastKind::Number).then_some(())?;
                layer
                    .number_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .initial_integer = value;
            }
            (InspectorField::NumberFraction, InspectorValue::F32(value)) => {
                (kind == SrCastKind::Number).then_some(())?;
                layer
                    .number_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .initial_fraction = value;
            }
            (InspectorField::ReferenceSource, InspectorValue::Bytes(value)) => {
                (kind == SrCastKind::Reference).then_some(())?;
                layer.reference_by_node.get_mut(node)?.as_mut()?.source_name = value;
            }
            (InspectorField::ReferenceLayer, InspectorValue::Bytes(value)) => {
                (kind == SrCastKind::Reference).then_some(())?;
                layer.reference_by_node.get_mut(node)?.as_mut()?.layer_name = value;
            }
            (InspectorField::ReferenceAnimation, InspectorValue::Bytes(value)) => {
                (kind == SrCastKind::Reference).then_some(())?;
                layer
                    .reference_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .animation_name = value;
            }
            (InspectorField::ReferenceFrame, InspectorValue::F32(value)) => {
                (kind == SrCastKind::Reference).then_some(())?;
                layer
                    .reference_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .default_frame = value;
            }
            (InspectorField::TextFont, InspectorValue::I32(value)) => {
                (kind == SrCastKind::Text).then_some(())?;
                *layer
                    .image_by_node
                    .get_mut(node)?
                    .as_mut()?
                    .text
                    .as_mut()?
                    .font_index
                    .as_mut()? = value;
            }
            _ => return None,
        }
        Some(())
    }

    fn edit_transform(&mut self, field: TransformField, value: String) -> bool {
        if !self.document_editing() {
            self.action_notice = Some("Enable Edit before changing Inspector values.".into());
            return false;
        }
        if self.selected_layer_locked() || self.selected_cast_locked() {
            self.action_notice = Some(
                "Unlock the selected layer and CAST hierarchy before editing its transform.".into(),
            );
            return false;
        }

        let draft = match field {
            TransformField::PositionX => &mut self.inspector.position[0],
            TransformField::PositionY => &mut self.inspector.position[1],
            TransformField::PositionZ => &mut self.inspector.position[2],
            TransformField::RotationX => &mut self.inspector.rotation[0],
            TransformField::RotationY => &mut self.inspector.rotation[1],
            TransformField::RotationZ => &mut self.inspector.rotation[2],
            TransformField::ScaleX => &mut self.inspector.scale[0],
            TransformField::ScaleY => &mut self.inspector.scale[1],
            TransformField::ScaleZ => &mut self.inspector.scale[2],
        };
        *draft = value.clone();
        let component = transform_component(field);
        let Ok(parsed) = value.parse::<f32>() else {
            return false;
        };
        if !parsed.is_finite() {
            self.action_notice = Some("Transform values must be finite to be saved to SRD.".into());
            return false;
        }
        if self.workspace == Workspace::Animate {
            let target = transform_animation_target(field);
            let value = if matches!(component, TransformComponent::Rotation(_)) {
                parsed.round().to_string()
            } else {
                parsed.to_string()
            };
            return self.animate_edit_inspector_value(target, value);
        }
        let Some(node) = self.selected_node else {
            self.action_notice = Some("Select a cast before editing its transform.".into());
            return false;
        };
        if let Err(error) = self
            .document
            .as_ref()
            .ok_or_else(|| "no SRD document is open".to_owned())
            .and_then(|document| {
                document.validate_transform_value(
                    self.selected_scene,
                    self.selected_layer,
                    node,
                    component,
                    parsed,
                )
            })
        {
            self.action_notice = Some(format!("Transform edit was not applied: {error}"));
            return false;
        }

        let Some(before) = self.selected_transform() else {
            self.action_notice = Some("Selected cast has no parsed transform.".into());
            return false;
        };
        let rotation = match component {
            TransformComponent::Rotation(_) => {
                let rounded = parsed.round();
                if !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
                    self.action_notice = Some("Rotation is outside the SRD i32 range.".into());
                    return false;
                }
                Some(rounded as i32)
            }
            TransformComponent::Translation(_) | TransformComponent::Scale(_) => None,
        };
        let changed = match component {
            TransformComponent::Translation(index) => before.translation[index] != parsed,
            TransformComponent::Rotation(index) => before.rotation[index] != rotation.unwrap(),
            TransformComponent::Scale(index) => before.scale[index] != parsed,
        };
        if !changed {
            return false;
        }
        if let TransformComponent::Rotation(index) = component {
            self.inspector.rotation[index] = rotation
                .expect("rotation component has a value")
                .to_string();
        }

        let transform = self
            .selected_transform_mut()
            .expect("validated selected transform remains available");
        match component {
            TransformComponent::Translation(index) => transform.translation[index] = parsed,
            TransformComponent::Rotation(index) => transform.rotation[index] = rotation.unwrap(),
            TransformComponent::Scale(index) => transform.scale[index] = parsed,
        }
        self.bump_revision();
        true
    }

    fn commit_canvas_transform(&mut self, edit: CanvasTransformEdit) -> bool {
        if self.animation_active() {
            self.action_notice = Some(
                "Select Base pose before dragging serialized transforms; the active animation preset owns the evaluated values."
                    .into(),
            );
            return false;
        }
        if self.selected_layer_locked() || self.selected_cast_locked() {
            self.action_notice = Some(
                "Unlock the selected layer and CAST hierarchy before dragging its transform."
                    .into(),
            );
            return false;
        }
        let Some(before) = self.selected_transform() else {
            self.action_notice =
                Some("Select a cast with a parsed transform before dragging.".into());
            return false;
        };
        let edits = match edit {
            CanvasTransformEdit::Move([x, y]) => [
                Some((TransformComponent::Translation(0), x)),
                Some((TransformComponent::Translation(1), y)),
            ],
            CanvasTransformEdit::RotateZ(value) => {
                [Some((TransformComponent::Rotation(2), value as f32)), None]
            }
            CanvasTransformEdit::Scale([x, y]) => [
                Some((TransformComponent::Scale(0), x)),
                Some((TransformComponent::Scale(1), y)),
            ],
        };

        let Some(node) = self.selected_node else {
            return false;
        };
        for (component, value) in edits.into_iter().flatten() {
            if !value.is_finite() {
                self.action_notice = Some("Canvas transforms must remain finite.".into());
                return false;
            }
            if let Err(error) = self
                .document
                .as_ref()
                .ok_or_else(|| "no SRD document is open".to_owned())
                .and_then(|document| {
                    document.validate_transform_value(
                        self.selected_scene,
                        self.selected_layer,
                        node,
                        component,
                        value,
                    )
                })
            {
                self.action_notice = Some(format!("Canvas transform was not applied: {error}"));
                return false;
            }
        }

        let changed = edits
            .into_iter()
            .flatten()
            .any(|(component, value)| match component {
                TransformComponent::Translation(index) => before.translation[index] != value,
                TransformComponent::Rotation(index) => {
                    before.rotation[index] != value.round() as i32
                }
                TransformComponent::Scale(index) => before.scale[index] != value,
            });
        if !changed {
            return false;
        }

        let transform = self
            .selected_transform_mut()
            .expect("validated selected transform remains available");
        for (component, value) in edits.into_iter().flatten() {
            match component {
                TransformComponent::Translation(index) => transform.translation[index] = value,
                TransformComponent::Rotation(index) => {
                    transform.rotation[index] = value.round() as i32
                }
                TransformComponent::Scale(index) => transform.scale[index] = value,
            }
        }
        self.sync_inspector();
        self.bump_revision();
        true
    }

    fn selected_transform_mut(&mut self) -> Option<&mut SpatialTransform> {
        let scene = self.selected_scene;
        let layer = self.selected_layer;
        let node = self.selected_node?;
        let transform = self
            .document
            .as_mut()?
            .project
            .scenes
            .get_mut(scene)?
            .layers
            .get_mut(layer)?
            .transforms
            .get_mut(node)?;
        Some(match transform {
            RawTransform::Trs2(value) | RawTransform::Trs3(value) => value,
        })
    }

    fn selected_transform(&self) -> Option<SpatialTransform> {
        let node = self.selected_node?;
        self.selected_layer()
            .and_then(|layer| layer.transforms.get(node))
            .map(|transform| transform.spatial())
    }

    fn history_state(&self) -> Option<HistoryState> {
        Some(HistoryState {
            project: self.document.as_ref()?.project.clone(),
            inspector: self.inspector.clone(),
            scene_origins: self.document.as_ref()?.scene_origins().to_vec(),
            selected_scene: self.selected_scene,
            selected_layer: self.selected_layer,
            selected_node: self.selected_node,
            frame: self.frame,
            hidden_layers: self.hidden_layers.clone(),
            locked_layers: self.locked_layers.clone(),
            solo_layers: self.solo_layers.clone(),
            hidden_casts: self.hidden_casts.clone(),
            locked_casts: self.locked_casts.clone(),
            solo_casts: self.solo_casts.clone(),
            runtime_text_inputs: self.runtime_text_inputs.clone(),
            animate: self.animate_history(),
        })
    }

    fn valid_cast_selection(&self, selection: PreviewCastSelection) -> bool {
        self.document
            .as_ref()
            .and_then(|document| document.project.scenes.get(selection.scene_index))
            .and_then(|scene| scene.layers.get(selection.layer_index))
            .is_some_and(|layer| selection.node_index < layer.nodes.len())
    }

    fn cast_or_ancestor_in(
        &self,
        selections: &BTreeSet<PreviewCastSelection>,
        scene: usize,
        layer: usize,
        mut node: usize,
    ) -> bool {
        let Some(source_layer) = self
            .document
            .as_ref()
            .and_then(|document| document.project.scenes.get(scene))
            .and_then(|source_scene| source_scene.layers.get(layer))
        else {
            return false;
        };
        loop {
            if selections.contains(&PreviewCastSelection {
                scene_index: scene,
                layer_index: layer,
                node_index: node,
            }) {
                return true;
            }
            let Some(parent) = cast_parent_index(source_layer, node) else {
                return false;
            };
            node = parent;
        }
    }

    fn apply_history_state(&mut self, state: HistoryState) -> bool {
        let Some(document) = self.document.as_mut() else {
            return false;
        };
        document.restore_scene_history(state.project, state.scene_origins);
        self.inspector = state.inspector;
        self.selected_scene = state.selected_scene;
        self.selected_layer = state.selected_layer;
        self.selected_node = state.selected_node;
        self.frame = state.frame;
        self.hidden_layers = state.hidden_layers;
        self.locked_layers = state.locked_layers;
        self.solo_layers = state.solo_layers;
        self.hidden_casts = state.hidden_casts;
        self.locked_casts = state.locked_casts;
        self.solo_casts = state.solo_casts;
        self.runtime_text_inputs = state.runtime_text_inputs;
        self.animate_restore_history(state.animate);
        true
    }

    fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_history.pop_back() else {
            return false;
        };
        let Some(current) = self.history_state() else {
            self.undo_history.push_back(previous);
            return false;
        };
        if !self.apply_history_state(previous) {
            self.undo_history.push_back(current);
            return false;
        }
        push_bounded(&mut self.redo_history, current);
        self.rebuild_timeline_tracks();
        self.animate_sync();
        self.bump_revision();
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo_history.pop_back() else {
            return false;
        };
        let Some(current) = self.history_state() else {
            self.redo_history.push_back(next);
            return false;
        };
        if !self.apply_history_state(next) {
            self.redo_history.push_back(current);
            return false;
        }
        push_bounded(&mut self.undo_history, current);
        self.rebuild_timeline_tracks();
        self.animate_sync();
        self.bump_revision();
        true
    }

    pub(super) fn sync_inspector(&mut self) {
        let Some(layer) = self.selected_layer() else {
            self.inspector = InspectorDraft::default();
            return;
        };
        let mut draft = InspectorDraft {
            layer_name: display_srd_name(&layer.name),
            layer_flags: layer.flags,
            ..InspectorDraft::default()
        };
        let Some(node) = self.selected_node else {
            self.inspector = draft;
            return;
        };
        let Some(classification) = layer.classify_cast(node) else {
            self.inspector = draft;
            return;
        };

        draft.cast_name = layer
            .nodes
            .get(node)
            .and_then(|record| record.name.as_deref())
            .map(display_srd_name)
            .unwrap_or_default();
        draft.role = cast_role_draft(layer, node, classification.kind);

        if let Some((spatial, runtime_image)) = self.inspected_runtime_values() {
            draft.position = spatial.translation.map(format_number);
            draft.rotation = spatial.rotation.map(|value| value.to_string());
            draft.scale = spatial.scale.map(format_number);
            draft.multiply_color = spatial.multiply_color.map(|value| value.to_string());
            draft.additive_color = spatial.additive_color.map(|value| value.to_string());
            draft.visibility = spatial.visibility_word != 0;

            if self.workspace == Workspace::Animate
                && let Some(runtime_image) = runtime_image
                && let Some(geometry) = draft.role.geometry_mut()
            {
                geometry.size = runtime_image.geometry.size.map(format_number);
                geometry.vertex_colors = runtime_image.coordinates[0]
                    .vertex_colors
                    .map(|color| color.map(|value| value.to_string()));
                if let Some(binding) = draft.role.image_binding_mut() {
                    binding.cref_index = runtime_image.coordinates[0].reference_index.to_string();
                    binding.cre1_index = runtime_image.coordinates[1].reference_index.to_string();
                }
            }
        }
        self.inspector = draft;
    }

    fn inspected_runtime_values(
        &self,
    ) -> Option<(SpatialTransform, Option<crate::image::RuntimeImageState>)> {
        let document = self.document.as_ref()?;
        let node = self.selected_node?;
        if self.workspace == Workspace::Design {
            return Some((self.selected_transform()?, None));
        }
        let mut runtime = ProjectRuntime::new(&document.project).ok()?;
        runtime
            .apply_animation_set_definition(
                &document.project,
                &document.textures,
                self.selected_scene,
                self.animate.effective_assignment(),
                self.animate.frame() as f32,
            )
            .ok()?;
        let layer = runtime
            .project_layers
            .get(self.selected_scene)?
            .get(self.selected_layer)?;
        Some((
            *layer.cast_transforms.get(node)?,
            layer.image_states.get(node).copied(),
        ))
    }

    fn rebuild_timeline_tracks(&mut self) {
        let mut tracks = Vec::new();
        let Some(layer) = self.selected_layer() else {
            self.timeline_tracks = tracks;
            return;
        };
        tracks.push(TimelineTrack {
            label: display_srd_name(&layer.name),
            keyframes: Vec::new(),
            child: false,
        });
        let animation = self
            .selected_animation_set()
            .and_then(|set| set.slots.get(self.selected_layer))
            .filter(|slot| slot.is_enabled() && !slot.animation_name.is_empty())
            .and_then(|slot| layer.find_animation(&slot.animation_name))
            .map(|(_, animation)| animation);
        let mut by_target = BTreeMap::<u16, BTreeSet<i32>>::new();
        if let (Some(animation), Some(node)) = (animation, self.selected_node) {
            for track in animation
                .motions
                .iter()
                .filter(|motion| motion.target == node as i32)
                .flat_map(|motion| &motion.tracks)
            {
                by_target
                    .entry(track.target)
                    .or_default()
                    .extend(track_key_frames(track));
            }
        }
        for (target, frames) in by_target {
            tracks.push(TimelineTrack {
                label: animation_target_name(target),
                keyframes: frames.into_iter().collect(),
                child: true,
            });
        }
        self.timeline_tracks = tracks;
    }

    fn set_frame(&mut self, frame: i32) -> bool {
        let (start, end) = self.frame_range();
        let frame = frame.clamp(start, end);
        if frame == self.frame {
            return false;
        }
        self.frame = frame;
        self.sync_inspector();
        true
    }

    pub(super) fn bump_revision(&mut self) {
        self.document_revision = self.document_revision.wrapping_add(1);
    }
}

const MAX_UNDO_HISTORY: usize = 100;

fn push_bounded<T>(history: &mut VecDeque<T>, value: T) {
    if history.len() == MAX_UNDO_HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

fn parse_inspector_value(field: InspectorField, value: &str) -> Option<InspectorValue> {
    let value = value.trim();
    match field {
        InspectorField::LayerName
        | InspectorField::CastName
        | InspectorField::ReferenceSource
        | InspectorField::ReferenceLayer
        | InspectorField::ReferenceAnimation => {
            (value.len() <= 64).then(|| InspectorValue::Bytes(value.as_bytes().to_vec()))
        }
        InspectorField::MultiplyColor(_)
        | InspectorField::AdditiveColor(_)
        | InspectorField::VertexColor(_, _)
        | InspectorField::OriginMode => parse_u32_text(value)
            .and_then(|value| u8::try_from(value).ok())
            .map(InspectorValue::U8),
        InspectorField::CrefIndex | InspectorField::Cre1Index => {
            value.parse::<i16>().ok().map(InspectorValue::I16)
        }
        InspectorField::NumberInteger | InspectorField::TextFont => {
            value.parse::<i32>().ok().map(InspectorValue::I32)
        }
        InspectorField::PayloadWidth
        | InspectorField::PayloadHeight
        | InspectorField::OriginX
        | InspectorField::OriginY
        | InspectorField::CoordinateOffset(_, _)
        | InspectorField::NumberFraction
        | InspectorField::ReferenceFrame => value
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .map(InspectorValue::F32),
    }
}

fn parse_u32_text(value: &str) -> Option<u32> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |digits| u32::from_str_radix(digits, 16).ok(),
        )
}

fn cast_role_draft(layer: &Layer, node: usize, kind: SrCastKind) -> CastRoleDraft {
    match kind {
        SrCastKind::Null => CastRoleDraft::Null,
        SrCastKind::Image => CastRoleDraft::Image(ImageCastDraft {
            geometry: visual_geometry_draft(layer, node, kind),
            binding: image_binding_draft(layer, node),
        }),
        SrCastKind::Text => {
            let text = layer
                .image_by_node
                .get(node)
                .and_then(Option::as_ref)
                .and_then(|image| image.text.as_ref());
            CastRoleDraft::Text(TextCastDraft {
                box_geometry: visual_geometry_draft(layer, node, kind),
                source_binding: image_binding_draft(layer, node),
                flags: text.and_then(|value| value.field_78),
                font: text
                    .and_then(|value| value.font_index)
                    .map_or_else(|| "0".into(), |value| value.to_string()),
            })
        }
        SrCastKind::Slice => CastRoleDraft::Slice(SliceCastDraft {
            grid: visual_geometry_draft(layer, node, kind),
        }),
        SrCastKind::Reference => {
            let reference = layer.reference_by_node.get(node).and_then(Option::as_ref);
            CastRoleDraft::Reference(ReferenceCastDraft {
                source: reference
                    .map(|value| display_srd_name(&value.source_name))
                    .unwrap_or_default(),
                layer: reference
                    .map(|value| display_srd_name(&value.layer_name))
                    .unwrap_or_default(),
                animation: reference
                    .map(|value| display_srd_name(&value.animation_name))
                    .unwrap_or_default(),
                enabled: reference.is_some_and(|value| value.animation_enabled != 0),
                frame: reference
                    .map_or_else(|| "0".into(), |value| format_number(value.default_frame)),
            })
        }
        SrCastKind::Number => {
            let number = layer.number_by_node.get(node).and_then(Option::as_ref);
            CastRoleDraft::Number(NumberCastDraft {
                layout: visual_geometry_draft(layer, node, kind),
                integer: number
                    .map_or_else(|| "0".into(), |value| value.initial_integer.to_string()),
                fraction: number
                    .map_or_else(|| "0".into(), |value| format_number(value.initial_fraction)),
                format_flags: number.map_or(0, |value| value.format_flags),
                alignment_flags: number.map_or(0, |value| value.field_78),
            })
        }
    }
}

fn visual_geometry_draft(layer: &Layer, node: usize, kind: SrCastKind) -> VisualGeometryDraft {
    let mut draft = VisualGeometryDraft::default();
    if let Some(size) = payload_size(layer, node, kind) {
        draft.size = size.map(format_number);
    }
    if let Some(origin) = payload_origin(layer, node, kind) {
        draft.origin = origin.map(format_number);
    }
    if let Some(flags) = payload_flags(layer, node, kind) {
        draft.flags = flags;
    }
    if let Some(origin_mode) = payload_origin_mode(layer, node, kind) {
        draft.origin_mode = origin_mode.to_string();
    }
    if let Some(colors) = payload_vertex_colors(layer, node, kind) {
        draft.vertex_colors = colors.map(|color| color.map(|value| value.to_string()));
    }
    draft
}

fn image_binding_draft(layer: &Layer, node: usize) -> ImageBindingDraft {
    let Some(image) = layer.image_by_node.get(node).and_then(Option::as_ref) else {
        return ImageBindingDraft::default();
    };
    ImageBindingDraft {
        cref_index: image.cref_index.to_string(),
        cre1_index: image.cre1_index.to_string(),
        coordinate_offsets: image
            .coordinate_offsets
            .map(|offset| offset.map(format_number)),
    }
}

fn payload_size(layer: &Layer, node: usize, kind: SrCastKind) -> Option<[f32; 2]> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => layer
            .image_by_node
            .get(node)?
            .as_ref()
            .map(|value| [value.width, value.height]),
        SrCastKind::Slice => layer
            .csli_by_node
            .get(node)?
            .as_ref()
            .map(|value| [value.width, value.height]),
        SrCastKind::Number => layer
            .number_by_node
            .get(node)?
            .as_ref()
            .map(|value| [value.width, value.height]),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_origin(layer: &Layer, node: usize, kind: SrCastKind) -> Option<[f32; 2]> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => layer
            .image_by_node
            .get(node)?
            .as_ref()
            .map(|value| value.custom_origin),
        SrCastKind::Slice => layer
            .csli_by_node
            .get(node)?
            .as_ref()
            .map(|value| value.custom_origin),
        SrCastKind::Number => layer
            .number_by_node
            .get(node)?
            .as_ref()
            .map(|value| value.custom_origin),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_flags(layer: &Layer, node: usize, kind: SrCastKind) -> Option<u32> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => {
            Some(layer.image_by_node.get(node)?.as_ref()?.flags)
        }
        SrCastKind::Slice => Some(layer.csli_by_node.get(node)?.as_ref()?.field_80),
        SrCastKind::Number => Some(layer.number_by_node.get(node)?.as_ref()?.flags),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_origin_mode(layer: &Layer, node: usize, kind: SrCastKind) -> Option<u8> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => {
            Some(layer.image_by_node.get(node)?.as_ref()?.origin_mode)
        }
        SrCastKind::Slice => Some(layer.csli_by_node.get(node)?.as_ref()?.origin_mode),
        SrCastKind::Number => Some(layer.number_by_node.get(node)?.as_ref()?.origin_mode),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_vertex_colors(layer: &Layer, node: usize, kind: SrCastKind) -> Option<[[u8; 4]; 4]> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => {
            Some(layer.image_by_node.get(node)?.as_ref()?.vertex_colors)
        }
        SrCastKind::Slice => Some(layer.csli_by_node.get(node)?.as_ref()?.field_44),
        SrCastKind::Number => Some(layer.number_by_node.get(node)?.as_ref()?.vertex_colors),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_transform_mut(layer: &mut Layer, node: usize) -> Option<&mut SpatialTransform> {
    match layer.transforms.get_mut(node)? {
        RawTransform::Trs2(value) | RawTransform::Trs3(value) => Some(value),
    }
}

fn payload_size_mut(
    layer: &mut Layer,
    node: usize,
    kind: SrCastKind,
    component: usize,
) -> Option<&mut f32> {
    let (width, height) = match kind {
        SrCastKind::Image | SrCastKind::Text => {
            let value = layer.image_by_node.get_mut(node)?.as_mut()?;
            (&mut value.width, &mut value.height)
        }
        SrCastKind::Slice => {
            let value = layer.csli_by_node.get_mut(node)?.as_mut()?;
            (&mut value.width, &mut value.height)
        }
        SrCastKind::Number => {
            let value = layer.number_by_node.get_mut(node)?.as_mut()?;
            (&mut value.width, &mut value.height)
        }
        SrCastKind::Null | SrCastKind::Reference => return None,
    };
    match component {
        0 => Some(width),
        1 => Some(height),
        _ => None,
    }
}

fn payload_origin_mut(
    layer: &mut Layer,
    node: usize,
    kind: SrCastKind,
    component: usize,
) -> Option<&mut f32> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => layer
            .image_by_node
            .get_mut(node)?
            .as_mut()?
            .custom_origin
            .get_mut(component),
        SrCastKind::Slice => layer
            .csli_by_node
            .get_mut(node)?
            .as_mut()?
            .custom_origin
            .get_mut(component),
        SrCastKind::Number => layer
            .number_by_node
            .get_mut(node)?
            .as_mut()?
            .custom_origin
            .get_mut(component),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_flags_mut(layer: &mut Layer, node: usize, kind: SrCastKind) -> Option<&mut u32> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => {
            Some(&mut layer.image_by_node.get_mut(node)?.as_mut()?.flags)
        }
        SrCastKind::Slice => Some(&mut layer.csli_by_node.get_mut(node)?.as_mut()?.field_80),
        SrCastKind::Number => Some(&mut layer.number_by_node.get_mut(node)?.as_mut()?.flags),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

pub(super) fn set_masked_flag(value: &mut u32, mask: u32, enabled: bool) -> bool {
    set_masked_value(value, mask, if enabled { mask } else { 0 })
}

fn set_masked_value(value: &mut u32, mask: u32, bits: u32) -> bool {
    let next = (*value & !mask) | (bits & mask);
    if next == *value {
        return false;
    }
    *value = next;
    true
}

fn payload_origin_mode_mut(layer: &mut Layer, node: usize, kind: SrCastKind) -> Option<&mut u8> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => {
            Some(&mut layer.image_by_node.get_mut(node)?.as_mut()?.origin_mode)
        }
        SrCastKind::Slice => Some(&mut layer.csli_by_node.get_mut(node)?.as_mut()?.origin_mode),
        SrCastKind::Number => Some(&mut layer.number_by_node.get_mut(node)?.as_mut()?.origin_mode),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}

fn payload_vertex_color_mut(
    layer: &mut Layer,
    node: usize,
    kind: SrCastKind,
    vertex: usize,
    component: usize,
) -> Option<&mut u8> {
    match kind {
        SrCastKind::Image | SrCastKind::Text => layer
            .image_by_node
            .get_mut(node)?
            .as_mut()?
            .vertex_colors
            .get_mut(vertex)?
            .get_mut(component),
        SrCastKind::Slice => layer
            .csli_by_node
            .get_mut(node)?
            .as_mut()?
            .field_44
            .get_mut(vertex)?
            .get_mut(component),
        SrCastKind::Number => layer
            .number_by_node
            .get_mut(node)?
            .as_mut()?
            .vertex_colors
            .get_mut(vertex)?
            .get_mut(component),
        SrCastKind::Null | SrCastKind::Reference => None,
    }
}
fn transform_animation_target(field: TransformField) -> u16 {
    match field {
        TransformField::PositionX => 0,
        TransformField::PositionY => 1,
        TransformField::PositionZ => 2,
        TransformField::RotationX => 3,
        TransformField::RotationY => 4,
        TransformField::RotationZ => 5,
        TransformField::ScaleX => 6,
        TransformField::ScaleY => 7,
        TransformField::ScaleZ => 8,
    }
}

fn transform_component(field: TransformField) -> TransformComponent {
    match field {
        TransformField::PositionX => TransformComponent::Translation(0),
        TransformField::PositionY => TransformComponent::Translation(1),
        TransformField::PositionZ => TransformComponent::Translation(2),
        TransformField::RotationX => TransformComponent::Rotation(0),
        TransformField::RotationY => TransformComponent::Rotation(1),
        TransformField::RotationZ => TransformComponent::Rotation(2),
        TransformField::ScaleX => TransformComponent::Scale(0),
        TransformField::ScaleY => TransformComponent::Scale(1),
        TransformField::ScaleZ => TransformComponent::Scale(2),
    }
}

fn toggle_set<T: Ord>(set: &mut BTreeSet<T>, value: T) {
    if !set.remove(&value) {
        set.insert(value);
    }
}

fn cast_parent_index(layer: &Layer, node_index: usize) -> Option<usize> {
    for (parent_index, parent) in layer.nodes.iter().enumerate() {
        let mut child_index = parent.first_child_index;
        for _ in 0..layer.nodes.len() {
            if child_index == -1 {
                break;
            }
            let child = usize::try_from(child_index).ok()?;
            let child_node = layer.nodes.get(child)?;
            if child == node_index {
                return Some(parent_index);
            }
            child_index = child_node.next_sibling_index;
        }
    }
    None
}

fn empty_usize_set() -> &'static BTreeSet<usize> {
    static EMPTY: std::sync::OnceLock<BTreeSet<usize>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(BTreeSet::new)
}

fn format_number(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn track_key_frames(track: &Track) -> impl Iterator<Item = i32> + '_ {
    enum Frames<'a> {
        Key8F32(std::slice::Iter<'a, crate::animation::Key8<f32>>),
        Key8I32(std::slice::Iter<'a, crate::animation::Key8<i32>>),
        Key8Bytes4(std::slice::Iter<'a, crate::animation::Key8<[u8; 4]>>),
        Key20F32(std::slice::Iter<'a, crate::animation::Key20<f32>>),
        Key20I32(std::slice::Iter<'a, crate::animation::Key20<i32>>),
        Empty,
    }
    impl Iterator for Frames<'_> {
        type Item = i32;

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                Self::Key8F32(values) => values.next().map(|key| key.frame),
                Self::Key8I32(values) => values.next().map(|key| key.frame),
                Self::Key8Bytes4(values) => values.next().map(|key| key.frame),
                Self::Key20F32(values) => values.next().map(|key| key.frame),
                Self::Key20I32(values) => values.next().map(|key| key.frame),
                Self::Empty => None,
            }
        }
    }
    match &track.keys {
        KeyData::Key8F32(values) => Frames::Key8F32(values.iter()),
        KeyData::Key8I32(values) => Frames::Key8I32(values.iter()),
        KeyData::Key8Bytes4(values) => Frames::Key8Bytes4(values.iter()),
        KeyData::Key20F32(values) => Frames::Key20F32(values.iter()),
        KeyData::Key20I32(values) => Frames::Key20I32(values.iter()),
        KeyData::Unsupported => Frames::Empty,
    }
}

fn animation_target_name(target: u16) -> String {
    match target {
        0 => "Position · X".into(),
        1 => "Position · Y".into(),
        2 => "Position · Z".into(),
        3 => "Rotation · X".into(),
        4 => "Rotation · Y".into(),
        5 => "Rotation · Z".into(),
        6 => "Scale · X".into(),
        7 => "Scale · Y".into(),
        8 => "Scale · Z".into(),
        9 => "Multiply color".into(),
        10 => "Visibility".into(),
        19 => "Additive color".into(),
        21 => "Opacity".into(),
        22 => "Additive opacity".into(),
        _ => format!("Channel {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationDefinition, Key8, KeyData, Motion, Track};
    use crate::number::NumberDefinition;
    use crate::reference::ReferenceDefinition;
    use crate::scene::{AnimationSetDefinition, SceneAnimationSlot};
    use crate::serialized_flags::{IMAGE_FLAGS, LAYER_FLAGS, NODE_FLAGS, TEXT_FLAGS};

    #[test]
    fn startup_without_a_path_is_an_editable_empty_project() {
        let mut model = EditorModel::new(None);

        assert!(model.document().is_some());
        assert!(model.project_path().is_none());
        assert_eq!(model.project_file_name(), "Untitled.srd");
        assert!(model.document_editing());
        assert!(model.selected_scene().is_none());
        assert!(model.selected_layer().is_none());
        assert!(model.can_add_scene());
        assert!(!model.is_dirty());

        let change = model.update(EditorAction::TogglePlaying);
        assert!(!change.preview);
        assert!(!model.playing());
    }

    #[test]
    fn first_scene_is_undoable_renderable_and_persistent() {
        let path = temporary_srd_path("first-scene");
        let mut model = EditorModel::new(None);
        let initial_revision = model.document_revision();

        let change = model.update(EditorAction::AddScene);
        assert!(change.preview);
        assert_eq!(model.document_revision(), initial_revision + 1);
        assert_eq!(model.selected_scene_name(), "Scene 1");
        assert!(model.selected_layer().is_none());
        assert!(model.can_add_scene());
        assert!(model.is_dirty());
        assert_eq!(model.document().unwrap().project.declared_scene_count, 1);
        assert!(
            model.document().unwrap().project.scenes[0]
                .layers
                .is_empty()
        );

        let change = model.update(EditorAction::Undo);
        assert!(change.preview);
        assert!(model.selected_scene().is_none());
        assert!(model.can_add_scene());
        assert!(!model.is_dirty());

        let change = model.update(EditorAction::Redo);
        assert!(change.preview);
        assert_eq!(model.selected_scene_name(), "Scene 1");
        model.save_as(&path).unwrap();

        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.declared_scene_count, 1);
        assert_eq!(reloaded.project.scenes.len(), 1);
        assert!(reloaded.project.scenes[0].layers.is_empty());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn scene_crud_is_undoable_and_persistent() {
        let path = temporary_srd_path("scene-crud");
        let mut model = EditorModel::new(None);

        assert!(model.update(EditorAction::AddScene).preview);
        assert!(model.update(EditorAction::AddScene).preview);
        assert_eq!(model.document().unwrap().project.scenes.len(), 2);
        assert_eq!(model.selected_scene_name(), "Scene 2");
        assert!(
            model
                .update(EditorAction::RenameScene("Gameplay".into()))
                .preview
        );
        assert_eq!(model.selected_scene_name(), "Gameplay");

        model.update(EditorAction::SelectScene(0));
        assert!(model.update(EditorAction::DeleteScene).preview);
        assert_eq!(model.document().unwrap().project.scenes.len(), 1);
        assert_eq!(model.selected_scene_name(), "Gameplay");

        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.document().unwrap().project.scenes.len(), 2);
        assert_eq!(model.selected_scene_name(), "Scene 1");
        assert!(model.update(EditorAction::Redo).preview);
        assert_eq!(model.document().unwrap().project.scenes.len(), 1);
        assert_eq!(model.selected_scene_name(), "Gameplay");

        model.save_as(&path).unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.declared_scene_count, 1);
        assert_eq!(reloaded.project.scenes[0].name, b"Gameplay");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn scene_history_restores_source_provenance() {
        let path = temporary_srd_path("scene-history-provenance");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(model.update(EditorAction::AddScene).preview);
        assert_eq!(model.document().unwrap().scene_origins(), &[Some(0), None]);
        model.update(EditorAction::SelectScene(0));
        assert!(model.update(EditorAction::DeleteScene).preview);
        assert_eq!(model.document().unwrap().scene_origins(), &[None]);

        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.document().unwrap().scene_origins(), &[Some(0), None]);
        assert!(
            model
                .update(EditorAction::RenameScene("Retained Source".into()))
                .preview
        );
        model.save().unwrap();

        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.scenes.len(), 2);
        assert_eq!(reloaded.project.scenes[0].name, b"Retained Source");
        assert_eq!(reloaded.project.scenes[1].name, b"Scene 2");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn animation_set_crud_is_persistent() {
        let path = temporary_srd_path("animation-set-crud");
        let mut model = EditorModel::new(None);
        model.update(EditorAction::AddScene);

        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::CreateSet))
                .preview
        );
        assert_eq!(model.selected_scene().unwrap().animation_sets.len(), 1);
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::DuplicateSet))
                .preview
        );
        assert_eq!(model.selected_scene().unwrap().animation_sets.len(), 2);
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::DeleteSet))
                .preview
        );
        assert_eq!(model.selected_scene().unwrap().animation_sets.len(), 1);

        model.save_as(&path).unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.scenes[0].animation_sets.len(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn layer_animation_crud_is_persistent() {
        let path = temporary_srd_path("layer-animation-crud");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);
        let initial_count = model.selected_layer().unwrap().animations.len();

        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::CreateAnimation))
                .preview
        );
        assert_eq!(
            model.selected_layer().unwrap().animations.len(),
            initial_count + 1
        );
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::DuplicateAnimation))
                .preview
        );
        assert_eq!(
            model.selected_layer().unwrap().animations.len(),
            initial_count + 2
        );
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::DeleteAnimation))
                .preview
        );
        assert_eq!(
            model.selected_layer().unwrap().animations.len(),
            initial_count + 1
        );

        model.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            reloaded.project.scenes[0].layers[0].animations.len(),
            initial_count + 1
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cast_expansion_is_independent_and_defaults_open() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        let mut model = EditorModel::new(Some(path));
        assert!(model.cast_expanded(0, 0, 0));
        assert!(model.cast_expanded(0, 0, 1));

        model.update(EditorAction::ToggleCastExpanded(0, 0, 0));
        assert!(!model.cast_expanded(0, 0, 0));
        assert!(model.cast_expanded(0, 0, 1));

        model.update(EditorAction::ToggleCastExpanded(0, 0, 0));
        assert!(model.cast_expanded(0, 0, 0));
    }

    #[test]
    fn selecting_a_layer_clears_cast_focus_and_populates_layer_metadata() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        let mut model = EditorModel::new(Some(path));
        assert!(model.selected_node_index().is_some());
        let scene = model.selected_scene_index();
        let layer = model.selected_layer_index();
        let expected_name = display_srd_name(&model.selected_layer().unwrap().name);
        let expected_flags = model.selected_layer().unwrap().flags;

        assert!(
            model
                .update(EditorAction::SelectSceneLayer(scene, layer))
                .preview
        );
        assert_eq!(model.selected_node_index(), None);
        assert_eq!(model.inspector().layer_name, expected_name);
        assert_eq!(model.inspector().layer_flags, expected_flags);
        assert!(model.inspector().cast_name.is_empty());
    }

    #[test]
    fn semantic_flag_controls_preserve_unlisted_bits_and_change_only_their_masks() {
        let path = temporary_srd_path("semantic-flag-masks");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Flags")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        {
            let layer = &mut model.document.as_mut().unwrap().project.scenes[0].layers[0];
            layer.flags = 0x8000_0001;
            layer.nodes[0].type_flags = Some(0x8000_0001);
            let image = layer.image_by_node[0].as_mut().unwrap();
            image.flags = 0x8000_0700;
            image.text.as_mut().unwrap().field_78 = Some(0x8000_003C);
        }
        model.sync_inspector();
        model.update(EditorAction::ToggleDocumentEditing);

        for action in [
            EditorAction::SetInspectorToggle(InspectorToggle::LayerActive, true),
            EditorAction::SetInspectorToggle(InspectorToggle::CastActive, true),
            EditorAction::SetInspectorToggle(InspectorToggle::PayloadFlipU, true),
            EditorAction::SetInspectorChoice(InspectorChoice::PayloadRenderSelector(
                PayloadRenderSelector::Preset5,
            )),
            EditorAction::SetInspectorChoice(InspectorChoice::UvVertexOrder(
                UvVertexOrder::BottomRightFirst,
            )),
            EditorAction::SetInspectorChoice(InspectorChoice::PayloadSpecialPreset(
                PayloadSpecialPreset::Preset20,
            )),
            EditorAction::SetInspectorToggle(InspectorToggle::PayloadPointSampling, true),
            EditorAction::SetInspectorToggle(InspectorToggle::TextLayoutBypass, true),
            EditorAction::SetInspectorChoice(InspectorChoice::TextHorizontalAlignment(
                HorizontalAlignment::Right,
            )),
            EditorAction::SetInspectorChoice(InspectorChoice::TextVerticalAlignment(
                VerticalAlignment::Middle,
            )),
        ] {
            assert!(model.update(action).preview);
        }

        let layer = &model.document().unwrap().project.scenes[0].layers[0];
        assert_eq!(layer.flags, 0x8000_0101);
        assert_eq!(layer.nodes[0].type_flags, Some(0x8000_0101));
        let image = layer.image_by_node[0].as_ref().unwrap();
        assert_eq!(image.flags, 0x8100_0392);
        assert_eq!(image.text.as_ref().unwrap().field_78, Some(0x8000_0019));
        assert_eq!(LAYER_FLAGS.unknown_set_bits(layer.flags), 0x8000_0000);
        assert_eq!(
            NODE_FLAGS.unknown_set_bits(layer.nodes[0].type_flags.unwrap()),
            0x8000_0000,
        );
        assert_eq!(IMAGE_FLAGS.unknown_set_bits(image.flags), 0x8000_0000);
        assert_eq!(TEXT_FLAGS.unknown_set_bits(0x8000_0019), 0x8000_0000);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn layer_and_cast_active_toggles_save_and_reload() {
        let path = temporary_srd_path("active-toggles");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Active")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        assert!(!model.selected_layer().unwrap().active());
        assert!(!model.selected_node().unwrap().active());
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::LayerActive,
                    true,
                ))
                .preview,
        );
        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::CastActive,
                    true,
                ))
                .preview,
        );
        model.save().unwrap();

        let reloaded = EditorDocument::load(&path).unwrap();
        let layer = &reloaded.project.scenes[0].layers[0];
        assert!(layer.active());
        assert!(layer.nodes[0].active());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn number_format_toggles_preserve_unlisted_bits() {
        let path = temporary_srd_path("semantic-number-format");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Number")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        {
            let layer = &mut model.document.as_mut().unwrap().project.scenes[0].layers[0];
            layer.nodes[0].type_flags = Some(4);
            layer.image_by_node[0] = None;
            layer.number_by_node[0] = Some(NumberDefinition {
                flags: 0,
                width: 128.0,
                height: 32.0,
                custom_origin: [0.0; 2],
                origin_mode: 0,
                vertex_colors: [[0xFF; 4]; 4],
                cref_count: 0,
                crefs: Vec::new(),
                field_4c: 0,
                cre1_count: 0,
                cre1s: Vec::new(),
                format_flags: 0x8000_0000,
                field_78: 0,
                initial_integer: 0,
                initial_fraction: 0.0,
                fields_83_8b: [0; 9],
                field_8c: [0.0; 2],
                fields_8d_94: [0; 8],
                node_index: 0,
            });
        }
        model.sync_inspector();
        model.update(EditorAction::ToggleDocumentEditing);

        for toggle in [
            InspectorToggle::NumberForcePlus,
            InspectorToggle::NumberGrouping,
            InspectorToggle::NumberPadInteger,
            InspectorToggle::NumberFractionalDigits,
            InspectorToggle::NumberPadFraction,
            InspectorToggle::NumberDigitSpacingAfterDecimal,
        ] {
            assert!(
                model
                    .update(EditorAction::SetInspectorToggle(toggle, true))
                    .preview
            );
        }
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].number_by_node[0]
                .as_ref()
                .unwrap()
                .format_flags,
            0x8000_003F
        );
        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::NumberGrouping,
                    false,
                ))
                .preview
        );
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].number_by_node[0]
                .as_ref()
                .unwrap()
                .format_flags,
            0x8000_003D
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn scalar_boolean_controls_preserve_noncanonical_true_until_state_changes() {
        let path = temporary_srd_path("semantic-scalar-booleans");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Boolean")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        payload_transform_mut(
            &mut model.document.as_mut().unwrap().project.scenes[0].layers[0],
            0,
        )
        .unwrap()
        .visibility_word = 7;
        model.sync_inspector();
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(model.inspector().visibility);
        assert!(
            !model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::CastVisible,
                    true,
                ))
                .preview
        );
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].transforms[0]
                .spatial()
                .visibility_word,
            7
        );
        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::CastVisible,
                    false,
                ))
                .preview
        );
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].transforms[0]
                .spatial()
                .visibility_word,
            0
        );

        {
            let layer = &mut model.document.as_mut().unwrap().project.scenes[0].layers[0];
            layer.nodes[0].type_flags = Some(3);
            layer.image_by_node[0] = None;
            layer.reference_by_node[0] = Some(ReferenceDefinition {
                source_name: b"scene".to_vec(),
                layer_name: b"layer".to_vec(),
                animation_enabled: 7,
                animation_name: b"anim".to_vec(),
                default_frame: 0.0,
                node_index: 0,
            });
        }
        model.sync_inspector();
        assert!(
            !model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::ReferenceAnimationEnabled,
                    true,
                ))
                .preview
        );
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].reference_by_node[0]
                .as_ref()
                .unwrap()
                .animation_enabled,
            7
        );
        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::ReferenceAnimationEnabled,
                    false,
                ))
                .preview
        );
        assert_eq!(
            model.document().unwrap().project.scenes[0].layers[0].reference_by_node[0]
                .as_ref()
                .unwrap()
                .animation_enabled,
            0
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cast_controls_are_editor_only_and_request_preview_when_visible() {
        let path = temporary_srd_path("cast-control-dirty-state");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));

        assert!(!model.is_dirty());
        assert!(
            model
                .update(EditorAction::ToggleCastVisible(0, 0, 0))
                .preview
        );
        assert!(model.update(EditorAction::ToggleCastSolo(0, 0, 0)).preview);
        assert!(
            !model
                .update(EditorAction::ToggleCastLocked(0, 0, 0))
                .preview
        );
        assert!(!model.is_dirty());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cast_controls_inherit_and_prevent_locked_descendant_edits() {
        let path = temporary_srd_path("cast-controls");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        {
            let layer = &mut model.document.as_mut().unwrap().project.scenes[0].layers[0];
            let mut child = layer.nodes[0].clone();
            child.first_child_index = -1;
            child.next_sibling_index = -1;
            layer.nodes[0].first_child_index = 1;
            layer.nodes.push(child);
            layer.transforms.push(layer.transforms[0]);
        }
        model.selected_node = Some(1);
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(
            model
                .update(EditorAction::ToggleCastVisible(0, 0, 0))
                .preview
        );
        assert!(!model.cast_visible(0, 0, 1));
        assert!(model.update(EditorAction::ToggleCastSolo(0, 0, 0)).preview);
        assert!(!model.cast_soloed(0, 0, 1));
        assert!(
            model
                .update(EditorAction::ToggleCastVisible(0, 0, 0))
                .preview
        );
        assert!(model.cast_visible(0, 0, 1));
        assert!(model.cast_soloed(0, 0, 1));

        assert!(
            !model
                .update(EditorAction::ToggleCastLocked(0, 0, 0))
                .preview
        );
        assert!(model.cast_locked(0, 0, 1));
        assert!(model.selected_cast_locked());
        let before = model.selected_transform();
        assert!(
            !model
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "99".into()
                ))
                .preview
        );
        assert_eq!(model.selected_transform(), before);

        assert!(model.update(EditorAction::Undo).preview);
        assert!(!model.cast_locked(0, 0, 1));
        assert!(model.update(EditorAction::Redo).preview);
        assert!(model.cast_locked(0, 0, 1));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn inspector_transform_is_reevaluated_when_the_timeline_frame_changes() {
        let path = temporary_srd_path("evaluated-inspector");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        {
            let scene = &mut model.document.as_mut().unwrap().project.scenes[0];
            scene.layers[0].animations = vec![AnimationDefinition {
                name: b"move".to_vec(),
                flags: 1,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 0,
                        key_count: 2,
                        format: 0x10,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8F32(vec![
                            Key8 {
                                frame: 0,
                                value: 10.0,
                            },
                            Key8 {
                                frame: 10,
                                value: 30.0,
                            },
                        ]),
                    }],
                }],
            }];
            scene.layers[0].animation_count = 1;
            scene.animation_sets = vec![AnimationSetDefinition {
                name: b"moving".to_vec(),
                start_frame: 0,
                runtime_duration: 10,
                declared_slot_count: 1,
                slots: vec![SceneAnimationSlot {
                    animation_name: b"move".to_vec(),
                    enabled: 1,
                }],
            }];
            scene.declared_animation_set_count = 1;
        }
        model.update(EditorAction::SetWorkspace(Workspace::Animate));

        assert_eq!(model.inspector().position[0], "1");
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))))
                .preview
        );
        assert_eq!(model.inspector().position[0], "10");
        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::SetFrame(5)))
                .preview
        );
        assert_eq!(model.inspector().position[0], "20");
        assert!(model.animation_active());

        assert!(
            model
                .update(EditorAction::Animate(AnimateAction::LoadStoredSet(None)))
                .preview
        );
        assert_eq!(model.inspector().position[0], "1");
        assert!(!model.animation_active());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn animate_inspector_edits_author_real_transform_visibility_and_opacity_keys() {
        fn channel(animation: &AnimationDefinition, target: u16) -> &Track {
            animation
                .motions
                .iter()
                .find(|motion| motion.target == 0)
                .and_then(|motion| motion.tracks.iter().find(|track| track.target == target))
                .unwrap_or_else(|| panic!("animation has no target {target} track"))
        }

        let path = temporary_srd_path("animate-inspector-keys");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Key me")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        {
            let scene = &mut model.document.as_mut().unwrap().project.scenes[0];
            scene.layers[0].animations = vec![AnimationDefinition {
                name: b"move".to_vec(),
                flags: 0,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 0,
                        key_count: 2,
                        format: 0x10,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8F32(vec![
                            Key8 {
                                frame: 0,
                                value: 10.0,
                            },
                            Key8 {
                                frame: 10,
                                value: 30.0,
                            },
                        ]),
                    }],
                }],
            }];
            scene.layers[0].animation_count = 1;
            scene.animation_sets = vec![AnimationSetDefinition {
                name: b"moving".to_vec(),
                start_frame: 0,
                runtime_duration: 10,
                declared_slot_count: 1,
                slots: vec![SceneAnimationSlot {
                    animation_name: b"move".to_vec(),
                    enabled: 1,
                }],
            }];
            scene.declared_animation_set_count = 1;
        }
        model.animate_sync();
        model.update(EditorAction::SetWorkspace(Workspace::Animate));
        model.update(EditorAction::Animate(AnimateAction::LoadStoredSet(Some(0))));
        model.update(EditorAction::ToggleDocumentEditing);
        model.update(EditorAction::Animate(AnimateAction::SetFrame(5)));

        let transform_change = model.update(EditorAction::EditTransform(
            TransformField::PositionX,
            "44".into(),
        ));
        let transform_notice = model.take_action_notice();
        assert!(transform_change.preview, "{transform_notice:?}");
        assert!(
            model
                .update(EditorAction::SetInspectorToggle(
                    InspectorToggle::CastVisible,
                    false,
                ))
                .preview
        );
        assert!(
            model
                .update(EditorAction::EditInspectorField(
                    InspectorField::MultiplyColor(3),
                    "128".into(),
                ))
                .preview
        );

        let layer = &model.document().unwrap().project.scenes[0].layers[0];
        assert_eq!(layer.transforms[0].spatial().translation[0], 1.0);
        let animation = &layer.animations[0];
        assert!(matches!(
            &channel(animation, 0).keys,
            KeyData::Key8F32(keys)
                if keys.iter().any(|key| key.frame == 5 && key.value == 44.0)
        ));
        assert!(matches!(
            &channel(animation, 10).keys,
            KeyData::Key20I32(keys)
                if keys.iter().any(|key| key.frame == 5 && key.value == 0)
        ));
        assert!(matches!(
            &channel(animation, 21).keys,
            KeyData::Key20F32(keys)
                if keys.iter().any(|key| {
                    key.frame == 5 && (key.value - 128.0 / 255.0).abs() < f32::EPSILON
                })
        ));

        model.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        let animation = &reloaded.project.scenes[0].layers[0].animations[0];
        assert!(matches!(
            &channel(animation, 0).keys,
            KeyData::Key8F32(keys)
                if keys.iter().any(|key| key.frame == 5 && key.value == 44.0)
        ));
        assert!(matches!(
            &channel(animation, 10).keys,
            KeyData::Key20I32(keys)
                if keys.iter().any(|key| key.frame == 5 && key.value == 0)
        ));
        assert!(matches!(
            &channel(animation, 21).keys,
            KeyData::Key20F32(keys)
                if keys.iter().any(|key| {
                    key.frame == 5 && (key.value - 128.0 / 255.0).abs() < f32::EPSILON
                })
        ));
        std::fs::remove_file(path).unwrap();
    }

    fn sample_srd(relative: &str) -> Option<PathBuf> {
        crate::test_support::game_data_path(PathBuf::from("surfboard").join(relative))
    }

    fn temporary_srd_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "srd-editor-model-{label}-{}-{}.srd",
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
    fn zoom_is_bounded_for_toolbar_actions() {
        let mut model = EditorModel::default();
        model.update(EditorAction::ZoomBy(-500));
        assert_eq!(model.zoom_percent(), 25);
        model.update(EditorAction::ZoomBy(500));
        assert_eq!(model.zoom_percent(), 400);
    }

    #[test]
    fn node_type_zero_is_labeled_as_a_null_cast() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        let model = EditorModel::new(Some(path));
        assert_eq!(
            model.selected_node().and_then(|node| node.cast_type()),
            Some(0)
        );
        assert_eq!(model.selected_cast_type_name(), "NullCast");
    }

    #[test]
    fn playback_wraps_inside_selected_range() {
        let mut model = EditorModel {
            frame: 60,
            playing: true,
            ..EditorModel::default()
        };
        model.update(EditorAction::Tick);
        assert_eq!(model.frame(), 0);
    }

    #[test]
    fn common_animation_targets_have_editor_labels() {
        assert_eq!(animation_target_name(0), "Position · X");
        assert_eq!(animation_target_name(21), "Opacity");
        assert_eq!(animation_target_name(99), "Channel 99");
    }

    #[test]
    fn undo_history_discards_the_oldest_snapshot_at_capacity() {
        let mut history = VecDeque::new();
        for value in 0..=MAX_UNDO_HISTORY {
            push_bounded(&mut history, value);
        }
        assert_eq!(history.len(), MAX_UNDO_HISTORY);
        assert_eq!(history.front(), Some(&1));
        assert_eq!(history.back(), Some(&MAX_UNDO_HISTORY));
    }

    #[test]
    fn undo_restores_the_selection_that_matches_its_inspector_snapshot() {
        let path = std::env::temp_dir().join(format!(
            "srd-editor-history-selection-{}-{}.srd",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        let layer = &mut model.document.as_mut().unwrap().project.scenes[0].layers[0];
        let second_node = layer.nodes[0].clone();
        let second_transform = layer.transforms[0];
        layer.nodes.push(second_node);
        layer.transforms.push(second_transform);
        model.update(EditorAction::ToggleDocumentEditing);

        model.update(EditorAction::EditTransform(
            TransformField::PositionX,
            "12".into(),
        ));
        model.update(EditorAction::SelectNode(1));
        assert_eq!(model.selected_node_index(), Some(1));
        assert_eq!(model.inspector().position[0], "1");

        model.update(EditorAction::Undo);
        assert_eq!(model.selected_node_index(), Some(0));
        assert_eq!(model.inspector().position[0], "1");

        model.update(EditorAction::Redo);
        assert_eq!(model.selected_node_index(), Some(1));
        assert_eq!(model.inspector().position[0], "1");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn fractional_rotation_draft_matches_the_serialized_integer() {
        let path = std::env::temp_dir().join(format!(
            "srd-editor-rotation-draft-{}-{}.srd",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);

        model.update(EditorAction::EditTransform(
            TransformField::RotationZ,
            "1.5".into(),
        ));

        assert_eq!(model.inspector().rotation[2], "2");
        let transform = model.document().unwrap().project.scenes[0].layers[0].transforms[0];
        assert_eq!(transform.spatial().rotation[2], 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn runtime_text_host_inputs_round_trip_through_undo_and_redo() {
        let Some(path) = sample_srd("advertise/chu_ui_advertise_00_v10.srd") else {
            return;
        };
        assert!(path.is_file());
        let mut model = EditorModel::new(Some(path));
        select_first_text_cast(&mut model);
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(model.selected_is_text_cast());
        assert_eq!(model.selected_cast_type_name(), "TextCast");
        assert!(!model.is_dirty());

        assert!(model.update(EditorAction::ToggleRuntimeTextInput).preview);

        assert_eq!(model.runtime_text_inputs().len(), 1);
        assert!(
            model
                .update(EditorAction::SetRuntimeTextSubstitution(3, "PLAYER".into()))
                .preview
        );
        assert_eq!(
            model.selected_runtime_text_input().unwrap().substitutions[3],
            b"PLAYER"
        );

        assert!(model.update(EditorAction::Undo).preview);
        assert!(model.selected_runtime_text_input().is_some());
        assert!(model.selected_runtime_text_input().unwrap().substitutions[3].is_empty());
        assert!(model.update(EditorAction::Undo).preview);
        assert!(model.selected_runtime_text_input().is_none());

        assert!(model.update(EditorAction::Redo).preview);
        assert!(model.selected_runtime_text_input().is_some());
        assert!(model.update(EditorAction::Redo).preview);
        assert_eq!(
            model.selected_runtime_text_input().unwrap().substitutions[3],
            b"PLAYER"
        );
        assert!(!model.is_dirty());
    }
    #[test]
    fn text_payload_edit_is_undoable_and_serializable() {
        let path = temporary_srd_path("text-payload");
        std::fs::write(&path, crate::document::tests::minimal_text_srd(b"Original")).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);

        assert_eq!(model.selected_cast_type_name(), "TextCast");
        assert!(
            model
                .update(EditorAction::EditTextContent("Edited text".into()))
                .preview
        );
        assert_eq!(
            model.selected_text_content(),
            Some(b"Edited text".as_slice())
        );
        assert!(model.is_dirty());
        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.selected_text_content(), Some(b"Original".as_slice()));
        assert!(model.update(EditorAction::Redo).preview);
        model.save().unwrap();

        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            reloaded.project.scenes[0].layers[0].image_by_node[0]
                .as_ref()
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .text,
            b"Edited text"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn transform_tools_report_their_active_edit_mode() {
        let path = temporary_srd_path("transform-tools");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));

        for (tool, expected) in [
            (TransformTool::Move, "drag the selected cast"),
            (TransformTool::Rotate, "drag around the selected cast"),
            (TransformTool::Scale, "drag the selected cast"),
        ] {
            assert!(!model.update(EditorAction::SetTransformTool(tool)).preview);
            assert_eq!(model.transform_tool(), tool);
            assert!(model.take_action_notice().unwrap().contains(expected));
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn canvas_gestures_commit_each_transform_as_one_undo_step() {
        let path = temporary_srd_path("canvas-transform-history");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(
            model
                .update(EditorAction::CommitCanvasTransform(
                    CanvasTransformEdit::Move([12.0, 34.0])
                ))
                .preview
        );
        assert_eq!(model.inspector().position[..2], ["12", "34"]);

        assert!(
            model
                .update(EditorAction::CommitCanvasTransform(
                    CanvasTransformEdit::RotateZ(16_384)
                ))
                .preview
        );
        assert_eq!(model.inspector().rotation[2], "16384");

        assert!(
            model
                .update(EditorAction::CommitCanvasTransform(
                    CanvasTransformEdit::Scale([2.0, 3.0])
                ))
                .preview
        );
        assert_eq!(model.inspector().scale[..2], ["2", "3"]);

        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.inspector().scale[..2], ["1", "1"]);
        assert_eq!(model.inspector().rotation[2], "16384");
        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.inspector().rotation[2], "3");
        assert_eq!(model.inspector().position[..2], ["12", "34"]);
        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.inspector().position[..2], ["1", "2"]);
        assert!(!model.is_dirty());

        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn inspector_edits_round_trip_through_the_shared_history_boundary() {
        let path = temporary_srd_path("inspector-history");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        let baseline = model.document().unwrap().project.clone();
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(
            model
                .update(EditorAction::EditInspectorField(
                    InspectorField::LayerName,
                    "renamed_layer".into(),
                ))
                .preview
        );
        let edited = model.document().unwrap().project.clone();
        assert_ne!(edited, baseline);

        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(model.document().unwrap().project, baseline);
        assert!(model.update(EditorAction::Redo).preview);
        assert_eq!(model.document().unwrap().project, edited);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn every_editor_state_control_round_trips_in_action_order() {
        let path = temporary_srd_path("editor-state-history");
        std::fs::write(&path, crate::document::tests::minimal_trs2_srd()).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);
        let selection = PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 0,
        };
        let actions = [
            EditorAction::ToggleLayerVisible(0),
            EditorAction::ToggleLayerLocked(0),
            EditorAction::ToggleLayerSolo(0),
            EditorAction::ToggleCastVisible(0, 0, 0),
            EditorAction::ToggleCastLocked(0, 0, 0),
            EditorAction::ToggleCastSolo(0, 0, 0),
        ];

        for action in actions.iter().cloned() {
            model.update(action);
        }
        assert!(model.hidden_layers[0].contains(&0));
        assert!(model.locked_layers[0].contains(&0));
        assert!(model.solo_layers[0].contains(&0));
        assert!(model.hidden_casts.contains(&selection));
        assert!(model.locked_casts.contains(&selection));
        assert!(model.solo_casts.contains(&selection));

        for _ in &actions {
            assert!(model.update(EditorAction::Undo).preview);
        }
        assert!(model.hidden_layers[0].is_empty());
        assert!(model.locked_layers[0].is_empty());
        assert!(model.solo_layers[0].is_empty());
        assert!(model.hidden_casts.is_empty());
        assert!(model.locked_casts.is_empty());
        assert!(model.solo_casts.is_empty());

        for _ in &actions {
            assert!(model.update(EditorAction::Redo).preview);
        }
        assert!(model.hidden_layers[0].contains(&0));
        assert!(model.locked_layers[0].contains(&0));
        assert!(model.solo_layers[0].contains(&0));
        assert!(model.hidden_casts.contains(&selection));
        assert!(model.locked_casts.contains(&selection));
        assert!(model.solo_casts.contains(&selection));
        assert!(!model.is_dirty());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn repeated_undo_redo_serializes_identically_to_the_direct_edit() {
        let direct_path = temporary_srd_path("direct-determinism");
        let replay_path = temporary_srd_path("history-determinism");
        let source = crate::document::tests::minimal_trs2_srd();
        std::fs::write(&direct_path, &source).unwrap();
        std::fs::write(&replay_path, &source).unwrap();

        let mut direct = EditorModel::new(Some(direct_path.clone()));
        direct.update(EditorAction::ToggleDocumentEditing);
        assert!(
            direct
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "12".into(),
                ))
                .preview
        );
        let edited = direct.document().unwrap().project.clone();
        direct.save().unwrap();
        let expected_bytes = std::fs::read(&direct_path).unwrap();

        let mut replay = EditorModel::new(Some(replay_path.clone()));
        let baseline = replay.document().unwrap().project.clone();
        replay.update(EditorAction::ToggleDocumentEditing);
        assert!(
            replay
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "12".into(),
                ))
                .preview
        );
        for _ in 0..4 {
            assert!(replay.update(EditorAction::Undo).preview);
            assert_eq!(replay.document().unwrap().project, baseline);
            assert!(!replay.is_dirty());
            assert!(replay.update(EditorAction::Redo).preview);
            assert_eq!(replay.document().unwrap().project, edited);
            assert!(replay.is_dirty());
        }
        replay.save().unwrap();
        assert_eq!(std::fs::read(&replay_path).unwrap(), expected_bytes);

        std::fs::remove_file(direct_path).unwrap();
        std::fs::remove_file(replay_path).unwrap();
    }

    #[test]
    fn no_op_undo_and_manual_reversion_never_touch_source_bytes() {
        let path = temporary_srd_path("byte-identical-reversions");
        let source = crate::document::tests::minimal_trs2_srd();
        std::fs::write(&path, &source).unwrap();
        let mut model = EditorModel::new(Some(path.clone()));
        model.update(EditorAction::ToggleDocumentEditing);

        assert!(
            !model
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "1".into(),
                ))
                .preview
        );
        assert!(!model.can_undo());
        assert!(
            model
                .save()
                .unwrap_err()
                .contains("no serializable SRD edits")
        );
        assert_eq!(std::fs::read(&path).unwrap(), source);

        assert!(
            model
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "12".into(),
                ))
                .preview
        );
        assert!(model.update(EditorAction::Undo).preview);
        assert!(!model.is_dirty());
        assert!(
            model
                .save()
                .unwrap_err()
                .contains("no serializable SRD edits")
        );
        assert_eq!(std::fs::read(&path).unwrap(), source);

        assert!(model.update(EditorAction::Redo).preview);
        assert!(
            model
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "1".into(),
                ))
                .preview
        );
        assert!(!model.is_dirty());
        assert!(
            model
                .save()
                .unwrap_err()
                .contains("no serializable SRD edits")
        );
        assert_eq!(std::fs::read(&path).unwrap(), source);

        std::fs::remove_file(path).unwrap();
    }
}
