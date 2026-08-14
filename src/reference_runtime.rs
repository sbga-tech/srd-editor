//! Resolved Surfride reference graph and mutable frame state.
//!
//! `plan` owns immutable topology, `state` advances animation values, and `world` composes the
//! matrices consumed by render compilation.

use std::collections::BTreeSet;
use std::fmt;

use crate::animation::RuntimeAnimationState;
use crate::csli::{add_color_saturating_game, multiply_color_game};
use crate::image::{ImageDefinition, RuntimeImageState};
use crate::projection::{Matrix4x4, inverse_matrix4x4_game};
use crate::reference::ReferenceAnimationRequest;
use crate::scene::{AnimationSetDefinition, Layer, Project, ReferenceTarget, SceneError};
use crate::texture::TextureList;
use crate::transform::{Affine3x4, SpatialTransform, build_local_matrix};
mod plan;
mod state;
mod world;

#[cfg(test)]
use world::compose_cast_matrix_game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceLayerParent {
    ProjectLayer(ReferenceTarget),
    ReferenceInstance(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLayerInstance {
    pub parent: ReferenceLayerParent,
    pub reference_node_index: usize,
    pub target: ReferenceTarget,
    pub is_2d: bool,
    pub flip_y: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeWorldState {
    pub matrix: Affine3x4,
    pub multiply_color: [u8; 4],
    pub additive_color: [u8; 4],
    pub visible: bool,
    pub render_gate: bool,
}

impl Default for RuntimeWorldState {
    fn default() -> Self {
        Self {
            matrix: Affine3x4::IDENTITY,
            multiply_color: [255; 4],
            additive_color: [0; 4],
            visible: true,
            render_gate: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceLayerLocalState {
    pub transform: SpatialTransform,
    pub enabled: bool,
}

impl Default for ReferenceLayerLocalState {
    fn default() -> Self {
        Self {
            transform: SpatialTransform::default(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReference {
    pub parent: ReferenceLayerParent,
    pub reference_node_index: usize,
    pub source_name: Vec<u8>,
    pub layer_name: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceRuntimePlan {
    pub instances: Vec<ReferenceLayerInstance>,
    pub unresolved: Vec<UnresolvedReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceRuntimeAncestryError {
    InvalidReferenceInstance { instance_index: usize },
    ReferenceCycle { instance_index: usize },
}

impl fmt::Display for ReferenceRuntimeAncestryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReferenceInstance { instance_index } => {
                write!(
                    formatter,
                    "reference instance {instance_index} is outside the runtime plan"
                )
            }
            Self::ReferenceCycle { instance_index } => {
                write!(
                    formatter,
                    "reference instance ancestry cycles at {instance_index}"
                )
            }
        }
    }
}

impl std::error::Error for ReferenceRuntimeAncestryError {}

/// One non-reference CAST in the exact structural traversal produced by
/// `srd_render_runtime_layer`: layer CAST vectors are visited forward, and a
/// resolved RefCast recursively expands its copied layer at that NODE
/// position before the parent vector continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCastDrawOrderEntry {
    pub owner: ReferenceLayerParent,
    pub source: ReferenceTarget,
    pub node_index: usize,
    /// Effective `SrRenderer+0x198` value at the CAST's draw call, after the
    /// inherited ExtParamData key and NODE `+0x58` low-byte offset.
    pub renderer_layer_key: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceLayerRuntimeState {
    pub instance_index: usize,
    pub local: ReferenceLayerLocalState,
    pub cast_transforms: Vec<SpatialTransform>,
    pub image_bases: Vec<ImageDefinition>,
    pub image_states: Vec<RuntimeImageState>,
    pub animations: Vec<RuntimeAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeAnimationApplication {
    pub common_channels: usize,
    pub image_channels: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeAnimationTreeApplication {
    pub animated_layers: usize,
    pub common_channels: usize,
    pub image_channels: usize,
    pub reference_requests: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceRuntime {
    pub plan: ReferenceRuntimePlan,
    pub layers: Vec<ReferenceLayerRuntimeState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectLayerRuntimeState {
    pub target: ReferenceTarget,
    pub enabled: bool,
    pub cast_transforms: Vec<SpatialTransform>,
    pub image_bases: Vec<ImageDefinition>,
    pub image_states: Vec<RuntimeImageState>,
    pub animations: Vec<RuntimeAnimationState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRuntime {
    pub project_layers: Vec<Vec<ProjectLayerRuntimeState>>,
    pub references: ReferenceRuntime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeLayerWorldStates {
    pub owner: ReferenceLayerParent,
    pub source: ReferenceTarget,
    pub is_2d: bool,
    pub layer: RuntimeWorldState,
    pub casts: Vec<RuntimeWorldState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRuntimeWorldStates {
    pub project_layers: Vec<Vec<RuntimeLayerWorldStates>>,
    pub references: Vec<RuntimeLayerWorldStates>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRuntimeError {
    pub repeating_layers: Vec<ReferenceTarget>,
}

impl fmt::Display for ReferenceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("reference-layer construction does not converge through")?;
        for layer in &self.repeating_layers {
            write!(
                formatter,
                " SCN[{}]/LAYR[{}]",
                layer.scene_index, layer.layer_index
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ReferenceRuntimeError {}

#[cfg(test)]
include!("reference_runtime/tests.rs");
