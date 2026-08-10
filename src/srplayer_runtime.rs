//! Evidence-backed Chusan 2.50 Surfride runtime descriptors.
//!
//! This is not a Rust declaration of the game's ABI: it stores only fixed x86
//! offsets and semantic roles. It contains no game pointers or unsafe access.

pub const CHUSAN_MATE_2_50_IMAGE: &str = "chusanApp_MATE_2.50.exe";
pub const CHUSAN_MATE_2_50_BITNESS: u8 = 32;
pub const CHUSAN_MATE_2_50_POINTER_BYTES: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86FieldLayout {
    pub offset: u32,
    pub name: &'static str,
}

/// A descriptive x86 layout, never a host ABI declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct X86ObjectLayout {
    pub name: &'static str,
    pub allocation_size: Option<u32>,
    pub fields: &'static [X86FieldLayout],
    pub evidence: &'static str,
}
// `allocation_size` also records an exact complete-object/embedded extent when
// the native object is not allocated independently.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeOwnership {
    SerializedResource,
    PlayerOwnedRuntime,
    HostOwnedOuterGraph,
    WeakTarget,
}

/// Semantic ownership without fake game-pointer ownership in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRole {
    pub name: &'static str,
    pub ownership: RuntimeOwnership,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePointerRelation {
    Embedded,
    OwnedHeap,
    OwnedVectorElement,
    BorrowedResourceHolder,
    BorrowedParsedRecord,
    BackPointer,
    Weak,
    NonOwningHierarchy,
    RefCountedResource,
}

/// One native pointer edge and the event that invalidates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePointerEdge {
    pub source: &'static str,
    pub offset: u32,
    pub target: &'static str,
    pub relation: RuntimePointerRelation,
    pub invalidated_by: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrPlayerLifecyclePhase {
    Constructed,
    LoadRequested,
    WaitingForResource,
    ProjectFinalized,
    RuntimeTreeBuilt,
    Ready,
    Evaluating,
    Rendering,
    Releasing,
    Released,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrPlayerLifecycleTransition {
    pub from: SrPlayerLifecyclePhase,
    pub to: SrPlayerLifecyclePhase,
    pub evidence: &'static str,
}

/// Factory selection after `srd_create_runtime_cast_for_node` (`0xACA030`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastFactoryLayout {
    pub cast_name: &'static str,
    pub node_type: Option<u8>,
    pub allocation_size: u32,
    pub factory_address: Option<u32>,
    pub constructor_address: Option<u32>,
    pub selection: &'static str,
}

macro_rules! fields {
    ($($offset:expr => $name:literal),+ $(,)?) => {
        &[$(X86FieldLayout { offset: $offset, name: $name }),+]
    };
}

/// SEA placement input, separate from the Surfride Scene/Layer/CAST tree.
pub static SEA_GRAPH_NODE_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "sea::GraphNode portion of projView::SrPlayer",
    allocation_size: None,
    fields: fields![
        0x10 => "child GraphNode vector begin",
        0x14 => "child GraphNode vector end",
        0x1C => "parent GraphNode",
        0x28 => "graph-name string",
        0x58 => "graph flags",
        0x64 => "local affine 3x4",
        0x94 => "composite affine 3x4",
    ],
    evidence: "sub_6095B0 clears graph links; sub_6096C0 initializes both matrices",
};

/// Embedded in concrete GameObject implementations at `+0x68`; the complete
/// x86 object extent is exactly `0x180` bytes.
pub static PROJ_VIEW_SRPLAYER_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "projView::SrPlayer",
    allocation_size: Some(0x180),
    fields: fields![
        0x00 => "primary vptr / inherited Surfride-AIR-SEA state",
        0x10 => "SEA child range begin; end is +0x14",
        0x1C => "SEA parent GraphNode",
        0x28 => "graph-name string",
        0x58 => "graph flags",
        0x64 => "local affine 3x4",
        0x94 => "composite affine 3x4",
        0xC4 => "FirstCalcMatrix selection byte",
        0xC8 => "AIR load/resource utility",
        0xDC => "WeakParent<surfride::SrPlayer>",
        0xE8 => "owned SrPlayer::Impl pointer",
        0xF0 => "embedded ParamCtrlTemplate",
        0x140 => "primary SRD filename string",
        0x158 => "fallback texture-root string",
        0x170 => "embedded SrCtrl",
        0x17C => "facade/controller state word",
        0x17E => "load-state bits",
    ],
    evidence: "sub_BA5080 constructs facade; 0xBA5390 deletes exactly 0x180 bytes",
};

pub static SR_CTRL_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrCtrl (projView::SrPlayer +0x170)",
    allocation_size: Some(0x0C),
    fields: fields![
        0x00 => "controller vptr",
        0x04 => "weak target object pointer",
        0x08 => "weak reference-control pointer",
    ],
    evidence: "0xACA960 constructs 0x0C-byte state; 0xACBE40 binds from player WeakParent +0xDC",
};

pub static SRPLAYER_IMPL_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrPlayer::Impl",
    allocation_size: Some(0x2E8),
    fields: fields![
        0x00 => "three-entry Impl vptr",
        0x08 => "borrowed SrResource project holder; holder +0x04 is SrProject",
        0x0C => "non-owning player/context backpointer",
        0x10 => "embedded 0x278-byte SrRenderer",
        0x294 => "owning vector<SrScene*>",
        0x2A0 => "ReferenceScene/runtime helper state",
        0x2D4 => "additional pointer vector",
        0x2E0 => "optional owned runtime helper",
    ],
    evidence: "0xAA6CA5 allocates 0x2E8; 0xAA67A0 constructs; 0xAACDF0 resets owned state",
};

pub static SR_RENDERER_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrRenderer (SrPlayer::Impl +0x10)",
    allocation_size: None,
    fields: fields![
        0x08 => "project-to-target matrix",
        0x48 => "culling projection/viewport matrix",
        0x88 => "inverse SRD-camera View 3x4",
        0xB8 => "outer graph root matrix",
        0x190 => "draw-target filter block",
        0x198 => "inherited renderer key",
        0x248 => "named target pointer",
        0x24C => "culling rectangle center/half extent",
        0x26C => "font/TextBox resource tree",
        0x274 => "renderer mask byte",
    ],
    evidence: "constructed from 0xAA67A0; camera configuration is 0xAC7400",
};

pub static SR_SCENE_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrScene",
    allocation_size: Some(0x78),
    fields: fields![
        0x04 => "parsed SCN",
        0x08 => "owning SrPlayer::Impl",
        0x0C => "scene name string",
        0x24 => "vector<SrLayer*>",
        0x30 => "vector<SrAnimationSet*>",
        0x3C => "embedded animation/runtime state",
        0x70 => "current SrAnimationSet",
        0x74 => "runtime state; +0x75 is visibility",
    ],
    evidence: "factory allocation 0x78; constructor 0xAC1BC0",
};

pub static SR_LAYER_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrLayer",
    allocation_size: Some(0x2AC),
    fields: fields![
        0x0C => "parsed LAYR",
        0x10 => "layer name string",
        0x28 => "vector<SrAnimation*>",
        0x4C => "vector<SrCast*>",
        0x64 => "root CAST vector",
        0x130 => "effective 2D mode",
        0x131 => "reference-layer flip-Y",
        0x13C => "local runtime transform",
        0x168 => "independent layer enable",
        0x16C => "world affine transform",
        0x208 => "composed multiplicative color",
        0x20C => "composed additive color",
        0x210 => "derived visibility",
        0x248 => "matched original layer",
        0x24C => "owning SrRefCast",
        0x250 => "reference binding index",
        0x254 => "reference binding value",
        0x268 => "tail string/subobject",
    ],
    evidence: "factory allocation 0x2AC; ctor 0xABC8D0; build 0xAC00B0",
};

pub static SR_ANIMATION_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrAnimation",
    allocation_size: Some(0x8C),
    fields: fields![
        0x04 => "animation name string",
        0x1C => "raw frame bits",
        0x20 => "runtime duration",
        0x24 => "playback factor 0",
        0x28 => "playback factor 1",
        0x2C => "playback factor 2",
        0x30 => "runtime flags",
        0x34 => "parsed ANIM",
        0x38 => "owning SrLayer",
        0x3C => "runtime motion vector",
        0x48 => "runtime motion vector",
        0x54 => "runtime string",
        0x6C => "runtime string",
        0x84 => "tail field",
        0x88 => "tail field",
    ],
    evidence: "factory allocation 0x8C; constructor 0xAD4D40",
};

pub static SR_ANIMATION_SET_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrAnimationSet",
    allocation_size: Some(0x30),
    fields: fields![
        0x04 => "parsed ANMS",
        0x08 => "animation-set name string",
        0x20 => "owning SrScene",
        0x24 => "per-layer entry vector",
    ],
    evidence: "constructor 0xAD7030; each per-layer entry allocates 0x28",
};

pub static SR_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrCast / SrNullCast base",
    allocation_size: Some(0x1F4),
    fields: fields![
        0x04 => "parsed NODE",
        0x0C => "typed parsed record (NODE +0x50)",
        0x30 => "parent SrCast",
        0x34 => "child SrCast vector",
        0x4C => "runtime flags",
        0x50 => "inherited renderer key",
        0x5C => "local transform",
        0x80 => "local multiplicative color",
        0x84 => "local additive color",
        0x8C => "world affine matrix",
        0xBC => "composed multiplicative color",
        0xC0 => "composed additive color",
        0xC4 => "derived visibility",
    ],
    evidence: "base constructor 0xAD3020",
};

pub static SR_IMAGE_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrImage (embedded at Image/Number cast +0xF8)",
    allocation_size: Some(0xD0),
    fields: fields![
        0x00 => "SrImage vptr",
        0x04 => "CIMG flags",
        0x08 => "Point-versus-Linear sampling selector",
        0x0C => "MultiTex0 variant input",
        0x10 => "stencil/depth mode",
        0x14 => "stencil shift count",
        0x18 => "stencil condition",
        0x1C => "runtime counter/sentinel state",
        0x28 => "borrowed parsed CIMG",
        0x80 => "width; height is +0x84",
        0x88 => "3x3 origin mode",
        0x8C => "custom origin x; y is +0x90",
        0xA0 => "12-byte ExtParamData",
        0xAC => "borrowed CREF array",
        0xB0 => "borrowed CRE1 array",
        0xB4 => "CREF count",
        0xB8 => "CRE1 count",
        0xBC => "refcounted resource-handle slot 0",
        0xC0 => "refcounted resource-handle slot 1",
    ],
    evidence: "constructor 0xAD2150; CIMG initialization 0xAD28E0; image-state consumers",
};

pub static SR_IMAGE_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrImageCast",
    allocation_size: Some(0x228),
    fields: fields![
        0xF8 => "embedded SrImage",
        0x100 => "point-sampling selector",
        0x1A4 => "CREF pointer",
        0x1A8 => "CRE1 pointer",
        0x1AC => "CREF count",
        0x1B0 => "CRE1 count",
        0x210 => "coordinate-offset multiplier",
    ],
    evidence: "factory 0xAC9BA0; CIMG initializes SrImage and multiplier",
};

pub static SR_SLICE_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrSliceCast",
    allocation_size: Some(0x228),
    fields: fields![0x1F4 => "slice tail subobject (semantics unknown)"],
    evidence: "factory 0xAC9EF0 constructs and vtable-initializes its +0x1F4 tail",
};

pub static SR_TEXT_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrTextCast",
    allocation_size: Some(0x350),
    fields: fields![
        0x1F4 => "text playback/layout state",
        0x1F8 => "string state",
        0x2FC => "FontParam no-wrap mode",
    ],
    evidence: "factory 0xAC9FB0; constructor 0xAD8270",
};

pub static SR_REF_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrRefCast",
    allocation_size: Some(0x1F8),
    fields: fields![0x1F4 => "independent copied SrLayer"],
    evidence: "constructor 0xADB3B0; binding/update/render use +0x1F4",
};

pub static SR_NUMBER_CAST_LAYOUT: X86ObjectLayout = X86ObjectLayout {
    name: "surfride::SrNumberCast",
    allocation_size: Some(0x270),
    fields: fields![
        0xF8 => "embedded SrImage",
        0x210 => "coordinate multiplier (initial 0.0)",
        0x228 => "56-byte glyph-history vector begin",
        0x22C => "glyph-history vector end",
        0x230 => "glyph-history vector capacity",
    ],
    evidence: "factory 0xAC9D70; constructor 0xADC2C0",
};

pub const CAST_FACTORIES: &[CastFactoryLayout] = &[
    CastFactoryLayout {
        cast_name: "surfride::SrNullCast",
        node_type: None,
        allocation_size: 0x1F4,
        factory_address: Some(0xAC9CE0),
        constructor_address: Some(0xAD3020),
        selection: "default for an unhandled NODE type",
    },
    CastFactoryLayout {
        cast_name: "surfride::SrImageCast",
        node_type: Some(1),
        allocation_size: 0x228,
        factory_address: Some(0xAC9BA0),
        constructor_address: Some(0xAD3020),
        selection: "type 1 without TEXT or without CIMG flag 0x100",
    },
    CastFactoryLayout {
        cast_name: "surfride::SrTextCast",
        node_type: Some(1),
        allocation_size: 0x350,
        factory_address: Some(0xAC9FB0),
        constructor_address: Some(0xAD8270),
        selection: "type 1 with TEXT and CIMG flag 0x100",
    },
    CastFactoryLayout {
        cast_name: "surfride::SrSliceCast",
        node_type: Some(2),
        allocation_size: 0x228,
        factory_address: Some(0xAC9EF0),
        constructor_address: Some(0xAD3020),
        selection: "NODE type 2",
    },
    CastFactoryLayout {
        cast_name: "surfride::SrRefCast",
        node_type: Some(3),
        allocation_size: 0x1F8,
        factory_address: None,
        constructor_address: Some(0xADB3B0),
        selection: "NODE type 3; allocation wrapper remains unnamed",
    },
    CastFactoryLayout {
        cast_name: "surfride::SrNumberCast",
        node_type: Some(4),
        allocation_size: 0x270,
        factory_address: Some(0xAC9D70),
        constructor_address: Some(0xADC2C0),
        selection: "NODE type 4",
    },
];

pub const SRPLAYER_RUNTIME_ROLES: &[RuntimeRole] = &[
    RuntimeRole {
        name: "parsed SrProject",
        ownership: RuntimeOwnership::SerializedResource,
        detail: "0x88 parsed record; CAM is +0x58 and enters through Impl +0x08 holder +0x04.",
    },
    RuntimeRole {
        name: "SrResource project holder",
        ownership: RuntimeOwnership::SerializedResource,
        detail: "Impl +0x08 borrows the holder; holder +0x04 owns the 0x88-byte SrProject product.",
    },
    RuntimeRole {
        name: "runtime scene tree",
        ownership: RuntimeOwnership::PlayerOwnedRuntime,
        detail: "Impl +0x294 owns mutable scenes, layers, casts, animations, and copied reference layers.",
    },
    RuntimeRole {
        name: "SEA GraphNode state",
        ownership: RuntimeOwnership::HostOwnedOuterGraph,
        detail: "FirstCalcMatrix placement is host scene-graph input, not serialized SRD state.",
    },
    RuntimeRole {
        name: "SrCtrl target",
        ownership: RuntimeOwnership::WeakTarget,
        detail: "Commands mediate reset/enable but do not establish a second Rust owner.",
    },
];

pub const SRPLAYER_POINTER_GRAPH: &[RuntimePointerEdge] = &[
    RuntimePointerEdge {
        source: "concrete GameObject::Impl",
        offset: 0x68,
        target: "projView::SrPlayer",
        relation: RuntimePointerRelation::Embedded,
        invalidated_by: "GameObject::Impl destruction",
    },
    RuntimePointerEdge {
        source: "projView::SrPlayer",
        offset: 0xE8,
        target: "surfride::SrPlayer::Impl",
        relation: RuntimePointerRelation::OwnedHeap,
        invalidated_by: "final player destruction",
    },
    RuntimePointerEdge {
        source: "projView::SrPlayer",
        offset: 0x170,
        target: "surfride::SrCtrl",
        relation: RuntimePointerRelation::Embedded,
        invalidated_by: "final player destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrCtrl",
        offset: 0x04,
        target: "surfride::SrPlayer weak target",
        relation: RuntimePointerRelation::Weak,
        invalidated_by: "target expiration or controller reset",
    },
    RuntimePointerEdge {
        source: "surfride::SrPlayer::Impl",
        offset: 0x08,
        target: "surfride::SrResource project holder",
        relation: RuntimePointerRelation::BorrowedResourceHolder,
        invalidated_by: "content release or reload",
    },
    RuntimePointerEdge {
        source: "surfride::SrResource project holder",
        offset: 0x04,
        target: "resource-owned SrProject",
        relation: RuntimePointerRelation::OwnedHeap,
        invalidated_by: "resource unload/destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrPlayer::Impl",
        offset: 0x10,
        target: "surfride::SrRenderer",
        relation: RuntimePointerRelation::Embedded,
        invalidated_by: "Impl destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrPlayer::Impl",
        offset: 0x294,
        target: "surfride::SrScene elements",
        relation: RuntimePointerRelation::OwnedVectorElement,
        invalidated_by: "content release or reload",
    },
    RuntimePointerEdge {
        source: "surfride::SrScene",
        offset: 0x04,
        target: "parsed SCN",
        relation: RuntimePointerRelation::BorrowedParsedRecord,
        invalidated_by: "resource unload",
    },
    RuntimePointerEdge {
        source: "surfride::SrScene",
        offset: 0x24,
        target: "original SrLayer elements",
        relation: RuntimePointerRelation::OwnedVectorElement,
        invalidated_by: "scene destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrScene",
        offset: 0x30,
        target: "SrAnimationSet elements",
        relation: RuntimePointerRelation::OwnedVectorElement,
        invalidated_by: "scene destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrLayer",
        offset: 0x0C,
        target: "parsed LAYR",
        relation: RuntimePointerRelation::BorrowedParsedRecord,
        invalidated_by: "resource unload",
    },
    RuntimePointerEdge {
        source: "surfride::SrLayer",
        offset: 0x28,
        target: "SrAnimation elements",
        relation: RuntimePointerRelation::OwnedVectorElement,
        invalidated_by: "layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrLayer",
        offset: 0x4C,
        target: "SrCast elements",
        relation: RuntimePointerRelation::OwnedVectorElement,
        invalidated_by: "layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrLayer",
        offset: 0x64,
        target: "root SrCast elements",
        relation: RuntimePointerRelation::NonOwningHierarchy,
        invalidated_by: "owning layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrAnimation",
        offset: 0x34,
        target: "parsed ANIM",
        relation: RuntimePointerRelation::BorrowedParsedRecord,
        invalidated_by: "resource unload",
    },
    RuntimePointerEdge {
        source: "surfride::SrAnimation",
        offset: 0x38,
        target: "owning SrLayer",
        relation: RuntimePointerRelation::BackPointer,
        invalidated_by: "owning layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrAnimationSet",
        offset: 0x04,
        target: "parsed ANMS",
        relation: RuntimePointerRelation::BorrowedParsedRecord,
        invalidated_by: "resource unload",
    },
    RuntimePointerEdge {
        source: "surfride::SrCast",
        offset: 0x04,
        target: "parsed NODE",
        relation: RuntimePointerRelation::BorrowedParsedRecord,
        invalidated_by: "resource unload",
    },
    RuntimePointerEdge {
        source: "surfride::SrCast",
        offset: 0x30,
        target: "parent SrCast",
        relation: RuntimePointerRelation::NonOwningHierarchy,
        invalidated_by: "owning layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrCast",
        offset: 0x34,
        target: "child SrCast elements",
        relation: RuntimePointerRelation::NonOwningHierarchy,
        invalidated_by: "owning layer destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrRefCast",
        offset: 0x1F4,
        target: "independent copied SrLayer",
        relation: RuntimePointerRelation::OwnedHeap,
        invalidated_by: "SrRefCast destruction",
    },
    RuntimePointerEdge {
        source: "surfride::SrImage",
        offset: 0xBC,
        target: "renderer/image resource handle",
        relation: RuntimePointerRelation::RefCountedResource,
        invalidated_by: "image state destruction or renderer cleanup",
    },
];

pub const SRPLAYER_LIFECYCLE: &[SrPlayerLifecycleTransition] = &[
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Constructed,
        to: SrPlayerLifecyclePhase::LoadRequested,
        evidence: "0xBA6BF0 validates filename, requests load, and sets load bits",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::LoadRequested,
        to: SrPlayerLifecyclePhase::WaitingForResource,
        evidence: "load invocation is asynchronous request/poll staging, not project readiness",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::WaitingForResource,
        to: SrPlayerLifecyclePhase::ProjectFinalized,
        evidence: "0xAAB990 stores loaded resource holder at Impl +0x08",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::ProjectFinalized,
        to: SrPlayerLifecyclePhase::RuntimeTreeBuilt,
        evidence: "0xAAAFE0 builds scenes; 0xAA9450 resolves copied reference layers",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::RuntimeTreeBuilt,
        to: SrPlayerLifecyclePhase::Ready,
        evidence: "original-scene font/resource pass follows reference construction",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Ready,
        to: SrPlayerLifecyclePhase::Evaluating,
        evidence: "set/frame selection and CAST-tree update mutate per-player runtime state",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Evaluating,
        to: SrPlayerLifecyclePhase::Rendering,
        evidence: "CAST virtual +0x18 enters 0xAD45E0 after render gates",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Rendering,
        to: SrPlayerLifecyclePhase::Ready,
        evidence: "packets are output; the evaluated runtime tree persists for the next frame",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Ready,
        to: SrPlayerLifecyclePhase::Releasing,
        evidence: "0xBA7B00 routes reset through SrCtrl then player virtual +0x30",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Releasing,
        to: SrPlayerLifecyclePhase::Released,
        evidence: "0xAACEA0 runs renderer cleanup and 0xAACDF0 resets the inner runtime tree",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Released,
        to: SrPlayerLifecyclePhase::LoadRequested,
        evidence: "the outer player and Impl survive content release and may submit another load",
    },
    SrPlayerLifecycleTransition {
        from: SrPlayerLifecyclePhase::Released,
        to: SrPlayerLifecyclePhase::Destroyed,
        evidence: "0xBA5230 destroys SrCtrl/strings/embedded player during final owner teardown",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    const RECOVERED_LAYOUTS: &[&X86ObjectLayout] = &[
        &PROJ_VIEW_SRPLAYER_LAYOUT,
        &SR_CTRL_LAYOUT,
        &SRPLAYER_IMPL_LAYOUT,
        &SR_SCENE_LAYOUT,
        &SR_LAYER_LAYOUT,
        &SR_ANIMATION_LAYOUT,
        &SR_ANIMATION_SET_LAYOUT,
        &SR_CAST_LAYOUT,
        &SR_IMAGE_LAYOUT,
        &SR_IMAGE_CAST_LAYOUT,
        &SR_SLICE_CAST_LAYOUT,
        &SR_TEXT_CAST_LAYOUT,
        &SR_REF_CAST_LAYOUT,
        &SR_NUMBER_CAST_LAYOUT,
    ];

    #[test]
    fn recovered_fields_are_ordered_and_fit_their_object_extent() {
        for layout in RECOVERED_LAYOUTS {
            for pair in layout.fields.windows(2) {
                assert!(
                    pair[0].offset < pair[1].offset,
                    "{} has unordered or duplicate fields at {:#x}",
                    layout.name,
                    pair[1].offset
                );
            }
            if let Some(size) = layout.allocation_size {
                for field in layout.fields {
                    assert!(
                        field.offset < size,
                        "{} field {} at {:#x} exceeds extent {:#x}",
                        layout.name,
                        field.name,
                        field.offset,
                        size
                    );
                }
            }
        }
    }

    #[test]
    fn cast_factory_allocations_match_the_concrete_layouts() {
        let concrete = [
            ("surfride::SrNullCast", SR_CAST_LAYOUT.allocation_size),
            (
                "surfride::SrImageCast",
                SR_IMAGE_CAST_LAYOUT.allocation_size,
            ),
            ("surfride::SrTextCast", SR_TEXT_CAST_LAYOUT.allocation_size),
            (
                "surfride::SrSliceCast",
                SR_SLICE_CAST_LAYOUT.allocation_size,
            ),
            ("surfride::SrRefCast", SR_REF_CAST_LAYOUT.allocation_size),
            (
                "surfride::SrNumberCast",
                SR_NUMBER_CAST_LAYOUT.allocation_size,
            ),
        ];
        for factory in CAST_FACTORIES {
            let size = concrete
                .iter()
                .find_map(|(name, size)| (*name == factory.cast_name).then_some(*size))
                .flatten();
            assert_eq!(size, Some(factory.allocation_size), "{}", factory.cast_name);
        }
    }

    #[test]
    fn project_holder_and_runtime_tree_have_distinct_ownership() {
        let holder = SRPLAYER_POINTER_GRAPH
            .iter()
            .find(|edge| edge.source == "surfride::SrPlayer::Impl" && edge.offset == 0x08)
            .unwrap();
        assert_eq!(
            holder.relation,
            RuntimePointerRelation::BorrowedResourceHolder
        );
        assert_eq!(holder.target, "surfride::SrResource project holder");

        let scenes = SRPLAYER_POINTER_GRAPH
            .iter()
            .find(|edge| edge.source == "surfride::SrPlayer::Impl" && edge.offset == 0x294)
            .unwrap();
        assert_eq!(scenes.relation, RuntimePointerRelation::OwnedVectorElement);

        let cast_children = SRPLAYER_POINTER_GRAPH
            .iter()
            .find(|edge| edge.source == "surfride::SrCast" && edge.offset == 0x34)
            .unwrap();
        assert_eq!(
            cast_children.relation,
            RuntimePointerRelation::NonOwningHierarchy
        );
    }
}
