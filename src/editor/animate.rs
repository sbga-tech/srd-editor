//! Animate workspace state: the working assignment, explicit Edit mode,
//! structural ANMS/ANIM/MOT/TRK/KEY authoring, direct/inherited motion
//! classification, dope-sheet row generation, and every-frame playback.
//!
//! The workspace never mutates the document from a selection. Selection only
//! auditions: it rebuilds the owned working assignment and the preview request.
//! Document mutation happens exclusively inside explicit Edit mode, which
//! records undo and carries an entry snapshot for Revert.

use std::collections::{BTreeMap, BTreeSet};

use crate::animation::{AnimationDefinition, Motion, Track};
use crate::document::display_srd_name;
use crate::renderer::{PreviewCastSelection, PreviewHighlightKind, PreviewHighlightRequest};
use crate::scene::{AnimationSetDefinition, Layer, Project, Scene, SceneAnimationSlot};

use super::channels::{self, ChannelValue, KeySemantics, KeyValue};
use super::model::{EditorModel, ModelChange, set_masked_flag};

/// Which workspace owns the window. Both live in the same OS window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Design,
    Animate,
}

impl Workspace {
    /// Startup selection for the screenshot harness. Anything other than
    /// `animate` (case-insensitive) keeps the Design workbench.
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value {
            Some(value) if value.trim().eq_ignore_ascii_case("animate") => Self::Animate,
            _ => Self::Design,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Design => "Design",
            Self::Animate => "Animate",
        }
    }
}

/// Explicit preview of one selected layer animation. It never rewrites the
/// working assignment; it only filters the preview submission and authoring view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationPreview {
    pub layer: usize,
    pub animation_name: Vec<u8>,
}

/// A destructive structural delete waiting for confirmation because it would
/// otherwise leave dangling ANMS slot names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDelete {
    pub layer: usize,
    /// Human-readable name presented by the confirmation strip.
    pub animation_name: String,
    /// Exact serialized name used to find the animation again after structural
    /// edits may have shifted its index.
    animation_identity: Vec<u8>,
    /// Stored ANMS in this scene whose slots reference the animation.
    pub referencing_sets: usize,
}

/// What a dope-sheet row represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DopeRowKind {
    Layer,
    Cast,
    Channel,
}

/// How a CAST receives motion from the resolved animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionSource {
    /// A MOT targets this CAST and carries at least one track.
    Direct,
    /// An ancestor is directly targeted; this CAST follows through the tree.
    Inherited,
    /// No motion reaches this CAST.
    None,
}

/// A concrete track in one layer's authored animation.
///
/// Dope-sheet input carries this address rather than depending on the current
/// inspector selection, which may be viewing a different layer or ANIM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackAddress {
    pub layer: usize,
    pub animation: usize,
    pub motion: usize,
    pub track: usize,
}

impl TrackAddress {
    pub const fn new(layer: usize, animation: usize, motion: usize, track: usize) -> Self {
        Self {
            layer,
            animation,
            motion,
            track,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DopeRow {
    pub kind: DopeRowKind,
    pub depth: u8,
    /// Compact CAST ancestry for CAST rows; channel rows use their channel name.
    pub label: String,
    /// Stable scene-layer identity for actions and cross-pane selection.
    pub layer: usize,
    /// Compact display token for the originating layer, such as `L0`.
    pub layer_badge: String,
    pub node: Option<usize>,
    /// Authored track for channel rows, including its layer and ANIM identity.
    pub track: Option<TrackAddress>,
    pub target: Option<u16>,
    pub source: MotionSource,
    pub has_children: bool,
    pub collapsed: bool,
    /// Half-open range into [`DopeSheet::keys`].
    pub keys: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DopeKey {
    pub row: u32,
    pub frame: i32,
    pub index: u32,
    pub semantics: KeySemantics,
    /// Outside the runtime range, so the runtime never reaches it.
    pub out_of_range: bool,
}

/// Batched dope-sheet geometry. Keys live in one flat array grouped by row so
/// the canvas can draw a visible window without touching every key.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DopeSheet {
    pub rows: Vec<DopeRow>,
    pub keys: Vec<DopeKey>,
    /// Row indices that survive collapse, in draw order.
    pub visible: Vec<u32>,
    pub key_min: i32,
    pub key_max: i32,
    pub out_of_range: usize,
}

impl DopeSheet {
    pub fn row_keys(&self, row: usize) -> &[DopeKey] {
        let Some(entry) = self.rows.get(row) else {
            return &[];
        };
        &self.keys[entry.keys.0 as usize..entry.keys.1 as usize]
    }
}

/// One layer line of the assignment table.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentRow {
    pub layer: usize,
    pub name: String,
    pub enabled: bool,
    pub manual_hidden: bool,
    pub animation: Option<usize>,
    pub animation_label: String,
    pub status: SlotStatus,
    pub direct: usize,
    pub inherited: usize,
    pub keys: usize,
    pub duration: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotStatus {
    /// Enabled with no animation name: the layer renders its serialized pose.
    EnabledBasePose,
    Animated,
    Disabled,
    /// Slot names an animation this layer does not define.
    Missing,
}

impl SlotStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::EnabledBasePose => "Enabled · Base pose",
            Self::Animated => "Animated",
            Self::Disabled => "Disabled",
            Self::Missing => "Missing animation",
        }
    }
}

/// Text drafts for the numeric and name editors, so a partially typed value
/// never destroys the stored one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnimateDrafts {
    pub set_name: String,
    pub set_start: String,
    pub set_duration: String,
    pub animation_name: String,
    pub animation_duration: String,
    pub key_frame: String,
    pub key_value: String,
    pub key_rgba: [String; 4],
    pub slope_in: String,
    pub slope_out: String,
    pub track_range_start: String,
    pub track_range_end: String,
    pub work_start: String,
    pub work_end: String,
}

/// Undoable slice of the workspace. The edit-entry snapshot is deliberately
/// excluded: it is the Revert baseline, not an undo step.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimateHistory {
    scene: usize,
    assignment: AnimationSetDefinition,
    source_set: Option<usize>,
    preview: Option<AnimationPreview>,
    selected_layer: usize,
    selected_animation: Option<usize>,
    selected_track: Option<(usize, usize)>,
    selected_key: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowKey {
    layer: usize,
    node: Option<usize>,
}

pub const DOPE_ROW_HEIGHT: f32 = 22.0;
/// Track-area width guaranteed by the Dope Sheet layout. Pan bounds use this
/// minimum so the range end cannot be pushed beyond every valid viewport.
pub(super) const DOPE_MIN_TIMELINE_WIDTH: f32 = 180.0;
const DEFAULT_DURATION: i32 = 60;

#[derive(Debug, Clone)]
pub struct AnimateState {
    /// Stored ANMS the working assignment was cloned from, when any.
    pub(super) source_set: Option<usize>,
    /// Owned dense assignment. Slots always match the scene's layer count.
    pub(super) assignment: AnimationSetDefinition,
    /// Assignment with the explicit animation preview filter applied.
    pub(super) effective: AnimationSetDefinition,
    pub(super) preview: Option<AnimationPreview>,
    pub(super) scene: usize,
    pub(super) selected_layer: usize,
    pub(super) selected_animation: Option<usize>,

    pub(super) selected_track: Option<(usize, usize)>,
    pub(super) selected_key: Option<usize>,
    pub(super) frame: i32,
    pub(super) playing: bool,

    pub(super) work_area: Option<(i32, i32)>,
    pub(super) editing: bool,
    pub(super) pending_delete: Option<PendingDelete>,
    pub(super) collapsed: Vec<RowKey>,
    pub(super) maximized: bool,
    /// Horizontal dope-sheet scale in pixels per frame.
    pub(super) zoom: f32,
    /// First frame drawn at the left edge of the dope sheet.
    pub(super) pan: f32,
    pub(super) scroll: f32,
    pub(super) sheet: DopeSheet,
    pub(super) assignment_rows: Vec<AssignmentRow>,
    pub(super) highlights: Vec<PreviewHighlightRequest>,
    pub(super) drafts: AnimateDrafts,
}

impl Default for AnimateState {
    fn default() -> Self {
        Self {
            source_set: None,
            assignment: empty_assignment(0),
            effective: empty_assignment(0),
            preview: None,
            scene: 0,
            selected_layer: 0,
            selected_animation: None,

            selected_track: None,
            selected_key: None,
            frame: 0,
            playing: false,

            work_area: None,
            editing: false,
            pending_delete: None,
            collapsed: Vec::new(),
            maximized: false,
            zoom: 6.0,
            pan: 0.0,
            scroll: 0.0,
            sheet: DopeSheet::default(),
            assignment_rows: Vec::new(),
            highlights: Vec::new(),
            drafts: AnimateDrafts::default(),
        }
    }
}

impl AnimateState {
    pub fn assignment(&self) -> &AnimationSetDefinition {
        &self.assignment
    }

    pub fn effective_assignment(&self) -> &AnimationSetDefinition {
        &self.effective
    }

    pub fn source_set(&self) -> Option<usize> {
        self.source_set
    }

    pub fn preview(&self) -> Option<&AnimationPreview> {
        self.preview.as_ref()
    }

    pub(super) fn end_preview(&mut self) {
        self.preview = None;
    }

    pub fn selected_layer(&self) -> usize {
        self.selected_layer
    }

    pub fn selected_animation(&self) -> Option<usize> {
        self.selected_animation
    }

    pub fn selected_track(&self) -> Option<(usize, usize)> {
        self.selected_track
    }

    pub fn selected_key(&self) -> Option<usize> {
        self.selected_key
    }

    pub fn frame(&self) -> i32 {
        self.frame
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn editing(&self) -> bool {
        self.editing
    }

    pub fn pending_delete(&self) -> Option<&PendingDelete> {
        self.pending_delete.as_ref()
    }

    pub fn maximized(&self) -> bool {
        self.maximized
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn pan(&self) -> f32 {
        self.pan
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub fn sheet(&self) -> &DopeSheet {
        &self.sheet
    }

    pub fn assignment_rows(&self) -> &[AssignmentRow] {
        &self.assignment_rows
    }

    pub fn highlights(&self) -> &[PreviewHighlightRequest] {
        &self.highlights
    }

    pub fn drafts(&self) -> &AnimateDrafts {
        &self.drafts
    }

    /// Whether the working assignment differs from its stored source. A stored
    /// set that was merely loaded is never dirty.
    pub fn assignment_is_scratch(&self, scene: Option<&Scene>) -> bool {
        match (self.source_set, scene) {
            (Some(index), Some(scene)) => scene
                .animation_sets
                .get(index)
                .is_none_or(|stored| *stored != self.assignment),
            (Some(_), None) => true,
            (None, _) => true,
        }
    }

    /// Playback range. The work area is an editable override and never dirties
    /// the document.
    pub fn work_range(&self) -> (i32, i32) {
        self.work_area.unwrap_or((0, 0))
    }

    pub fn has_work_area(&self) -> bool {
        self.work_area.is_some()
    }

    pub fn row_collapsed(&self, layer: usize, node: Option<usize>) -> bool {
        self.collapsed.contains(&RowKey { layer, node })
    }

    fn toggle_collapsed(&mut self, key: RowKey) {
        if let Some(position) = self.collapsed.iter().position(|entry| *entry == key) {
            self.collapsed.remove(position);
        } else {
            self.collapsed.push(key);
        }
    }

    fn history(&self) -> AnimateHistory {
        AnimateHistory {
            scene: self.scene,
            assignment: self.assignment.clone(),
            source_set: self.source_set,
            preview: self.preview.clone(),
            selected_layer: self.selected_layer,
            selected_animation: self.selected_animation,
            selected_track: self.selected_track,
            selected_key: self.selected_key,
        }
    }

    fn restore(&mut self, history: AnimateHistory) {
        self.scene = history.scene;
        self.assignment = history.assignment;
        self.source_set = history.source_set;
        self.preview = history.preview;
        self.selected_layer = history.selected_layer;
        self.selected_animation = history.selected_animation;
        self.selected_track = history.selected_track;
        self.selected_key = history.selected_key;
    }
}

/// Every Animate workspace intent. Selection and preview are independent; the
/// mutating variants are refused outside explicit Edit mode.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimateAction {
    LoadStoredSet(Option<usize>),
    SelectLayer(usize),
    SelectAnimation(usize, usize),
    ToggleAnimationPreview,
    SelectTrack(TrackAddress),
    SelectKey(TrackAddress, usize),
    ClearSelectedKey,
    ConfirmDelete,
    CancelDelete,
    // ANMS authoring.
    CreateSet,
    DuplicateSet,
    DeleteSet,
    SetName(String),
    SetStartFrame(String),
    SetDuration(String),
    ToggleSlotEnabled(usize),
    AssignSlotAnimation(usize, Option<usize>),
    // ANIM authoring.
    CreateAnimation,
    DuplicateAnimation,
    DeleteAnimation,
    SetAnimationName(String),
    SetAnimationDuration(String),
    SetAnimationLoop(bool),
    // Legacy raw MOT/TRK/KEY editing remains while existing UI consumers
    // migrate to property-based authoring below.
    AddMotion(usize),
    DeleteMotion(usize),
    SetMotionTarget(usize, usize),
    AddTrack(usize, u16),
    DeleteTrack(usize, usize),
    SetTrackChannel(usize, usize, u16),
    SetTrackFormat(usize, usize, u32),
    SetTrackWrap(usize, usize, bool),
    SetTrackRangeStart(usize, usize, String),
    SetTrackRangeEnd(usize, usize, String),
    AddKey(TrackAddress, i32),
    DeleteKey(TrackAddress, usize),
    MoveKey(TrackAddress, usize, i32),
    SetKeyFrame(String),
    SetKeyValue(String),
    SetKeyByte(usize, String),
    SetKeySemantics(KeySemantics),
    SetKeySlope(bool, String),
    // Property-oriented authoring. A property owns one channel track on one
    // concrete CAST; its actions never depend on the current inspector focus.
    AddProperty {
        layer: usize,
        animation: usize,
        node: usize,
        target: u16,
    },
    ToggleTrackAnimation(TrackAddress),
    ToggleKeyAtFrame(TrackAddress, i32),
    SetTrackValueAtFrame(TrackAddress, i32, String),
    // Playback and range.
    SetFrame(i32),
    StepFrame(i32),
    JumpToPreviousTrackKey(TrackAddress),
    JumpToNextTrackKey(TrackAddress),
    TogglePlaying,
    JumpToStart,
    JumpToEnd,
    FrameCompleted,
    SetWorkStart(String),
    SetWorkEnd(String),
    FitAllKeys,
    ClearWorkArea,
    // Dope sheet presentation.
    ToggleRowCollapsed(usize),
    ToggleMaximized,
    ZoomBy(f32),
    PanBy(f32),
    ScrollBy(f32, f32),
}

impl AnimateAction {
    /// Whether the action can write to the SRD project. This exhaustive match
    /// forces every new intent to declare its Edit-mode and history contract.
    pub fn mutates_document(&self) -> bool {
        match self {
            Self::ConfirmDelete
            | Self::CreateSet
            | Self::DuplicateSet
            | Self::DeleteSet
            | Self::SetName(_)
            | Self::SetStartFrame(_)
            | Self::SetDuration(_)
            | Self::ToggleSlotEnabled(_)
            | Self::AssignSlotAnimation(..)
            | Self::CreateAnimation
            | Self::DuplicateAnimation
            | Self::DeleteAnimation
            | Self::SetAnimationName(_)
            | Self::SetAnimationDuration(_)
            | Self::SetAnimationLoop(_)
            | Self::AddMotion(_)
            | Self::DeleteMotion(_)
            | Self::SetMotionTarget(..)
            | Self::AddTrack(..)
            | Self::DeleteTrack(..)
            | Self::SetTrackChannel(..)
            | Self::SetTrackFormat(..)
            | Self::SetTrackWrap(..)
            | Self::SetTrackRangeStart(..)
            | Self::SetTrackRangeEnd(..)
            | Self::AddKey(..)
            | Self::DeleteKey(..)
            | Self::MoveKey(..)
            | Self::SetKeyFrame(_)
            | Self::SetKeyValue(_)
            | Self::SetKeyByte(..)
            | Self::SetKeySemantics(_)
            | Self::SetKeySlope(..)
            | Self::AddProperty { .. }
            | Self::ToggleTrackAnimation(..)
            | Self::ToggleKeyAtFrame(..)
            | Self::SetTrackValueAtFrame(..) => true,
            Self::LoadStoredSet(_)
            | Self::SelectLayer(_)
            | Self::SelectAnimation(..)
            | Self::ToggleAnimationPreview
            | Self::SelectTrack(_)
            | Self::SelectKey(..)
            | Self::ClearSelectedKey
            | Self::CancelDelete
            | Self::SetFrame(_)
            | Self::StepFrame(_)
            | Self::JumpToPreviousTrackKey(_)
            | Self::JumpToNextTrackKey(_)
            | Self::TogglePlaying
            | Self::JumpToStart
            | Self::JumpToEnd
            | Self::FrameCompleted
            | Self::SetWorkStart(_)
            | Self::SetWorkEnd(_)
            | Self::FitAllKeys
            | Self::ClearWorkArea
            | Self::ToggleRowCollapsed(_)
            | Self::ToggleMaximized
            | Self::ZoomBy(_)
            | Self::PanBy(_)
            | Self::ScrollBy(..) => false,
        }
    }
}

fn empty_assignment(layer_count: usize) -> AnimationSetDefinition {
    AnimationSetDefinition {
        name: Vec::new(),
        start_frame: 0,
        runtime_duration: 0,
        declared_slot_count: layer_count as i32,
        slots: (0..layer_count)
            .map(|_| SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: 0,
            })
            .collect(),
    }
}

/// Dense scratch assignment: one slot per layer, active from the authored
/// layer state, with no animation names assigned yet.
fn scratch_assignment(scene: &Scene, duration: i32) -> AnimationSetDefinition {
    AnimationSetDefinition {
        name: Vec::new(),
        start_frame: 0,
        runtime_duration: duration,
        declared_slot_count: scene.layers.len() as i32,
        slots: scene
            .layers
            .iter()
            .map(|layer| SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: i32::from(layer.active()),
            })
            .collect(),
    }
}

/// Grows or truncates an assignment so slot `n` always controls layer `n`.
fn densify(assignment: &mut AnimationSetDefinition, layer_count: usize) {
    assignment.slots.resize(
        layer_count,
        SceneAnimationSlot {
            animation_name: Vec::new(),
            enabled: 0,
        },
    );
    assignment.declared_slot_count = layer_count as i32;
}

fn unique_name(existing: impl Iterator<Item = String>, base: &str) -> Vec<u8> {
    let taken: BTreeSet<String> = existing.collect();
    if !taken.contains(base) {
        return base.as_bytes().to_vec();
    }
    let mut suffix = 2usize;
    loop {
        let candidate = format!("{base} {suffix}");
        if !taken.contains(&candidate) {
            return candidate.into_bytes();
        }
        suffix += 1;
    }
}

/// Direct and inherited CAST classification for one resolved animation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MotionTargets {
    pub direct: BTreeSet<usize>,
    pub inherited: BTreeSet<usize>,
}

/// Classifies CASTs for `animation`. `target = -1` zero-track placeholders and
/// motions without tracks are ignored: they submit no channel.
pub fn motion_targets(layer: &Layer, animation: &AnimationDefinition) -> MotionTargets {
    let node_count = layer.nodes.len();
    let mut targets = MotionTargets::default();
    for motion in &animation.motions {
        if motion.target < 0 || motion.tracks.is_empty() {
            continue;
        }
        let node = motion.target as usize;
        if node < node_count {
            targets.direct.insert(node);
        }
    }
    if targets.direct.is_empty() {
        return targets;
    }
    let Ok(hierarchy) = layer.build_hierarchy() else {
        return targets;
    };
    let mut stack: Vec<usize> = targets.direct.iter().copied().collect();
    while let Some(node) = stack.pop() {
        for &child in &hierarchy.children[node] {
            if targets.direct.contains(&child) || !targets.inherited.insert(child) {
                continue;
            }
            stack.push(child);
        }
    }
    targets
}

/// Whether `node` names a runtime CAST with a supported NODE type. A missing
/// type is the serialized form of the default NullCast, not a padding record.
pub fn is_authorable_motion_target(layer: &Layer, node: usize) -> bool {
    layer
        .nodes
        .get(node)
        .is_some_and(|record| record.cast_type().is_none_or(|cast_type| cast_type <= 4))
}

/// Whether the runtime consumer for `channel` exists on this concrete CAST.
/// Missing NODE type defaults to NullCast (`0`), matching cast construction.
/// This is shared by the mutation guard and picker so both accept the same target.
pub fn is_authorable_track_target(layer: &Layer, node: usize, channel: u16) -> bool {
    let Some(spec) = channels::channel(channel) else {
        return false;
    };
    let Some(record) = layer.nodes.get(node) else {
        return false;
    };
    let cast_type = record.cast_type().unwrap_or(0);
    match spec.scope {
        channels::ChannelScope::Common => matches!(cast_type, 0..=4),
        channels::ChannelScope::Image => matches!(cast_type, 1 | 2 | 4),
        channels::ChannelScope::Reference => cast_type == 3,
    }
}

/// Nodes in hierarchy order, so cast rows read like the composition tree.
fn hierarchy_order(layer: &Layer) -> Vec<(usize, u8)> {
    let Ok(hierarchy) = layer.build_hierarchy() else {
        return (0..layer.nodes.len()).map(|node| (node, 0)).collect();
    };
    let mut order = Vec::with_capacity(layer.nodes.len());
    let mut stack: Vec<(usize, u8)> = hierarchy
        .roots
        .iter()
        .rev()
        .map(|&node| (node, 0u8))
        .collect();
    while let Some((node, depth)) = stack.pop() {
        order.push((node, depth));
        for &child in hierarchy.children[node].iter().rev() {
            stack.push((child, depth.saturating_add(1)));
        }
    }
    order
}

/// A compact root-to-CAST path, so a flat dope sheet retains hierarchy context
/// without inserting non-animatable hierarchy rows.
fn compact_cast_paths(layer: &Layer) -> Vec<String> {
    let labels: Vec<String> = (0..layer.nodes.len())
        .map(|node| cast_label(layer, node))
        .collect();
    let Ok(hierarchy) = layer.build_hierarchy() else {
        return labels;
    };
    (0..labels.len())
        .map(|node| {
            let mut lineage = Vec::new();
            let mut seen = BTreeSet::new();
            let mut current = Some(node);
            while let Some(index) = current.filter(|index| seen.insert(*index)) {
                lineage.push(labels[index].as_str());
                current = hierarchy.parents.get(index).copied().flatten();
            }
            lineage.reverse();
            lineage.join(" / ")
        })
        .collect()
}

fn resolved_animation<'a>(
    layer: &'a Layer,
    slot: &SceneAnimationSlot,
) -> Option<&'a AnimationDefinition> {
    if slot.animation_name.is_empty() {
        return None;
    }
    layer
        .find_animation(&slot.animation_name)
        .map(|(_, animation)| animation)
}

fn track_key_count(track: &Track) -> usize {
    channels::key_count(&track.keys)
}

/// The in-memory key storage must agree with the parser-selected format before
/// property actions can edit it. This keeps a malformed imported TRK from
/// becoming an apparently valid but unserializable authoring result.
fn track_storage_is_parser_legal(track: &Track) -> bool {
    track.key_count as usize == track_key_count(track)
        && matches!(
            (track.format & 3, track.format & 0x70, &track.keys),
            (0 | 1, 0x10, crate::animation::KeyData::Key8F32(_))
                | (0 | 1, 0x40, crate::animation::KeyData::Key8I32(_))
                | (0 | 1, 0x50, crate::animation::KeyData::Key8Bytes4(_))
                | (3, 0x10, crate::animation::KeyData::Key20F32(_))
                | (3, 0x20 | 0x40, crate::animation::KeyData::Key20I32(_))
        )
}

/// A parser-valid zero-key TRK has no `KEY ` child and is represented as
/// `Unsupported` on load. It becomes authorable by materializing its legal
/// empty key storage before the first key is inserted.
fn track_storage_is_authorable(track: &Track) -> bool {
    track_storage_is_parser_legal(track)
        || (track.key_count == 0
            && matches!(&track.keys, crate::animation::KeyData::Unsupported)
            && channels::empty_keys(track.format).is_some())
}

fn materialize_empty_track_keys(track: &mut Track) {
    if track.key_count == 0 && matches!(&track.keys, crate::animation::KeyData::Unsupported) {
        track.keys = channels::empty_keys(track.format)
            .expect("authorable zero-key track has a parser-legal format");
    }
}

impl EditorModel {
    pub fn workspace(&self) -> Workspace {
        self.workspace
    }

    pub fn animate(&self) -> &AnimateState {
        &self.animate
    }

    /// Assignment submitted with the preview request for the current workspace.
    pub fn preview_assignment(&self) -> Option<&AnimationSetDefinition> {
        match self.workspace {
            Workspace::Design => self.selected_animation_set(),
            Workspace::Animate => {
                let scene = self.selected_scene()?;
                (!self.animate.effective.slots.is_empty()
                    && self.animate.scene == self.selected_scene_index()
                    && scene.layers.len() == self.animate.effective.slots.len())
                .then_some(&self.animate.effective)
            }
        }
    }

    pub fn preview_frame(&self) -> i32 {
        match self.workspace {
            Workspace::Design => self.frame(),
            Workspace::Animate => self.animate.frame,
        }
    }

    pub fn preview_highlights(&self) -> &[PreviewHighlightRequest] {
        match self.workspace {
            Workspace::Design => &[],
            Workspace::Animate => &self.animate.highlights,
        }
    }

    pub fn animate_editing(&self) -> bool {
        self.animate.editing
    }
    /// Runtime range implied by the assignment: explicit animation preview,
    /// stored ANMS duration, or the longest assigned animation.
    pub fn animate_runtime_range(&self) -> (i32, i32) {
        let start = self.animate.assignment.start_frame;
        let Some(scene) = self.selected_scene() else {
            return (start, start.max(DEFAULT_DURATION));
        };
        if let Some(preview) = &self.animate.preview {
            let duration = scene
                .layers
                .get(preview.layer)
                .and_then(|layer| layer.find_animation(&preview.animation_name))
                .map(|(_, animation)| animation.duration)
                .unwrap_or(DEFAULT_DURATION);
            return (start, start.max(duration));
        }
        if self.animate.source_set.is_some() && self.animate.assignment.runtime_duration > 0 {
            return (start, start.max(self.animate.assignment.runtime_duration));
        }
        let assigned = self
            .animate
            .assignment
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_enabled())
            .filter_map(|(index, slot)| {
                let layer = scene.layers.get(index)?;
                resolved_animation(layer, slot).map(|animation| animation.duration)
            })
            .max()
            .unwrap_or(0);
        let duration = if assigned > 0 {
            assigned
        } else if self.animate.assignment.runtime_duration > 0 {
            self.animate.assignment.runtime_duration
        } else {
            DEFAULT_DURATION
        };
        (start, start.max(duration))
    }

    /// Playback range: the editable work area when set, else the runtime range.
    pub fn animate_play_range(&self) -> (i32, i32) {
        self.animate
            .work_area
            .map(|(start, end)| (start, end.max(start)))
            .unwrap_or_else(|| self.animate_runtime_range())
    }

    pub fn animate_layer_animations(&self) -> Vec<(usize, String, i32, usize)> {
        let Some(layer) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(self.animate.selected_layer))
        else {
            return Vec::new();
        };
        layer
            .animations
            .iter()
            .enumerate()
            .map(|(index, animation)| {
                let keys = animation
                    .motions
                    .iter()
                    .flat_map(|motion| &motion.tracks)
                    .map(track_key_count)
                    .sum();
                (
                    index,
                    nonempty(&animation.name, format!("ANIM {index}")),
                    animation.duration,
                    keys,
                )
            })
            .collect()
    }

    /// Animation currently selected for authoring on the selected layer. The
    /// selection is independent of the working ANMS slot and Solo preview.
    pub fn animate_active_animation(&self) -> Option<(usize, &AnimationDefinition)> {
        let layer = self
            .selected_scene()?
            .layers
            .get(self.animate.selected_layer)?;
        let index = self.animate.selected_animation?;
        layer
            .animations
            .get(index)
            .map(|animation| (index, animation))
    }

    pub(super) fn animate_sync(&mut self) {
        let scene_index = self.selected_scene_index();
        let layer_count = self.selected_scene().map_or(0, |scene| scene.layers.len());
        if self.animate.scene != scene_index
            || (self.animate.assignment.slots.is_empty() && layer_count > 0)
        {
            self.animate.scene = scene_index;
            self.animate.source_set = None;
            self.animate.preview = None;
            self.animate.selected_layer = 0;
            self.animate.selected_animation = None;
            self.animate.selected_track = None;
            self.animate.selected_key = None;
            self.animate.work_area = None;
            self.animate.assignment = self
                .selected_scene()
                .map_or_else(|| empty_assignment(0), |scene| scratch_assignment(scene, 0));
        }
        densify(&mut self.animate.assignment, layer_count);
        if self.animate.selected_layer >= layer_count.max(1) {
            self.animate.selected_layer = 0;
        }
        self.animate_clamp_selection();
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_clamp_frame();
        self.animate_sync_drafts();
        self.sync_inspector();
    }

    fn animate_clamp_selection(&mut self) {
        let selected_layer = self.animate.selected_layer;
        let slot_name = self
            .animate
            .assignment
            .slots
            .get(selected_layer)
            .map(|slot| slot.animation_name.clone())
            .unwrap_or_default();
        let Some(layer) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(selected_layer))
        else {
            self.animate.selected_animation = None;
            self.animate.selected_track = None;
            self.animate.selected_key = None;
            return;
        };
        let mut animation_index = self
            .animate
            .selected_animation
            .filter(|index| *index < layer.animations.len());
        if animation_index.is_none() && !slot_name.is_empty() {
            animation_index = layer.find_animation(&slot_name).map(|(index, _)| index);
        }
        let bounds = animation_index
            .and_then(|index| layer.animations.get(index))
            .map(|animation| {
                animation
                    .motions
                    .iter()
                    .map(|motion| {
                        motion
                            .tracks
                            .iter()
                            .map(|track| channels::key_count(&track.keys))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            });
        self.animate.selected_animation = animation_index;
        let Some(bounds) = bounds else {
            self.animate.selected_track = None;
            self.animate.selected_key = None;
            return;
        };
        let track_valid = self
            .animate
            .selected_track
            .is_some_and(|(motion, track)| bounds.get(motion).is_some_and(|m| track < m.len()));
        if !track_valid {
            self.animate.selected_track = None;
            self.animate.selected_key = None;
            return;
        }
        if let (Some((motion, track)), Some(key)) =
            (self.animate.selected_track, self.animate.selected_key)
            && bounds[motion][track] <= key
        {
            self.animate.selected_key = None;
        }
    }

    fn animate_rebuild_effective(&mut self) {
        let mut effective = self.animate.assignment.clone();
        if let Some(preview) = &self.animate.preview {
            for (index, slot) in effective.slots.iter_mut().enumerate() {
                if index == preview.layer {
                    slot.enabled = 1;
                    slot.animation_name = preview.animation_name.clone();
                } else {
                    slot.enabled = 0;
                }
            }
        }
        self.animate.effective = effective;
    }

    fn animate_rebuild_rows(&mut self) {
        let (runtime_start, runtime_end) = self.animate_runtime_range();
        let Some(scene) = self.selected_scene() else {
            self.animate.sheet = DopeSheet::default();
            self.animate.assignment_rows.clear();
            self.animate.highlights.clear();
            self.animate_clamp_pan();
            return;
        };
        // Assignment rows always describe the working ANMS. Explicit preview
        // narrows the dope sheet to the selected animation on its own layer.
        let assignment = self.animate.assignment.clone();
        let preview = self.animate.preview.clone();
        let hidden = self.hidden_layers().clone();
        let selected_layer = self.animate.selected_layer;
        let selected_animation = self.animate.selected_animation;

        let mut sheet = DopeSheet::default();
        let mut assignment_rows = Vec::with_capacity(scene.layers.len());
        let mut highlights = Vec::new();
        let mut key_min = i32::MAX;
        let mut key_max = i32::MIN;
        let mut out_of_range = 0usize;

        for (layer_index, layer) in scene.layers.iter().enumerate() {
            let slot = assignment
                .slots
                .get(layer_index)
                .cloned()
                .unwrap_or(SceneAnimationSlot {
                    animation_name: Vec::new(),
                    enabled: 0,
                });
            let assigned_animation = (!slot.animation_name.is_empty())
                .then(|| layer.find_animation(&slot.animation_name))
                .flatten();
            let preview_animation = preview
                .as_ref()
                .filter(|preview| preview.layer == layer_index)
                .and_then(|preview| layer.find_animation(&preview.animation_name));
            let dope_animation = if preview.is_some() {
                preview_animation
            } else {
                assigned_animation
            };
            let authored = layer_index == selected_layer;
            let authored_animation = authored
                .then(|| {
                    selected_animation.and_then(|index| {
                        layer
                            .animations
                            .get(index)
                            .map(|animation| (index, animation))
                    })
                })
                .flatten();
            // Selection is an authoring focus only. Assignment controls alone
            // decide which resolved animations contribute dope rows.
            let highlight_animation = if authored {
                authored_animation.or(assigned_animation)
            } else {
                assigned_animation
            };
            let targets = highlight_animation
                .map(|(_, animation)| motion_targets(layer, animation))
                .unwrap_or_default();
            let key_total: usize = assigned_animation
                .map(|(_, animation)| {
                    animation
                        .motions
                        .iter()
                        .flat_map(|motion| &motion.tracks)
                        .map(track_key_count)
                        .sum()
                })
                .unwrap_or(0);

            let status = if !slot.is_enabled() {
                SlotStatus::Disabled
            } else if slot.animation_name.is_empty() {
                SlotStatus::EnabledBasePose
            } else if assigned_animation.is_some() {
                SlotStatus::Animated
            } else {
                SlotStatus::Missing
            };
            assignment_rows.push(AssignmentRow {
                layer: layer_index,
                name: nonempty(&layer.name, format!("Layer {layer_index}")),
                enabled: slot.is_enabled(),
                manual_hidden: hidden.contains(&layer_index),
                animation: assigned_animation.map(|(index, _)| index),
                animation_label: if slot.animation_name.is_empty() {
                    "—".into()
                } else {
                    display_srd_name(&slot.animation_name)
                },
                status,
                direct: targets.direct.len(),
                inherited: targets.inherited.len(),
                keys: key_total,
                duration: assigned_animation.map_or(0, |(_, animation)| animation.duration),
            });

            if authored {
                for &node in &targets.direct {
                    highlights.push(PreviewHighlightRequest {
                        selection: PreviewCastSelection {
                            scene_index: self.animate.scene,
                            layer_index,
                            node_index: node,
                        },
                        kind: PreviewHighlightKind::Direct,
                    });
                }
                for &node in &targets.inherited {
                    highlights.push(PreviewHighlightRequest {
                        selection: PreviewCastSelection {
                            scene_index: self.animate.scene,
                            layer_index,
                            node_index: node,
                        },
                        kind: PreviewHighlightKind::Inherited,
                    });
                }
            }

            // Normal authoring follows every enabled assignment slot. Explicit
            // preview instead shows only the selected animation and layer.
            if preview
                .as_ref()
                .is_some_and(|preview| preview.layer != layer_index)
            {
                continue;
            }
            if preview.is_none() && !slot.is_enabled() {
                continue;
            }
            let Some((animation_index, animation)) = dope_animation else {
                continue;
            };
            let mut tracks_by_node: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
            for (motion_index, motion) in animation.motions.iter().enumerate() {
                let Ok(node) = usize::try_from(motion.target) else {
                    continue;
                };
                if node >= layer.nodes.len() {
                    continue;
                }
                for (track_index, track) in motion.tracks.iter().enumerate() {
                    // The catalog deliberately excludes channel 18 and all
                    // unimplemented targets, so neither can leak into rows.
                    if channels::channel(track.target).is_some() {
                        tracks_by_node
                            .entry(node)
                            .or_default()
                            .push((motion_index, track_index));
                    }
                }
            }

            let path_labels = compact_cast_paths(layer);
            let layer_badge = format!("L{layer_index}");
            let mut ordered_nodes = hierarchy_order(layer);
            let mut ordered = ordered_nodes
                .iter()
                .map(|(node, _)| *node)
                .collect::<BTreeSet<_>>();
            for &node in tracks_by_node.keys() {
                if ordered.insert(node) {
                    ordered_nodes.push((node, 0));
                }
            }
            for (node, _) in ordered_nodes {
                let Some(node_tracks) = tracks_by_node.get(&node) else {
                    continue;
                };
                let node_collapsed = self.animate.row_collapsed(layer_index, Some(node));
                let cast_row = sheet.rows.len() as u32;
                sheet.rows.push(DopeRow {
                    kind: DopeRowKind::Cast,
                    depth: 0,
                    label: path_labels
                        .get(node)
                        .cloned()
                        .unwrap_or_else(|| cast_label(layer, node)),
                    layer: layer_index,
                    layer_badge: layer_badge.clone(),
                    node: Some(node),
                    track: None,
                    target: None,
                    source: MotionSource::Direct,
                    has_children: true,
                    collapsed: node_collapsed,
                    keys: (0, 0),
                });
                sheet.visible.push(cast_row);
                if node_collapsed {
                    continue;
                }
                for &(motion_index, track_index) in node_tracks {
                    let track = &animation.motions[motion_index].tracks[track_index];
                    let start = sheet.keys.len() as u32;
                    let row_index = sheet.rows.len() as u32;
                    let count = track_key_count(track);
                    for key_index in 0..count {
                        let Some(view) = channels::key_view(&track.keys, key_index) else {
                            continue;
                        };
                        let outside = view.frame < runtime_start || view.frame > runtime_end;
                        out_of_range += usize::from(outside);
                        key_min = key_min.min(view.frame);
                        key_max = key_max.max(view.frame);
                        sheet.keys.push(DopeKey {
                            row: row_index,
                            frame: view.frame,
                            index: key_index as u32,
                            semantics: view.semantics,
                            out_of_range: outside,
                        });
                    }
                    let end = sheet.keys.len() as u32;
                    sheet.rows.push(DopeRow {
                        kind: DopeRowKind::Channel,
                        depth: 1,
                        label: channels::channel_name(track.target),
                        layer: layer_index,
                        layer_badge: layer_badge.clone(),
                        node: Some(node),
                        track: Some(TrackAddress::new(
                            layer_index,
                            animation_index,
                            motion_index,
                            track_index,
                        )),
                        target: Some(track.target),
                        source: MotionSource::Direct,
                        has_children: false,
                        collapsed: false,
                        keys: (start, end),
                    });
                    sheet.visible.push(row_index);
                }
            }
        }

        sheet.key_min = if key_min == i32::MAX { 0 } else { key_min };
        sheet.key_max = if key_max == i32::MIN { 0 } else { key_max };
        sheet.out_of_range = out_of_range;
        self.animate.sheet = sheet;
        self.animate.assignment_rows = assignment_rows;
        self.animate.highlights = highlights;
        self.animate_clamp_pan();
    }

    fn animate_clamp_frame(&mut self) {
        let (start, end) = self.animate_play_range();
        self.animate.frame = self.animate.frame.clamp(start, end);
    }

    fn animate_clamp_pan(&mut self) {
        let (start, end) = self.animate_play_range();
        let minimum_visible_span = DOPE_MIN_TIMELINE_WIDTH / self.animate.zoom.max(0.5);
        let latest_pan = (end as f32 - minimum_visible_span).max(start as f32);
        self.animate.pan = self.animate.pan.clamp(start as f32, latest_pan);
    }

    fn animate_sync_drafts(&mut self) {
        let assignment = self.animate.assignment.clone();
        self.animate.drafts.set_name = display_srd_name(&assignment.name);
        self.animate.drafts.set_start = assignment.start_frame.to_string();
        self.animate.drafts.set_duration = assignment.runtime_duration.to_string();
        let (start, end) = self.animate_play_range();
        self.animate.drafts.work_start = start.to_string();
        self.animate.drafts.work_end = end.to_string();
        let animation = self
            .animate_active_animation()
            .map(|(_, animation)| animation.clone());
        if let Some(animation) = animation {
            self.animate.drafts.animation_name = display_srd_name(&animation.name);
            self.animate.drafts.animation_duration = animation.duration.to_string();
        } else {
            self.animate.drafts.animation_name.clear();
            self.animate.drafts.animation_duration.clear();
        }
        self.animate_sync_key_drafts();
    }

    fn animate_sync_key_drafts(&mut self) {
        let selected = self.animate_selected_track_snapshot();
        let Some((track, key_index)) = selected else {
            self.animate.drafts.key_frame.clear();
            self.animate.drafts.key_value.clear();
            self.animate.drafts.key_rgba = Default::default();
            self.animate.drafts.slope_in.clear();
            self.animate.drafts.slope_out.clear();
            self.animate.drafts.track_range_start.clear();
            self.animate.drafts.track_range_end.clear();
            return;
        };
        self.animate.drafts.track_range_start = track.range_start.to_string();
        self.animate.drafts.track_range_end = track.range_end.to_string();
        let Some(view) = key_index.and_then(|index| channels::key_view(&track.keys, index)) else {
            self.animate.drafts.key_frame.clear();
            self.animate.drafts.key_value.clear();
            self.animate.drafts.key_rgba = Default::default();
            self.animate.drafts.slope_in.clear();
            self.animate.drafts.slope_out.clear();
            return;
        };
        self.animate.drafts.key_frame = view.frame.to_string();
        self.animate.drafts.key_value = match view.value {
            KeyValue::Float(value) => format!("{value}"),
            KeyValue::Integer(value) => value.to_string(),
            KeyValue::Rgba(bytes) => format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                bytes[0], bytes[1], bytes[2], bytes[3]
            ),
        };
        self.animate.drafts.key_rgba = view.value.as_rgba().map(|byte| byte.to_string());
        self.animate.drafts.slope_in = format!("{}", view.slope_in);
        self.animate.drafts.slope_out = format!("{}", view.slope_out);
    }

    fn animate_selected_track_snapshot(&self) -> Option<(Track, Option<usize>)> {
        let (_, animation) = self.animate_active_animation()?;
        let (motion, track) = self.animate.selected_track?;
        let track = animation.motions.get(motion)?.tracks.get(track)?.clone();
        Some((track, self.animate.selected_key))
    }

    fn animate_activate_track(&mut self, address: TrackAddress) -> bool {
        let exists = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(address.layer))
            .and_then(|layer| layer.animations.get(address.animation))
            .and_then(|animation| animation.motions.get(address.motion))
            .is_some_and(|motion| address.track < motion.tracks.len());
        if !exists {
            return false;
        }
        self.animate.selected_layer = address.layer;
        self.animate.selected_animation = Some(address.animation);
        self.animate.selected_track = Some((address.motion, address.track));
        self.animate.selected_key = None;
        true
    }

    fn animate_select_track_address(&mut self, address: TrackAddress) -> ModelChange {
        if !self.animate_activate_track(address) {
            return ModelChange::default();
        }
        self.animate_rebuild_rows();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    fn animate_select_key_address(&mut self, address: TrackAddress, key: usize) -> ModelChange {
        if !self.animate_activate_track(address)
            || self
                .animate_track_snapshot(address.motion, address.track)
                .is_none_or(|(track, _)| key >= channels::key_count(&track.keys))
        {
            return ModelChange::default();
        }
        self.animate.selected_key = Some(key);
        self.animate_rebuild_rows();
        self.animate_sync_key_drafts();
        ModelChange::PREVIEW
    }

    pub(super) fn update_animate(&mut self, action: AnimateAction) -> ModelChange {
        let scratch_assignment_change = self.animate.source_set.is_none()
            && matches!(action, AnimateAction::AssignSlotAnimation(..));
        if action.mutates_document() && !self.animate.editing && !scratch_assignment_change {
            self.set_action_notice("Enable Edit before changing the SRD document.");
            return ModelChange::default();
        }
        match action {
            AnimateAction::LoadStoredSet(index) => self.animate_load_set(index),
            AnimateAction::SelectLayer(layer) => self.animate_select_layer(layer),
            AnimateAction::SelectAnimation(layer, animation) => {
                self.animate_select_animation(layer, animation)
            }
            AnimateAction::ToggleAnimationPreview => self.animate_toggle_animation_preview(),
            AnimateAction::SelectTrack(address) => self.animate_select_track_address(address),
            AnimateAction::SelectKey(address, key) => self.animate_select_key_address(address, key),
            AnimateAction::ClearSelectedKey => {
                self.animate.selected_key = None;
                self.animate_sync_key_drafts();
                ModelChange::default()
            }
            AnimateAction::DuplicateSet => self.animate_duplicate_set(),
            AnimateAction::ConfirmDelete => self.animate_confirm_delete(),
            AnimateAction::CancelDelete => {
                self.animate.pending_delete = None;
                ModelChange::default()
            }
            AnimateAction::CreateSet => self.animate_create_set(),
            AnimateAction::DeleteSet => self.animate_delete_set(),
            AnimateAction::SetName(value) => self.animate_set_name(value),
            AnimateAction::SetStartFrame(value) => self.animate_set_start(value),
            AnimateAction::SetDuration(value) => self.animate_set_duration(value),
            AnimateAction::ToggleSlotEnabled(layer) => self.animate_toggle_slot(layer),
            AnimateAction::AssignSlotAnimation(layer, animation) => {
                self.animate_assign_slot(layer, animation)
            }
            AnimateAction::CreateAnimation => self.animate_create_animation(false),
            AnimateAction::DuplicateAnimation => self.animate_create_animation(true),
            AnimateAction::DeleteAnimation => self.animate_delete_animation(),
            AnimateAction::SetAnimationName(value) => self.animate_set_animation_name(value),
            AnimateAction::SetAnimationDuration(value) => {
                self.animate_set_animation_duration(value)
            }
            AnimateAction::SetAnimationLoop(enabled) => self.animate_set_animation_loop(enabled),
            AnimateAction::AddMotion(node) => self.animate_add_motion(node),
            AnimateAction::DeleteMotion(motion) => self.animate_delete_motion(motion),
            AnimateAction::SetMotionTarget(motion, node) => {
                self.animate_set_motion_target(motion, node)
            }
            AnimateAction::AddTrack(motion, target) => self.animate_add_track(motion, target),
            AnimateAction::DeleteTrack(motion, track) => self.animate_delete_track(motion, track),
            AnimateAction::SetTrackChannel(motion, track, channel) => {
                self.animate_set_track_channel(motion, track, channel)
            }
            AnimateAction::SetTrackFormat(motion, track, format) => {
                self.animate_set_track_format(motion, track, format)
            }
            AnimateAction::SetTrackWrap(motion, track, wrap) => {
                self.animate_set_track_wrap(motion, track, wrap)
            }
            AnimateAction::SetTrackRangeStart(motion, track, value) => {
                self.animate_set_track_range(motion, track, value, true)
            }
            AnimateAction::SetTrackRangeEnd(motion, track, value) => {
                self.animate_set_track_range(motion, track, value, false)
            }
            AnimateAction::AddProperty {
                layer,
                animation,
                node,
                target,
            } => self.animate_add_property(layer, animation, node, target),
            AnimateAction::ToggleTrackAnimation(address) => {
                self.animate_toggle_track_animation(address)
            }
            AnimateAction::ToggleKeyAtFrame(address, frame) => {
                self.animate_toggle_key_at_frame(address, frame)
            }
            AnimateAction::SetTrackValueAtFrame(address, frame, value) => {
                self.animate_set_track_value_at_frame(address, frame, value)
            }
            AnimateAction::AddKey(address, frame) => {
                if !self.animate_activate_track(address) {
                    return ModelChange::default();
                }
                self.animate_add_key(address.motion, address.track, frame)
            }
            AnimateAction::DeleteKey(address, key) => {
                if !self.animate_activate_track(address) {
                    return ModelChange::default();
                }
                self.animate_delete_key(address.motion, address.track, key)
            }
            AnimateAction::MoveKey(address, key, frame) => {
                if !self.animate_activate_track(address) {
                    return ModelChange::default();
                }
                self.animate_move_key(address.motion, address.track, key, frame)
            }
            AnimateAction::SetKeyFrame(value) => {
                self.animate.drafts.key_frame = value.clone();
                match value.trim().parse::<i32>() {
                    Ok(frame) => {
                        let Some((motion, track)) = self.animate.selected_track else {
                            return ModelChange::default();
                        };
                        let Some(key) = self.animate.selected_key else {
                            return ModelChange::default();
                        };
                        self.animate_move_key(motion, track, key, frame)
                    }
                    Err(_) => ModelChange::default(),
                }
            }
            AnimateAction::SetKeyValue(value) => self.animate_set_key_value(value),
            AnimateAction::SetKeyByte(component, value) => {
                self.animate_set_key_byte(component, value)
            }
            AnimateAction::SetKeySemantics(semantics) => self.animate_set_key_semantics(semantics),
            AnimateAction::SetKeySlope(incoming, value) => {
                self.animate_set_key_slope(incoming, value)
            }
            AnimateAction::SetFrame(frame) => self.animate_set_frame(frame),
            AnimateAction::StepFrame(delta) => {
                let frame = self.animate.frame.saturating_add(delta);
                self.animate_set_frame(frame)
            }
            AnimateAction::JumpToPreviousTrackKey(address) => {
                self.animate_jump_track_key(address, false)
            }
            AnimateAction::JumpToNextTrackKey(address) => {
                self.animate_jump_track_key(address, true)
            }
            AnimateAction::TogglePlaying => {
                self.animate.playing = self.selected_scene().is_some() && !self.animate.playing;
                if self.animate.playing {
                    return ModelChange::PREVIEW;
                }
                ModelChange::default()
            }
            AnimateAction::JumpToStart => {
                let start = self.animate_play_range().0;
                self.animate_set_frame(start)
            }
            AnimateAction::JumpToEnd => {
                let end = self.animate_play_range().1;
                self.animate_set_frame(end)
            }
            AnimateAction::FrameCompleted => self.animate_frame_completed(),
            AnimateAction::SetWorkStart(value) => self.animate_set_work(value, true),
            AnimateAction::SetWorkEnd(value) => self.animate_set_work(value, false),
            AnimateAction::FitAllKeys => self.animate_fit_all_keys(),
            AnimateAction::ClearWorkArea => {
                self.animate.work_area = None;
                self.animate_clamp_frame();
                self.animate_clamp_pan();
                self.animate_sync_drafts();
                ModelChange::PREVIEW
            }
            AnimateAction::ToggleRowCollapsed(row) => {
                if let Some(entry) = self.animate.sheet.rows.get(row) {
                    let key = RowKey {
                        layer: entry.layer,
                        node: entry.node,
                    };
                    self.animate.toggle_collapsed(key);
                    self.animate_rebuild_rows();
                }
                ModelChange::default()
            }
            AnimateAction::ToggleMaximized => {
                self.animate.maximized = !self.animate.maximized;
                ModelChange::default()
            }
            AnimateAction::ZoomBy(delta) => {
                self.animate.zoom = (self.animate.zoom * (1.0 + delta)).clamp(0.5, 64.0);
                self.animate_clamp_pan();
                ModelChange::default()
            }
            AnimateAction::PanBy(delta) => {
                self.animate.pan += delta;
                self.animate_clamp_pan();
                ModelChange::default()
            }
            AnimateAction::ScrollBy(delta, viewport_height) => {
                let content_height = self.animate.sheet.visible.len() as f32 * DOPE_ROW_HEIGHT;
                let max = (content_height - viewport_height).max(0.0);
                self.animate.scroll = (self.animate.scroll + delta).clamp(0.0, max);
                ModelChange::default()
            }
        }
    }

    fn animate_load_set(&mut self, index: Option<usize>) -> ModelChange {
        let Some(scene) = self.selected_scene() else {
            return ModelChange::default();
        };
        let layer_count = scene.layers.len();
        match index {
            Some(index) => {
                let Some(stored) = scene.animation_sets.get(index) else {
                    return ModelChange::default();
                };
                let mut assignment = stored.clone();
                densify(&mut assignment, layer_count);
                self.animate.assignment = assignment;
                self.animate.source_set = Some(index);
            }
            None => {
                self.animate.assignment = scratch_assignment(scene, 0);
                self.animate.source_set = None;
            }
        }
        self.animate.frame = self.animate_play_range().0;
        self.animate.playing = false;
        self.animate.preview = None;
        self.animate.work_area = None;
        self.animate.selected_key = None;
        self.animate.selected_track = None;
        self.animate.selected_animation = None;
        self.animate_clamp_selection();
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_clamp_frame();
        self.animate_sync_drafts();
        self.sync_inspector();
        ModelChange::PREVIEW
    }

    fn animate_select_layer(&mut self, layer: usize) -> ModelChange {
        if self
            .selected_scene()
            .is_none_or(|scene| layer >= scene.layers.len())
        {
            return ModelChange::default();
        }
        self.animate.preview = None;
        self.animate.selected_layer = layer;
        self.animate.selected_animation = None;
        self.animate.selected_track = None;
        self.animate.selected_key = None;
        self.animate_clamp_selection();
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    /// Selects one animation for authoring without changing the evaluated set.
    fn animate_select_animation(&mut self, layer: usize, animation: usize) -> ModelChange {
        if self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer))
            .and_then(|layer| layer.animations.get(animation))
            .is_none()
        {
            return ModelChange::default();
        }
        self.animate.preview = None;
        self.animate.selected_layer = layer;
        self.animate.selected_animation = Some(animation);
        self.animate.selected_track = None;
        self.animate.selected_key = None;
        self.animate.work_area = None;
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_clamp_frame();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    /// Toggles an isolated preview of the selected animation. The working set
    /// remains unchanged and is restored exactly when preview is disabled.
    fn animate_toggle_animation_preview(&mut self) -> ModelChange {
        let layer = self.animate.selected_layer;
        let Some(animation) = self.animate.selected_animation else {
            self.set_action_notice("Select an animation before previewing it.");
            return ModelChange::default();
        };
        let Some(name) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer))
            .and_then(|layer| layer.animations.get(animation))
            .map(|animation| animation.name.clone())
        else {
            return ModelChange::default();
        };
        let active = self
            .animate
            .preview
            .as_ref()
            .is_some_and(|preview| preview.layer == layer && preview.animation_name == name);
        self.animate.preview = (!active).then_some(AnimationPreview {
            layer,
            animation_name: name,
        });
        self.animate.work_area = None;
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_clamp_frame();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    pub(super) fn animate_enter_edit(&mut self) -> ModelChange {
        if self.animate.editing {
            return ModelChange::default();
        }
        if self.document().is_none() {
            self.set_action_notice("Open an SRD before enabling Edit.");
            return ModelChange::default();
        }
        self.animate.editing = true;
        ModelChange::default()
    }

    /// Creates a stored copy of the working assignment and enters Edit on it.
    fn animate_duplicate_set(&mut self) -> ModelChange {
        let Some(scene) = self.selected_scene() else {
            return ModelChange::default();
        };
        let layer_count = scene.layers.len();
        let name = unique_name(
            scene
                .animation_sets
                .iter()
                .map(|set| display_srd_name(&set.name)),
            &nonempty(&self.animate.assignment.name, "Animation Set".into()),
        );
        let mut duplicate = self.animate.assignment.clone();
        densify(&mut duplicate, layer_count);
        duplicate.name = name;
        if duplicate.runtime_duration <= 0 {
            duplicate.runtime_duration = self.animate_runtime_range().1.max(DEFAULT_DURATION);
        }
        let scene_index = self.selected_scene_index();

        let index = {
            let scene = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .expect("validated selected scene");
            scene.animation_sets.push(duplicate.clone());
            scene.declared_animation_set_count = scene.animation_sets.len() as u32;
            scene.animation_sets.len() - 1
        };
        self.bump_revision();
        self.animate.assignment = duplicate;
        self.animate.source_set = Some(index);
        self.animate.preview = None;
        self.animate.editing = true;
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_sync_drafts();
        self.set_action_notice("Duplicated animation set.");
        ModelChange::PREVIEW
    }

    pub(super) fn animate_done(&mut self) -> ModelChange {
        if !self.animate.editing {
            return ModelChange::default();
        }
        self.animate.editing = false;
        self.animate.pending_delete = None;
        ModelChange::default()
    }

    fn animate_create_set(&mut self) -> ModelChange {
        let Some(scene) = self.selected_scene() else {
            return ModelChange::default();
        };
        let layer_count = scene.layers.len();
        let name = unique_name(
            scene
                .animation_sets
                .iter()
                .map(|set| display_srd_name(&set.name)),
            "Animation Set",
        );
        let mut created = scratch_assignment(scene, 0);
        created.name = name;
        created.runtime_duration = self.animate_default_duration();
        densify(&mut created, layer_count);
        let scene_index = self.selected_scene_index();

        let index = {
            let scene = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .expect("validated selected scene");
            scene.animation_sets.push(created.clone());
            scene.declared_animation_set_count = scene.animation_sets.len() as u32;
            scene.animation_sets.len() - 1
        };
        self.bump_revision();
        self.animate.assignment = created;
        self.animate.source_set = Some(index);
        self.animate.preview = None;
        self.animate_sync();
        ModelChange::PREVIEW
    }

    fn animate_delete_set(&mut self) -> ModelChange {
        let Some(index) = self.animate.source_set else {
            self.set_action_notice("Load a stored animation set before deleting one.");
            return ModelChange::default();
        };
        let scene_index = self.selected_scene_index();
        if self
            .selected_scene()
            .is_none_or(|scene| index >= scene.animation_sets.len())
        {
            return ModelChange::default();
        }

        {
            let scene = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .expect("validated selected scene");
            scene.animation_sets.remove(index);
            scene.declared_animation_set_count = scene.animation_sets.len() as u32;
        }
        self.bump_revision();
        self.animate.source_set = None;
        self.animate.assignment = self
            .selected_scene()
            .map_or_else(|| empty_assignment(0), |scene| scratch_assignment(scene, 0));
        self.animate_sync();
        self.set_action_notice("Deleted the stored animation set.");
        ModelChange::PREVIEW
    }

    fn animate_default_duration(&self) -> i32 {
        let (_, end) = self.animate_play_range();
        if end > 0 { end } else { DEFAULT_DURATION }
    }

    fn animate_set_name(&mut self, value: String) -> ModelChange {
        self.animate.drafts.set_name = value.clone();
        let bytes = value.trim().as_bytes().to_vec();
        if bytes.is_empty() {
            self.set_action_notice("Animation set names cannot be empty.");
            return ModelChange::default();
        }
        let duplicate = self.selected_scene().is_some_and(|scene| {
            scene
                .animation_sets
                .iter()
                .enumerate()
                .any(|(index, set)| Some(index) != self.animate.source_set && set.name == bytes)
        });
        if duplicate {
            self.set_action_notice("Another animation set in this scene already uses that name.");
            return ModelChange::default();
        }
        self.animate_write_assignment(move |set| set.name = bytes.clone())
    }

    fn animate_set_start(&mut self, value: String) -> ModelChange {
        self.animate.drafts.set_start = value.clone();
        let Ok(start) = value.trim().parse::<i32>() else {
            return ModelChange::default();
        };
        let change = self.animate_write_assignment(move |set| set.start_frame = start);
        self.animate_clamp_frame();
        change
    }

    fn animate_set_duration(&mut self, value: String) -> ModelChange {
        self.animate.drafts.set_duration = value.clone();
        let Ok(duration) = value.trim().parse::<i32>() else {
            return ModelChange::default();
        };
        if duration <= 0 {
            self.set_action_notice("Animation set duration must be positive.");
            return ModelChange::default();
        }
        let change = self.animate_write_assignment(move |set| set.runtime_duration = duration);
        self.animate_clamp_frame();
        change
    }

    fn animate_toggle_slot(&mut self, layer: usize) -> ModelChange {
        let Some(slot) = self.animate.assignment.slots.get(layer) else {
            return ModelChange::default();
        };
        let enabled = i32::from(!slot.is_enabled());
        let change = self.animate_write_assignment(move |set| {
            if let Some(slot) = set.slots.get_mut(layer) {
                slot.enabled = enabled;
            }
        });
        change
    }

    fn animate_assign_slot(&mut self, layer: usize, animation: Option<usize>) -> ModelChange {
        let has_animation = animation.is_some();
        let name = match animation {
            Some(index) => {
                let Some(name) = self
                    .selected_scene()
                    .and_then(|scene| scene.layers.get(layer))
                    .and_then(|layer| layer.animations.get(index))
                    .map(|animation| animation.name.clone())
                else {
                    return ModelChange::default();
                };
                name
            }
            None => Vec::new(),
        };
        let scratch = self.animate.source_set.is_none();
        self.animate_write_assignment(move |set| {
            if let Some(slot) = set.slots.get_mut(layer) {
                slot.animation_name = name.clone();
                if scratch && has_animation {
                    slot.enabled = 1;
                }
            }
        })
    }

    /// Applies one change to the owned working assignment and, when it came
    /// from a stored ANMS, to that record too.
    fn animate_write_assignment(
        &mut self,
        write: impl Fn(&mut AnimationSetDefinition),
    ) -> ModelChange {
        let scene_index = self.selected_scene_index();
        let stored = self.animate.source_set.filter(|index| {
            self.selected_scene()
                .is_some_and(|scene| *index < scene.animation_sets.len())
        });
        let mut assignment = self.animate.assignment.clone();
        write(&mut assignment);
        let stored_update = stored.map(|index| {
            let mut set = self
                .selected_scene()
                .expect("validated selected scene")
                .animation_sets[index]
                .clone();
            write(&mut set);
            (index, set)
        });
        let assignment_changed = assignment != self.animate.assignment;
        let stored_changed = stored_update.as_ref().is_some_and(|(index, set)| {
            self.selected_scene()
                .is_some_and(|scene| scene.animation_sets[*index] != *set)
        });
        if !assignment_changed && !stored_changed {
            self.animate_sync_drafts();
            return ModelChange::default();
        }

        self.animate.assignment = assignment;
        if let Some((index, set)) = stored_update
            && stored_changed
        {
            self.document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .and_then(|scene| scene.animation_sets.get_mut(index))
                .map(|stored| *stored = set)
                .expect("validated stored animation set");
            self.bump_revision();
        }
        self.animate_rebuild_effective();
        self.animate_rebuild_rows();
        self.animate_clamp_frame();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    fn animate_create_animation(&mut self, duplicate: bool) -> ModelChange {
        let layer_index = self.animate.selected_layer;
        let scene_index = self.selected_scene_index();
        let Some(layer) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
        else {
            return ModelChange::default();
        };
        let source = if duplicate {
            let Some(index) = self.animate.selected_animation else {
                self.set_action_notice("Select an animation before duplicating it.");
                return ModelChange::default();
            };
            layer.animations.get(index).cloned()
        } else {
            None
        };
        let base = source
            .as_ref()
            .map(|animation| display_srd_name(&animation.name))
            .unwrap_or_else(|| "Animation".into());
        let name = unique_name(
            layer
                .animations
                .iter()
                .map(|animation| display_srd_name(&animation.name)),
            if base.is_empty() { "Animation" } else { &base },
        );
        let duration = source
            .as_ref()
            .map(|animation| animation.duration)
            .unwrap_or_else(|| self.animate_default_duration());
        let created = AnimationDefinition {
            name,
            flags: source.as_ref().map_or(0, |animation| animation.flags),
            declared_motion_count: source
                .as_ref()
                .map_or(0, |animation| animation.motions.len() as u32),
            duration,
            motions: source
                .as_ref()
                .map(|animation| animation.motions.clone())
                .unwrap_or_default(),
        };

        let index = {
            let layer = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .and_then(|scene| scene.layers.get_mut(layer_index))
                .expect("validated selected layer");
            layer.animations.push(created);
            layer.animation_count = layer.animations.len() as u32;
            layer.animations.len() - 1
        };
        self.bump_revision();
        self.animate.selected_animation = Some(index);
        self.animate.selected_track = None;
        self.animate.selected_key = None;

        self.animate_sync();
        self.set_action_notice(if duplicate {
            "Duplicated the animation; the copy has a unique name."
        } else {
            "Created an animation. Add motions to target casts."
        });
        ModelChange::PREVIEW
    }

    fn animate_delete_animation(&mut self) -> ModelChange {
        let layer_index = self.animate.selected_layer;
        let Some(animation_index) = self.animate.selected_animation else {
            return ModelChange::default();
        };
        let Some(scene) = self.selected_scene() else {
            return ModelChange::default();
        };
        let Some(layer) = scene.layers.get(layer_index) else {
            return ModelChange::default();
        };
        let Some(animation) = layer.animations.get(animation_index) else {
            return ModelChange::default();
        };
        let name = animation.name.clone();
        let referencing = scene
            .animation_sets
            .iter()
            .filter(|set| {
                set.slots
                    .get(layer_index)
                    .is_some_and(|slot| slot.animation_name == name)
            })
            .count();
        if referencing > 0 {
            self.animate.pending_delete = Some(PendingDelete {
                layer: layer_index,
                animation_name: display_srd_name(&name),
                animation_identity: name,
                referencing_sets: referencing,
            });
            return ModelChange::default();
        }
        self.animate_remove_animation(layer_index, animation_index, &name)
    }

    fn animate_confirm_delete(&mut self) -> ModelChange {
        let Some(pending) = self.animate.pending_delete.take() else {
            return ModelChange::default();
        };
        let Some((animation_index, _)) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(pending.layer))
            .and_then(|layer| layer.find_animation(&pending.animation_identity))
        else {
            self.set_action_notice(
                "The pending animation changed before confirmation; nothing was deleted.",
            );
            return ModelChange::default();
        };
        self.animate_remove_animation(pending.layer, animation_index, &pending.animation_identity)
    }

    /// Removes an animation and clears every slot that referenced it, so the
    /// corpus invariant of zero dangling slot names is preserved.
    fn animate_remove_animation(
        &mut self,
        layer_index: usize,
        animation_index: usize,
        name: &[u8],
    ) -> ModelChange {
        let scene_index = self.selected_scene_index();

        {
            let scene = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .expect("validated selected scene");
            if let Some(layer) = scene.layers.get_mut(layer_index) {
                layer.animations.remove(animation_index);
                layer.animation_count = layer.animations.len() as u32;
            }
            for set in &mut scene.animation_sets {
                if let Some(slot) = set.slots.get_mut(layer_index)
                    && slot.animation_name == name
                {
                    slot.animation_name.clear();
                }
            }
        }
        self.bump_revision();
        if let Some(slot) = self.animate.assignment.slots.get_mut(layer_index)
            && slot.animation_name == name
        {
            slot.animation_name.clear();
        }
        if self
            .animate
            .preview
            .as_ref()
            .is_some_and(|preview| preview.animation_name == name)
        {
            self.animate.preview = None;
        }
        self.animate.selected_animation = None;
        self.animate.selected_track = None;
        self.animate.selected_key = None;
        self.animate_sync();
        self.set_action_notice("Deleted the animation and cleared every slot that named it.");
        ModelChange::PREVIEW
    }

    fn animate_set_animation_name(&mut self, value: String) -> ModelChange {
        self.animate.drafts.animation_name = value.clone();
        let layer_index = self.animate.selected_layer;
        let Some(animation_index) = self.animate.selected_animation else {
            return ModelChange::default();
        };
        let bytes = value.trim().as_bytes().to_vec();
        if bytes.is_empty() {
            self.set_action_notice("Animation names cannot be empty.");
            return ModelChange::default();
        }
        let Some(scene) = self.selected_scene() else {
            return ModelChange::default();
        };
        let Some(layer) = scene.layers.get(layer_index) else {
            return ModelChange::default();
        };
        if layer
            .animations
            .iter()
            .enumerate()
            .any(|(index, animation)| index != animation_index && animation.name == bytes)
        {
            self.set_action_notice("Another animation on this layer already uses that name.");
            return ModelChange::default();
        }
        let previous = layer.animations[animation_index].name.clone();
        if previous == bytes {
            return ModelChange::default();
        }
        let scene_index = self.selected_scene_index();

        {
            let scene = self
                .document_mut()
                .and_then(|document| document.project.scenes.get_mut(scene_index))
                .expect("validated selected scene");
            scene.layers[layer_index].animations[animation_index].name = bytes.clone();
            // Renaming must not orphan a slot that named the animation.
            for set in &mut scene.animation_sets {
                if let Some(slot) = set.slots.get_mut(layer_index)
                    && slot.animation_name == previous
                {
                    slot.animation_name = bytes.clone();
                }
            }
        }
        self.bump_revision();
        if let Some(slot) = self.animate.assignment.slots.get_mut(layer_index)
            && slot.animation_name == previous
        {
            slot.animation_name = bytes.clone();
        }
        if let Some(preview) = self.animate.preview.as_mut()
            && preview.animation_name == previous
        {
            preview.animation_name = bytes;
        }
        self.animate_sync();
        ModelChange::PREVIEW
    }

    fn animate_set_animation_duration(&mut self, value: String) -> ModelChange {
        self.animate.drafts.animation_duration = value.clone();
        let Ok(duration) = value.trim().parse::<i32>() else {
            return ModelChange::default();
        };
        if duration <= 0 {
            self.set_action_notice("Animation duration must be positive.");
            return ModelChange::default();
        }
        self.animate_write_animation(move |animation| animation.duration = duration)
    }

    fn animate_set_animation_loop(&mut self, enabled: bool) -> ModelChange {
        let layer = self.animate.selected_layer;
        if self.layer_locked(layer) {
            self.set_action_notice("Unlock the selected layer before editing its animation flags.");
            return ModelChange::default();
        }
        self.animate_write_animation(move |animation| {
            set_masked_flag(&mut animation.flags, 0x01, enabled);
        })
    }

    fn animate_write_animation(
        &mut self,
        write: impl FnOnce(&mut AnimationDefinition),
    ) -> ModelChange {
        let Some(animation_index) = self.animate.selected_animation else {
            return ModelChange::default();
        };
        self.animate_write_animation_at(self.animate.selected_layer, animation_index, write)
    }

    /// Writes a specific ANIM address so property actions do not accidentally
    /// mutate whichever animation the inspector happens to be focused on.
    fn animate_write_animation_at(
        &mut self,
        layer_index: usize,
        animation_index: usize,
        write: impl FnOnce(&mut AnimationDefinition),
    ) -> ModelChange {
        let scene_index = self.selected_scene_index();
        let Some(current) = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
            .and_then(|layer| layer.animations.get(animation_index))
            .cloned()
        else {
            return ModelChange::default();
        };
        let mut updated = current.clone();
        write(&mut updated);
        updated.declared_motion_count = updated.motions.len() as u32;
        if updated == current {
            self.animate_sync_drafts();
            return ModelChange::default();
        }

        *self
            .document_mut()
            .and_then(|document| document.project.scenes.get_mut(scene_index))
            .and_then(|scene| scene.layers.get_mut(layer_index))
            .and_then(|layer| layer.animations.get_mut(animation_index))
            .expect("validated animation address") = updated;
        self.bump_revision();
        self.animate_sync();
        ModelChange::PREVIEW
    }

    /// CAST locks are editor-only but protect every animation mutation whose
    /// receiver is that CAST (or one of its locked ancestors).
    fn animate_cast_mutation_allowed(&mut self, layer: usize, node: usize) -> bool {
        if !self.cast_locked(self.selected_scene_index(), layer, node) {
            return true;
        }
        self.set_action_notice("Unlock the CAST hierarchy before editing its animation.");
        false
    }

    pub(super) fn animate_edit_inspector_value(&mut self, target: u16, value: String) -> bool {
        if !self.animate.editing {
            self.set_action_notice("Enter animation Edit mode before changing evaluated values.");
            return false;
        }
        let layer_index = self.selected_layer_index();
        let Some(node) = self.selected_node_index() else {
            return false;
        };
        let animation_name = self
            .animate
            .effective_assignment()
            .slots
            .get(layer_index)
            .filter(|slot| slot.is_enabled() && !slot.animation_name.is_empty())
            .map(|slot| slot.animation_name.clone());
        let animation_index = animation_name.and_then(|name| {
            self.selected_scene()?
                .layers
                .get(layer_index)?
                .animations
                .iter()
                .position(|animation| animation.name == name)
        });
        let Some(animation_index) = animation_index.or_else(|| {
            (self.animate.selected_layer == layer_index)
                .then_some(self.animate.selected_animation)
                .flatten()
        }) else {
            self.set_action_notice("Assign or select an animation before keying Inspector values.");
            return false;
        };
        if parse_track_value(target, &value).is_none() {
            return false;
        }
        let property_change = self.animate_add_property(layer_index, animation_index, node, target);
        let Some((motion, track, key, selected_target)) = self.animate_selected_key_target() else {
            return false;
        };
        if selected_target != target {
            return false;
        }
        let Some(parsed) = parse_track_value(target, &value) else {
            return false;
        };
        let change = self.animate_write_track(motion, track, move |motion, index| {
            channels::set_key_value(&mut motion.tracks[index].keys, key, parsed);
        });
        property_change.preview || change.preview
    }

    fn animate_addressed_track_snapshot(&self, address: TrackAddress) -> Option<(i32, Track)> {
        let motion = self
            .selected_scene()?
            .layers
            .get(address.layer)?
            .animations
            .get(address.animation)?
            .motions
            .get(address.motion)?;
        Some((motion.target, motion.tracks.get(address.track)?.clone()))
    }

    fn animate_add_property(
        &mut self,
        layer_index: usize,
        animation_index: usize,
        node: usize,
        target: u16,
    ) -> ModelChange {
        let Ok(node_target) = i32::try_from(node) else {
            return ModelChange::default();
        };
        let Some(spec) = channels::channel(target) else {
            self.set_action_notice("That channel has no runtime consumer and cannot be authored.");
            return ModelChange::default();
        };
        let valid_target = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
            .is_some_and(|layer| {
                animation_index < layer.animations.len()
                    && is_authorable_track_target(layer, node, target)
            });
        if !valid_target {
            self.set_action_notice("Choose a concrete CAST that consumes that channel.");
            return ModelChange::default();
        }
        if !self.animate_cast_mutation_allowed(layer_index, node) {
            return ModelChange::default();
        }

        #[derive(Clone, Copy)]
        enum PropertyPlan {
            Existing {
                motion: usize,
                track: usize,
                key: Option<usize>,
            },
            NewTrack {
                motion: usize,
            },
            NewMotion,
        }

        let plan = {
            let animation = self
                .selected_scene()
                .and_then(|scene| scene.layers.get(layer_index))
                .and_then(|layer| layer.animations.get(animation_index))
                .expect("validated property animation");
            if let Some((motion, track)) =
                animation
                    .motions
                    .iter()
                    .enumerate()
                    .find_map(|(motion, entry)| {
                        if entry.target != node_target {
                            return None;
                        }
                        entry
                            .tracks
                            .iter()
                            .position(|track| track.target == target)
                            .map(|track| (motion, track))
                    })
            {
                let current = &animation.motions[motion].tracks[track];
                if channels::format_choice(spec.value, current.format).is_none()
                    || !track_storage_is_authorable(current)
                {
                    self.set_action_notice(
                        "The existing property track has an unsupported key format.",
                    );
                    return ModelChange::default();
                }
                let key = (0..track_key_count(current)).find(|&key| {
                    channels::key_frame(&current.keys, key) == Some(self.animate.frame)
                });
                if key.is_none() && track_key_count(current) >= usize::from(u16::MAX) {
                    self.set_action_notice("A track cannot contain more than 65,535 keys.");
                    return ModelChange::default();
                }
                PropertyPlan::Existing { motion, track, key }
            } else if let Some(motion) = animation
                .motions
                .iter()
                .position(|entry| entry.target == node_target)
            {
                PropertyPlan::NewTrack { motion }
            } else {
                PropertyPlan::NewMotion
            }
        };

        let frame = self.animate.frame;
        let duration = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
            .and_then(|layer| layer.animations.get(animation_index))
            .map_or(1, |animation| animation.duration.max(1));
        let address = match plan {
            PropertyPlan::Existing {
                motion,
                track,
                key: Some(key),
            } => {
                self.animate.selected_layer = layer_index;
                self.animate.selected_animation = Some(animation_index);
                self.animate.selected_track = Some((motion, track));
                self.animate.selected_key = Some(key);
                self.animate_rebuild_rows();
                self.animate_sync_key_drafts();
                return ModelChange::PREVIEW;
            }
            PropertyPlan::Existing {
                motion,
                track,
                key: None,
            } => TrackAddress::new(layer_index, animation_index, motion, track),
            PropertyPlan::NewTrack { motion } => TrackAddress::new(
                layer_index,
                animation_index,
                motion,
                self.selected_scene()
                    .and_then(|scene| scene.layers.get(layer_index))
                    .and_then(|layer| layer.animations.get(animation_index))
                    .map_or(0, |animation| animation.motions[motion].tracks.len()),
            ),
            PropertyPlan::NewMotion => TrackAddress::new(
                layer_index,
                animation_index,
                self.selected_scene()
                    .and_then(|scene| scene.layers.get(layer_index))
                    .and_then(|layer| layer.animations.get(animation_index))
                    .map_or(0, |animation| animation.motions.len()),
                0,
            ),
        };
        let format = spec.default_format;
        let default = spec.default_semantics;
        let change =
            self.animate_write_animation_at(layer_index, animation_index, move |animation| {
                let track = match plan {
                    PropertyPlan::Existing { motion, track, .. } => {
                        &mut animation.motions[motion].tracks[track]
                    }
                    PropertyPlan::NewTrack { motion } => {
                        animation.motions[motion].tracks.push(Track {
                            target,
                            key_count: 0,
                            format,
                            range_start: 0,
                            range_end: duration,
                            keys: channels::empty_keys(format)
                                .expect("catalog formats are parser-legal"),
                        });
                        animation.motions[motion]
                            .tracks
                            .last_mut()
                            .expect("pushed property track")
                    }
                    PropertyPlan::NewMotion => {
                        animation.motions.push(Motion {
                            target: node_target,
                            tracks: vec![Track {
                                target,
                                key_count: 0,
                                format,
                                range_start: 0,
                                range_end: duration,
                                keys: channels::empty_keys(format)
                                    .expect("catalog formats are parser-legal"),
                            }],
                        });
                        &mut animation
                            .motions
                            .last_mut()
                            .expect("pushed property motion")
                            .tracks[0]
                    }
                };
                materialize_empty_track_keys(track);
                channels::insert_key(&mut track.keys, track.format, frame, default);
                track.key_count = channels::key_count(&track.keys) as u16;
            });
        self.animate.selected_layer = layer_index;
        self.animate.selected_animation = Some(animation_index);
        self.animate.selected_track = Some((address.motion, address.track));
        self.animate.selected_key =
            self.animate_addressed_track_snapshot(address)
                .and_then(|(_, track)| {
                    (0..track_key_count(&track))
                        .find(|&key| channels::key_frame(&track.keys, key) == Some(frame))
                });
        self.animate_sync_key_drafts();
        change
    }

    fn animate_toggle_track_animation(&mut self, address: TrackAddress) -> ModelChange {
        let Some((motion_target, _)) = self.animate_addressed_track_snapshot(address) else {
            return ModelChange::default();
        };
        let Ok(node) = usize::try_from(motion_target) else {
            return ModelChange::default();
        };
        if !self.animate_cast_mutation_allowed(address.layer, node) {
            return ModelChange::default();
        }
        let change =
            self.animate_write_animation_at(address.layer, address.animation, move |animation| {
                let remove_motion = {
                    let motion = animation
                        .motions
                        .get_mut(address.motion)
                        .expect("validated property motion");
                    motion.tracks.remove(address.track);
                    motion.tracks.is_empty()
                };
                if remove_motion {
                    animation.motions.remove(address.motion);
                }
            });
        self.animate.selected_layer = address.layer;
        self.animate.selected_animation = Some(address.animation);
        self.animate.selected_track = None;
        self.animate.selected_key = None;
        self.animate_sync_key_drafts();
        change
    }

    fn animate_toggle_key_at_frame(&mut self, address: TrackAddress, frame: i32) -> ModelChange {
        let Some((motion_target, current)) = self.animate_addressed_track_snapshot(address) else {
            return ModelChange::default();
        };
        let Ok(node) = usize::try_from(motion_target) else {
            return ModelChange::default();
        };
        let Some(spec) = channels::channel(current.target) else {
            return ModelChange::default();
        };
        if channels::format_choice(spec.value, current.format).is_none()
            || !track_storage_is_authorable(&current)
        {
            self.set_action_notice("The property track has an unsupported key format.");
            return ModelChange::default();
        }
        if !self.animate_cast_mutation_allowed(address.layer, node) {
            return ModelChange::default();
        }
        let existing = (0..track_key_count(&current))
            .find(|&key| channels::key_frame(&current.keys, key) == Some(frame));
        if existing.is_none() && track_key_count(&current) >= usize::from(u16::MAX) {
            self.set_action_notice("A track cannot contain more than 65,535 keys.");
            return ModelChange::default();
        }
        let format = current.format;
        let default = spec.default_semantics;
        let change =
            self.animate_write_animation_at(address.layer, address.animation, move |animation| {
                let track = &mut animation.motions[address.motion].tracks[address.track];
                if let Some(key) = existing {
                    channels::remove_key(&mut track.keys, key);
                } else {
                    materialize_empty_track_keys(track);
                    channels::insert_key(&mut track.keys, format, frame, default);
                }
                track.key_count = channels::key_count(&track.keys) as u16;
            });
        self.animate.selected_layer = address.layer;
        self.animate.selected_animation = Some(address.animation);
        self.animate.selected_track = Some((address.motion, address.track));
        self.animate.selected_key = if existing.is_some() {
            None
        } else {
            self.animate_addressed_track_snapshot(address)
                .and_then(|(_, track)| {
                    (0..track_key_count(&track))
                        .find(|&key| channels::key_frame(&track.keys, key) == Some(frame))
                })
        };
        self.animate_sync_key_drafts();
        change
    }

    fn animate_set_track_value_at_frame(
        &mut self,
        address: TrackAddress,
        frame: i32,
        value: String,
    ) -> ModelChange {
        let Some((motion_target, current)) = self.animate_addressed_track_snapshot(address) else {
            return ModelChange::default();
        };
        let Ok(node) = usize::try_from(motion_target) else {
            return ModelChange::default();
        };
        let Some(spec) = channels::channel(current.target) else {
            return ModelChange::default();
        };
        if channels::format_choice(spec.value, current.format).is_none()
            || !track_storage_is_authorable(&current)
        {
            self.set_action_notice("The property track has an unsupported key format.");
            return ModelChange::default();
        }
        let Some(value) = parse_track_value(current.target, &value) else {
            self.set_action_notice("Enter a value compatible with this animation channel.");
            return ModelChange::default();
        };
        if !self.animate_cast_mutation_allowed(address.layer, node) {
            return ModelChange::default();
        }
        let existing = (0..track_key_count(&current))
            .find(|&key| channels::key_frame(&current.keys, key) == Some(frame));
        if existing.is_none() && track_key_count(&current) >= usize::from(u16::MAX) {
            self.set_action_notice("A track cannot contain more than 65,535 keys.");
            return ModelChange::default();
        }
        let format = current.format;
        let default = spec.default_semantics;
        let change =
            self.animate_write_animation_at(address.layer, address.animation, move |animation| {
                let track = &mut animation.motions[address.motion].tracks[address.track];
                materialize_empty_track_keys(track);
                let key = existing.unwrap_or_else(|| {
                    channels::insert_key(&mut track.keys, format, frame, default)
                });
                channels::set_key_value(&mut track.keys, key, value);
                track.key_count = channels::key_count(&track.keys) as u16;
            });
        self.animate.selected_layer = address.layer;
        self.animate.selected_animation = Some(address.animation);
        self.animate.selected_track = Some((address.motion, address.track));
        self.animate.selected_key =
            self.animate_addressed_track_snapshot(address)
                .and_then(|(_, track)| {
                    (0..track_key_count(&track))
                        .find(|&key| channels::key_frame(&track.keys, key) == Some(frame))
                });
        self.animate_sync_key_drafts();
        change
    }

    fn animate_add_motion(&mut self, node: usize) -> ModelChange {
        let layer_index = self.animate.selected_layer;
        let Ok(target) = i32::try_from(node) else {
            return ModelChange::default();
        };
        let valid_target = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
            .is_some_and(|layer| is_authorable_motion_target(layer, node));
        if !valid_target {
            self.set_action_notice("Choose a concrete runtime CAST for a motion target.");
            return ModelChange::default();
        }
        if !self.animate_cast_mutation_allowed(layer_index, node) {
            return ModelChange::default();
        }
        let existing = self
            .animate_active_animation()
            .is_some_and(|(_, animation)| {
                animation
                    .motions
                    .iter()
                    .any(|motion| motion.target == target && !motion.tracks.is_empty())
            });
        if existing {
            self.set_action_notice("That cast already has a motion in this animation.");
            return ModelChange::default();
        }
        self.animate_write_animation(move |animation| {
            animation.motions.push(Motion {
                target,
                tracks: Vec::new(),
            });
        })
    }

    fn animate_delete_motion(&mut self, motion: usize) -> ModelChange {
        let Some((_, animation)) = self.animate_active_animation() else {
            return ModelChange::default();
        };
        let Some(current) = animation.motions.get(motion) else {
            return ModelChange::default();
        };
        let target = current.target;
        if let Ok(node) = usize::try_from(target)
            && !self.animate_cast_mutation_allowed(self.animate.selected_layer, node)
        {
            return ModelChange::default();
        }
        self.animate.selected_track = None;
        self.animate.selected_key = None;
        self.animate_write_animation(move |animation| {
            animation.motions.remove(motion);
        })
    }

    fn animate_set_motion_target(&mut self, motion: usize, node: usize) -> ModelChange {
        let layer_index = self.animate.selected_layer;
        let Ok(target) = i32::try_from(node) else {
            return ModelChange::default();
        };
        let Some((_, animation)) = self.animate_active_animation() else {
            return ModelChange::default();
        };
        let Some(current) = animation.motions.get(motion) else {
            return ModelChange::default();
        };
        let current_target = current.target;
        if current_target == target {
            return ModelChange::default();
        }
        let valid_target = self
            .selected_scene()
            .and_then(|scene| scene.layers.get(layer_index))
            .is_some_and(|layer| {
                is_authorable_motion_target(layer, node)
                    && current
                        .tracks
                        .iter()
                        .all(|track| is_authorable_track_target(layer, node, track.target))
            });
        if !valid_target {
            self.set_action_notice(
                "That CAST is padding, unsupported, or cannot consume this motion's channels.",
            );
            return ModelChange::default();
        }
        if let Ok(current_node) = usize::try_from(current_target)
            && !self.animate_cast_mutation_allowed(layer_index, current_node)
        {
            return ModelChange::default();
        }
        if !self.animate_cast_mutation_allowed(layer_index, node) {
            return ModelChange::default();
        }
        self.animate_write_animation(move |animation| {
            animation.motions[motion].target = target;
        })
    }

    fn animate_add_track(&mut self, motion: usize, target: u16) -> ModelChange {
        let Some(spec) = channels::channel(target) else {
            self.set_action_notice("That channel has no runtime consumer and cannot be authored.");
            return ModelChange::default();
        };
        let Some((_, animation)) = self.animate_active_animation() else {
            return ModelChange::default();
        };
        let Some(existing) = animation.motions.get(motion) else {
            return ModelChange::default();
        };
        let valid_target = usize::try_from(existing.target).ok().is_some_and(|node| {
            self.selected_scene()
                .and_then(|scene| scene.layers.get(self.animate.selected_layer))
                .is_some_and(|layer| is_authorable_track_target(layer, node, target))
        });
        if !valid_target {
            self.set_action_notice(
                "The motion target is padding, unsupported, or cannot consume that channel.",
            );
            return ModelChange::default();
        }
        let node = usize::try_from(existing.target).expect("validated track target");
        let duplicate = existing.tracks.iter().any(|track| track.target == target);
        let duration = animation.duration.max(1);
        if !self.animate_cast_mutation_allowed(self.animate.selected_layer, node) {
            return ModelChange::default();
        }
        if duplicate {
            self.set_action_notice("That channel already exists on this motion.");
            return ModelChange::default();
        }
        let format = spec.default_format;
        let Some(keys) = channels::empty_keys(format) else {
            return ModelChange::default();
        };
        self.animate_write_animation(move |animation| {
            if let Some(motion) = animation.motions.get_mut(motion) {
                motion.tracks.push(Track {
                    target,
                    key_count: 0,
                    format,
                    range_start: 0,
                    range_end: duration,
                    keys,
                });
            }
        })
    }

    fn animate_set_track_channel(
        &mut self,
        motion: usize,
        track: usize,
        target: u16,
    ) -> ModelChange {
        let Some(spec) = channels::channel(target) else {
            self.set_action_notice("That channel has no runtime consumer and cannot be authored.");
            return ModelChange::default();
        };
        let Some((current, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        if current.target == target {
            return ModelChange::default();
        }
        let Some((_, animation)) = self.animate_active_animation() else {
            return ModelChange::default();
        };
        let Some(existing) = animation.motions.get(motion) else {
            return ModelChange::default();
        };
        let valid_target = usize::try_from(existing.target).ok().is_some_and(|node| {
            self.selected_scene()
                .and_then(|scene| scene.layers.get(self.animate.selected_layer))
                .is_some_and(|layer| is_authorable_track_target(layer, node, target))
        });
        if !valid_target {
            self.set_action_notice(
                "The motion target is padding, unsupported, or cannot consume that channel.",
            );
            return ModelChange::default();
        }
        if existing
            .tracks
            .iter()
            .enumerate()
            .any(|(index, other)| index != track && other.target == target)
        {
            self.set_action_notice("That channel already exists on this motion.");
            return ModelChange::default();
        }

        let converted = if channels::format_choice(spec.value, current.format).is_some() {
            None
        } else {
            let format = spec.default_format;
            let Some(keys) = channels::convert_keys(&current.keys, format) else {
                self.set_action_notice("The existing keys cannot be converted for that channel.");
                return ModelChange::default();
            };
            Some((format, keys))
        };
        self.animate_write_track(motion, track, move |motion, index| {
            let track = &mut motion.tracks[index];
            track.target = target;
            if let Some((format, keys)) = converted {
                track.format = format;
                track.keys = keys;
            }
        })
    }

    fn animate_delete_track(&mut self, motion: usize, track: usize) -> ModelChange {
        let selected = self.animate.selected_track == Some((motion, track));
        let change = self.animate_write_track(motion, track, |motion, track| {
            motion.tracks.remove(track);
        });
        if selected && change.preview {
            self.animate.selected_track = None;
            self.animate.selected_key = None;
            self.animate_sync_key_drafts();
        }
        change
    }

    fn animate_write_track(
        &mut self,
        motion_index: usize,
        track_index: usize,
        write: impl FnOnce(&mut Motion, usize),
    ) -> ModelChange {
        let Some((_, animation)) = self.animate_active_animation() else {
            return ModelChange::default();
        };
        let Some(motion) = animation.motions.get(motion_index) else {
            return ModelChange::default();
        };
        if track_index >= motion.tracks.len() {
            return ModelChange::default();
        }
        let motion_target = motion.target;
        if let Ok(node) = usize::try_from(motion_target)
            && !self.animate_cast_mutation_allowed(self.animate.selected_layer, node)
        {
            return ModelChange::default();
        }
        self.animate_write_animation(move |animation| {
            let motion = animation
                .motions
                .get_mut(motion_index)
                .expect("validated motion");
            write(motion, track_index);
            for track in &mut motion.tracks {
                track.key_count = channels::key_count(&track.keys) as u16;
            }
        })
    }

    fn animate_set_track_format(
        &mut self,
        motion: usize,
        track: usize,
        format: u32,
    ) -> ModelChange {
        let Some((current_track, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        let wrapped = channels::with_wrap(format, channels::wraps(current_track.format));
        let Some(keys) = channels::convert_keys(&current_track.keys, wrapped) else {
            self.set_action_notice("That key format is not decodable by the runtime parser.");
            return ModelChange::default();
        };
        self.animate_write_track(motion, track, move |motion, index| {
            let track = &mut motion.tracks[index];
            track.format = wrapped;
            track.keys = keys;
        })
    }

    fn animate_set_track_wrap(&mut self, motion: usize, track: usize, wrap: bool) -> ModelChange {
        let Some((current, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        let format = channels::with_wrap(current.format, wrap);
        self.animate_write_track(motion, track, move |motion, index| {
            motion.tracks[index].format = format;
        })
    }

    fn animate_set_track_range(
        &mut self,
        motion: usize,
        track: usize,
        value: String,
        start: bool,
    ) -> ModelChange {
        if start {
            self.animate.drafts.track_range_start = value.clone();
        } else {
            self.animate.drafts.track_range_end = value.clone();
        }
        let Ok(frame) = value.trim().parse::<i32>() else {
            return ModelChange::default();
        };
        let Some((current, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        // `wrap_time` divides by the span, so an empty range is never written.
        if start && frame >= current.range_end {
            self.set_action_notice("Track range start must stay below the range end.");
            return ModelChange::default();
        }
        if !start && frame <= current.range_start {
            self.set_action_notice("Track range end must stay above the range start.");
            return ModelChange::default();
        }
        self.animate_write_track(motion, track, move |motion, index| {
            let track = &mut motion.tracks[index];
            if start {
                track.range_start = frame;
            } else {
                track.range_end = frame;
            }
        })
    }

    fn animate_track_snapshot(&self, motion: usize, track: usize) -> Option<(Track, u32)> {
        let (_, animation) = self.animate_active_animation()?;
        let track = animation.motions.get(motion)?.tracks.get(track)?;
        Some((track.clone(), track.format))
    }

    fn animate_add_key(&mut self, motion: usize, track: usize, frame: i32) -> ModelChange {
        let Some((current, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        let Some(spec) = channels::channel(current.target) else {
            return ModelChange::default();
        };
        if channels::format_choice(spec.value, current.format).is_none()
            || !track_storage_is_authorable(&current)
        {
            self.set_action_notice("The property track has an unsupported key format.");
            return ModelChange::default();
        }
        if track_key_count(&current) >= usize::from(u16::MAX) {
            self.set_action_notice("A track cannot contain more than 65,535 keys.");
            return ModelChange::default();
        }
        let default = spec.default_semantics;
        let format = current.format;
        let change = self.animate_write_track(motion, track, move |motion, index| {
            let track = &mut motion.tracks[index];
            materialize_empty_track_keys(track);
            channels::insert_key(&mut track.keys, format, frame, default);
        });
        self.animate.selected_track = Some((motion, track));
        self.animate.selected_key =
            self.animate_track_snapshot(motion, track)
                .and_then(|(track, _)| {
                    (0..channels::key_count(&track.keys))
                        .find(|&index| channels::key_frame(&track.keys, index) == Some(frame))
                });
        self.animate_sync_key_drafts();
        change
    }

    fn animate_delete_key(&mut self, motion: usize, track: usize, key: usize) -> ModelChange {
        let change = self.animate_write_track(motion, track, move |motion, index| {
            channels::remove_key(&mut motion.tracks[index].keys, key);
        });
        self.animate.selected_key = None;
        self.animate_sync_key_drafts();
        change
    }

    fn animate_move_key(
        &mut self,
        motion: usize,
        track: usize,
        key: usize,
        frame: i32,
    ) -> ModelChange {
        let change = self.animate_write_track(motion, track, move |motion, index| {
            channels::set_key_frame(&mut motion.tracks[index].keys, key, frame);
        });
        self.animate.selected_key =
            self.animate_track_snapshot(motion, track)
                .and_then(|(track, _)| {
                    (0..channels::key_count(&track.keys))
                        .find(|&index| channels::key_frame(&track.keys, index) == Some(frame))
                });
        self.animate_sync_key_drafts();
        change
    }

    fn animate_selected_key_target(&self) -> Option<(usize, usize, usize, u16)> {
        let (motion, track) = self.animate.selected_track?;
        let key = self.animate.selected_key?;
        let (current, _) = self.animate_track_snapshot(motion, track)?;
        Some((motion, track, key, current.target))
    }

    fn animate_set_key_value(&mut self, value: String) -> ModelChange {
        self.animate.drafts.key_value = value.clone();
        let Some((motion, track, key, target)) = self.animate_selected_key_target() else {
            return ModelChange::default();
        };
        let Some(parsed) = parse_track_value(target, &value) else {
            return ModelChange::default();
        };
        self.animate_write_track(motion, track, move |motion, index| {
            channels::set_key_value(&mut motion.tracks[index].keys, key, parsed);
        })
    }

    fn animate_set_key_byte(&mut self, component: usize, value: String) -> ModelChange {
        if component >= 4 {
            return ModelChange::default();
        }
        self.animate.drafts.key_rgba[component] = value.clone();
        let Ok(byte) = value.trim().parse::<u8>() else {
            return ModelChange::default();
        };
        let Some((motion, track, key, _)) = self.animate_selected_key_target() else {
            return ModelChange::default();
        };
        let Some((current, _)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        let Some(view) = channels::key_view(&current.keys, key) else {
            return ModelChange::default();
        };
        let mut bytes = view.value.as_rgba();
        bytes[component] = byte;
        self.animate_write_track(motion, track, move |motion, index| {
            channels::set_key_value(&mut motion.tracks[index].keys, key, KeyValue::Rgba(bytes));
        })
    }

    fn animate_set_key_semantics(&mut self, semantics: KeySemantics) -> ModelChange {
        let Some((motion, track, key, _)) = self.animate_selected_key_target() else {
            return ModelChange::default();
        };
        let Some((current, format)) = self.animate_track_snapshot(motion, track) else {
            return ModelChange::default();
        };
        if !channels::available_semantics(format).contains(&semantics) {
            self.set_action_notice("The runtime does not evaluate that mode for this key format.");
            return ModelChange::default();
        }
        let _ = current;
        self.animate_write_track(motion, track, move |motion, index| {
            let track = &mut motion.tracks[index];
            channels::set_key_semantics(&mut track.keys, format, key, semantics);
        })
    }

    fn animate_set_key_slope(&mut self, incoming: bool, value: String) -> ModelChange {
        if incoming {
            self.animate.drafts.slope_in = value.clone();
        } else {
            self.animate.drafts.slope_out = value.clone();
        }
        let Ok(slope) = value.trim().parse::<f32>() else {
            return ModelChange::default();
        };
        let Some((motion, track, key, _)) = self.animate_selected_key_target() else {
            return ModelChange::default();
        };
        self.animate_write_track(motion, track, move |motion, index| {
            channels::set_key_slope(&mut motion.tracks[index].keys, key, incoming, slope);
        })
    }

    fn animate_set_frame(&mut self, frame: i32) -> ModelChange {
        let (start, end) = self.animate_play_range();
        let frame = frame.clamp(start, end);
        if frame == self.animate.frame {
            return ModelChange::default();
        }
        self.animate.frame = frame;
        self.sync_inspector();
        ModelChange::PREVIEW
    }

    fn animate_jump_track_key(&mut self, address: TrackAddress, next: bool) -> ModelChange {
        let current = self.animate.frame;
        let (range_start, range_end) = self.animate_play_range();
        let target = {
            let Some(track) = self
                .selected_scene()
                .and_then(|scene| scene.layers.get(address.layer))
                .and_then(|layer| layer.animations.get(address.animation))
                .and_then(|animation| animation.motions.get(address.motion))
                .and_then(|motion| motion.tracks.get(address.track))
            else {
                return ModelChange::default();
            };
            let candidates = (0..channels::key_count(&track.keys)).filter_map(|index| {
                let frame = channels::key_frame(&track.keys, index)?;
                (frame >= range_start
                    && frame <= range_end
                    && if next {
                        frame > current
                    } else {
                        frame < current
                    })
                .then_some((index, frame))
            });
            if next {
                candidates.min_by_key(|(_, frame)| *frame)
            } else {
                candidates.max_by_key(|(_, frame)| *frame)
            }
        };
        let Some((key, frame)) = target else {
            self.set_action_notice(if next {
                "No next keyframe on this property."
            } else {
                "No previous keyframe on this property."
            });
            return ModelChange::default();
        };
        let _ = self.animate_select_key_address(address, key);
        let _ = self.animate_set_frame(frame);
        ModelChange::PREVIEW
    }

    /// Every-frame playback: the next frame is only scheduled once the previous
    /// preview allocation has completed, so no frame is ever skipped.
    fn animate_frame_completed(&mut self) -> ModelChange {
        if !self.animate.playing {
            return ModelChange::default();
        }
        let (start, end) = self.animate_play_range();
        let next = if self.animate.frame >= end {
            start
        } else {
            self.animate.frame + 1
        };
        if next == self.animate.frame {
            return ModelChange::default();
        }
        self.animate.frame = next;
        self.sync_inspector();

        ModelChange::PREVIEW
    }

    fn animate_set_work(&mut self, value: String, start: bool) -> ModelChange {
        if start {
            self.animate.drafts.work_start = value.clone();
        } else {
            self.animate.drafts.work_end = value.clone();
        }
        let Ok(frame) = value.trim().parse::<i32>() else {
            return ModelChange::default();
        };
        let (current_start, current_end) = self.animate_play_range();
        let range = if start {
            (frame.min(current_end), current_end)
        } else {
            (current_start, frame.max(current_start))
        };
        self.animate.work_area = Some(range);
        self.animate_clamp_frame();
        self.animate_rebuild_rows();
        ModelChange::PREVIEW
    }

    /// Temporary work area covering every key, including keys past the runtime
    /// duration. It is editor-only and never dirties the document.
    fn animate_fit_all_keys(&mut self) -> ModelChange {
        if self.animate.sheet.keys.is_empty() {
            self.set_action_notice("No keys to fit; the work area is unchanged.");
            return ModelChange::default();
        }
        let (runtime_start, runtime_end) = self.animate_runtime_range();
        let start = self.animate.sheet.key_min.min(runtime_start);
        let end = self.animate.sheet.key_max.max(runtime_end);
        self.animate.work_area = Some((start, end));
        // Fit must expose authored negative frames rather than leaving them
        // beyond a non-negative timeline origin.
        self.animate.pan = start as f32;
        self.animate_clamp_frame();
        self.animate_rebuild_rows();
        self.animate_sync_drafts();
        ModelChange::PREVIEW
    }

    pub(super) fn animate_history(&self) -> AnimateHistory {
        self.animate.history()
    }

    pub(super) fn animate_restore_history(&mut self, history: AnimateHistory) {
        let scene = history.scene;
        if self
            .document()
            .is_some_and(|document| scene < document.project.scenes.len())
        {
            self.select_scene(scene);
        }
        self.animate.restore(history);
    }
}

fn cast_label(layer: &Layer, node: usize) -> String {
    layer
        .nodes
        .get(node)
        .and_then(|record| record.name.as_deref())
        .map(display_srd_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Cast {node}"))
}

fn nonempty(raw: &[u8], fallback: String) -> String {
    let name = display_srd_name(raw);
    if name.is_empty() { fallback } else { name }
}

fn parse_track_value(target: u16, value: &str) -> Option<KeyValue> {
    match channels::value_kind(target) {
        ChannelValue::Float => value
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .map(KeyValue::Float),
        ChannelValue::Integer | ChannelValue::Selector => {
            value.trim().parse::<i32>().ok().map(KeyValue::Integer)
        }
        ChannelValue::Rgba => parse_rgba(value.trim()).map(KeyValue::Rgba),
    }
}

fn parse_rgba(value: &str) -> Option<[u8; 4]> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 8 {
        return None;
    }
    let mut bytes = [0u8; 4];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Layer count of the scene an assignment belongs to, used by tests and the UI
/// to confirm the assignment stays dense.
pub fn assignment_is_dense(assignment: &AnimationSetDefinition, scene: &Scene) -> bool {
    assignment.slots.len() == scene.layers.len()
        && assignment.declared_slot_count as usize == assignment.slots.len()
}

/// Project-wide helper for the UI: every stored set of the selected scene.
pub fn stored_set_labels(project: &Project, scene_index: usize) -> Vec<(usize, String)> {
    project
        .scenes
        .get(scene_index)
        .map(|scene| {
            scene
                .animation_sets
                .iter()
                .enumerate()
                .map(|(index, set)| (index, nonempty(&set.name, format!("ANMS {index}"))))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Key8, Key20, KeyData, Track};
    use crate::document::EditorDocument;
    use crate::editor::model::{EditorAction, TransformField};
    use crate::serialized_flags::ANIMATION_FLAGS;
    use std::path::PathBuf;

    fn temporary_srd_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "srd-editor-animate-{label}-{}-{}.srd",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    /// Two-layer scene: layer 0 owns a parent CAST with one child, an
    /// animation whose single MOT targets the parent, and a `target = -1`
    /// zero-track padding motion. Layer 1 has no animations.
    fn authored_model(label: &str) -> (EditorModel, PathBuf) {
        let path = temporary_srd_path(label);
        std::fs::write(
            &path,
            crate::document::tests::minimal_two_layer_two_node_trs2_srd(),
        )
        .unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        {
            let scene = &mut document.project.scenes[0];
            let layer = &mut scene.layers[0];
            layer.animations = vec![AnimationDefinition {
                name: b"move".to_vec(),
                flags: 0,
                declared_motion_count: 2,
                duration: 30,
                motions: vec![
                    Motion {
                        target: 0,
                        tracks: vec![Track {
                            target: 0,
                            key_count: 2,
                            format: 0x13,
                            range_start: 0,
                            range_end: 30,
                            keys: KeyData::Key20F32(vec![
                                Key20 {
                                    frame: 0,
                                    value: 0.0,
                                    mode: 1,
                                    slope_in: 0.0,
                                    slope_out: 0.0,
                                },
                                Key20 {
                                    frame: 45,
                                    value: 20.0,
                                    mode: 1,
                                    slope_in: 0.0,
                                    slope_out: 0.0,
                                },
                            ]),
                        }],
                    },
                    // Padding convention measured across the shipped corpus.
                    Motion {
                        target: -1,
                        tracks: Vec::new(),
                    },
                ],
            }];
            layer.animation_count = 1;
            scene.animation_sets = vec![AnimationSetDefinition {
                name: b"stored".to_vec(),
                start_frame: 0,
                runtime_duration: 30,
                declared_slot_count: 2,
                slots: vec![
                    SceneAnimationSlot {
                        animation_name: b"move".to_vec(),
                        enabled: 1,
                    },
                    SceneAnimationSlot {
                        animation_name: Vec::new(),
                        enabled: 1,
                    },
                ],
            }];
            scene.declared_animation_set_count = 1;
        }
        document.save().expect("serialized animation fixture");

        let mut model = EditorModel::with_workspace(Some(path.clone()), Workspace::Animate);
        model.animate_sync();
        (model, path)
    }

    fn cleanup(path: PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    fn act(model: &mut EditorModel, action: AnimateAction) {
        model.update(EditorAction::Animate(action));
    }

    #[test]
    fn loading_a_stored_set_clones_dense_slots_without_dirtying() {
        let (mut model, path) = authored_model("load-dense");
        let baseline = model.document().unwrap().project.clone();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));

        let scene = model.selected_scene().unwrap();
        assert!(assignment_is_dense(model.animate().assignment(), scene));
        assert_eq!(model.animate().source_set(), Some(0));
        assert!(!model.animate().assignment_is_scratch(Some(scene)));
        assert_eq!(model.document().unwrap().project, baseline);
        cleanup(path);
    }

    #[test]
    fn initial_animate_workspace_uses_layer_flags_for_scratch_assignment() {
        let path = temporary_srd_path("initial-scratch");
        std::fs::write(
            &path,
            crate::document::tests::minimal_two_layer_two_node_trs2_srd(),
        )
        .unwrap();

        let model = EditorModel::with_workspace(Some(path.clone()), Workspace::Animate);

        assert!(
            model
                .animate()
                .assignment()
                .slots
                .iter()
                .all(SceneAnimationSlot::is_enabled)
        );
        cleanup(path);
    }

    #[test]
    fn animation_loop_toggle_preserves_unlisted_bits() {
        let (mut model, path) = authored_model("animation-loop-mask");
        model.document_mut().unwrap().project.scenes[0].layers[0].animations[0].flags = 0x8000_0000;
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        model.update(EditorAction::ToggleDocumentEditing);

        act(&mut model, AnimateAction::SetAnimationLoop(true));
        let enabled = model.animate_active_animation().unwrap().1.flags;
        assert_eq!(enabled, 0x8000_0001);
        assert_eq!(ANIMATION_FLAGS.unknown_set_bits(enabled), 0x8000_0000);

        act(&mut model, AnimateAction::SetAnimationLoop(false));
        assert_eq!(
            model.animate_active_animation().unwrap().1.flags,
            0x8000_0000
        );
        cleanup(path);
    }

    #[test]
    fn animation_loop_toggle_respects_the_selected_layer_lock() {
        let (mut model, path) = authored_model("animation-loop-lock");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        model.update(EditorAction::ToggleDocumentEditing);
        model.update(EditorAction::ToggleLayerLocked(0));

        act(&mut model, AnimateAction::SetAnimationLoop(true));

        assert_eq!(model.animate_active_animation().unwrap().1.flags, 0);
        assert_eq!(
            model.take_action_notice().as_deref(),
            Some("Unlock the selected layer before editing its animation flags.")
        );
        cleanup(path);
    }

    #[test]
    fn property_authoring_is_edit_gated_and_undoable() {
        let (mut model, path) = authored_model("property-edit-gate");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let baseline = model.document().unwrap().project.clone();

        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        assert!(
            model
                .animate_active_animation()
                .unwrap()
                .1
                .motions
                .iter()
                .all(|motion| motion.target != 1)
        );
        assert_eq!(model.document().unwrap().project, baseline);

        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let motion = model
            .animate_active_animation()
            .unwrap()
            .1
            .motions
            .iter()
            .find(|motion| motion.target == 1)
            .expect("property creates its CAST motion");
        assert_eq!(motion.tracks[0].key_count, 1);
        assert_eq!(channels::key_frame(&motion.tracks[0].keys, 0), Some(0));
        assert!(model.update(EditorAction::Undo).preview);
        assert!(
            model
                .animate_active_animation()
                .unwrap()
                .1
                .motions
                .iter()
                .all(|motion| motion.target != 1)
        );
        cleanup(path);
    }

    #[test]
    fn add_property_reuses_a_matching_empty_motion() {
        let (mut model, path) = authored_model("property-reuses-motion");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        {
            let animation =
                &mut model.document_mut().unwrap().project.scenes[0].layers[0].animations[0];
            animation.motions.push(Motion {
                target: 1,
                tracks: vec![Track {
                    target: 0,
                    key_count: 0,
                    format: 0x13,
                    range_start: 0,
                    range_end: 30,
                    keys: KeyData::Unsupported,
                }],
            });
            animation.declared_motion_count = animation.motions.len() as u32;
        }
        model.bump_revision();
        model.animate_sync();
        let motion_count = model.animate_active_animation().unwrap().1.motions.len();

        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let animation = model.animate_active_animation().unwrap().1;
        assert_eq!(animation.motions.len(), motion_count);
        let motion = animation
            .motions
            .iter()
            .find(|motion| motion.target == 1)
            .expect("reused motion");
        assert_eq!(motion.tracks.len(), 1);
        assert_eq!(motion.tracks[0].key_count, 1);
        assert_eq!(
            model.animate().selected_track(),
            Some((motion_count - 1, 0))
        );
        assert_eq!(model.animate().selected_key(), Some(0));
        cleanup(path);
    }

    #[test]
    fn selecting_an_animation_does_not_preview_until_toggled() {
        let (mut model, path) = authored_model("explicit-preview");
        act(&mut model, AnimateAction::LoadStoredSet(None));
        let assignment = model.animate().assignment().clone();
        let baseline = model.document().unwrap().project.clone();

        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        assert!(model.animate().preview().is_none());
        assert_eq!(model.animate().effective_assignment(), &assignment);

        act(&mut model, AnimateAction::ToggleAnimationPreview);
        let preview = model
            .animate()
            .preview()
            .expect("explicit preview is active");
        assert_eq!(preview.layer, 0);
        assert_eq!(preview.animation_name, b"move");
        assert!(model.animate().effective_assignment().slots[0].is_enabled());
        assert_eq!(
            model.animate().effective_assignment().slots[0].animation_name,
            b"move"
        );
        assert!(!model.animate().effective_assignment().slots[1].is_enabled());
        assert_eq!(model.animate().assignment(), &assignment);
        assert_eq!(model.document().unwrap().project, baseline);

        act(&mut model, AnimateAction::ToggleAnimationPreview);
        assert!(model.animate().preview().is_none());
        assert_eq!(model.animate().effective_assignment(), &assignment);
        cleanup(path);
    }

    #[test]
    fn scratch_bindings_enable_temporarily_without_editing_the_project() {
        let (mut model, path) = authored_model("scratch-bindings");
        act(&mut model, AnimateAction::LoadStoredSet(None));
        let baseline = model.document().unwrap().project.clone();
        assert!(!model.document_editing());

        act(&mut model, AnimateAction::AssignSlotAnimation(0, Some(0)));
        let slot = &model.animate().assignment().slots[0];
        assert!(slot.is_enabled());
        assert_eq!(slot.animation_name, b"move");
        assert_eq!(model.animate().source_set(), None);
        assert_eq!(model.document().unwrap().project, baseline);

        act(&mut model, AnimateAction::AssignSlotAnimation(0, None));
        let slot = &model.animate().assignment().slots[0];
        assert!(slot.is_enabled());
        assert!(slot.animation_name.is_empty());
        assert!(model.animate().effective_assignment().slots[0].is_enabled());
        assert!(model.animate().assignment_rows()[0].enabled);
        assert_eq!(
            model.animate().assignment_rows()[0].status,
            SlotStatus::EnabledBasePose
        );
        assert_eq!(model.document().unwrap().project, baseline);
        cleanup(path);
    }

    #[test]
    fn an_enabled_empty_slot_reports_base_pose_and_still_renders() {
        let (mut model, path) = authored_model("enabled-empty");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let row = &model.animate().assignment_rows()[1];
        assert_eq!(row.status, SlotStatus::EnabledBasePose);
        assert_eq!(row.status.label(), "Enabled · Base pose");
        assert!(row.enabled);
        assert!(!row.manual_hidden);
        cleanup(path);
    }

    #[test]
    fn manual_hide_is_independent_of_the_assignment_enable_flag() {
        let (mut model, path) = authored_model("manual-hide");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleLayerVisible(1));
        model.animate_sync();

        let row = &model.animate().assignment_rows()[1];
        assert!(row.enabled, "manual hide must not clear the slot flag");
        assert!(row.manual_hidden);
        assert!(model.hidden_layers().contains(&1));
        assert!(model.animate().effective_assignment().slots[1].is_enabled());

        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::ToggleSlotEnabled(1));
        model.animate_sync();
        let row = &model.animate().assignment_rows()[1];
        assert!(!row.enabled);
        assert!(row.manual_hidden, "disabling a slot must not unhide");
        cleanup(path);
    }

    #[test]
    fn direct_targets_ignore_placeholder_motions_and_descendants_inherit() {
        let (mut model, path) = authored_model("direct-inherited");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let layer = &model.selected_scene().unwrap().layers[0];
        let animation = &layer.animations[0];
        let targets = motion_targets(layer, animation);

        assert_eq!(targets.direct, BTreeSet::from([0]));
        assert_eq!(targets.inherited, BTreeSet::from([1]));

        let highlights = model.preview_highlights();
        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].kind, PreviewHighlightKind::Direct);
        assert_eq!(highlights[0].selection.node_index, 0);
        assert_eq!(highlights[1].kind, PreviewHighlightKind::Inherited);
        assert_eq!(highlights[1].selection.node_index, 1);
        cleanup(path);
    }

    #[test]
    fn a_zero_track_motion_never_creates_a_direct_target() {
        let (mut model, path) = authored_model("zero-track");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        // This is a parsed-data classification case, not a public authoring
        // action: property authoring always creates its first track and key.
        let animation =
            &mut model.document_mut().unwrap().project.scenes[0].layers[0].animations[0];
        animation.motions.push(Motion {
            target: 1,
            tracks: Vec::new(),
        });
        animation.declared_motion_count = animation.motions.len() as u32;
        model.bump_revision();
        model.animate_sync();

        let layer = &model.selected_scene().unwrap().layers[0];
        let animation = &layer.animations[0];
        assert!(animation.motions.iter().any(|motion| motion.target == 1));
        let targets = motion_targets(layer, animation);
        assert!(!targets.direct.contains(&1));
        cleanup(path);
    }

    #[test]
    fn selection_auditions_and_only_document_edit_mode_mutates_the_document() {
        let (mut model, path) = authored_model("edit-gate");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let baseline = model.document().unwrap().project.clone();

        act(&mut model, AnimateAction::ToggleSlotEnabled(0));
        assert_eq!(model.document().unwrap().project, baseline);
        assert!(!model.is_dirty());

        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::ToggleSlotEnabled(0));
        assert!(model.is_dirty());
        let edited = model.document().unwrap().project.clone();

        model.update(EditorAction::ToggleDocumentEditing);
        assert!(!model.document_editing());
        act(&mut model, AnimateAction::ToggleSlotEnabled(0));
        assert_eq!(model.document().unwrap().project, edited);
        cleanup(path);
    }

    #[test]
    fn document_edit_mode_persists_across_workspaces() {
        let (mut model, path) = authored_model("cross-workspace-edit");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        assert!(model.document_editing());

        assert!(
            model
                .update(EditorAction::SetWorkspace(Workspace::Design))
                .preview
        );
        assert!(model.document_editing());

        let baseline = model.document().expect("open SRD").project.clone();
        assert!(
            model
                .update(EditorAction::EditTransform(
                    TransformField::PositionX,
                    "9".into(),
                ))
                .preview
        );
        assert_ne!(model.document().expect("open SRD").project, baseline);

        model.update(EditorAction::ToggleDocumentEditing);
        assert!(!model.document_editing());
        cleanup(path);
    }

    #[test]
    fn leaving_animate_clears_explicit_preview() {
        let (mut model, path) = authored_model("preview-workspace-switch");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        let assignment = model.animate().assignment().clone();

        act(&mut model, AnimateAction::ToggleAnimationPreview);
        assert!(model.animate().preview().is_some());
        assert_ne!(model.animate().effective_assignment(), &assignment);

        model.update(EditorAction::SetWorkspace(Workspace::Design));
        assert!(model.animate().preview().is_none());
        assert_eq!(model.animate().effective_assignment(), &assignment);

        model.update(EditorAction::SetWorkspace(Workspace::Animate));
        assert!(model.animate().preview().is_none());
        assert_eq!(model.preview_assignment(), Some(&assignment));
        cleanup(path);
    }

    #[test]
    fn animation_set_navigation_keeps_document_edits() {
        let (mut model, path) = authored_model("set-navigation");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetDuration("48".into()));

        act(&mut model, AnimateAction::LoadStoredSet(None));
        assert_eq!(model.animate().source_set(), None);
        assert_eq!(
            model.selected_scene().unwrap().animation_sets[0].runtime_duration,
            48,
        );
        assert!(model.document_editing());

        model.update(EditorAction::ToggleDocumentEditing);
        assert!(!model.document_editing());
        cleanup(path);
    }

    #[test]
    fn property_actions_are_undoable_in_one_step_each() {
        let (mut model, path) = authored_model("property-history");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let motion_index = model
            .animate_active_animation()
            .unwrap()
            .1
            .motions
            .iter()
            .position(|motion| motion.target == 1)
            .expect("property creates a motion");
        let address = TrackAddress::new(0, 0, motion_index, 0);
        let track = &model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0];
        assert_eq!(track.target, 0);
        assert_eq!(track.key_count, 1);

        act(&mut model, AnimateAction::SetFrame(12));
        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 12));
        let track = &model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0];
        assert_eq!(channels::key_count(&track.keys), 2);
        assert_eq!(model.animate().selected_key(), Some(1));

        assert!(model.update(EditorAction::Undo).preview);
        let track = &model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0];
        assert_eq!(channels::key_count(&track.keys), 1);

        assert!(model.update(EditorAction::Redo).preview);
        let track = &model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0];
        assert_eq!(channels::key_count(&track.keys), 2);
        assert!(model.update(EditorAction::Undo).preview);
        let track = &model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0];
        assert_eq!(channels::key_count(&track.keys), 1);

        act(&mut model, AnimateAction::ToggleTrackAnimation(address));
        assert!(
            model
                .animate_active_animation()
                .unwrap()
                .1
                .motions
                .iter()
                .all(|motion| motion.target != 1)
        );
        assert!(model.update(EditorAction::Undo).preview);
        assert_eq!(
            model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0].key_count,
            1
        );

        assert!(model.update(EditorAction::Undo).preview);
        assert!(
            model
                .animate_active_animation()
                .unwrap()
                .1
                .motions
                .iter()
                .all(|motion| motion.target != 1)
        );
        assert!(model.update(EditorAction::Redo).preview);
        assert_eq!(
            model.animate_active_animation().unwrap().1.motions[motion_index].tracks[0].key_count,
            1
        );
        cleanup(path);
    }

    #[test]
    fn diamond_toggles_only_the_exact_playhead_key() {
        let (mut model, path) = authored_model("diamond-exact-frame");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let motion = model
            .animate_active_animation()
            .unwrap()
            .1
            .motions
            .iter()
            .position(|motion| motion.target == 1)
            .expect("property motion");
        let address = TrackAddress::new(0, 0, motion, 0);
        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 12));
        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 13));
        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 12));

        let keys = &model.animate_active_animation().unwrap().1.motions[motion].tracks[0].keys;
        assert_eq!(
            (0..channels::key_count(keys))
                .map(|index| channels::key_frame(keys, index).unwrap())
                .collect::<Vec<_>>(),
            vec![0, 13]
        );
        assert_eq!(model.animate().selected_track(), Some((motion, 0)));
        assert_eq!(model.animate().selected_key(), None);
        cleanup(path);
    }

    #[test]
    fn typed_property_values_auto_key_and_replace_at_the_same_frame() {
        let (mut model, path) = authored_model("typed-auto-key");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let motion = model
            .animate_active_animation()
            .unwrap()
            .1
            .motions
            .iter()
            .position(|motion| motion.target == 1)
            .expect("property motion");
        let address = TrackAddress::new(0, 0, motion, 0);
        act(
            &mut model,
            AnimateAction::SetTrackValueAtFrame(address, 12, "7.5".into()),
        );
        act(
            &mut model,
            AnimateAction::SetTrackValueAtFrame(address, 12, "9.25".into()),
        );

        let track = &model.animate_active_animation().unwrap().1.motions[motion].tracks[0];
        assert_eq!(track.key_count, 2);
        assert_eq!(channels::key_frame(&track.keys, 1), Some(12));
        assert_eq!(
            channels::key_view(&track.keys, 1).unwrap().value,
            KeyValue::Float(9.25)
        );
        assert_eq!(model.animate().selected_key(), Some(1));
        cleanup(path);
    }

    #[test]
    fn locked_cast_ancestors_make_property_tracks_read_only() {
        let (mut model, path) = authored_model("locked-property");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );
        let motion = model
            .animate_active_animation()
            .unwrap()
            .1
            .motions
            .iter()
            .position(|motion| motion.target == 1)
            .expect("property motion");
        let address = TrackAddress::new(0, 0, motion, 0);
        model.update(EditorAction::ToggleCastLocked(0, 0, 0));
        let baseline = model.document().unwrap().project.clone();

        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 0));
        act(
            &mut model,
            AnimateAction::SetTrackValueAtFrame(address, 12, "4".into()),
        );
        act(&mut model, AnimateAction::ToggleTrackAnimation(address));

        assert_eq!(model.document().unwrap().project, baseline);
        assert_eq!(model.animate().selected_track(), Some((motion, 0)));
        cleanup(path);
    }

    #[test]
    fn deleting_a_referenced_animation_requires_confirmation_and_clears_slots() {
        let (mut model, path) = authored_model("delete-referenced");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SelectAnimation(0, 0));

        act(&mut model, AnimateAction::DeleteAnimation);
        let pending = model.animate().pending_delete().expect("confirmation");
        assert_eq!(pending.referencing_sets, 1);
        assert_eq!(
            model.selected_scene().unwrap().layers[0].animations.len(),
            1
        );

        act(&mut model, AnimateAction::ConfirmDelete);
        assert!(
            model.selected_scene().unwrap().layers[0]
                .animations
                .is_empty()
        );
        assert!(
            model.selected_scene().unwrap().animation_sets[0].slots[0]
                .animation_name
                .is_empty(),
            "no slot may dangle after the delete"
        );
        cleanup(path);
    }

    #[test]
    fn pending_delete_follows_the_animation_identity_after_reindexing() {
        let (mut model, path) = authored_model("delete-reindexed");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        act(&mut model, AnimateAction::DeleteAnimation);
        assert!(model.animate().pending_delete().is_some());

        // An intervening structural change shifts the requested animation from
        // index 0 to index 1. Confirm must remove `move`, not this new entry.
        model.document_mut().expect("open SRD").project.scenes[0].layers[0]
            .animations
            .insert(
                0,
                AnimationDefinition {
                    name: b"intervening".to_vec(),
                    flags: 0,
                    declared_motion_count: 0,
                    duration: 30,
                    motions: Vec::new(),
                },
            );
        act(&mut model, AnimateAction::ConfirmDelete);

        let layer = &model.selected_scene().expect("scene").layers[0];
        assert!(layer.find_animation(b"intervening").is_some());
        assert!(layer.find_animation(b"move").is_none());
        assert!(
            model.selected_scene().expect("scene").animation_sets[0].slots[0]
                .animation_name
                .is_empty()
        );
        cleanup(path);
    }

    #[test]
    fn renaming_an_animation_rewrites_every_slot_that_named_it() {
        let (mut model, path) = authored_model("rename");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        act(&mut model, AnimateAction::SetAnimationName("glide".into()));

        assert_eq!(
            model.selected_scene().unwrap().layers[0].animations[0].name,
            b"glide"
        );
        assert_eq!(
            model.selected_scene().unwrap().animation_sets[0].slots[0].animation_name,
            b"glide"
        );
        assert_eq!(
            model.animate().assignment().slots[0].animation_name,
            b"glide"
        );
        cleanup(path);
    }

    #[test]
    fn channel_eighteen_can_never_be_authored_or_shown() {
        let (mut model, path) = authored_model("channel-18");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        {
            let tracks = &mut model.document_mut().unwrap().project.scenes[0].layers[0].animations
                [0]
            .motions[0]
                .tracks;
            tracks.push(Track {
                target: 18,
                key_count: 1,
                format: 0x13,
                range_start: 0,
                range_end: 30,
                keys: KeyData::Key20F32(vec![Key20 {
                    frame: 0,
                    value: 1.0,
                    mode: 1,
                    slope_in: 0.0,
                    slope_out: 0.0,
                }]),
            });
        }
        model.bump_revision();
        model.animate_sync();
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 0,
                target: 18,
            },
        );

        let animation = model.animate_active_animation().unwrap().1;
        assert_eq!(
            animation
                .motions
                .iter()
                .flat_map(|motion| &motion.tracks)
                .filter(|track| track.target == 18)
                .count(),
            1
        );
        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .all(|row| row.target != Some(18))
        );
        assert!(channels::channel(18).is_none());
        cleanup(path);
    }

    #[test]
    fn track_channel_updates_are_gated_preserve_legal_keys_and_reject_duplicates() {
        let (mut model, path) = authored_model("track-channel");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        let original = model.animate_active_animation().unwrap().1.motions[0].tracks[0].clone();

        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 1));
        assert_eq!(
            model.animate_active_animation().unwrap().1.motions[0].tracks[0],
            original
        );

        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 1));
        let legal = model.animate_active_animation().unwrap().1.motions[0].tracks[0].clone();
        assert_eq!(legal.target, 1);
        assert_eq!(legal.format, original.format);
        assert_eq!(legal.keys, original.keys);

        let expected_keys =
            channels::convert_keys(&legal.keys, channels::channel(9).unwrap().default_format)
                .unwrap();
        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 9));
        let converted = model.animate_active_animation().unwrap().1.motions[0].tracks[0].clone();
        assert_eq!(converted.target, 9);
        assert_eq!(
            converted.format,
            channels::channel(9).unwrap().default_format
        );
        assert_eq!(converted.keys, expected_keys);

        act(&mut model, AnimateAction::AddTrack(0, 21));
        let before_rejection =
            model.animate_active_animation().unwrap().1.motions[0].tracks[0].clone();
        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 21));
        assert_eq!(
            model.animate_active_animation().unwrap().1.motions[0].tracks[0],
            before_rejection
        );
        assert!(
            model
                .take_action_notice()
                .is_some_and(|notice| notice.contains("already exists"))
        );

        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 18));
        act(&mut model, AnimateAction::SetTrackChannel(0, 0, 99));
        assert_eq!(
            model.animate_active_animation().unwrap().1.motions[0].tracks[0],
            before_rejection
        );
        cleanup(path);
    }

    #[test]
    fn property_key_arrows_navigate_only_their_track_in_read_only_mode() {
        let (mut model, path) = authored_model("property-key-arrows");
        {
            let motion = &mut model.document_mut().unwrap().project.scenes[0].layers[0].animations
                [0]
            .motions[0];
            let KeyData::Key20F32(keys) = &mut motion.tracks[0].keys else {
                panic!("fixture track format changed");
            };
            keys[1].frame = 20;
            motion.tracks[0].range_end = 30;
            motion.tracks.push(Track {
                target: 1,
                key_count: 1,
                format: 0x13,
                range_start: 0,
                range_end: 30,
                keys: KeyData::Key20F32(vec![Key20 {
                    frame: 5,
                    value: 1.0,
                    mode: 1,
                    slope_in: 0.0,
                    slope_out: 0.0,
                }]),
            });
        }
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let baseline = model.document().unwrap().project.clone();
        let address = TrackAddress::new(0, 0, 0, 0);

        act(&mut model, AnimateAction::JumpToNextTrackKey(address));
        assert_eq!(
            model.animate().frame(),
            20,
            "the other track's F5 key is ignored"
        );
        assert_eq!(model.animate().selected_track(), Some((0, 0)));
        assert_eq!(model.animate().selected_key(), Some(1));

        act(&mut model, AnimateAction::JumpToPreviousTrackKey(address));
        assert_eq!(model.animate().frame(), 0);
        assert_eq!(model.animate().selected_key(), Some(0));
        assert!(!model.document_editing());
        assert_eq!(model.document().unwrap().project, baseline);
        cleanup(path);
    }

    #[test]
    fn every_frame_playback_advances_only_on_completion() {
        let (mut model, path) = authored_model("every-frame");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::TogglePlaying);
        assert!(model.animate().playing());
        assert_eq!(model.animate().frame(), 0);

        for expected in 1..=3 {
            let change = model.update(EditorAction::Animate(AnimateAction::FrameCompleted));
            assert!(change.preview);
            assert_eq!(model.animate().frame(), expected);
        }

        act(&mut model, AnimateAction::SetFrame(30));
        let change = model.update(EditorAction::Animate(AnimateAction::FrameCompleted));
        assert!(change.preview);
        assert_eq!(model.animate().frame(), 0, "playback wraps to the start");

        act(&mut model, AnimateAction::TogglePlaying);
        let change = model.update(EditorAction::Animate(AnimateAction::FrameCompleted));
        assert!(!change.preview, "a paused workspace requests nothing");
        cleanup(path);
    }

    #[test]
    fn the_work_area_is_editor_only_and_fit_covers_out_of_range_keys() {
        let (mut model, path) = authored_model("work-area");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let baseline = model.document().unwrap().project.clone();

        assert_eq!(model.animate_runtime_range(), (0, 30));
        // The fixture key at frame 45 sits beyond the runtime duration.
        assert_eq!(model.animate().sheet().out_of_range, 1);
        assert!(
            model
                .animate()
                .sheet()
                .keys
                .iter()
                .any(|key| key.frame == 45 && key.out_of_range)
        );

        act(&mut model, AnimateAction::FitAllKeys);
        assert_eq!(model.animate_play_range(), (0, 45));
        assert!(model.animate().has_work_area());
        assert_eq!(model.document().unwrap().project, baseline);
        assert!(!model.is_dirty());

        act(&mut model, AnimateAction::SetWorkEnd("20".into()));
        assert_eq!(model.animate_play_range(), (0, 20));
        act(&mut model, AnimateAction::ClearWorkArea);
        assert_eq!(model.animate_play_range(), (0, 30));
        assert!(!model.is_dirty());
        cleanup(path);
    }

    #[test]
    fn active_dope_rows_are_flat_direct_paths_with_layer_badges() {
        let (mut model, path) = authored_model("flat-rows");
        {
            let layer = &mut model.document_mut().unwrap().project.scenes[0].layers[0];
            layer.nodes[0].name = Some(b"Root".to_vec());
            layer.nodes[1].name = Some(b"Child".to_vec());
        }
        model.bump_revision();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );

        let sheet = model.animate().sheet();
        assert!(
            sheet
                .rows
                .iter()
                .all(|row| matches!(row.kind, DopeRowKind::Cast | DopeRowKind::Channel))
        );
        assert!(
            sheet
                .rows
                .iter()
                .all(|row| row.layer == 0 && row.layer_badge == "L0")
        );
        let root = sheet
            .rows
            .iter()
            .position(|row| row.kind == DopeRowKind::Cast && row.node == Some(0))
            .expect("direct root CAST row");
        let child = sheet
            .rows
            .iter()
            .position(|row| row.kind == DopeRowKind::Cast && row.node == Some(1))
            .expect("direct child CAST row");
        assert_eq!(sheet.rows[root].label, "Root");
        assert_eq!(sheet.rows[child].label, "Root / Child");
        assert_eq!(sheet.rows[root].depth, 0);
        assert_eq!(sheet.rows[root + 1].kind, DopeRowKind::Channel);
        assert_eq!(sheet.rows[root + 1].depth, 1);
        assert_eq!(sheet.rows[root + 1].keys.1 - sheet.rows[root + 1].keys.0, 2);

        let before = sheet.visible.len();
        act(&mut model, AnimateAction::ToggleRowCollapsed(root));
        assert_eq!(model.animate().sheet().visible.len(), before - 1);
        act(&mut model, AnimateAction::ToggleRowCollapsed(root));
        assert_eq!(model.animate().sheet().visible.len(), before);
        cleanup(path);
    }

    #[test]
    fn dope_rows_include_every_enabled_resolved_assignment_layer() {
        let (mut model, path) = authored_model("multiple-active-layers");
        {
            let scene = &mut model.document_mut().unwrap().project.scenes[0];
            let mut copied = scene.layers[0].animations[0].clone();
            copied.name = b"second-move".to_vec();
            scene.layers[1].animations = vec![copied];
            scene.layers[1].animation_count = 1;
        }
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::AssignSlotAnimation(1, Some(0)));

        let layers: BTreeSet<usize> = model
            .animate()
            .sheet()
            .rows
            .iter()
            .filter(|row| row.kind == DopeRowKind::Cast)
            .map(|row| row.layer)
            .collect();
        assert_eq!(layers, BTreeSet::from([0, 1]));

        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        assert!(model.animate().preview().is_none());
        let selected_layers: BTreeSet<usize> = model
            .animate()
            .sheet()
            .rows
            .iter()
            .filter(|row| row.kind == DopeRowKind::Cast)
            .map(|row| row.layer)
            .collect();
        assert_eq!(selected_layers, BTreeSet::from([0, 1]));

        act(&mut model, AnimateAction::ToggleAnimationPreview);
        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .all(|row| row.layer == 0),
            "explicit preview narrows the dope sheet to its selected layer"
        );
        act(&mut model, AnimateAction::ToggleAnimationPreview);
        let restored_layers: BTreeSet<usize> = model
            .animate()
            .sheet()
            .rows
            .iter()
            .filter(|row| row.kind == DopeRowKind::Cast)
            .map(|row| row.layer)
            .collect();
        assert_eq!(restored_layers, BTreeSet::from([0, 1]));
        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .all(|row| matches!(row.kind, DopeRowKind::Cast | DopeRowKind::Channel))
        );

        act(&mut model, AnimateAction::ToggleSlotEnabled(1));
        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .all(|row| row.layer == 0)
        );
        cleanup(path);
    }

    #[test]
    fn large_scenes_generate_rows_and_keys_without_per_key_widgets() {
        let (mut model, path) = authored_model("large");
        {
            let scene = &mut model.document_mut().unwrap().project.scenes[0];
            let layer = &mut scene.layers[0];
            // 410 casts: one root chain plus siblings, all under CAST 0.
            let template = layer.nodes[1].clone();
            let transform = layer.transforms[0];
            for index in 2..410 {
                let mut node = template.clone();
                node.next_sibling_index = -1;
                layer.nodes.push(node);
                layer.transforms.push(transform);
                layer.image_by_node.push(None);
                layer.number_by_node.push(None);
                layer.reference_by_node.push(None);
                layer.csli_by_node.push(None);
                layer.cast_attribute_list_by_node.push(None);
                let previous = index - 1;
                layer.nodes[previous].next_sibling_index = index as i16;
            }
            // 1001 keys on a single channel.
            let keys: Vec<Key20<f32>> = (0..1001)
                .map(|frame| Key20 {
                    frame,
                    value: frame as f32,
                    mode: 1,
                    slope_in: 0.0,
                    slope_out: 0.0,
                })
                .collect();
            layer.animations[0].motions[0].tracks[0].keys = KeyData::Key20F32(keys);
            layer.animations[0].motions[0].tracks[0].key_count = 1001;
            layer.animations[0].duration = 1001;
        }
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));

        let sheet = model.animate().sheet();
        // Descendants inherit at runtime but are not dope rows unless their
        // own MOT directly carries a track.
        assert_eq!(
            sheet
                .rows
                .iter()
                .filter(|row| row.kind == DopeRowKind::Cast)
                .count(),
            1
        );
        assert_eq!(sheet.keys.len(), 1001);
        // Keys live in one flat array indexed by row slices, not per widget.
        let channel_row = sheet
            .rows
            .iter()
            .position(|row| row.kind == DopeRowKind::Channel)
            .unwrap();
        assert_eq!(sheet.row_keys(channel_row).len(), 1001);
        assert_eq!(sheet.key_min, 0);
        assert_eq!(sheet.key_max, 1000);
        cleanup(path);
    }

    #[test]
    fn key_authoring_writes_only_runtime_supported_semantics() {
        let (mut model, path) = authored_model("semantics");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        act(
            &mut model,
            AnimateAction::SelectKey(TrackAddress::new(0, 0, 0, 0), 0),
        );

        act(
            &mut model,
            AnimateAction::SetKeySemantics(KeySemantics::Cubic),
        );
        let track = &model.animate_active_animation().unwrap().1.motions[0].tracks[0];
        assert_eq!(
            channels::key_view(&track.keys, 0).unwrap().semantics,
            KeySemantics::Cubic
        );

        // 0x23 has no cubic evaluator, so the conversion demotes to linear and
        // further cubic requests are refused.
        act(&mut model, AnimateAction::SetTrackFormat(0, 0, 0x23));
        let track = &model.animate_active_animation().unwrap().1.motions[0].tracks[0];
        assert_eq!(track.format & 0x70, 0x20);
        assert_eq!(
            channels::key_view(&track.keys, 0).unwrap().semantics,
            KeySemantics::Linear
        );
        act(
            &mut model,
            AnimateAction::SetKeySemantics(KeySemantics::Cubic),
        );
        let track = &model.animate_active_animation().unwrap().1.motions[0].tracks[0];
        assert_eq!(
            channels::key_view(&track.keys, 0).unwrap().semantics,
            KeySemantics::Linear
        );
        cleanup(path);
    }

    #[test]
    fn a_track_range_can_never_become_empty() {
        let (mut model, path) = authored_model("track-range");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        act(
            &mut model,
            AnimateAction::SelectTrack(TrackAddress::new(0, 0, 0, 0)),
        );

        act(
            &mut model,
            AnimateAction::SetTrackRangeStart(0, 0, "30".into()),
        );
        let track = &model.animate_active_animation().unwrap().1.motions[0].tracks[0];
        assert_eq!(track.range_start, 0, "start may not reach the end");

        act(&mut model, AnimateAction::SetTrackWrap(0, 0, true));
        let track = &model.animate_active_animation().unwrap().1.motions[0].tracks[0];
        assert!(channels::wraps(track.format));
        assert_eq!(track.format, 0x113);
        cleanup(path);
    }

    #[test]
    fn duplicating_a_set_in_edit_mode_creates_a_unique_name() {
        let (mut model, path) = authored_model("duplicate");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::DuplicateSet);

        let scene = model.selected_scene().unwrap();
        assert_eq!(scene.animation_sets.len(), 2);
        assert_ne!(scene.animation_sets[0].name, scene.animation_sets[1].name);
        assert_eq!(model.animate().source_set(), Some(1));
        assert!(model.document_editing());
        assert!(assignment_is_dense(model.animate().assignment(), scene));
        cleanup(path);
    }

    #[test]
    fn saving_keeps_document_edit_mode_enabled() {
        let (mut model, path) = authored_model("save-keeps-edit");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetDuration("36".into()));
        assert!(model.is_dirty());

        model.save().expect("minimal SRD round-trips");
        assert!(model.document_editing());
        assert!(!model.is_dirty());

        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            reloaded.project.scenes[0].animation_sets[0].runtime_duration,
            36
        );
        cleanup(path);
    }

    #[test]
    fn failed_save_preserves_edit_mode_and_changes() {
        let (mut model, path) = authored_model("failed-save");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetDuration("36".into()));
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        assert!(model.save().is_err());
        assert!(model.document_editing());
        assert!(model.is_dirty());
        assert_eq!(model.animate().assignment().runtime_duration, 36);
        std::fs::remove_dir(path).unwrap();
    }

    #[test]
    fn failed_save_as_preserves_edit_mode_and_changes() {
        let (mut model, path) = authored_model("failed-save-as");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetDuration("36".into()));

        assert!(model.save_as(PathBuf::new()).is_err());
        assert!(model.document_editing());
        assert!(model.is_dirty());
        assert_eq!(model.animate().assignment().runtime_duration, 36);
        cleanup(path);
    }

    #[test]
    fn dope_rows_mutate_their_own_layer_and_authored_animation() {
        let (mut model, path) = authored_model("row-address");
        {
            let scene = &mut model.document_mut().unwrap().project.scenes[0];
            let mut other = scene.layers[0].animations[0].clone();
            other.name = b"other".to_vec();
            scene.layers[1].animations = vec![other];
            scene.layers[1].animation_count = 1;
            scene.animation_sets[0].slots[1].animation_name = b"other".to_vec();
        }
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);

        let address = model
            .animate()
            .sheet()
            .rows
            .iter()
            .find_map(|row| row.track.filter(|address| address.layer == 1))
            .expect("layer 1 channel row carries its own address");
        act(&mut model, AnimateAction::ToggleKeyAtFrame(address, 12));

        let scene = model.selected_scene().unwrap();
        assert!(
            (0..channels::key_count(&scene.layers[0].animations[0].motions[0].tracks[0].keys)).all(
                |key| {
                    channels::key_frame(
                        &scene.layers[0].animations[0].motions[0].tracks[0].keys,
                        key,
                    ) != Some(12)
                }
            )
        );
        assert!(
            (0..channels::key_count(&scene.layers[1].animations[0].motions[0].tracks[0].keys)).any(
                |key| {
                    channels::key_frame(
                        &scene.layers[1].animations[0].motions[0].tracks[0].keys,
                        key,
                    ) == Some(12)
                }
            )
        );
        cleanup(path);
    }

    #[test]
    fn selected_duplicate_does_not_replace_active_assignment_rows() {
        let (mut model, path) = authored_model("duplicate-row-address");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::DuplicateAnimation);

        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .filter_map(|row| row.track)
                .all(|address| address.animation == 0)
        );
        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 1,
                node: 1,
                target: 0,
            },
        );

        let layer = &model.selected_scene().unwrap().layers[0];
        assert!(
            layer.animations[0]
                .motions
                .iter()
                .all(|motion| motion.target != 1)
        );
        assert!(
            layer.animations[1]
                .motions
                .iter()
                .any(|motion| motion.target == 1)
        );
        assert!(
            model
                .animate()
                .sheet()
                .rows
                .iter()
                .filter_map(|row| row.track)
                .all(|address| address.animation == 0)
        );
        cleanup(path);
    }

    #[test]
    fn moving_across_a_neighbour_keeps_the_dragged_key_identity() {
        let (mut model, path) = authored_model("stable-key-drag");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        let address = TrackAddress::new(0, 0, 0, 0);

        act(&mut model, AnimateAction::MoveKey(address, 0, 50));
        // The canvas updates its held index after the sort, so the next motion
        // still targets the key that began at frame 0 rather than its neighbour.
        act(&mut model, AnimateAction::MoveKey(address, 1, 60));

        let keys =
            &model.selected_scene().unwrap().layers[0].animations[0].motions[0].tracks[0].keys;
        assert_eq!(channels::key_frame(keys, 0), Some(45));
        assert_eq!(channels::key_frame(keys, 1), Some(60));
        assert_eq!(
            channels::key_view(keys, 1).unwrap().value,
            KeyValue::Float(0.0)
        );
        cleanup(path);
    }

    #[test]
    fn explicit_preview_does_not_change_assignment_controls() {
        let (mut model, path) = authored_model("preview-working-assignment");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        act(&mut model, AnimateAction::ToggleAnimationPreview);
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::ToggleSlotEnabled(0));

        assert!(!model.animate().assignment().slots[0].is_enabled());
        assert!(!model.animate().assignment_rows()[0].enabled);
        assert!(
            model.animate().effective_assignment().slots[0].is_enabled(),
            "preview may force evaluation without changing assignment controls"
        );
        cleanup(path);
    }

    #[test]
    fn scratch_assigned_missing_type_null_cast_accepts_common_property_track() {
        let (mut model, path) = authored_model("missing-type-null-property");
        model.document_mut().unwrap().project.scenes[0].layers[0].nodes[1].type_flags = None;
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(None));
        act(&mut model, AnimateAction::AssignSlotAnimation(0, Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        model.update(EditorAction::ToggleDocumentEditing);

        let layer = &model.selected_scene().unwrap().layers[0];
        assert_eq!(model.animate().source_set(), None);
        assert_eq!(model.animate().assignment_rows()[0].animation, Some(0));
        assert!(is_authorable_motion_target(layer, 1));
        assert!(is_authorable_track_target(layer, 1, 0));

        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 1,
                target: 0,
            },
        );

        let animation = model.animate_active_animation().unwrap().1;
        let motion = animation
            .motions
            .iter()
            .find(|motion| motion.target == 1)
            .expect("missing-type NullCast receives a property motion");
        assert!(motion.tracks.iter().any(|track| track.target == 0));
        cleanup(path);
    }

    #[test]
    fn unsupported_channel_target_combination_does_not_mutate() {
        let (mut model, path) = authored_model("unsupported-target");
        model.document_mut().unwrap().project.scenes[0].layers[0].nodes[0].type_flags = Some(0);
        model.bump_revision();
        model.animate_sync();
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        act(&mut model, AnimateAction::SelectAnimation(0, 0));
        model.update(EditorAction::ToggleDocumentEditing);
        let before = model.animate_active_animation().unwrap().1.clone();

        act(
            &mut model,
            AnimateAction::AddProperty {
                layer: 0,
                animation: 0,
                node: 0,
                target: 11,
            },
        );

        let layer = &model.selected_scene().unwrap().layers[0];
        assert!(!is_authorable_track_target(layer, 0, 11));
        let mut unsupported_cast = layer.clone();
        unsupported_cast.nodes[0].type_flags = Some(7);
        assert!(!is_authorable_motion_target(&unsupported_cast, 0));
        assert!(!is_authorable_track_target(&unsupported_cast, 0, 0));
        assert_eq!(model.animate_active_animation().unwrap().1, &before);
        cleanup(path);
    }

    #[test]
    fn negative_authored_keys_fit_without_allowing_timeline_overscroll() {
        let (mut model, path) = authored_model("negative-key-view");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        model.update(EditorAction::ToggleDocumentEditing);
        act(
            &mut model,
            AnimateAction::MoveKey(TrackAddress::new(0, 0, 0, 0), 0, -12),
        );
        act(&mut model, AnimateAction::FitAllKeys);
        assert_eq!(model.animate_play_range(), (-12, 45));
        assert_eq!(model.animate().pan(), -12.0);

        act(&mut model, AnimateAction::PanBy(-8.0));
        assert_eq!(model.animate().pan(), -12.0);
        act(&mut model, AnimateAction::PanBy(1_000.0));
        assert_eq!(model.animate().pan(), 15.0);

        // Zooming out widens the guaranteed visible span and tightens the
        // right bound, so the end frame remains inside the viewport.
        act(&mut model, AnimateAction::ZoomBy(-0.5));
        assert_eq!(model.animate().pan(), -12.0);
        act(&mut model, AnimateAction::SetFrame(-10));
        assert_eq!(model.animate().frame(), -10);
        cleanup(path);
    }

    #[test]
    fn assignment_duration_refreshes_effective_and_out_of_range_rows() {
        let (mut model, path) = authored_model("duration-derivations");
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        assert_eq!(model.animate().sheet().out_of_range, 1);
        model.update(EditorAction::ToggleDocumentEditing);
        act(&mut model, AnimateAction::SetDuration("60".into()));

        assert_eq!(model.animate_runtime_range(), (0, 60));
        assert_eq!(model.animate().effective_assignment().runtime_duration, 60);
        assert_eq!(model.animate().sheet().out_of_range, 0);
        cleanup(path);
    }

    #[test]
    fn restored_animation_history_reselects_its_owning_scene_before_sync() {
        let (mut model, path) = authored_model("history-owning-scene");
        {
            let document = model.document_mut().unwrap();
            let mut second = document.project.scenes[0].clone();
            second.name = b"second".to_vec();
            second.animation_sets[0].runtime_duration = 17;
            document.project.scenes.push(second);
            document.project.declared_scene_count = 2;
        }
        model.bump_revision();
        model.update(EditorAction::SelectScene(1));
        act(&mut model, AnimateAction::LoadStoredSet(Some(0)));
        let history = model.animate_history();

        model.update(EditorAction::SelectScene(0));
        model.animate_restore_history(history);
        model.animate_sync();

        assert_eq!(model.selected_scene_index(), 1);
        assert_eq!(model.animate().assignment().runtime_duration, 17);
        cleanup(path);
    }

    #[test]
    fn key8_tracks_have_no_mode_word_and_stay_linear() {
        let mut keys = KeyData::Key8F32(vec![Key8 {
            frame: 0,
            value: 1.0,
        }]);
        assert_eq!(
            channels::key_view(&keys, 0).unwrap().semantics,
            KeySemantics::Linear
        );
        assert!(!channels::set_key_semantics(
            &mut keys,
            0x10,
            0,
            KeySemantics::Hold
        ));
    }
}
