//! Animation channel catalog and key-format facts used by the Animate console.
//!
//! Every entry is transcribed from `docs/evidence/animation-channels.md`, which
//! records the measured shipped corpus and the exact runtime consumer for each
//! `Track.target`. Channel 18 is deliberately absent: it has zero corpus tracks
//! and no runtime consumer, so the editor must not offer it for authoring.
//!
//! Format facts come from the parser dispatch at `crate::animation::Track`:
//! `format & 3` selects the key record layout, `format & 0x70` selects the value
//! family, and `format & 0x300` selects the wrapping path. Only the
//! `(layout, family)` pairs the parser accepts are offered here, so the Advanced
//! editors cannot construct a combination that would decode as `Unsupported`.

use crate::animation::{Key8, Key20, KeyData};

/// Value kind an editor must present for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelValue {
    /// IEEE float channel: translation, scale, opacity, geometry, frame request.
    Float,
    /// Signed integer channel: rotation units and visibility words.
    Integer,
    /// Packed 4-byte colour channel.
    Rgba,
    /// Signed integer reference selector (CREF / CRE1 image + rectangle index).
    Selector,
}

/// Which runtime object consumes the channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelScope {
    /// Applied by `SpatialTransform::apply_common_track` for every CAST.
    Common,
    /// Applied by `ImageDefinition::apply_runtime_track` for SrImage casts.
    Image,
    /// Applied by `SrRefCast` only.
    Reference,
}

/// Semantic interpolation the runtime actually implements for a `Key20` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySemantics {
    /// `mode == 0`: the runtime holds the left key's value across the segment.
    Hold,
    /// Any mode the runtime does not treat as hold or cubic.
    Linear,
    /// `mode == 2`: the runtime evaluates the slope-driven cubic.
    Cubic,
}

impl KeySemantics {
    pub const ALL: [Self; 3] = [Self::Hold, Self::Linear, Self::Cubic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Hold => "Hold",
            Self::Linear => "Linear",
            Self::Cubic => "Cubic",
        }
    }

    pub fn mode(self) -> u32 {
        match self {
            Self::Hold => 0,
            Self::Linear => 1,
            Self::Cubic => 2,
        }
    }

    pub fn from_mode(mode: u32) -> Self {
        match mode {
            0 => Self::Hold,
            2 => Self::Cubic,
            _ => Self::Linear,
        }
    }

    /// Cubic needs stored slopes, so it only exists on `Key20` records.
    pub fn uses_slopes(self) -> bool {
        matches!(self, Self::Cubic)
    }
}

/// Key record layout selected by `format & 3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyLayout {
    /// 8-byte records: frame plus value, always interpolated linearly.
    Key8,
    /// 20-byte records: frame, value, mode, and both slopes.
    Key20,
}

impl KeyLayout {
    pub fn label(self) -> &'static str {
        match self {
            Self::Key8 => "Key8",
            Self::Key20 => "Key20",
        }
    }
}

/// A parser-legal `Track.format` the editor may write for a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatChoice {
    /// Base format without the wrap bit.
    pub format: u32,
    pub layout: KeyLayout,
    /// Whether `mode == 2` reaches a cubic evaluator for this format.
    pub supports_cubic: bool,
    pub label: &'static str,
}

/// Wrap bit observed across the whole corpus (`0x100`); `0x200` selects the
/// same wrapping path in `wrap_time` and is preserved when already present.
pub const WRAP_BIT: u32 = 0x100;

const FLOAT_FORMATS: &[FormatChoice] = &[
    FormatChoice {
        format: 0x13,
        layout: KeyLayout::Key20,
        supports_cubic: true,
        label: "Key20 float · hold/linear/cubic",
    },
    FormatChoice {
        format: 0x10,
        layout: KeyLayout::Key8,
        supports_cubic: false,
        label: "Key8 float · linear",
    },
];

const INTEGER_FORMATS: &[FormatChoice] = &[
    FormatChoice {
        format: 0x43,
        layout: KeyLayout::Key20,
        supports_cubic: true,
        label: "Key20 int · hold/linear/cubic",
    },
    FormatChoice {
        format: 0x23,
        layout: KeyLayout::Key20,
        supports_cubic: false,
        label: "Key20 int · hold/linear",
    },
    FormatChoice {
        format: 0x40,
        layout: KeyLayout::Key8,
        supports_cubic: false,
        label: "Key8 int · linear",
    },
];

const RGBA_FORMATS: &[FormatChoice] = &[FormatChoice {
    format: 0x51,
    layout: KeyLayout::Key8,
    supports_cubic: false,
    label: "Key8 RGBA · linear",
}];

/// One authorable animation channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSpec {
    pub target: u16,
    /// Established runtime meaning from the evidence table.
    pub name: &'static str,
    /// Short group used to cluster the channel picker.
    pub group: &'static str,
    pub value: ChannelValue,
    pub scope: ChannelScope,
    /// Format written for a newly created track on this channel.
    pub default_format: u32,
    /// Semantics assigned to a newly created key on this channel.
    pub default_semantics: KeySemantics,
    /// Exact consumer citation, surfaced in the channel inspector.
    pub evidence: &'static str,
}

/// Every channel the runtime consumes. Channel 18 is intentionally missing.
pub const CHANNELS: &[ChannelSpec] = &[
    ChannelSpec {
        target: 0,
        name: "Position · X",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:55 — 0..=2 writes translation[target]",
    },
    ChannelSpec {
        target: 1,
        name: "Position · Y",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:55 — 0..=2 writes translation[target]",
    },
    ChannelSpec {
        target: 2,
        name: "Position · Z",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:55 — 0..=2 writes translation[target]",
    },
    ChannelSpec {
        target: 3,
        name: "Rotation · X",
        group: "Transform",
        value: ChannelValue::Integer,
        scope: ChannelScope::Common,
        default_format: 0x43,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:56 — 3..=5 writes rotation[target - 3]",
    },
    ChannelSpec {
        target: 4,
        name: "Rotation · Y",
        group: "Transform",
        value: ChannelValue::Integer,
        scope: ChannelScope::Common,
        default_format: 0x43,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:56 — 3..=5 writes rotation[target - 3]",
    },
    ChannelSpec {
        target: 5,
        name: "Rotation · Z",
        group: "Transform",
        value: ChannelValue::Integer,
        scope: ChannelScope::Common,
        default_format: 0x43,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:56 — 3..=5 writes rotation[target - 3]",
    },
    ChannelSpec {
        target: 6,
        name: "Scale · X",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:57 — 6..=8 writes scale[target - 6]",
    },
    ChannelSpec {
        target: 7,
        name: "Scale · Y",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:57 — 6..=8 writes scale[target - 6]",
    },
    ChannelSpec {
        target: 8,
        name: "Scale · Z",
        group: "Transform",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:57 — 6..=8 writes scale[target - 6]",
    },
    ChannelSpec {
        target: 9,
        name: "Multiply colour · RGB",
        group: "Colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Common,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:62-75 — 9 | 19 writes colour bytes",
    },
    ChannelSpec {
        target: 10,
        name: "Visibility word",
        group: "Visibility",
        value: ChannelValue::Integer,
        scope: ChannelScope::Common,
        default_format: 0x23,
        default_semantics: KeySemantics::Hold,
        evidence: "transform.rs:58 — 10 writes visibility_word",
    },
    ChannelSpec {
        target: 11,
        name: "Image geometry · width",
        group: "Image geometry",
        value: ChannelValue::Float,
        scope: ChannelScope::Image,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:320-332 maps 11 to size component 0",
    },
    ChannelSpec {
        target: 12,
        name: "Image geometry · height",
        group: "Image geometry",
        value: ChannelValue::Float,
        scope: ChannelScope::Image,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:320-332 maps 12 to size component 1",
    },
    ChannelSpec {
        target: 13,
        name: "CREF vertex colour · v0",
        group: "Vertex colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Image,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:558-572 maps 13 to vertex 0",
    },
    ChannelSpec {
        target: 14,
        name: "CREF vertex colour · v2",
        group: "Vertex colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Image,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:558-572 maps 14 to vertex 2",
    },
    ChannelSpec {
        target: 15,
        name: "CREF vertex colour · v1",
        group: "Vertex colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Image,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:558-572 maps 15 to vertex 1",
    },
    ChannelSpec {
        target: 16,
        name: "CREF vertex colour · v3",
        group: "Vertex colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Image,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "image.rs:558-572 maps 16 to vertex 3",
    },
    ChannelSpec {
        target: 17,
        name: "CREF selector · image + rectangle",
        group: "Selector",
        value: ChannelValue::Selector,
        scope: ChannelScope::Image,
        default_format: 0x23,
        default_semantics: KeySemantics::Hold,
        evidence: "image.rs:589-595 dispatches ImageReferenceChannel::Cref",
    },
    ChannelSpec {
        target: 19,
        name: "Additive colour · RGB",
        group: "Colour",
        value: ChannelValue::Rgba,
        scope: ChannelScope::Common,
        default_format: 0x51,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:62-75 — 9 | 19 writes colour bytes",
    },
    ChannelSpec {
        target: 20,
        name: "CRE1 selector · image + rectangle",
        group: "Selector",
        value: ChannelValue::Selector,
        scope: ChannelScope::Image,
        default_format: 0x23,
        default_semantics: KeySemantics::Hold,
        evidence: "image.rs:596-602 dispatches ImageReferenceChannel::Cre1",
    },
    ChannelSpec {
        target: 21,
        name: "Multiply Alpha",
        group: "Colour",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:76-85 — target 21 writes multiply_color[3]",
    },
    ChannelSpec {
        target: 22,
        name: "Additive Alpha",
        group: "Colour",
        value: ChannelValue::Float,
        scope: ChannelScope::Common,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "transform.rs:76-85 — the other branch writes additive_color[3]",
    },
    ChannelSpec {
        target: 23,
        name: "Reference animation frame request",
        group: "Reference",
        value: ChannelValue::Float,
        scope: ChannelScope::Reference,
        default_format: 0x13,
        default_semantics: KeySemantics::Linear,
        evidence: "reference.rs:72-95 gates target 23 for SrRefCast",
    },
];

pub fn channel(target: u16) -> Option<&'static ChannelSpec> {
    CHANNELS.iter().find(|spec| spec.target == target)
}

/// Display name for any parsed target, including ids the editor cannot author.
pub fn channel_name(target: u16) -> String {
    channel(target).map_or_else(
        || format!("Channel {target} · unattested"),
        |spec| spec.name.to_owned(),
    )
}

pub fn value_kind(target: u16) -> ChannelValue {
    channel(target).map_or(ChannelValue::Integer, |spec| spec.value)
}

pub fn format_choices(value: ChannelValue) -> &'static [FormatChoice] {
    match value {
        ChannelValue::Float => FLOAT_FORMATS,
        ChannelValue::Integer | ChannelValue::Selector => INTEGER_FORMATS,
        ChannelValue::Rgba => RGBA_FORMATS,
    }
}

/// Resolves a stored format to its offered choice, ignoring wrap bits and the
/// low-bit alias (`format & 3 == 1` decodes exactly like `0`).
pub fn format_choice(value: ChannelValue, format: u32) -> Option<&'static FormatChoice> {
    let layout = key_layout(format)?;
    let family = format & 0x70;
    format_choices(value)
        .iter()
        .find(|choice| choice.layout == layout && choice.format & 0x70 == family)
}

pub fn key_layout(format: u32) -> Option<KeyLayout> {
    match format & 3 {
        0 | 1 => Some(KeyLayout::Key8),
        3 => Some(KeyLayout::Key20),
        // `2` reaches `Evaluation::Unchanged`: the runtime ignores the keys.
        _ => None,
    }
}

pub fn wraps(format: u32) -> bool {
    format & 0x300 != 0
}

/// Applies or clears wrapping while preserving an existing `0x200` selection.
pub fn with_wrap(format: u32, wrap: bool) -> u32 {
    if wrap {
        if format & 0x300 == 0 {
            format | WRAP_BIT
        } else {
            format
        }
    } else {
        format & !0x300
    }
}

/// Whether `mode == 2` reaches a cubic evaluator for this exact format.
pub fn supports_cubic(format: u32) -> bool {
    format & 3 == 3 && matches!(format & 0x70, 0x10 | 0x40)
}

/// Semantics the runtime can actually express for a format. `Key8` records have
/// no mode word, so they are always linear.
pub fn available_semantics(format: u32) -> &'static [KeySemantics] {
    const HOLD_LINEAR_CUBIC: [KeySemantics; 3] = [
        KeySemantics::Hold,
        KeySemantics::Linear,
        KeySemantics::Cubic,
    ];
    const HOLD_LINEAR: [KeySemantics; 2] = [KeySemantics::Hold, KeySemantics::Linear];
    const LINEAR: [KeySemantics; 1] = [KeySemantics::Linear];

    match key_layout(format) {
        Some(KeyLayout::Key20) if supports_cubic(format) => &HOLD_LINEAR_CUBIC,
        Some(KeyLayout::Key20) => &HOLD_LINEAR,
        _ => &LINEAR,
    }
}

/// One key presented to the editors, independent of the stored record layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyView {
    pub frame: i32,
    pub value: KeyValue,
    pub semantics: KeySemantics,
    pub slope_in: f32,
    pub slope_out: f32,
    /// Raw mode word, preserved when the user does not change semantics.
    pub raw_mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyValue {
    Float(f32),
    Integer(i32),
    Rgba([u8; 4]),
}

impl KeyValue {
    pub fn as_float(self) -> f32 {
        match self {
            Self::Float(value) => value,
            Self::Integer(value) => value as f32,
            Self::Rgba(bytes) => f32::from_le_bytes(bytes),
        }
    }

    pub fn as_integer(self) -> i32 {
        match self {
            Self::Float(value) => value as i32,
            Self::Integer(value) => value,
            Self::Rgba(bytes) => i32::from_le_bytes(bytes),
        }
    }

    pub fn as_rgba(self) -> [u8; 4] {
        match self {
            Self::Float(value) => value.to_le_bytes(),
            Self::Integer(value) => value.to_le_bytes(),
            Self::Rgba(bytes) => bytes,
        }
    }
}

pub fn key_count(keys: &KeyData) -> usize {
    match keys {
        KeyData::Key8F32(values) => values.len(),
        KeyData::Key8I32(values) => values.len(),
        KeyData::Key8Bytes4(values) => values.len(),
        KeyData::Key20F32(values) => values.len(),
        KeyData::Key20I32(values) => values.len(),
        KeyData::Unsupported => 0,
    }
}

pub fn key_frame(keys: &KeyData, index: usize) -> Option<i32> {
    Some(match keys {
        KeyData::Key8F32(values) => values.get(index)?.frame,
        KeyData::Key8I32(values) => values.get(index)?.frame,
        KeyData::Key8Bytes4(values) => values.get(index)?.frame,
        KeyData::Key20F32(values) => values.get(index)?.frame,
        KeyData::Key20I32(values) => values.get(index)?.frame,
        KeyData::Unsupported => return None,
    })
}

pub fn key_view(keys: &KeyData, index: usize) -> Option<KeyView> {
    Some(match keys {
        KeyData::Key8F32(values) => {
            let key = values.get(index)?;
            KeyView {
                frame: key.frame,
                value: KeyValue::Float(key.value),
                semantics: KeySemantics::Linear,
                slope_in: 0.0,
                slope_out: 0.0,
                raw_mode: 0,
            }
        }
        KeyData::Key8I32(values) => {
            let key = values.get(index)?;
            KeyView {
                frame: key.frame,
                value: KeyValue::Integer(key.value),
                semantics: KeySemantics::Linear,
                slope_in: 0.0,
                slope_out: 0.0,
                raw_mode: 0,
            }
        }
        KeyData::Key8Bytes4(values) => {
            let key = values.get(index)?;
            KeyView {
                frame: key.frame,
                value: KeyValue::Rgba(key.value),
                semantics: KeySemantics::Linear,
                slope_in: 0.0,
                slope_out: 0.0,
                raw_mode: 0,
            }
        }
        KeyData::Key20F32(values) => {
            let key = values.get(index)?;
            KeyView {
                frame: key.frame,
                value: KeyValue::Float(key.value),
                semantics: KeySemantics::from_mode(key.mode),
                slope_in: key.slope_in,
                slope_out: key.slope_out,
                raw_mode: key.mode,
            }
        }
        KeyData::Key20I32(values) => {
            let key = values.get(index)?;
            KeyView {
                frame: key.frame,
                value: KeyValue::Integer(key.value),
                semantics: KeySemantics::from_mode(key.mode),
                slope_in: key.slope_in,
                slope_out: key.slope_out,
                raw_mode: key.mode,
            }
        }
        KeyData::Unsupported => return None,
    })
}

/// Empty key storage matching `format`. `None` means the parser would decode the
/// format as `Unsupported`, so the editor refuses to write it.
pub fn empty_keys(format: u32) -> Option<KeyData> {
    Some(match (key_layout(format)?, format & 0x70) {
        (KeyLayout::Key8, 0x10) => KeyData::Key8F32(Vec::new()),
        (KeyLayout::Key8, 0x40) => KeyData::Key8I32(Vec::new()),
        (KeyLayout::Key8, 0x50) => KeyData::Key8Bytes4(Vec::new()),
        (KeyLayout::Key20, 0x10) => KeyData::Key20F32(Vec::new()),
        (KeyLayout::Key20, 0x20 | 0x40) => KeyData::Key20I32(Vec::new()),
        _ => return None,
    })
}

/// Rewrites existing keys into the storage `format` selects, preserving frames,
/// values, and — where the destination keeps them — modes and slopes.
pub fn convert_keys(keys: &KeyData, format: u32) -> Option<KeyData> {
    let count = key_count(keys);
    let views: Vec<KeyView> = (0..count)
        .filter_map(|index| key_view(keys, index))
        .collect();
    Some(match empty_keys(format)? {
        KeyData::Key8F32(_) => KeyData::Key8F32(
            views
                .iter()
                .map(|view| Key8 {
                    frame: view.frame,
                    value: view.value.as_float(),
                })
                .collect(),
        ),
        KeyData::Key8I32(_) => KeyData::Key8I32(
            views
                .iter()
                .map(|view| Key8 {
                    frame: view.frame,
                    value: view.value.as_integer(),
                })
                .collect(),
        ),
        KeyData::Key8Bytes4(_) => KeyData::Key8Bytes4(
            views
                .iter()
                .map(|view| Key8 {
                    frame: view.frame,
                    value: view.value.as_rgba(),
                })
                .collect(),
        ),
        KeyData::Key20F32(_) => KeyData::Key20F32(
            views
                .iter()
                .map(|view| Key20 {
                    frame: view.frame,
                    value: view.value.as_float(),
                    mode: legal_mode(format, view),
                    slope_in: view.slope_in,
                    slope_out: view.slope_out,
                })
                .collect(),
        ),
        KeyData::Key20I32(_) => KeyData::Key20I32(
            views
                .iter()
                .map(|view| Key20 {
                    frame: view.frame,
                    value: view.value.as_integer(),
                    mode: legal_mode(format, view),
                    slope_in: view.slope_in,
                    slope_out: view.slope_out,
                })
                .collect(),
        ),
        KeyData::Unsupported => return None,
    })
}

/// Keeps the raw mode word when the semantics survive the format change, and
/// demotes cubic to linear when the destination format has no cubic evaluator.
fn legal_mode(format: u32, view: &KeyView) -> u32 {
    match view.semantics {
        KeySemantics::Cubic if !supports_cubic(format) => KeySemantics::Linear.mode(),
        _ if view.raw_mode != 0 || view.semantics == KeySemantics::Hold => view.raw_mode,
        semantics => semantics.mode(),
    }
}

/// Inserts a key at `frame`, inheriting the semantics of the segment it lands in
/// and falling back to the channel default for an empty track.
pub fn insert_key(keys: &mut KeyData, format: u32, frame: i32, default: KeySemantics) -> usize {
    let inherited = segment_semantics(keys, frame).unwrap_or(default);
    let mode = if available_semantics(format).contains(&inherited) {
        inherited.mode()
    } else {
        KeySemantics::Linear.mode()
    };
    match keys {
        KeyData::Key8F32(values) => {
            let index = insertion_index(values.iter().map(|key| key.frame), frame);
            let value = values
                .get(index.saturating_sub(1))
                .map_or(0.0, |key| key.value);
            values.insert(index, Key8 { frame, value });
            index
        }
        KeyData::Key8I32(values) => {
            let index = insertion_index(values.iter().map(|key| key.frame), frame);
            let value = values
                .get(index.saturating_sub(1))
                .map_or(0, |key| key.value);
            values.insert(index, Key8 { frame, value });
            index
        }
        KeyData::Key8Bytes4(values) => {
            let index = insertion_index(values.iter().map(|key| key.frame), frame);
            let value = values
                .get(index.saturating_sub(1))
                .map_or([255, 255, 255, 255], |key| key.value);
            values.insert(index, Key8 { frame, value });
            index
        }
        KeyData::Key20F32(values) => {
            let index = insertion_index(values.iter().map(|key| key.frame), frame);
            let previous = values.get(index.saturating_sub(1));
            let value = previous.map_or(0.0, |key| key.value);
            values.insert(
                index,
                Key20 {
                    frame,
                    value,
                    mode,
                    slope_in: 0.0,
                    slope_out: 0.0,
                },
            );
            index
        }
        KeyData::Key20I32(values) => {
            let index = insertion_index(values.iter().map(|key| key.frame), frame);
            let previous = values.get(index.saturating_sub(1));
            let value = previous.map_or(0, |key| key.value);
            values.insert(
                index,
                Key20 {
                    frame,
                    value,
                    mode,
                    slope_in: 0.0,
                    slope_out: 0.0,
                },
            );
            index
        }
        KeyData::Unsupported => 0,
    }
}

/// Semantics of the segment containing `frame`, taken from its left key.
fn segment_semantics(keys: &KeyData, frame: i32) -> Option<KeySemantics> {
    let count = key_count(keys);
    if count == 0 {
        return None;
    }
    let mut left = None;
    for index in 0..count {
        let key_frame = key_frame(keys, index)?;
        if key_frame <= frame {
            left = Some(index);
        } else {
            break;
        }
    }
    key_view(keys, left.unwrap_or(0)).map(|view| view.semantics)
}

fn insertion_index(frames: impl Iterator<Item = i32>, frame: i32) -> usize {
    let mut index = 0usize;
    for existing in frames {
        if existing > frame {
            break;
        }
        index += 1;
    }
    index
}

pub fn remove_key(keys: &mut KeyData, index: usize) -> bool {
    let count = key_count(keys);
    if index >= count {
        return false;
    }
    match keys {
        KeyData::Key8F32(values) => drop(values.remove(index)),
        KeyData::Key8I32(values) => drop(values.remove(index)),
        KeyData::Key8Bytes4(values) => drop(values.remove(index)),
        KeyData::Key20F32(values) => drop(values.remove(index)),
        KeyData::Key20I32(values) => drop(values.remove(index)),
        KeyData::Unsupported => return false,
    }
    true
}

pub fn set_key_frame(keys: &mut KeyData, index: usize, frame: i32) -> bool {
    match keys {
        KeyData::Key8F32(values) => match values.get_mut(index) {
            Some(key) => key.frame = frame,
            None => return false,
        },
        KeyData::Key8I32(values) => match values.get_mut(index) {
            Some(key) => key.frame = frame,
            None => return false,
        },
        KeyData::Key8Bytes4(values) => match values.get_mut(index) {
            Some(key) => key.frame = frame,
            None => return false,
        },
        KeyData::Key20F32(values) => match values.get_mut(index) {
            Some(key) => key.frame = frame,
            None => return false,
        },
        KeyData::Key20I32(values) => match values.get_mut(index) {
            Some(key) => key.frame = frame,
            None => return false,
        },
        KeyData::Unsupported => return false,
    }
    sort_keys(keys);
    true
}

pub fn sort_keys(keys: &mut KeyData) {
    match keys {
        KeyData::Key8F32(values) => values.sort_by_key(|key| key.frame),
        KeyData::Key8I32(values) => values.sort_by_key(|key| key.frame),
        KeyData::Key8Bytes4(values) => values.sort_by_key(|key| key.frame),
        KeyData::Key20F32(values) => values.sort_by_key(|key| key.frame),
        KeyData::Key20I32(values) => values.sort_by_key(|key| key.frame),
        KeyData::Unsupported => {}
    }
}

pub fn set_key_value(keys: &mut KeyData, index: usize, value: KeyValue) -> bool {
    match keys {
        KeyData::Key8F32(values) => match values.get_mut(index) {
            Some(key) => key.value = value.as_float(),
            None => return false,
        },
        KeyData::Key8I32(values) => match values.get_mut(index) {
            Some(key) => key.value = value.as_integer(),
            None => return false,
        },
        KeyData::Key8Bytes4(values) => match values.get_mut(index) {
            Some(key) => key.value = value.as_rgba(),
            None => return false,
        },
        KeyData::Key20F32(values) => match values.get_mut(index) {
            Some(key) => key.value = value.as_float(),
            None => return false,
        },
        KeyData::Key20I32(values) => match values.get_mut(index) {
            Some(key) => key.value = value.as_integer(),
            None => return false,
        },
        KeyData::Unsupported => return false,
    }
    true
}

/// Applies semantics only when the format's evaluator implements them.
pub fn set_key_semantics(
    keys: &mut KeyData,
    format: u32,
    index: usize,
    semantics: KeySemantics,
) -> bool {
    if !available_semantics(format).contains(&semantics) {
        return false;
    }
    match keys {
        KeyData::Key20F32(values) => match values.get_mut(index) {
            Some(key) => key.mode = semantics.mode(),
            None => return false,
        },
        KeyData::Key20I32(values) => match values.get_mut(index) {
            Some(key) => key.mode = semantics.mode(),
            None => return false,
        },
        _ => return false,
    }
    true
}

pub fn set_key_slope(keys: &mut KeyData, index: usize, incoming: bool, slope: f32) -> bool {
    match keys {
        KeyData::Key20F32(values) => match values.get_mut(index) {
            Some(key) => {
                if incoming {
                    key.slope_in = slope;
                } else {
                    key.slope_out = slope;
                }
            }
            None => return false,
        },
        KeyData::Key20I32(values) => match values.get_mut(index) {
            Some(key) => {
                if incoming {
                    key.slope_in = slope;
                } else {
                    key.slope_out = slope;
                }
            }
            None => return false,
        },
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_eighteen_is_not_authorable() {
        assert!(channel(18).is_none());
        assert!(CHANNELS.iter().all(|spec| spec.target != 18));
    }

    #[test]
    fn catalog_covers_every_consumed_channel_exactly_once() {
        let targets: Vec<u16> = CHANNELS.iter().map(|spec| spec.target).collect();
        let expected: Vec<u16> = (0..=17).chain(19..=23).collect();
        assert_eq!(targets, expected);
    }

    #[test]
    fn alpha_track_names_match_the_inspector_groups() {
        assert_eq!(channel_name(21), "Multiply Alpha");
        assert_eq!(channel_name(22), "Additive Alpha");
    }

    #[test]
    fn cubic_is_offered_only_where_the_runtime_evaluates_it() {
        assert!(supports_cubic(0x13));
        assert!(supports_cubic(0x43));
        assert!(!supports_cubic(0x23));
        assert!(!supports_cubic(0x51));
        assert_eq!(available_semantics(0x23).len(), 2);
        assert_eq!(available_semantics(0x51), &[KeySemantics::Linear]);
    }

    #[test]
    fn every_offered_format_decodes_to_real_storage() {
        for value in [
            ChannelValue::Float,
            ChannelValue::Integer,
            ChannelValue::Rgba,
            ChannelValue::Selector,
        ] {
            for choice in format_choices(value) {
                assert!(empty_keys(choice.format).is_some());
                assert!(empty_keys(with_wrap(choice.format, true)).is_some());
            }
        }
    }

    #[test]
    fn wrap_preserves_an_existing_high_bit_selection() {
        assert_eq!(with_wrap(0x13, true), 0x113);
        assert_eq!(with_wrap(0x213, true), 0x213);
        assert_eq!(with_wrap(0x213, false), 0x13);
    }

    #[test]
    fn inserted_key_inherits_the_segment_semantics() {
        let mut keys = KeyData::Key20F32(vec![
            Key20 {
                frame: 0,
                value: 1.0,
                mode: 0,
                slope_in: 0.0,
                slope_out: 0.0,
            },
            Key20 {
                frame: 20,
                value: 4.0,
                mode: 2,
                slope_in: 0.0,
                slope_out: 0.0,
            },
        ]);
        let index = insert_key(&mut keys, 0x13, 10, KeySemantics::Linear);
        assert_eq!(index, 1);
        assert_eq!(key_view(&keys, 1).unwrap().semantics, KeySemantics::Hold);
        assert_eq!(key_view(&keys, 1).unwrap().value, KeyValue::Float(1.0));
    }

    #[test]
    fn converting_to_a_linear_only_format_demotes_cubic_keys() {
        let keys = KeyData::Key20I32(vec![Key20 {
            frame: 3,
            value: 9,
            mode: 2,
            slope_in: 0.5,
            slope_out: 0.25,
        }]);
        let converted = convert_keys(&keys, 0x23).unwrap();
        assert_eq!(
            key_view(&converted, 0).unwrap().semantics,
            KeySemantics::Linear
        );
        let key8 = convert_keys(&keys, 0x40).unwrap();
        assert!(matches!(key8, KeyData::Key8I32(_)));
        assert_eq!(key_frame(&key8, 0), Some(3));
    }

    #[test]
    fn semantics_are_rejected_for_formats_without_an_evaluator() {
        let mut keys = KeyData::Key20I32(vec![Key20 {
            frame: 0,
            value: 0,
            mode: 1,
            slope_in: 0.0,
            slope_out: 0.0,
        }]);
        assert!(!set_key_semantics(&mut keys, 0x23, 0, KeySemantics::Cubic));
        assert!(set_key_semantics(&mut keys, 0x23, 0, KeySemantics::Hold));
        assert!(set_key_semantics(&mut keys, 0x43, 0, KeySemantics::Cubic));
    }

    #[test]
    fn visibility_and_selector_channels_default_to_hold() {
        assert_eq!(channel(10).unwrap().default_semantics, KeySemantics::Hold);
        assert_eq!(channel(17).unwrap().default_semantics, KeySemantics::Hold);
        assert_eq!(channel(20).unwrap().default_semantics, KeySemantics::Hold);
        assert_eq!(channel(0).unwrap().default_semantics, KeySemantics::Linear);
    }
}
