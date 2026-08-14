use crate::attribute::FontParamData;
use crate::projection::{Matrix4x4, identity_matrix4x4_game, mul_matrix4x4_game};
use crate::ruhuna::{RuhunaRuntimeFont, RuhunaRuntimeGlyphRecord};
use crate::text::{TextDefinition, fennel_alignment_code_from_text_flags};

/// The FontManager initialization at `0x7B719C..0x7B71A4` passes encoding
/// mode zero and 0x20 font slots. `font::TextBoxObject::setTextByString`
/// (`sub_7C8F40`) dispatches mode zero to `sub_F525D0`, the game's UTF-8 to
/// UTF-16 converter.
pub const FENNEL_GAME_ENCODING_UTF8: u32 = 0;

/// `sub_F32B40` is constructed with `0x20` entries by the game startup path.
/// Each entry is twelve bytes and is considered occupied when its resource
/// pointer at `+0x04` is non-null.
pub const FENNEL_FONT_SLOT_COUNT: u16 = 0x20;

/// One acquisition of a cached font resource.
///
/// The resource handle is deliberately editor-opaque. The game also returns a
/// cache handle distinct from the global Fennel slot; consumers must use
/// `font_slot_id` for `$F[n]` glyph lookup and the handle for lifetime release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelFontSlotRequest {
    pub resource_handle: usize,
    pub font_slot_id: u16,
    pub registered: bool,
    pub first_request: bool,
    pub lease_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelFontSlotRelease {
    Retained {
        resource_handle: usize,
        remaining_leases: u32,
    },
    Destroyed {
        resource_handle: usize,
        font_slot_id: u16,
        was_registered: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelUnknownFontResourceHandle(pub usize);

impl std::fmt::Display for FennelUnknownFontResourceHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unknown Fennel font resource handle {}", self.0)
    }
}

impl std::error::Error for FennelUnknownFontResourceHandle {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FennelCachedFontResource<K> {
    key: K,
    font_slot_id: u16,
    registered: bool,
    lease_count: u32,
}

/// Reproduces the resource-cache and global-slot behavior closed through
/// `sub_7B96D0`, `sub_7B7300`, `sub_7BAF00`, `sub_F33200`, `sub_7BBA60`, and
/// `sub_F33310`.
///
/// A first request receives the lowest currently unoccupied slot. Repeated
/// requests for the same caller-defined resource identity reuse the cached
/// resource and slot. Releasing its final lease destroys the resource and
/// clears that exact slot, making it the next lowest-free candidate.
///
/// The full-table edge is intentionally preserved: `sub_7BAF00` returns the
/// slot count (`0x20`), registration rejects that out-of-range id, and the
/// resource remains cached with slot id `0x20` but no manager registration.
/// Absolute slot ids are process-global in the game, so callers may populate
/// this registry with earlier host requests before applying an SRD's requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelFontSlotRegistry<K> {
    resources: Vec<Option<FennelCachedFontResource<K>>>,
    slots: [Option<usize>; FENNEL_FONT_SLOT_COUNT as usize],
}

impl<K> Default for FennelFontSlotRegistry<K> {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            slots: [None; FENNEL_FONT_SLOT_COUNT as usize],
        }
    }
}

impl<K: Eq> FennelFontSlotRegistry<K> {
    pub fn request(&mut self, key: K) -> FennelFontSlotRequest {
        if let Some((resource_index, resource)) =
            self.resources
                .iter_mut()
                .enumerate()
                .find_map(|(index, resource)| {
                    resource
                        .as_mut()
                        .filter(|resource| resource.key == key)
                        .map(|resource| (index, resource))
                })
        {
            resource.lease_count = resource
                .lease_count
                .checked_add(1)
                .expect("Fennel font resource lease count overflow is outside the proven domain");
            return FennelFontSlotRequest {
                resource_handle: resource_index + 1,
                font_slot_id: resource.font_slot_id,
                registered: resource.registered,
                first_request: false,
                lease_count: resource.lease_count,
            };
        }

        let free_slot = self.slots.iter().position(Option::is_none);
        let font_slot_id = free_slot
            .and_then(|slot| u16::try_from(slot).ok())
            .unwrap_or(FENNEL_FONT_SLOT_COUNT);
        let registered = free_slot.is_some();
        let resource_index = self.resources.len();
        let resource_handle = resource_index + 1;
        self.resources.push(Some(FennelCachedFontResource {
            key,
            font_slot_id,
            registered,
            lease_count: 1,
        }));
        if let Some(slot) = free_slot {
            self.slots[slot] = Some(resource_handle);
        }
        FennelFontSlotRequest {
            resource_handle,
            font_slot_id,
            registered,
            first_request: true,
            lease_count: 1,
        }
    }

    pub fn release(
        &mut self,
        resource_handle: usize,
    ) -> Result<FennelFontSlotRelease, FennelUnknownFontResourceHandle> {
        let resource_index = resource_handle
            .checked_sub(1)
            .ok_or(FennelUnknownFontResourceHandle(resource_handle))?;
        let resource = self
            .resources
            .get_mut(resource_index)
            .and_then(Option::as_mut)
            .ok_or(FennelUnknownFontResourceHandle(resource_handle))?;
        if resource.lease_count > 1 {
            resource.lease_count -= 1;
            return Ok(FennelFontSlotRelease::Retained {
                resource_handle,
                remaining_leases: resource.lease_count,
            });
        }

        let font_slot_id = resource.font_slot_id;
        let was_registered = resource.registered;
        if was_registered {
            let slot = usize::from(font_slot_id);
            debug_assert_eq!(self.slots[slot], Some(resource_handle));
            self.slots[slot] = None;
        }
        self.resources[resource_index] = None;
        Ok(FennelFontSlotRelease::Destroyed {
            resource_handle,
            font_slot_id,
            was_registered,
        })
    }

    pub fn resource_for_slot(&self, font_slot_id: u16) -> Option<&K> {
        let resource_handle = *self.slots.get(usize::from(font_slot_id))?.as_ref()?;
        self.resources
            .get(resource_handle - 1)?
            .as_ref()
            .map(|resource| &resource.key)
    }

    pub fn request_for_key(&self, key: &K) -> Option<FennelFontSlotRequest> {
        self.resources
            .iter()
            .enumerate()
            .find_map(|(resource_index, resource)| {
                let resource = resource.as_ref().filter(|resource| &resource.key == key)?;
                Some(FennelFontSlotRequest {
                    resource_handle: resource_index + 1,
                    font_slot_id: resource.font_slot_id,
                    registered: resource.registered,
                    first_request: false,
                    lease_count: resource.lease_count,
                })
            })
    }
}

/// `sub_F32B40` stores this value at FontManager implementation offset +0x08;
/// `sub_F321E0` returns it to the token iterator as the command prefix.
pub const FENNEL_CONTROL_PREFIX: u16 = b'$' as u16;
const FENNEL_NEWLINE_COMMAND_UPPER: u16 = b'N' as u16;
const FENNEL_NEWLINE_COMMAND_LOWER: u16 = b'n' as u16;
const FENNEL_POSITION_COMMAND_UPPER: u16 = b'T' as u16;
const FENNEL_POSITION_COMMAND_LOWER: u16 = b't' as u16;
const FENNEL_EFFECT_COMMAND_UPPER: u16 = b'S' as u16;
const FENNEL_EFFECT_COMMAND_LOWER: u16 = b's' as u16;
const FENNEL_COLOR_COMMAND_UPPER: u16 = b'C' as u16;
const FENNEL_COLOR_COMMAND_LOWER: u16 = b'c' as u16;
const FENNEL_FONT_COMMAND_UPPER: u16 = b'F' as u16;
const FENNEL_FONT_COMMAND_LOWER: u16 = b'f' as u16;

pub const FENNEL_RECORD_LINE_END: i32 = -1;
pub const FENNEL_RECORD_ZERO_WIDTH_GLYPH: i32 = -2;
pub const FENNEL_RECORD_LINE_TABLE_OVERFLOW: i32 = -0xFE;
pub const FENNEL_RECORD_STREAM_END: i32 = -0xFF;
pub const FENNEL_LINE_START_CAPACITY: usize = 0x80;
pub const FENNEL_UV_BIAS: f32 = f32::from_bits(0x3727_C5AC);
pub const FENNEL_TRIANGLE_CORNER_INDICES: [usize; 6] = [1, 0, 2, 3, 1, 2];
/// Iterator state `+0x820`, token `+0x10`, and layout record `+0x0C` use this
/// bit to request the second/effect glyph segment.
pub const FENNEL_EFFECT_GLYPH_FLAG: u32 = 0x0004_0000;
/// `fennel::FontObject` construction (`sub_F2BB60`) initializes the four
/// effect colors at TextBoxObject `+0x80..+0x8C` to opaque black.
pub const FENNEL_DEFAULT_EFFECT_COLORS: [u32; 4] = [0xFF00_0000; 4];
/// The same constructor initializes the effect displacement at
/// TextBoxObject `+0x90/+0x94` to `(2.0, 2.0)`.
pub const FENNEL_DEFAULT_EFFECT_OFFSET: [f32; 2] = [2.0, 2.0];
/// `sub_7BEB80` asks the hash-container prime table for at least 11 buckets;
/// the first table entry is `0x11`, so a TextBoxObject starts with 17 buckets.
pub const FENNEL_INITIAL_TEXTURE_BATCH_BUCKET_COUNT: u32 = 17;

/// `sub_7C10B0` selects its local-rectangle clipping and UV-remap branch from
/// TextBoxObject flags bit `0x400`.
pub const FENNEL_TEXTBOX_CLIP_FLAG: u32 = 0x400;
/// In the clipping branch, bit `0x4000` changes the Y clamp interval from
/// `[-height * 0.5, height * 1.5]` to `[0, height]`.
pub const FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG: u32 = 0x4000;

/// The three-way layout dispatch at `sub_7C90A0` after record-stream
/// construction. Names intentionally retain the selecting flag because the
/// two non-default layout semantics are not yet fully closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelLayoutDispatch {
    Flag20,
    Flag40,
    Default,
}

/// Reproduces the exact priority of the `sub_7C90A0` layout switch:
/// `0x20` wins when both bits are present, then `0x40`, then `sub_7C1F90`.
pub const fn fennel_layout_dispatch(textbox_flags: u32) -> FennelLayoutDispatch {
    if textbox_flags & 0x20 != 0 {
        FennelLayoutDispatch::Flag20
    } else if textbox_flags & 0x40 != 0 {
        FennelLayoutDispatch::Flag40
    } else {
        FennelLayoutDispatch::Default
    }
}

/// Reproduces the flags left on a freshly constructed SRD TextBoxObject by
/// `sub_AC6F50` after it clears the old alignment/special-mode bits and applies
/// the SrTextCast state mode at `+0x108`.
///
/// TextBox construction supplies the preserved low bits `3`. TEXT flags bit
/// zero skips the mode switch entirely. Modes `2..=6` all include clipping;
/// modes `5..=6` additionally select the zero-based Y interval.
pub const fn fennel_fresh_srd_textbox_flags(text_flags: u32, mode: u32) -> u32 {
    const BASE: u32 = 3;
    if text_flags & 1 != 0 {
        return BASE;
    }
    BASE | match mode {
        0 => 0x0004,
        1 => 0x000c,
        2 => 0x1ca0,
        3 => 0x0ca0,
        4 => 0x2ca0,
        5 => 0x6c00,
        6 => 0x7c00,
        _ => 0,
    }
}

/// Applies the three TextBox flag setters called by `sub_AC6F50` after its
/// mode switch. These `FontParamData` values can clear the constructor's low
/// bits as well as set the monospaced-layout bit `0x200`.
pub const fn fennel_srd_textbox_flags(text_flags: u32, font_param: FontParamData) -> u32 {
    let flags = fennel_fresh_srd_textbox_flags(text_flags, font_param.no_wrap_put_mode);
    (flags & !(1 | 2 | 0x200))
        | (font_param.prohibition as u32)
        | ((font_param.word_wrap as u32) << 1)
        | ((font_param.monospaced as u32) << 9)
}

/// Converts the color stored by the CATR parser into the AARRGGBB value
/// written to TextBox `+0x80..+0x8C`. `sub_AC6F50` reads the source bytes in
/// the exact order `2,1,0,3` before calling the four-byte setter.
pub const fn fennel_font_param_effect_color(source: u32) -> u32 {
    let bytes = source.to_le_bytes();
    u32::from_le_bytes([bytes[2], bytes[1], bytes[0], bytes[3]])
}

/// The binary-proven parts of the 24-byte FontObject style copied by
/// `sub_AC6F50 -> sub_F2C100`. The per-token character code occupying style
/// bytes `+0x00/+0x01` is deliberately omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelSrdFontStyle {
    /// Style `+0x02`; `sub_F2C100` clears its high byte after the initializer
    /// has selected `max(source, 1)` using a signed comparison.
    pub point_x: u8,
    /// Style `+0x04`, transformed by the same instruction chain as point X.
    pub point_y: u8,
    /// Style `+0x08`; copied by `FontDriverRFO` to runtime glyph `+0x08` and
    /// by `sub_7C90A0` to layout record `+0x0C`.
    pub record_flags: u32,
    /// Style `+0x0C`. A zero FontParam leaves the constructor value `-1`.
    pub bold_value: i32,
    /// Style `+0x10`, always masked to four bits by `sub_F2C100`.
    pub outline_value: u32,
    /// Style `+0x14`, always masked to two bits by `sub_F2C100`.
    pub italic_value: u32,
}

const fn fennel_srd_point_byte(source: i32) -> u8 {
    let selected = if source > 1 { source } else { 1 };
    selected as u8
}

/// Reproduces the initial SrTextCast style mutation before the token iterator
/// writes a character code into style `+0x00`. `faceId` is absent from
/// `sub_AC6F50`; the RFZ `FontDriverRFO` lookup reads only that character code
/// and copies `record_flags`, so it does not consume either point byte or the
/// three numeric style values when selecting a runtime glyph.
pub const fn fennel_srd_font_style(font_param: FontParamData) -> FennelSrdFontStyle {
    let mut record_flags = 1;
    if font_param.bold != 0 {
        record_flags |= 2;
    }
    if font_param.italic != 0 {
        record_flags |= 4;
    }
    if font_param.outline != 0 {
        record_flags |= 8;
    }
    if font_param.display_shadow {
        record_flags |= FENNEL_EFFECT_GLYPH_FLAG;
    }
    let bold_value = if font_param.bold == 0 {
        -1
    } else if font_param.bold > 0 {
        font_param.bold & 0xF
    } else {
        font_param.bold
    };
    FennelSrdFontStyle {
        point_x: fennel_srd_point_byte(font_param.point_x),
        point_y: fennel_srd_point_byte(font_param.point_y),
        record_flags,
        bold_value,
        outline_value: font_param.outline as u32 & 0xF,
        italic_value: font_param.italic as u32 & 3,
    }
}

/// The exact state touched by the non-virtual SrTextCast method at
/// `0xADA300..0xADA348`. Field names retain their offsets within the text
/// state at `SrTextCast+0x1F4`; their higher-level authoring names are not yet
/// proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelSrdMode6ControlState {
    /// The first DWORD of the referenced TEXT definition.
    pub text_flags: u32,
    /// Text state `+0x108` (`SrTextCast+0x2FC`).
    pub mode_108: u32,
    /// Text state `+0x12C` (`SrTextCast+0x320`).
    pub field_12c: i32,
    /// Text state `+0x130` (`SrTextCast+0x324`).
    pub field_130: i32,
    /// Text state `+0x134` (`SrTextCast+0x328`).
    pub field_134: i32,
}

/// Reproduces the complete state mutation at `0xADA300..0xADA348`.
///
/// Enabling clears TEXT flags bit zero, stores the three arguments in the
/// binary's `+0x130`, `+0x134`, `+0x12C` order, and selects mode 6. Disabling
/// sets TEXT flags bit zero and restores mode 0 without clearing the three
/// retained values.
pub fn fennel_apply_srd_mode6_control(
    state: &mut FennelSrdMode6ControlState,
    enabled: bool,
    field_130: i32,
    field_134: i32,
    field_12c: i32,
) {
    if enabled {
        state.text_flags &= !1;
        state.field_130 = field_130;
        state.field_134 = field_134;
        state.mode_108 = 6;
        state.field_12c = field_12c;
    } else {
        state.text_flags |= 1;
        state.mode_108 = 0;
    }
}

/// Exact single-precision constant at `0x190E308`, multiplied after the
/// host-frame value and the SrTextCast update argument by `sub_AD8D00`.
pub const FENNEL_SRD_CLOCK_SCALE: f32 = f32::from_bits(0x3C88_8889);

/// SrTextCast runtime fields used by `sub_AD8D50`, `sub_AD8D00`, and
/// `sub_AC5740`. Offset names are retained until the surrounding runtime text
/// state has a complete evidence-backed host representation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelSrdScrollState {
    /// Text state `+0xF4` (`SrTextCast+0x2E8`).
    pub field_f4: f32,
    /// Text state `+0xF8` (`SrTextCast+0x2EC`).
    pub field_f8: i32,
    /// Text state `+0xFC` (`SrTextCast+0x2F0`).
    pub field_fc: i32,
    pub field_130: i32,
    pub field_134: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelSrdScrollOutputs {
    /// TextBoxObject `+0x2C0`, or `-1` when state `+0xF8 <= 0`.
    pub maximum_glyphs: i32,
    pub field_2c4: f32,
    pub field_2c8: f32,
    pub field_2cc: f32,
}

/// Exact single-precision threshold compared by the two horizontal period
/// branches in `sub_7C04F0` (`dword_1796030`).
pub const FENNEL_SRD_SCROLL_PERIOD_EPSILON: f32 = f32::from_bits(0x3400_0000);
/// `font::FontManager` constructor field `+0x34`, read by the horizontal
/// mode-4 repeated-string builder in `sub_7C04F0`.
pub const FENNEL_DEFAULT_REPEAT_SPACE_COUNT: i32 = 3;
/// `font::FontManager` constructor field `+0x38`, used as `$D`'s value when
/// the selected control has no complete bracket argument.
pub const FENNEL_DEFAULT_D_VALUE: i32 = 20;
/// `unk_18E8078` is CP932 `0x81,0x40`; `sub_103DC90` converts it through
/// UTF-16 to the UTF-8 text consumed by Fennel.
pub const FENNEL_REPEAT_SPACE_UTF8: &[u8; 3] = b"\xE3\x80\x80";

/// Measurements consumed after `sub_7C04F0` has laid out the current text.
/// `repeated_text_size` is deliberately supplied by the caller: the binary
/// measures a second, actually laid-out string rather than deriving its size
/// arithmetically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelSrdDrawPreparationInput {
    pub textbox_flags: u32,
    pub text_size: [f32; 2],
    pub clip_size: [f32; 2],
    pub font_point_y: u16,
    pub scroll: FennelSrdScrollOutputs,
    /// Horizontal non-`0x1000`: `text + gap + text`, where mode `0x2000`
    /// inserts the host-configured count of full-width spaces. Vertical:
    /// `text + "$n$n$n" + text`.
    pub repeated_text_size: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FennelSrdDrawPreparation {
    DrawOffset([f32; 2]),
    /// The `0x800` fit guard temporarily clears exactly `0x7CA0`, rebuilds
    /// the TextBox records, restores the original flags, and then draws with
    /// a zero offset.
    RelayoutWithoutScrollModes {
        layout_flags: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelSrdDrawPreparationError {
    MissingRepeatedTextMeasurement { textbox_flags: u32 },
}

impl std::fmt::Display for FennelSrdDrawPreparationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRepeatedTextMeasurement { textbox_flags } => write!(
                formatter,
                "Fennel TextBox flags {textbox_flags:#010x} require the binary's repeated-text measurement"
            ),
        }
    }
}

impl std::error::Error for FennelSrdDrawPreparationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelSrdRepeatedTextError {
    LengthOverflow,
    AllocationFailed,
}

impl std::fmt::Display for FennelSrdRepeatedTextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LengthOverflow => {
                formatter.write_str("Fennel repeated text length overflows usize")
            }
            Self::AllocationFailed => formatter.write_str("Fennel repeated text allocation failed"),
        }
    }
}

impl std::error::Error for FennelSrdRepeatedTextError {}

/// Constructs the exact auxiliary text that `sub_7C04F0` submits for a second
/// layout/measurement pass. `None` means the selected branch does not issue
/// that pass.
pub fn build_fennel_srd_repeated_text(
    source: &[u8],
    textbox_flags: u32,
    repeat_space_count: i32,
) -> Result<Option<Vec<u8>>, FennelSrdRepeatedTextError> {
    if textbox_flags & 0x4020 == 0 || textbox_flags & 0x20 != 0 && textbox_flags & 0x1000 != 0 {
        return Ok(None);
    }

    let separator = if textbox_flags & 0x20 != 0 {
        if textbox_flags & 0x2000 != 0 {
            FENNEL_REPEAT_SPACE_UTF8.as_slice()
        } else {
            &[]
        }
    } else {
        b"$n$n$n"
    };
    let separator_count = if textbox_flags & 0x20 != 0 {
        usize::try_from(repeat_space_count.max(0)).expect("nonnegative i32 fits usize")
    } else {
        1
    };
    let separator_bytes = separator
        .len()
        .checked_mul(separator_count)
        .ok_or(FennelSrdRepeatedTextError::LengthOverflow)?;
    let capacity = source
        .len()
        .checked_mul(2)
        .and_then(|length| length.checked_add(separator_bytes))
        .ok_or(FennelSrdRepeatedTextError::LengthOverflow)?;
    let mut repeated = Vec::new();
    repeated
        .try_reserve_exact(capacity)
        .map_err(|_| FennelSrdRepeatedTextError::AllocationFailed)?;
    repeated.extend_from_slice(source);
    for _ in 0..separator_count {
        repeated.extend_from_slice(separator);
    }
    repeated.extend_from_slice(source);
    Ok(Some(repeated))
}

/// Reproduces the fit guard and final two-float draw preparation at
/// `0x7C0582..0x7C0A4A`. The caller remains responsible for constructing and
/// laying out the repeated string documented by `repeated_text_size`.
pub fn prepare_fennel_srd_draw(
    input: FennelSrdDrawPreparationInput,
) -> Result<FennelSrdDrawPreparation, FennelSrdDrawPreparationError> {
    let flags = input.textbox_flags;
    if flags & 0x4020 == 0 {
        return Ok(FennelSrdDrawPreparation::DrawOffset([0.0; 2]));
    }

    let [text_width, text_height] = input.text_size;
    let [clip_width, clip_height] = input.clip_size;
    if flags & 0x800 != 0 {
        let fits = if flags & 0x20 != 0 {
            // `comiss text_width, clip_width` followed by `jbe` also treats
            // unordered input as fitting.
            !(text_width > clip_width)
        } else {
            let point_y_half = (input.font_point_y >> 1) as f32;
            !(text_height - point_y_half > clip_height)
        };
        if fits {
            return Ok(FennelSrdDrawPreparation::RelayoutWithoutScrollModes {
                layout_flags: flags & 0xFFFF_835F,
            });
        }
    }

    if flags & 0x20 != 0 {
        let offset_x = if flags & 0x1000 != 0 {
            let period = clip_width + text_width;
            if period > FENNEL_SRD_SCROLL_PERIOD_EPSILON {
                fennel_cifmod(input.scroll.field_2c4 + clip_width, period) - clip_width
            } else {
                0.0
            }
        } else {
            let repeated_width = input.repeated_text_size.ok_or(
                FennelSrdDrawPreparationError::MissingRepeatedTextMeasurement {
                    textbox_flags: flags,
                },
            )?[0];
            // Preserve `subss repeated, text*2; addss text` rather than
            // algebraically shortening the single-precision chain.
            let period = (repeated_width - text_width * 2.0) + text_width;
            if period > FENNEL_SRD_SCROLL_PERIOD_EPSILON {
                fennel_cifmod(input.scroll.field_2c4, period)
            } else {
                0.0
            }
        };
        return Ok(FennelSrdDrawPreparation::DrawOffset([offset_x, 0.0]));
    }

    let repeated_height = input.repeated_text_size.ok_or(
        FennelSrdDrawPreparationError::MissingRepeatedTextMeasurement {
            textbox_flags: flags,
        },
    )?[1];
    let offset_y = if flags & 0x1000 != 0 {
        let overflow = text_height - clip_height;
        let tail_end = input.scroll.field_2cc + overflow;
        let period_core = ((repeated_height + tail_end) - text_height) - text_height + clip_height;
        let remainder = fennel_cifmod(input.scroll.field_2c4, input.scroll.field_2c8 + period_core);
        if overflow > remainder {
            remainder
        } else if tail_end > remainder {
            overflow
        } else if period_core > remainder {
            (remainder + overflow) - tail_end
        } else {
            0.0
        }
    } else {
        let remainder = fennel_cifmod(input.scroll.field_2c4, clip_height + repeated_height);
        if repeated_height - text_height > remainder {
            remainder
        } else {
            0.0
        }
    };
    Ok(FennelSrdDrawPreparation::DrawOffset([0.0, offset_y]))
}

fn fennel_cifmod(dividend: f32, divisor: f32) -> f32 {
    // `sub_10461A0` loads both f32 arguments onto the x87 stack and tail-jumps
    // to UCRT `_CIfmod`; f64 represents both source operands exactly before
    // the result is rounded back by the caller's `fstp dword ptr`.
    ((dividend as f64) % (divisor as f64)) as f32
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelPreparedRuntimeText {
    pub text: Vec<u8>,
    pub scroll_state: FennelSrdScrollState,
    pub found_d: bool,
    pub found_l: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelRuntimeTextControlError {
    AtoiOverflow { control: u8, value: Vec<u8> },
}

impl std::fmt::Display for FennelRuntimeTextControlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtoiOverflow { control, value } => write!(
                formatter,
                "Fennel ${} control atoi input overflows i32: {:?}",
                char::from(*control),
                String::from_utf8_lossy(value)
            ),
        }
    }
}

impl std::error::Error for FennelRuntimeTextControlError {}

/// Reproduces `sub_AD8D50`'s narrow-string preprocessing order:
///
/// 1. replace every `$[0]..$[7]` occurrence in ascending index order;
/// 2. remove one `$D` (or `$d` only when no uppercase occurrence exists),
///    optionally consuming an immediately following complete `[value]`;
/// 3. repeat the same operation for `$L`/`$l` on the D-stripped text.
///
/// The game initializes the D value from its host-global default before the
/// optional bracketed `atoi`. Integer overflow is rejected here because the
/// exact UCRT overflow result is outside the currently proven domain.
pub fn prepare_fennel_srd_runtime_text(
    source: &[u8],
    substitutions: [&[u8]; 8],
    default_d: i32,
    font_param: FontParamData,
) -> Result<FennelPreparedRuntimeText, FennelRuntimeTextControlError> {
    let mut text = source.to_vec();
    for (index, replacement) in substitutions.into_iter().enumerate() {
        let token = [b'$', b'[', b'0' + index as u8, b']'];
        fennel_replace_all_forward(&mut text, &token, replacement);
    }

    let (found_d, d_value) = fennel_remove_runtime_control(&mut text, b'D', default_d)?;
    let (found_l, _) = fennel_remove_runtime_control(&mut text, b'L', d_value)?;
    Ok(FennelPreparedRuntimeText {
        text,
        scroll_state: FennelSrdScrollState {
            field_f4: 0.0,
            field_f8: if found_d { d_value } else { -1 },
            field_fc: if found_l { 0 } else { font_param.scroll_speed },
            field_130: font_param.scroll_wait,
            field_134: font_param.field_34,
        },
        found_d,
        found_l,
    })
}

fn fennel_replace_all_forward(text: &mut Vec<u8>, token: &[u8], replacement: &[u8]) {
    let mut start = 0usize;
    while let Some(relative) = text[start..]
        .windows(token.len())
        .position(|candidate| candidate == token)
    {
        let position = start + relative;
        text.splice(
            position..position + token.len(),
            replacement.iter().copied(),
        );
        // `sub_5FDDB0` resumes after the inserted replacement, so a token
        // inside replacement text is not recursively expanded in this pass.
        start = position + replacement.len();
    }
}

fn fennel_remove_runtime_control(
    text: &mut Vec<u8>,
    upper: u8,
    default_value: i32,
) -> Result<(bool, i32), FennelRuntimeTextControlError> {
    let uppercase = [b'$', upper];
    let lowercase = [b'$', upper.to_ascii_lowercase()];
    let position = text
        .windows(uppercase.len())
        .position(|candidate| candidate == uppercase)
        .or_else(|| {
            text.windows(lowercase.len())
                .position(|candidate| candidate == lowercase)
        });
    let Some(position) = position else {
        return Ok((false, default_value));
    };

    let mut remove_end = position + 2;
    let mut value = default_value;
    if text.get(remove_end) == Some(&b'[') {
        if let Some(close_offset) = text[remove_end + 1..].iter().position(|byte| *byte == b']') {
            let close = remove_end + 1 + close_offset;
            let source = &text[remove_end + 1..close];
            value = fennel_checked_c_atoi(source).ok_or_else(|| {
                FennelRuntimeTextControlError::AtoiOverflow {
                    control: upper,
                    value: source.to_vec(),
                }
            })?;
            remove_end = close + 1;
        }
    }
    text.drain(position..remove_end);
    Ok((true, value))
}

fn fennel_checked_c_atoi(bytes: &[u8]) -> Option<i32> {
    let mut index = 0usize;
    while bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C))
    {
        index += 1;
    }
    let negative = match bytes.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };
    let mut found = false;
    let mut magnitude = 0u64;
    while let Some(byte @ b'0'..=b'9') = bytes.get(index) {
        found = true;
        magnitude = magnitude
            .checked_mul(10)?
            .checked_add(u64::from(byte - b'0'))?;
        index += 1;
    }
    if !found {
        return Some(0);
    }
    if negative {
        if magnitude == 0x8000_0000 {
            Some(i32::MIN)
        } else {
            i32::try_from(magnitude).ok().map(|value| -value)
        }
    } else {
        i32::try_from(magnitude).ok()
    }
}

impl FennelSrdScrollState {
    /// Reproduces the `sub_AD8D50` result when the runtime text contains
    /// neither `$D` nor `$L`: elapsed state is reset, `+0xF8` is `-1`, and
    /// `+0xFC` receives FontParam `scrollSpeed` from state `+0x12C`.
    pub const fn initial_without_controls(font_param: FontParamData) -> Self {
        Self {
            field_f4: 0.0,
            field_f8: -1,
            field_fc: font_param.scroll_speed,
            field_130: font_param.scroll_wait,
            field_134: font_param.field_34,
        }
    }

    /// Reproduces `sub_AD8D00`. A disabled SrTextCast leaves the clock
    /// unchanged; the enabled chain preserves the binary multiplication order.
    pub fn advance(&mut self, enabled: bool, host_frame_value: f32, argument: f32) {
        if enabled {
            self.field_f4 = (host_frame_value * argument) * FENNEL_SRD_CLOCK_SCALE + self.field_f4;
        }
    }

    /// Reproduces the four state-derived writes at
    /// `0xAC5CDC..0xAC5D5E` before `sub_7C04F0` computes its draw offset.
    pub fn outputs(self) -> FennelSrdScrollOutputs {
        let speed = self.field_fc as f32;
        let wait = self.field_130 as f32;
        FennelSrdScrollOutputs {
            maximum_glyphs: if self.field_f8 > 0 {
                fennel_cvttss2si((self.field_f8 as f32) * self.field_f4)
            } else {
                -1
            },
            field_2c4: fennel_sse_max((self.field_f4 - wait) * speed, 0.0),
            field_2c8: fennel_sse_max(wait * speed, 0.0),
            field_2cc: fennel_sse_max((self.field_134 as f32) * speed, 0.0),
        }
    }
}

/// The 45 UTF-16 values copied from `word_1940AB8` into the FontManager
/// implementation's `+0xE0` lookup by `sub_F33C30`.
pub const FENNEL_FONT_MANAGER_SET_E0: [u16; 45] = [
    0x0029, 0x002C, 0x002E, 0x2019, 0x201D, 0x2025, 0x2026, 0x226B, 0x3001, 0x3002, 0x3009, 0x300B,
    0x300D, 0x300F, 0x3011, 0x3015, 0x3041, 0x3043, 0x3045, 0x3047, 0x3049, 0x3063, 0x3083, 0x3085,
    0x3087, 0x309B, 0x309C, 0x30A1, 0x30A3, 0x30A5, 0x30A7, 0x30A9, 0x30C3, 0x30E3, 0x30E5, 0x30E7,
    0x30FC, 0xFF01, 0xFF09, 0xFF0C, 0xFF0E, 0xFF1F, 0xFF3D, 0xFF5D, 0xFF5E,
];

/// The 14 UTF-16 values copied from `word_1940A9C` into the FontManager
/// implementation's `+0xE4` lookup by `sub_F33C30`.
pub const FENNEL_FONT_MANAGER_SET_E4: [u16; 14] = [
    0x0025, 0x0028, 0x2018, 0x201C, 0x226A, 0x3008, 0x300A, 0x300C, 0x300E, 0x3010, 0x3014, 0xFF08,
    0xFF3B, 0xFF5B,
];

/// Exact membership queried by `sub_F38DB0` from FontManager `+0xE0`.
pub fn fennel_font_manager_set_e0_contains(code: u16) -> bool {
    FENNEL_FONT_MANAGER_SET_E0.binary_search(&code).is_ok()
}

/// Exact membership queried by `sub_F38D20` from FontManager `+0xE4`.
pub fn fennel_font_manager_set_e4_contains(code: u16) -> bool {
    FENNEL_FONT_MANAGER_SET_E4.binary_search(&code).is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelTextDecodeError {
    pub valid_prefix_len: usize,
}

impl std::fmt::Display for FennelTextDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Fennel UTF-8 text becomes invalid at byte {}",
            self.valid_prefix_len
        )
    }
}

impl std::error::Error for FennelTextDecodeError {}

/// Decodes the valid-input domain of the game's encoding-mode-zero path.
///
/// The source is a C string, so bytes after the first NUL are ignored. On
/// valid UTF-8 this produces the same UTF-16 code units, including surrogate
/// pairs for supplementary characters, as `sub_F525D0`. Invalid byte handling
/// remains explicit instead of silently choosing a replacement policy.
pub fn decode_fennel_game_text(bytes: &[u8]) -> Result<Vec<u16>, FennelTextDecodeError> {
    let c_string = &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())];
    let text = std::str::from_utf8(c_string).map_err(|error| FennelTextDecodeError {
        valid_prefix_len: error.valid_up_to(),
    })?;
    Ok(text.encode_utf16().collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelPlainToken {
    Glyph(u16),
    NewLine,
    SetPosition { x: i32, y: i32 },
    ToggleEffect,
    SetColors([u32; 4]),
    ResetColors,
    SetFontSlot(u16),
    ResetFontSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedFennelControl {
    pub unit_index: usize,
    pub command: Option<u16>,
}

impl std::fmt::Display for UnsupportedFennelControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.command {
            Some(command) => write!(
                formatter,
                "unsupported Fennel control ${command:04X} at UTF-16 unit {}",
                self.unit_index
            ),
            None => write!(
                formatter,
                "unterminated Fennel control prefix at UTF-16 unit {}",
                self.unit_index
            ),
        }
    }
}

impl std::error::Error for UnsupportedFennelControl {}

/// Reproduces the supported plain-token subset of `sub_F3BD40`.
///
/// Direct UTF-16 units produce type-zero glyph tokens. CRLF, CR, LF and the
/// proven `$N`/`$n` aliases produce type-three newline tokens. `$$` produces a
/// literal dollar glyph. The `$t[x:y]` branch writes signed decimal x/y values
/// to iterator/output offsets +0x20/+0x24. `$s`/`$S` toggles iterator state
/// `+0x820` bit `0x40000`, which the normal-glyph output copy places at token
/// `+0x10`. `$C` restores the iterator's saved colors, while bracketed `$C`
/// parses the binary's colon-separated hexadecimal color state. `$F[n]`
/// selects a decimal font slot and `$F` restores the saved initial slot.
/// Other commands are rejected until their parameter grammar and state writes
/// are closed from the binary.
pub fn tokenize_fennel_plain_text(
    units: &[u16],
) -> Result<Vec<FennelPlainToken>, UnsupportedFennelControl> {
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < units.len() {
        let unit = units[index];
        if unit == 0 || unit == 0x1A {
            break;
        }
        match unit {
            0x0D => {
                if units.get(index + 1) == Some(&0x0A) {
                    index += 1;
                }
                tokens.push(FennelPlainToken::NewLine);
            }
            0x0A => tokens.push(FennelPlainToken::NewLine),
            FENNEL_CONTROL_PREFIX => {
                let Some(&command) = units.get(index + 1) else {
                    break;
                };
                match command {
                    FENNEL_CONTROL_PREFIX => {
                        tokens.push(FennelPlainToken::Glyph(FENNEL_CONTROL_PREFIX));
                        index += 1;
                    }
                    FENNEL_NEWLINE_COMMAND_UPPER | FENNEL_NEWLINE_COMMAND_LOWER => {
                        tokens.push(FennelPlainToken::NewLine);
                        index += 1;
                    }
                    FENNEL_POSITION_COMMAND_UPPER | FENNEL_POSITION_COMMAND_LOWER => {
                        let Some((x, y, end_index)) = parse_fennel_position_control(units, index)
                        else {
                            return Err(UnsupportedFennelControl {
                                unit_index: index,
                                command: Some(command),
                            });
                        };
                        tokens.push(FennelPlainToken::SetPosition { x, y });
                        index = end_index;
                    }
                    FENNEL_EFFECT_COMMAND_UPPER | FENNEL_EFFECT_COMMAND_LOWER => {
                        tokens.push(FennelPlainToken::ToggleEffect);
                        index += 1;
                    }
                    FENNEL_COLOR_COMMAND_UPPER | FENNEL_COLOR_COMMAND_LOWER => {
                        if units.get(index + 2) == Some(&(b'[' as u16)) {
                            let Some((colors, end_index)) =
                                parse_fennel_color_control(units, index)
                            else {
                                return Err(UnsupportedFennelControl {
                                    unit_index: index,
                                    command: Some(command),
                                });
                            };
                            tokens.push(FennelPlainToken::SetColors(colors));
                            index = end_index;
                        } else {
                            tokens.push(FennelPlainToken::ResetColors);
                            index += 1;
                        }
                    }
                    FENNEL_FONT_COMMAND_UPPER | FENNEL_FONT_COMMAND_LOWER => {
                        if units.get(index + 2) == Some(&(b'[' as u16)) {
                            let Some((font_slot, end_index)) =
                                parse_fennel_font_slot_control(units, index)
                            else {
                                return Err(UnsupportedFennelControl {
                                    unit_index: index,
                                    command: Some(command),
                                });
                            };
                            tokens.push(FennelPlainToken::SetFontSlot(font_slot));
                            index = end_index;
                        } else {
                            tokens.push(FennelPlainToken::ResetFontSlot);
                            index += 1;
                        }
                    }
                    _ => {
                        return Err(UnsupportedFennelControl {
                            unit_index: index,
                            command: Some(command),
                        });
                    }
                }
            }
            _ => tokens.push(FennelPlainToken::Glyph(unit)),
        }
        index += 1;
    }
    Ok(tokens)
}

fn parse_fennel_position_control(units: &[u16], prefix_index: usize) -> Option<(i32, i32, usize)> {
    let open_index = prefix_index.checked_add(2)?;
    if units.get(open_index) != Some(&(b'[' as u16)) {
        return None;
    }
    let colon_index = units[open_index + 1..]
        .iter()
        .position(|unit| *unit == b':' as u16)?
        + open_index
        + 1;
    let close_index = units[colon_index + 1..]
        .iter()
        .position(|unit| *unit == b']' as u16)?
        + colon_index
        + 1;
    let x = parse_fennel_signed_decimal(&units[open_index + 1..colon_index])?;
    let y = parse_fennel_signed_decimal(&units[colon_index + 1..close_index])?;
    Some((x, y, close_index))
}

fn parse_fennel_signed_decimal(units: &[u16]) -> Option<i32> {
    let (negative, digits) = match units {
        [minus, rest @ ..] if *minus == b'-' as u16 => (true, rest),
        _ => (false, units),
    };
    if digits.is_empty() {
        return None;
    }
    let mut value = 0i32;
    for &unit in digits {
        if !(b'0' as u16..=b'9' as u16).contains(&unit) {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(i32::from(unit - b'0' as u16))?;
    }
    negative.then_some(value.checked_neg()?).or(Some(value))
}

fn parse_fennel_color_control(units: &[u16], prefix_index: usize) -> Option<([u32; 4], usize)> {
    let mut cursor = prefix_index.checked_add(3)?;
    let (first, first_delimiter, first_end) = parse_fennel_hex_until_delimiter(units, cursor)?;
    let first = first.swap_bytes();
    cursor = first_end + 1;
    if first_delimiter == b']' as u16 {
        return Some(([first; 4], first_end));
    }

    let (second, second_delimiter, second_end) = parse_fennel_hex_until_delimiter(units, cursor)?;
    cursor = second_end + 1;
    if second_delimiter == b']' as u16 {
        // `0xF3C68C -> 0xF3C82C` does not commit the second parsed value.
        return Some(([first; 4], second_end));
    }
    let second = second.swap_bytes();

    let (third, third_delimiter, third_end) = parse_fennel_hex_until_delimiter(units, cursor)?;
    cursor = third_end + 1;
    if third_delimiter == b']' as u16 {
        // `0xF3C6F9 -> 0xF3C7EA` preserves color 1 but fills 2 and 3 from 0.
        return Some(([first, second, first, first], third_end));
    }
    let third = third.swap_bytes();

    let close_offset = units[cursor..]
        .iter()
        .position(|unit| *unit == b']' as u16)?;
    let close_index = cursor + close_offset;
    let fourth = parse_fennel_hex(&units[cursor..close_index])?.swap_bytes();
    Some(([first, second, third, fourth], close_index))
}

fn parse_fennel_hex_until_delimiter(units: &[u16], start: usize) -> Option<(u32, u16, usize)> {
    let delimiter_offset = units[start..]
        .iter()
        .position(|unit| *unit == b':' as u16 || *unit == b']' as u16)?;
    let delimiter_index = start + delimiter_offset;
    let value = parse_fennel_hex(&units[start..delimiter_index])?;
    Some((value, units[delimiter_index], delimiter_index))
}

fn parse_fennel_hex(units: &[u16]) -> Option<u32> {
    if units.is_empty() || units.len() > 8 {
        return None;
    }
    let mut value = 0u32;
    for &unit in units {
        let digit = match unit {
            unit @ 0x30..=0x39 => u32::from(unit - 0x30),
            unit @ 0x41..=0x46 => u32::from(unit - 0x41 + 10),
            unit @ 0x61..=0x66 => u32::from(unit - 0x61 + 10),
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(digit)?;
    }
    Some(value)
}

fn parse_fennel_font_slot_control(units: &[u16], prefix_index: usize) -> Option<(u16, usize)> {
    let value_start = prefix_index.checked_add(3)?;
    let close_offset = units[value_start..]
        .iter()
        .position(|unit| *unit == b']' as u16)?;
    let close_index = value_start + close_offset;
    let value = parse_fennel_unsigned_decimal(&units[value_start..close_index])?;
    // `0xF3BE90..0xF3BE9C` parses to u32, then stores AX at iterator +0x814.
    Some((value as u16, close_index))
}

fn parse_fennel_unsigned_decimal(units: &[u16]) -> Option<u32> {
    if units.is_empty() {
        return None;
    }
    let mut value = 0u32;
    for &unit in units {
        if !(b'0' as u16..=b'9' as u16).contains(&unit) {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(u32::from(unit - b'0' as u16))?;
    }
    Some(value)
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelPlainRecordStream {
    pub records: Vec<FennelGlyphLayoutRecord>,
    pub line_start_indices: Vec<usize>,
    pub glyph_count: usize,
}

/// Runtime-glyph fields read by the three Fennel layout routines. Flag `0x200`
/// substitutes a point-size-derived fixed cell for horizontal visual width and
/// advance, then applies a final centering correction against `advance_x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelLayoutGlyphMetrics {
    pub code: u16,
    pub point_x: u16,
    pub bearing_x: i32,
    pub width: u32,
    pub advance_x: u32,
    pub em_pixels_y: i32,
}

impl From<&RuhunaRuntimeGlyphRecord> for FennelLayoutGlyphMetrics {
    fn from(glyph: &RuhunaRuntimeGlyphRecord) -> Self {
        Self {
            code: glyph.code,
            point_x: glyph.point_x,
            bearing_x: glyph.bearing_x,
            width: glyph.width,
            advance_x: glyph.advance_x,
            em_pixels_y: glyph.em_pixels_y,
        }
    }
}

/// Inputs copied by `sub_AC6F50` into the TextBoxObject before the static SRD
/// TextCast path calls `setTextByWideString`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelStaticLayoutInput {
    pub text_flags: u32,
    pub box_width: f32,
    pub box_height: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub line_spacing: i32,
}

/// Static SRD values whose source is closed from `sub_AC6F50`, the
/// TextBoxObject constructors, and the normal-token branch of `sub_F3BD40`.
///
/// The iterator starts x/y at zero. Its output `+0x6C`, copied to layout-record
/// `+0x20`, comes from TEXT property `0x7C`. The remaining values are copied to
/// the TextBoxObject by the SrTextCast initializer. Missing properties are
/// rejected because their serialized default behavior has not been proven.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelStaticTextProperties {
    pub initial_x: i32,
    pub initial_y: i32,
    pub glyph_spacing: i32,
    pub layout: FennelStaticLayoutInput,
}

impl FennelStaticTextProperties {
    pub fn from_text_definition(
        text: &TextDefinition,
        box_width: f32,
        box_height: f32,
    ) -> Result<Self, FennelStaticTextPropertyError> {
        let text_flags = text
            .field_78
            .ok_or(FennelStaticTextPropertyError::MissingProperty { code: 0x78 })?;
        let [scale_x, scale_y] = text
            .field_36
            .ok_or(FennelStaticTextPropertyError::MissingProperty { code: 0x36 })?;
        let glyph_spacing = i32::from(
            text.field_7c
                .ok_or(FennelStaticTextPropertyError::MissingProperty { code: 0x7C })?,
        );
        let line_spacing = i32::from(
            text.field_41
                .ok_or(FennelStaticTextPropertyError::MissingProperty { code: 0x41 })?,
        );
        Ok(Self {
            initial_x: 0,
            initial_y: 0,
            glyph_spacing,
            layout: FennelStaticLayoutInput {
                text_flags,
                box_width,
                box_height,
                scale_x,
                scale_y,
                line_spacing,
            },
        })
    }

    pub const fn glyph_placement(
        self,
        glyph_token: u32,
        field_0c: u32,
        colors: [u32; 4],
    ) -> FennelGlyphPlacementInput {
        FennelGlyphPlacementInput {
            glyph_token,
            field_0c,
            x: self.initial_x,
            y: self.initial_y,
            field_20: self.glyph_spacing,
            colors,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelStaticTextPropertyError {
    MissingProperty { code: u8 },
}

impl std::fmt::Display for FennelStaticTextPropertyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProperty { code } => {
                write!(
                    formatter,
                    "static Fennel TEXT property {code:#04x} is absent"
                )
            }
        }
    }
}

impl std::error::Error for FennelStaticTextPropertyError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelFittingLayoutResult {
    pub effective_scale_x: f32,
    pub effective_scale_y: f32,
    pub total_height: f32,
    /// TextBoxObject `+0x108`. The layout records do not contain this
    /// center/bottom-alignment displacement; `sub_7C7F90` adds it to the
    /// TextBox translation immediately before drawing.
    pub textbox_vertical_offset: f32,
    pub line_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelAutomaticWrap {
    pub explicit_line_index: usize,
    pub record_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelVerticalOverflow {
    pub explicit_line_index: usize,
    pub positioned_line_count: usize,
    pub record_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelLinePosition {
    /// TextBoxObject `+0x12C` entry `+0x00`.
    pub x_offset: f32,
    /// TextBoxObject `+0x12C` entry `+0x04`. Unlike glyph-record Y, this
    /// already includes TextBoxObject `+0x108` vertical alignment.
    pub y_bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelLineDescription {
    /// Host-independent replacement for the x86 record pointer at entry
    /// `+0x00` of the TextBoxObject `+0x34C` vector.
    pub first_record_index: usize,
    /// Entry `+0x04` in the binary's 16-byte record.
    pub record_count: usize,
    /// Entry `+0x08`: sum of each positioned record's horizontal advance.
    pub advance_width: f32,
    /// Entry `+0x0C`: maximum effective glyph height, or the proven fallback
    /// height for an otherwise zero-height line.
    pub line_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelDefaultLayoutResult {
    pub effective_scale_x: f32,
    pub effective_scale_y: f32,
    pub total_height: f32,
    /// TextBoxObject `+0x108`, kept separate from record `y` exactly as in
    /// `sub_7C1F90` and consumed by `sub_7C7F90` during batch drawing.
    pub textbox_vertical_offset: f32,
    pub positioned_line_count: usize,
    pub automatic_wrap_count: usize,
    pub first_automatic_wrap: Option<FennelAutomaticWrap>,
    pub vertical_overflow: Option<FennelVerticalOverflow>,
    /// Non-empty visual lines appended to TextBoxObject `+0x12C` during the
    /// final positioning pass of `sub_7C1F90`.
    pub line_positions: Vec<FennelLinePosition>,
    /// Entries appended in lockstep to TextBoxObject `+0x34C`.
    pub line_descriptions: Vec<FennelLineDescription>,
    /// Exact value returned by the tail of `sub_7C90A0`: glyph count when no
    /// `-254` record exists, otherwise the last marker's record index, with a
    /// marker at index zero represented as `-1`.
    pub record_limit: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelFlag20LayoutResult {
    pub layout: FennelDefaultLayoutResult,
    /// TextBoxObject `+0x358`. `sub_7C4070` initializes it to zero and, after
    /// each non-empty line description append, replaces it with the new line
    /// index only when the currently selected `advance_width` is strictly
    /// greater. Equal widths preserve the earlier index.
    pub field_358: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelSrdLayoutResult {
    pub layout: FennelDefaultLayoutResult,
    pub textbox_flags: u32,
    /// Present only when the final flags dispatch to `sub_7C4070`.
    pub field_358: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelDefaultLayoutError {
    InvalidLineTable,
    UnsupportedRecordKind {
        record_index: usize,
        kind: i32,
    },
    MissingGlyphMetrics {
        record_index: usize,
        glyph_token: u32,
    },
    NonProgressingZeroHeightWrap {
        record_index: usize,
    },
    NonTerminatingWrapWithoutVerticalCutoff {
        record_index: usize,
    },
}

impl std::fmt::Display for FennelDefaultLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLineTable => formatter.write_str("invalid Fennel line-start table"),
            Self::UnsupportedRecordKind { record_index, kind } => write!(
                formatter,
                "unsupported Fennel record kind {kind} at record {record_index}"
            ),
            Self::MissingGlyphMetrics {
                record_index,
                glyph_token,
            } => write!(
                formatter,
                "no Fennel glyph metrics for token {glyph_token:#010x} at record {record_index}"
            ),
            Self::NonProgressingZeroHeightWrap { record_index } => write!(
                formatter,
                "Fennel wrap at record {record_index} neither advances the record pointer nor the vertical position"
            ),
            Self::NonTerminatingWrapWithoutVerticalCutoff { record_index } => write!(
                formatter,
                "Fennel wrap at record {record_index} does not advance the record pointer while flag 0x4000 disables the game's vertical termination"
            ),
        }
    }
}

impl std::error::Error for FennelDefaultLayoutError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelFlag20LayoutError {
    UnsupportedTextBoxFlags { textbox_flags: u32 },
    TextFlagBypassesModeSwitch { text_flags: u32 },
    Layout(FennelDefaultLayoutError),
}

impl std::fmt::Display for FennelFlag20LayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedTextBoxFlags { textbox_flags } => write!(
                formatter,
                "unsupported Fennel Flag20 TextBox flags {textbox_flags:#010x}"
            ),
            Self::TextFlagBypassesModeSwitch { text_flags } => write!(
                formatter,
                "Fennel TEXT flags {text_flags:#010x} bypass the Flag20 mode switch"
            ),
            Self::Layout(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FennelFlag20LayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedTextBoxFlags { .. } | Self::TextFlagBypassesModeSwitch { .. } => None,
            Self::Layout(error) => Some(error),
        }
    }
}

impl From<FennelDefaultLayoutError> for FennelFlag20LayoutError {
    fn from(error: FennelDefaultLayoutError) -> Self {
        Self::Layout(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelMode56LayoutError {
    UnsupportedMode { mode: u32 },
    TextFlagBypassesModeSwitch { text_flags: u32 },
    Layout(FennelDefaultLayoutError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelMode1LayoutError {
    TextFlagBypassesModeSwitch { text_flags: u32 },
    Layout(FennelDefaultLayoutError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelSrdLayoutError {
    UnsupportedMode { mode: u32 },
    Layout(FennelDefaultLayoutError),
}

impl std::fmt::Display for FennelSrdLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedMode { mode } => {
                write!(formatter, "unsupported Fennel FontParamData mode {mode}")
            }
            Self::Layout(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FennelSrdLayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedMode { .. } => None,
            Self::Layout(error) => Some(error),
        }
    }
}

impl From<FennelDefaultLayoutError> for FennelSrdLayoutError {
    fn from(error: FennelDefaultLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl std::fmt::Display for FennelMode1LayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextFlagBypassesModeSwitch { text_flags } => write!(
                formatter,
                "Fennel TEXT flags {text_flags:#010x} bypass the mode-1 switch"
            ),
            Self::Layout(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FennelMode1LayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TextFlagBypassesModeSwitch { .. } => None,
            Self::Layout(error) => Some(error),
        }
    }
}

impl From<FennelDefaultLayoutError> for FennelMode1LayoutError {
    fn from(error: FennelDefaultLayoutError) -> Self {
        Self::Layout(error)
    }
}

impl std::fmt::Display for FennelMode56LayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedMode { mode } => {
                write!(formatter, "unsupported Fennel mode-{mode} default layout")
            }
            Self::TextFlagBypassesModeSwitch { text_flags } => write!(
                formatter,
                "Fennel TEXT flags {text_flags:#010x} bypass the mode-5/6 switch"
            ),
            Self::Layout(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FennelMode56LayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedMode { .. } | Self::TextFlagBypassesModeSwitch { .. } => None,
            Self::Layout(error) => Some(error),
        }
    }
}

impl From<FennelDefaultLayoutError> for FennelMode56LayoutError {
    fn from(error: FennelDefaultLayoutError) -> Self {
        Self::Layout(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelTextureBatchMembership {
    pub texture_token: u32,
    pub normal_glyph_count: usize,
    pub effect_glyph_count: usize,
    pub record_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelTextureBatchStop {
    EndOfRecordArray,
    LineTableOverflow { record_index: usize },
    MaximumGlyphs { record_index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelTextureBatchBuild {
    /// Global forward-list order used by `sub_7C0D40` and traversed by
    /// `sub_7C7F90`, not sorted texture-token order.
    pub batches: Vec<FennelTextureBatchMembership>,
    pub processed_glyph_count: usize,
    pub stop: FennelTextureBatchStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelTextureBatchRehashRequired {
    pub unique_texture_count: usize,
}

impl std::fmt::Display for FennelTextureBatchRehashRequired {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Fennel texture batch count {} reaches the not-yet-ported hash rehash path",
            self.unique_texture_count
        )
    }
}

impl std::error::Error for FennelTextureBatchRehashRequired {}

/// Reproduces `sub_7C0D40` for the binary-proven no-rehash domain used by the
/// game's RFZ fonts (the complete local fonts have at most seven atlas pages,
/// below the initial 17-bucket threshold).
///
/// Negative records are skipped except `-254`, which immediately stops the
/// scan. Nonnegative records are grouped by texture token. New hash buckets
/// are inserted at the global forward-list head; a collision is inserted at
/// the front of its existing contiguous bucket group. The optional maximum is
/// TextBoxObject `+0x2C0`; static SrTextCast initialization supplies `-1`.
pub fn build_fennel_texture_batch_membership(
    stream: &FennelPlainRecordStream,
    maximum_glyphs: i32,
) -> Result<FennelTextureBatchBuild, FennelTextureBatchRehashRequired> {
    let mut batches = Vec::<FennelTextureBatchMembership>::new();
    let mut processed_glyph_count = 0usize;
    for (record_index, record) in stream.records.iter().enumerate() {
        if record.kind < 0 {
            if record.kind == FENNEL_RECORD_LINE_TABLE_OVERFLOW {
                return Ok(FennelTextureBatchBuild {
                    batches,
                    processed_glyph_count,
                    stop: FennelTextureBatchStop::LineTableOverflow { record_index },
                });
            }
            continue;
        }

        let next_glyph_count = processed_glyph_count.wrapping_add(1);
        if maximum_glyphs >= 0 && next_glyph_count >= maximum_glyphs as usize {
            return Ok(FennelTextureBatchBuild {
                batches,
                processed_glyph_count,
                stop: FennelTextureBatchStop::MaximumGlyphs { record_index },
            });
        }

        if let Some(batch) = batches
            .iter_mut()
            .find(|batch| batch.texture_token == record.texture_token)
        {
            batch.normal_glyph_count += 1;
            batch.effect_glyph_count +=
                usize::from(record.field_0c & FENNEL_EFFECT_GLYPH_FLAG != 0);
            batch.record_indices.push(record_index);
        } else {
            let unique_texture_count = batches.len() + 1;
            if unique_texture_count > FENNEL_INITIAL_TEXTURE_BATCH_BUCKET_COUNT as usize {
                return Err(FennelTextureBatchRehashRequired {
                    unique_texture_count,
                });
            }
            let bucket = fennel_texture_batch_bucket(record.texture_token);
            let insertion_index = batches
                .iter()
                .position(|batch| fennel_texture_batch_bucket(batch.texture_token) == bucket)
                .unwrap_or(0);
            batches.insert(
                insertion_index,
                FennelTextureBatchMembership {
                    texture_token: record.texture_token,
                    normal_glyph_count: 1,
                    effect_glyph_count: usize::from(
                        record.field_0c & FENNEL_EFFECT_GLYPH_FLAG != 0,
                    ),
                    record_indices: vec![record_index],
                },
            );
        }
        processed_glyph_count = next_glyph_count;
    }
    Ok(FennelTextureBatchBuild {
        batches,
        processed_glyph_count,
        stop: FennelTextureBatchStop::EndOfRecordArray,
    })
}

fn fennel_texture_batch_bucket(texture_token: u32) -> u32 {
    texture_token.wrapping_add(texture_token >> 3) % FENNEL_INITIAL_TEXTURE_BATCH_BUCKET_COUNT
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelStaticUnclippedDrawInput {
    pub is_2d: bool,
    /// TextBoxObject `+0xB8/+0xBC/+0xC0`. `sub_AC6F50` supplies
    /// `(-SrImage.origin_x, -SrImage.origin_y, 0)` for the static SrTextCast
    /// path.
    pub textbox_position: [f32; 3],
    /// TextBoxObject `+0xC4/+0xC8`, sourced from TEXT property `0x36`.
    pub textbox_scale: [f32; 2],
    /// TextBoxObject `+0x108`, returned separately by the layout routine.
    pub textbox_vertical_offset: f32,
    /// TextBoxObject `+0x2EC`, written by `sub_AC5740`.
    pub textbox_transform: Matrix4x4,
    /// TextBoxObject `+0x32C`, after the SrTextCast color chain.
    pub secondary_color: u32,
}

/// Normal-glyph inputs shared by the supported clipped and unclipped branches
/// of `sub_7C7F90 -> sub_7C10B0`.
///
/// The runtime state source remains explicit: this type does not infer a mode
/// from SRD properties or animation data. `textbox_flags` selects the branch
/// exactly as the game does; `clip_size` is TextBoxObject `+0x2E4/+0x2E8`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelNormalDrawInput {
    pub is_2d: bool,
    pub textbox_position: [f32; 3],
    pub textbox_scale: [f32; 2],
    pub textbox_vertical_offset: f32,
    pub textbox_transform: Matrix4x4,
    pub secondary_color: u32,
    pub textbox_flags: u32,
    pub clip_size: [f32; 2],
    /// The two floats passed by `sub_7C04F0` as `sub_7C7F90` argument 7.
    /// `sub_7C7F90` subtracts them from the normal and effect glyph origins.
    pub draw_offset: [f32; 2],
    /// TextBoxObject `+0x80..+0x8C`. `sub_F2D730` replaces the effect record's
    /// RGB with these values and multiplies the two alpha bytes.
    pub effect_colors: [u32; 4],
    /// TextBoxObject `+0x90/+0x94`, added only to the effect glyph origin.
    pub effect_offset: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelOwnedTextureBatch {
    pub texture_token: u32,
    pub vertices: Vec<FennelRenderVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelStaticUnclippedBatchBuild {
    pub batches: Vec<FennelOwnedTextureBatch>,
    pub processed_glyph_count: usize,
    pub stop: FennelTextureBatchStop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelStaticUnclippedBatchError {
    TextureBatchRehashRequired(FennelTextureBatchRehashRequired),
    MissingRuntimeGlyph {
        record_index: usize,
        glyph_token: u32,
    },
    TextureTokenMismatch {
        record_index: usize,
        record_texture_token: u32,
        glyph_texture_token: u32,
    },
}

impl std::fmt::Display for FennelStaticUnclippedBatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextureBatchRehashRequired(error) => error.fmt(formatter),
            Self::MissingRuntimeGlyph {
                record_index,
                glyph_token,
            } => write!(
                formatter,
                "Fennel record {record_index} has no runtime glyph for token {glyph_token:#010x}"
            ),
            Self::TextureTokenMismatch {
                record_index,
                record_texture_token,
                glyph_texture_token,
            } => write!(
                formatter,
                "Fennel record {record_index} texture token {record_texture_token:#010x} does not match runtime glyph token {glyph_texture_token:#010x}"
            ),
        }
    }
}

impl std::error::Error for FennelStaticUnclippedBatchError {}

impl From<FennelTextureBatchRehashRequired> for FennelStaticUnclippedBatchError {
    fn from(error: FennelTextureBatchRehashRequired) -> Self {
        Self::TextureBatchRehashRequired(error)
    }
}

/// Reproduces the supported normal-glyph portion of `sub_7C7F90` for
/// the static mode-zero SrTextCast path, then calls the already-ported
/// unclipped `sub_7C10B0` vertex builder in exact texture-batch order.
///
/// Static construction proves correction mode byte `+0x98 == 0`, transform
/// adjustment byte `+0x10C == 0`, draw offset `(0, 0)`, and unclipped flags
/// `3` or `7`. A supplied `0x40000` record emits the exact default
/// effect-first segment; static plain SRD text does not set that bit.
pub fn build_fennel_static_unclipped_vertex_batches<F>(
    stream: &FennelPlainRecordStream,
    maximum_glyphs: i32,
    input: FennelStaticUnclippedDrawInput,
    runtime_glyph: F,
) -> Result<FennelStaticUnclippedBatchBuild, FennelStaticUnclippedBatchError>
where
    F: FnMut(u32) -> Option<RuhunaRuntimeGlyphRecord>,
{
    build_fennel_normal_vertex_batches(
        stream,
        maximum_glyphs,
        FennelNormalDrawInput {
            is_2d: input.is_2d,
            textbox_position: input.textbox_position,
            textbox_scale: input.textbox_scale,
            textbox_vertical_offset: input.textbox_vertical_offset,
            textbox_transform: input.textbox_transform,
            secondary_color: input.secondary_color,
            // Fresh TextBoxObject low bits. Both possible initial values, 3
            // and 7, select the same non-clipping branch.
            textbox_flags: 3,
            clip_size: [0.0; 2],
            draw_offset: [0.0; 2],
            effect_colors: FENNEL_DEFAULT_EFFECT_COLORS,
            effect_offset: FENNEL_DEFAULT_EFFECT_OFFSET,
        },
        runtime_glyph,
    )
}

/// Reproduces the `sub_7BFAB0` geometry maximum used by `sub_7C04F0` for the
/// SRD-proven correction mode `TextBox+0x98 == 0` and adjustment byte
/// `TextBox+0x10C == 0`. The function walks the already-built texture batch
/// membership, so negative records and the maximum-glyph stop match
/// `sub_7C0D40` before the geometry scan.
pub fn measure_fennel_srd_text_size_mode0<F>(
    stream: &FennelPlainRecordStream,
    maximum_glyphs: i32,
    textbox_scale: [f32; 2],
    effect_offset: [f32; 2],
    mut runtime_glyph: F,
) -> Result<[f32; 2], FennelStaticUnclippedBatchError>
where
    F: FnMut(u32) -> Option<RuhunaRuntimeGlyphRecord>,
{
    let membership = build_fennel_texture_batch_membership(stream, maximum_glyphs)?;
    let mut size = [0.0f32; 2];
    for batch in &membership.batches {
        for &record_index in &batch.record_indices {
            let record = &stream.records[record_index];
            let glyph = runtime_glyph(record.glyph_token).ok_or(
                FennelStaticUnclippedBatchError::MissingRuntimeGlyph {
                    record_index,
                    glyph_token: record.glyph_token,
                },
            )?;
            if glyph.texture_token != record.texture_token {
                return Err(FennelStaticUnclippedBatchError::TextureTokenMismatch {
                    record_index,
                    record_texture_token: record.texture_token,
                    glyph_texture_token: glyph.texture_token,
                });
            }

            let effective_scale = [
                record.scale_x * textbox_scale[0],
                record.scale_y * textbox_scale[1],
            ];
            let enabled = glyph.enabled as i32;
            let correction_y = glyph
                .bearing_y
                .wrapping_sub(glyph.flag_mode as i32)
                .wrapping_sub(glyph.line_height as i32)
                .wrapping_add(3);
            let mut origin = [
                record.x + (glyph.bearing_x.wrapping_sub(enabled) as f32) * effective_scale[0],
                record.y + (correction_y.wrapping_sub(enabled) as f32) * effective_scale[1],
            ];
            if record.field_0c & FENNEL_EFFECT_GLYPH_FLAG != 0 {
                // `sub_7BFAB0` replaces the candidate origin with the effect
                // copy for such a record; it does not separately retain the
                // normal-copy maximum when an effect flag is present.
                origin[0] += effect_offset[0];
                origin[1] += effect_offset[1];
            }
            let candidates = [
                record.width * effective_scale[0] + origin[0],
                record.height * effective_scale[1] + origin[1],
            ];
            for axis in 0..2 {
                // `comiss candidate, maximum; jbe` leaves unordered values
                // unchanged as well as ordered candidates <= the maximum.
                if candidates[axis] > size[axis] {
                    size[axis] = candidates[axis];
                }
            }
        }
    }
    Ok(size)
}

/// Reproduces the normal-glyph portion of `sub_7C7F90` and selects the exact
/// clipped or unclipped `sub_7C10B0` branch from caller-supplied runtime state.
/// Records with `record+0x0C & 0x40000` emit the game's effect segment before
/// the normal segment. The upstream control-token source of that bit remains
/// outside this function and is not inferred.
pub fn build_fennel_normal_vertex_batches<F>(
    stream: &FennelPlainRecordStream,
    maximum_glyphs: i32,
    input: FennelNormalDrawInput,
    mut runtime_glyph: F,
) -> Result<FennelStaticUnclippedBatchBuild, FennelStaticUnclippedBatchError>
where
    F: FnMut(u32) -> Option<RuhunaRuntimeGlyphRecord>,
{
    let membership = build_fennel_texture_batch_membership(stream, maximum_glyphs)?;
    let mut translation = identity_matrix4x4_game();
    translation.rows[0][3] = input.textbox_position[0];
    translation.rows[1][3] = input.textbox_position[1] + input.textbox_vertical_offset;
    translation.rows[2][3] = input.textbox_position[2];
    let cpu_transform = if input.is_2d {
        mul_matrix4x4_game(&input.textbox_transform, &translation)
    } else {
        // The 3D branch submits TextBoxObject +0x2EC to the renderer and gives
        // `sub_7C10B0` only the local translation matrix.
        translation
    };

    let mut batches = Vec::with_capacity(membership.batches.len());
    for batch in &membership.batches {
        let mut effect_vertices = Vec::with_capacity(batch.effect_glyph_count * 6);
        let mut normal_vertices = Vec::with_capacity(batch.normal_glyph_count * 6);
        for &record_index in &batch.record_indices {
            let record = &stream.records[record_index];
            let glyph = runtime_glyph(record.glyph_token).ok_or(
                FennelStaticUnclippedBatchError::MissingRuntimeGlyph {
                    record_index,
                    glyph_token: record.glyph_token,
                },
            )?;
            if glyph.texture_token != record.texture_token {
                return Err(FennelStaticUnclippedBatchError::TextureTokenMismatch {
                    record_index,
                    record_texture_token: record.texture_token,
                    glyph_texture_token: glyph.texture_token,
                });
            }

            let effective_scale = [
                record.scale_x * input.textbox_scale[0],
                record.scale_y * input.textbox_scale[1],
            ];
            let enabled = glyph.enabled as i32;
            let correction_y = glyph
                .bearing_y
                .wrapping_sub(glyph.flag_mode as i32)
                .wrapping_sub(glyph.line_height as i32)
                .wrapping_add(3);
            let origin = [
                record.x - input.draw_offset[0]
                    + (glyph.bearing_x.wrapping_sub(enabled) as f32) * effective_scale[0],
                record.y - input.draw_offset[1]
                    + (correction_y.wrapping_sub(enabled) as f32) * effective_scale[1],
            ];
            let glyph_vertices = if input.textbox_flags & FENNEL_TEXTBOX_CLIP_FLAG != 0 {
                build_fennel_clipped_vertices(
                    record,
                    origin,
                    effective_scale,
                    &cpu_transform,
                    input.secondary_color,
                    input.textbox_flags,
                    input.clip_size[0],
                    input.clip_size[1],
                )
            } else {
                build_fennel_unclipped_vertices(
                    record,
                    origin,
                    effective_scale,
                    &cpu_transform,
                    input.secondary_color,
                )
            };
            normal_vertices.extend_from_slice(&glyph_vertices);

            if record.field_0c & FENNEL_EFFECT_GLYPH_FLAG != 0 {
                let mut effect_record = *record;
                effect_record.colors = std::array::from_fn(|corner| {
                    fennel_effect_color(record.colors[corner], input.effect_colors[corner])
                });
                let effect_origin = [
                    origin[0] + input.effect_offset[0],
                    origin[1] + input.effect_offset[1],
                ];
                let effect_glyph_vertices = if input.textbox_flags & FENNEL_TEXTBOX_CLIP_FLAG != 0 {
                    build_fennel_clipped_vertices(
                        &effect_record,
                        effect_origin,
                        effective_scale,
                        &cpu_transform,
                        input.secondary_color,
                        input.textbox_flags,
                        input.clip_size[0],
                        input.clip_size[1],
                    )
                } else {
                    build_fennel_unclipped_vertices(
                        &effect_record,
                        effect_origin,
                        effective_scale,
                        &cpu_transform,
                        input.secondary_color,
                    )
                };
                effect_vertices.extend_from_slice(&effect_glyph_vertices);
            }
        }
        // `sub_7C7F90` sets the normal write pointer to
        // `base + effect_glyph_count * 0xA8`, while the effect pointer starts
        // at `base`. The submitted buffer is therefore effect-first.
        effect_vertices.extend(normal_vertices);
        batches.push(FennelOwnedTextureBatch {
            texture_token: batch.texture_token,
            vertices: effect_vertices,
        });
    }

    Ok(FennelStaticUnclippedBatchBuild {
        batches,
        processed_glyph_count: membership.processed_glyph_count,
        stop: membership.stop,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelFittingLayoutError {
    InvalidLineTable,
    UnsupportedRecordKind {
        record_index: usize,
        kind: i32,
    },
    MissingGlyphMetrics {
        record_index: usize,
        glyph_token: u32,
    },
    HorizontalWrapRequired {
        line_index: usize,
        record_index: usize,
    },
    VerticalOverflow {
        line_index: usize,
    },
    NonTerminatingWrapWithoutVerticalCutoff {
        record_index: usize,
    },
}

impl std::fmt::Display for FennelFittingLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLineTable => formatter.write_str("invalid Fennel line-start table"),
            Self::UnsupportedRecordKind { record_index, kind } => write!(
                formatter,
                "unsupported Fennel record kind {kind} at record {record_index}"
            ),
            Self::MissingGlyphMetrics {
                record_index,
                glyph_token,
            } => write!(
                formatter,
                "no Fennel glyph metrics for token {glyph_token:#010x} at record {record_index}"
            ),
            Self::HorizontalWrapRequired {
                line_index,
                record_index,
            } => write!(
                formatter,
                "Fennel line {line_index} requires the not-yet-implemented wrap branch at record {record_index}"
            ),
            Self::VerticalOverflow { line_index } => write!(
                formatter,
                "Fennel line {line_index} reaches the game's vertical-overflow branch"
            ),
            Self::NonTerminatingWrapWithoutVerticalCutoff { record_index } => write!(
                formatter,
                "Fennel wrap at record {record_index} would not terminate while vertical cutoff is disabled"
            ),
        }
    }
}

impl std::error::Error for FennelFittingLayoutError {}

#[derive(Debug, Clone, Copy)]
struct FennelMeasuredLine {
    start: usize,
    end: usize,
    x_offset: f32,
    y_bottom: f32,
    line_height: f32,
}

#[derive(Debug, Clone, Copy)]
struct FennelPreparedStaticLayout {
    effective_scale_x: f32,
    effective_scale_y: f32,
    scale_x_multiplier: f32,
    scale_y_multiplier: f32,
    initial_fallback_line_height: f32,
}

/// Reproduces the record-positioning and marker effects of the static
/// mode-zero `sub_7C1F90` path, including its automatic wrap, two fixed
/// FontManager membership tables, space candidate, and vertical-overflow
/// behavior and both internal line-metadata vectors.
///
/// Static SrTextCast initialization reaches this routine with TextBoxObject
/// flags exactly `3` or `7`. Both enable automatic wrapping and the space
/// candidate; flags `7` additionally run the proven first-explicit-line
/// horizontal auto-fit pass. If a logical line exceeds the vertical box, the
/// game changes that line's first record kind to `-254` and stops positioning.
pub fn layout_fennel_static_default<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    glyph_metrics: F,
) -> Result<FennelDefaultLayoutResult, FennelDefaultLayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    let textbox_flags = fennel_fresh_srd_textbox_flags(input.text_flags, 0);
    layout_fennel_static_common(stream, input, textbox_flags, glyph_metrics)
}

/// Reproduces fresh mode 1 (`TextBoxObject+0x2E0 == 0x000F`). Bit `0x08`
/// couples first-line horizontal auto-fit to Y scale when the original Y scale
/// is larger than the fitted X scale.
pub fn layout_fennel_static_mode1<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    glyph_metrics: F,
) -> Result<FennelDefaultLayoutResult, FennelMode1LayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    if input.text_flags & 1 != 0 {
        return Err(FennelMode1LayoutError::TextFlagBypassesModeSwitch {
            text_flags: input.text_flags,
        });
    }
    Ok(layout_fennel_static_common(
        stream,
        input,
        fennel_fresh_srd_textbox_flags(input.text_flags, 1),
        glyph_metrics,
    )?)
}

/// Reproduces the evidence-closed `sub_7C4070` subset reached by the three
/// fresh SrTextCast Flag20 states. Other bit combinations are rejected rather
/// than generalized from the similar binary control flow.
pub fn layout_fennel_static_flag20<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    textbox_flags: u32,
    glyph_metrics: F,
) -> Result<FennelFlag20LayoutResult, FennelFlag20LayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    if !matches!(textbox_flags, 0x0CA3 | 0x1CA3 | 0x2CA3) {
        return Err(FennelFlag20LayoutError::UnsupportedTextBoxFlags { textbox_flags });
    }
    if input.text_flags & 1 != 0 {
        return Err(FennelFlag20LayoutError::TextFlagBypassesModeSwitch {
            text_flags: input.text_flags,
        });
    }
    let layout = layout_fennel_static_common(stream, input, textbox_flags, glyph_metrics)?;
    let mut field_358 = 0usize;
    for line_index in 1..layout.line_descriptions.len() {
        if layout.line_descriptions[field_358].advance_width
            > layout.line_descriptions[line_index].advance_width
        {
            field_358 = line_index;
        }
    }
    Ok(FennelFlag20LayoutResult { layout, field_358 })
}

/// Reproduces the fresh mode-5/6 states that still dispatch to
/// `sub_7C1F90`. Their exact flags are `0x6C03` and `0x7C03`; bit `0x4000`
/// disables the vertical `-254` termination used by mode zero.
pub fn layout_fennel_static_mode56<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    mode: u32,
    glyph_metrics: F,
) -> Result<FennelDefaultLayoutResult, FennelMode56LayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    if !matches!(mode, 5 | 6) {
        return Err(FennelMode56LayoutError::UnsupportedMode { mode });
    }
    if input.text_flags & 1 != 0 {
        return Err(FennelMode56LayoutError::TextFlagBypassesModeSwitch {
            text_flags: input.text_flags,
        });
    }
    let textbox_flags = fennel_fresh_srd_textbox_flags(input.text_flags, mode);
    Ok(layout_fennel_static_common(
        stream,
        input,
        textbox_flags,
        glyph_metrics,
    )?)
}

/// Reproduces the actual SRD initialization path after all matching
/// `FontParamData` records have been applied in CATR order. Altered low bits
/// and `0x200` enter this API only through that binary-proven state object;
/// the narrower mode-specific APIs keep their exact-state whitelists.
pub fn layout_fennel_static_srd_font_param<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    font_param: FontParamData,
    glyph_metrics: F,
) -> Result<FennelSrdLayoutResult, FennelSrdLayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    if font_param.no_wrap_put_mode > 6 {
        return Err(FennelSrdLayoutError::UnsupportedMode {
            mode: font_param.no_wrap_put_mode,
        });
    }
    let textbox_flags = fennel_srd_textbox_flags(input.text_flags, font_param);
    layout_fennel_static_srd_explicit_flags(stream, input, textbox_flags, glyph_metrics)
}

/// Runs the same TextBox layout dispatch with an already-materialized runtime
/// flag word. `sub_7C04F0` needs this after its fit guard temporarily clears
/// `0x7CA0`; it does not reapply `FontParamData` before the second set-text.
pub fn layout_fennel_static_srd_explicit_flags<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    textbox_flags: u32,
    glyph_metrics: F,
) -> Result<FennelSrdLayoutResult, FennelSrdLayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    let layout = layout_fennel_static_common(stream, input, textbox_flags, glyph_metrics)?;
    let field_358 = if textbox_flags & 0x20 != 0 {
        let mut minimum_line = 0usize;
        for line_index in 1..layout.line_descriptions.len() {
            if layout.line_descriptions[minimum_line].advance_width
                > layout.line_descriptions[line_index].advance_width
            {
                minimum_line = line_index;
            }
        }
        Some(minimum_line)
    } else {
        None
    };
    Ok(FennelSrdLayoutResult {
        layout,
        textbox_flags,
        field_358,
    })
}

fn layout_fennel_static_common<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    textbox_flags: u32,
    mut glyph_metrics: F,
) -> Result<FennelDefaultLayoutResult, FennelDefaultLayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    let (line_ranges, metrics, stream_end_index) =
        prepare_fennel_line_ranges(stream, &mut glyph_metrics)?;
    let prepared = prepare_fennel_static_scales(
        stream,
        input,
        textbox_flags,
        &line_ranges,
        &metrics,
        stream_end_index,
    );
    let horizontal_alignment = fennel_alignment_code_from_text_flags(input.text_flags) % 3;
    let mut measured_lines = Vec::with_capacity(line_ranges.len());
    let mut current_y = 0.0f32;
    let mut previous_line_height = prepared.initial_fallback_line_height;
    let mut automatic_wrap_count = 0usize;
    let mut first_automatic_wrap = None;
    let mut vertical_overflow = None;
    let vertical_cutoff_enabled = textbox_flags & FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG == 0;

    'explicit_lines: for (explicit_line_index, &(explicit_start, explicit_end)) in
        line_ranges.iter().enumerate()
    {
        let mut segment_start = explicit_start;
        loop {
            let mut scan = segment_start;
            let mut advance = 0.0f32;
            let mut visual_extent = 0.0f32;
            let mut accepted_set_e4_overflow = false;
            let mut space_candidate = None;
            let mut automatic_break = None;

            while scan < explicit_end {
                let record = &stream.records[scan];
                let metric = metrics[scan].expect("metrics were populated during validation");
                let glyph_visual_end = fennel_visual_width(metric, textbox_flags)
                    * prepared.effective_scale_x
                    + advance;
                let advance_increment =
                    fennel_advance(record, metric, prepared.effective_scale_x, textbox_flags);
                if glyph_visual_end <= input.box_width {
                    if metric.code == 0x20 {
                        // `sub_7C1F90` saves the pointer after the space, but
                        // uses the visual extent from before it for alignment.
                        space_candidate = Some((scan + 1, visual_extent));
                    }
                    visual_extent = glyph_visual_end;
                    advance += advance_increment;
                    scan += 1;
                    continue;
                }

                if fennel_font_manager_set_e4_contains(metric.code) && !accepted_set_e4_overflow {
                    visual_extent = glyph_visual_end;
                    advance += advance_increment;
                    accepted_set_e4_overflow = true;
                    scan += 1;
                    continue;
                }

                let mut break_index = scan;
                let mut break_visual_extent = visual_extent;
                if scan > segment_start {
                    let previous_index = scan - 1;
                    let previous_metric =
                        metrics[previous_index].expect("previous metrics were populated");
                    if fennel_font_manager_set_e0_contains(previous_metric.code) {
                        break_index = previous_index;
                        break_visual_extent -= fennel_visual_width(previous_metric, textbox_flags)
                            * prepared.effective_scale_x;
                    }
                }
                if let Some((candidate_index, candidate_visual_extent)) =
                    space_candidate.filter(|(candidate_index, _)| *candidate_index > segment_start)
                {
                    break_index = candidate_index;
                    break_visual_extent = candidate_visual_extent;
                }
                automatic_break = Some((scan, break_index, break_visual_extent));
                break;
            }

            let (segment_end, segment_visual_extent, is_automatic_wrap) = match automatic_break {
                Some((overflow_record, break_index, break_visual_extent)) => {
                    first_automatic_wrap.get_or_insert(FennelAutomaticWrap {
                        explicit_line_index,
                        record_index: overflow_record,
                    });
                    (break_index, break_visual_extent, true)
                }
                None => (explicit_end, visual_extent, false),
            };

            let mut line_height = 0.0f32;
            for record_index in segment_start..segment_end {
                let metric =
                    metrics[record_index].expect("metrics were populated during validation");
                let glyph_height = (metric.em_pixels_y as f32) * prepared.effective_scale_y;
                if glyph_height > line_height {
                    line_height = glyph_height;
                }
            }
            if line_height == 0.0 {
                line_height = previous_line_height;
            }
            previous_line_height = line_height;

            if vertical_cutoff_enabled && current_y.abs() + line_height > input.box_height {
                vertical_overflow = Some(FennelVerticalOverflow {
                    explicit_line_index,
                    positioned_line_count: measured_lines.len(),
                    record_index: segment_start,
                });
                break 'explicit_lines;
            }
            let x_offset = match horizontal_alignment {
                0 => 0.0,
                1 => fennel_cvttss2si_as_f32((input.box_width - segment_visual_extent) * 0.5),
                2 => input.box_width - segment_visual_extent,
                _ => unreachable!(),
            };
            let line_advance = if measured_lines.is_empty() {
                line_height
            } else {
                line_height + input.line_spacing as f32
            };
            measured_lines.push(FennelMeasuredLine {
                start: segment_start,
                end: segment_end,
                x_offset,
                y_bottom: current_y + line_advance,
                line_height,
            });
            current_y += line_advance;

            if !is_automatic_wrap {
                break;
            }
            automatic_wrap_count += 1;
            if segment_end == segment_start && line_advance == 0.0 {
                return Err(FennelDefaultLayoutError::NonProgressingZeroHeightWrap {
                    record_index: segment_start,
                });
            }
            if segment_end == segment_start && !vertical_cutoff_enabled {
                return Err(
                    FennelDefaultLayoutError::NonTerminatingWrapWithoutVerticalCutoff {
                        record_index: segment_start,
                    },
                );
            }
            segment_start = segment_end;
        }
    }

    let alignment_code = fennel_alignment_code_from_text_flags(input.text_flags);
    let vertical_offset = if vertical_overflow.is_some() {
        // On a center/bottom first pass overflow, the game jumps directly to
        // its positioning pass without writing TextBoxObject+0x108. The
        // resulting positioned prefix is therefore top-aligned.
        0.0
    } else {
        match alignment_code / 3 {
            0 => 0.0,
            1 => fennel_cvttss2si_as_f32((input.box_height - current_y) * 0.5),
            2 => input.box_height - current_y,
            _ => unreachable!(),
        }
    };

    let mut line_positions = Vec::new();
    let mut line_descriptions = Vec::new();
    for line in &measured_lines {
        let mut advance = 0.0f32;
        for record_index in line.start..line.end {
            let record = &mut stream.records[record_index];
            let metric = metrics[record_index].expect("metrics were populated during validation");
            record.x += line.x_offset + advance;
            // `sub_7C1F90` writes only the per-line bottom into record +0x14.
            // Its center/bottom displacement lives separately at
            // TextBoxObject +0x108 and is added to the draw translation by
            // `sub_7C7F90`.
            record.y += line.y_bottom;
            record.scale_x *= prepared.scale_x_multiplier;
            record.scale_y *= prepared.scale_y_multiplier;
            advance += fennel_advance(record, metric, prepared.effective_scale_x, textbox_flags);
        }
        if line.start != line.end {
            line_positions.push(FennelLinePosition {
                x_offset: line.x_offset,
                y_bottom: line.y_bottom + vertical_offset,
            });
            line_descriptions.push(FennelLineDescription {
                first_record_index: line.start,
                record_count: line.end - line.start,
                advance_width: advance,
                line_height: line.line_height,
            });
        }
    }
    if textbox_flags & 0x200 != 0 {
        for &(start, end) in &line_ranges {
            for record_index in start..end {
                let metric =
                    metrics[record_index].expect("metrics were populated during validation");
                let fixed_cell = fennel_flag_200_cell_width_i32(metric);
                let correction = (metric.advance_x as i32).wrapping_sub(fixed_cell) as f32
                    * FENNEL_HALF
                    * prepared.effective_scale_x;
                stream.records[record_index].x -= correction;
            }
        }
    }
    if let Some(overflow) = vertical_overflow {
        stream.records[overflow.record_index].kind = FENNEL_RECORD_LINE_TABLE_OVERFLOW;
    }

    let mut record_limit = stream.glyph_count as i32;
    for (record_index, record) in stream.records.iter().enumerate() {
        if record.kind == FENNEL_RECORD_LINE_TABLE_OVERFLOW {
            record_limit = if record_index == 0 {
                -1
            } else {
                record_index as i32
            };
        }
    }

    Ok(FennelDefaultLayoutResult {
        effective_scale_x: prepared.effective_scale_x,
        effective_scale_y: prepared.effective_scale_y,
        total_height: current_y,
        textbox_vertical_offset: vertical_offset,
        positioned_line_count: measured_lines.len(),
        automatic_wrap_count,
        first_automatic_wrap,
        vertical_overflow,
        line_positions,
        line_descriptions,
        record_limit,
    })
}

fn prepare_fennel_line_ranges<F>(
    stream: &FennelPlainRecordStream,
    glyph_metrics: &mut F,
) -> Result<
    (
        Vec<(usize, usize)>,
        Vec<Option<FennelLayoutGlyphMetrics>>,
        usize,
    ),
    FennelDefaultLayoutError,
>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    let line_count = stream
        .line_start_indices
        .len()
        .checked_sub(1)
        .ok_or(FennelDefaultLayoutError::InvalidLineTable)?;
    if line_count == 0 {
        return Err(FennelDefaultLayoutError::InvalidLineTable);
    }
    let Some(&stream_end_index) = stream.line_start_indices.last() else {
        return Err(FennelDefaultLayoutError::InvalidLineTable);
    };
    if stream_end_index >= stream.records.len()
        || stream.records[stream_end_index].kind != FENNEL_RECORD_STREAM_END
    {
        return Err(FennelDefaultLayoutError::InvalidLineTable);
    }
    let mut metrics = vec![None; stream.records.len()];
    let mut line_ranges = Vec::with_capacity(line_count);
    for line_index in 0..line_count {
        let start = stream.line_start_indices[line_index];
        let next_start = stream.line_start_indices[line_index + 1];
        let Some(end) = next_start.checked_sub(1) else {
            return Err(FennelDefaultLayoutError::InvalidLineTable);
        };
        if start > end
            || end >= stream.records.len()
            || stream.records[end].kind != FENNEL_RECORD_LINE_END
        {
            return Err(FennelDefaultLayoutError::InvalidLineTable);
        }
        for record_index in start..end {
            let record = &stream.records[record_index];
            if record.kind != 0 && record.kind != FENNEL_RECORD_ZERO_WIDTH_GLYPH {
                return Err(FennelDefaultLayoutError::UnsupportedRecordKind {
                    record_index,
                    kind: record.kind,
                });
            }
            metrics[record_index] = Some(glyph_metrics(record.glyph_token).ok_or(
                FennelDefaultLayoutError::MissingGlyphMetrics {
                    record_index,
                    glyph_token: record.glyph_token,
                },
            )?);
        }
        line_ranges.push((start, end));
    }
    Ok((line_ranges, metrics, stream_end_index))
}

fn prepare_fennel_static_scales(
    stream: &FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    textbox_flags: u32,
    line_ranges: &[(usize, usize)],
    metrics: &[Option<FennelLayoutGlyphMetrics>],
    stream_end_index: usize,
) -> FennelPreparedStaticLayout {
    let mut effective_scale_x = input.scale_x;
    let mut effective_scale_y = input.scale_y;
    let mut scale_x_multiplier = 1.0f32;
    let mut scale_y_multiplier = 1.0f32;
    if textbox_flags & 4 != 0
        && !fennel_float_nearly_equal(input.scale_x, 0.0)
        && !fennel_float_nearly_equal(input.scale_y, 0.0)
    {
        let (first_start, first_end) = line_ranges[0];
        let mut advance = 0.0f32;
        let mut visual_extent = 0.0f32;
        for record_index in first_start..first_end {
            let record = &stream.records[record_index];
            let metric = metrics[record_index].expect("metrics were populated during validation");
            visual_extent = fennel_visual_width(metric, textbox_flags) * input.scale_x + advance;
            advance += fennel_advance(record, metric, input.scale_x, textbox_flags);
        }
        if visual_extent > input.box_width {
            effective_scale_x =
                ((input.scale_x * input.box_width) / visual_extent) * FENNEL_AUTO_FIT_BIAS;
            scale_x_multiplier = effective_scale_x / input.scale_x;
            if textbox_flags & 8 != 0 && input.scale_y > effective_scale_x {
                effective_scale_y = effective_scale_x;
                scale_y_multiplier = effective_scale_x / input.scale_y;
            }
        }
    }
    let initial_fallback_line_height = stream
        .records
        .iter()
        .enumerate()
        .take(stream_end_index)
        .find(|(_, record)| record.kind >= 0)
        .and_then(|(index, _)| metrics[index])
        .map(|metric| (metric.em_pixels_y as f32) * effective_scale_y)
        .unwrap_or(0.0);
    FennelPreparedStaticLayout {
        effective_scale_x,
        effective_scale_y,
        scale_x_multiplier,
        scale_y_multiplier,
        initial_fallback_line_height,
    }
}

/// Reproduces the default `sub_7C1F90` path while every explicit line fits
/// horizontally and vertically.
///
/// The function is deliberately atomic: it first measures every line and only
/// mutates the records after proving that none enters the game's automatic
/// wrapping, kinsoku, word-break, or vertical-overflow branches. Those paths
/// return an explicit error until their complete binary behavior is ported.
pub fn layout_fennel_static_fitting_lines<F>(
    stream: &mut FennelPlainRecordStream,
    input: FennelStaticLayoutInput,
    glyph_metrics: F,
) -> Result<FennelFittingLayoutResult, FennelFittingLayoutError>
where
    F: FnMut(u32) -> Option<FennelLayoutGlyphMetrics>,
{
    let mut positioned = stream.clone();
    let result = layout_fennel_static_default(&mut positioned, input, glyph_metrics)
        .map_err(fennel_default_error_to_fitting)?;
    if let Some(wrap) = result.first_automatic_wrap {
        return Err(FennelFittingLayoutError::HorizontalWrapRequired {
            line_index: wrap.explicit_line_index,
            record_index: wrap.record_index,
        });
    }
    if let Some(overflow) = result.vertical_overflow {
        return Err(FennelFittingLayoutError::VerticalOverflow {
            line_index: overflow.explicit_line_index,
        });
    }
    *stream = positioned;
    Ok(FennelFittingLayoutResult {
        effective_scale_x: result.effective_scale_x,
        effective_scale_y: result.effective_scale_y,
        total_height: result.total_height,
        textbox_vertical_offset: result.textbox_vertical_offset,
        line_count: result.positioned_line_count,
    })
}

fn fennel_default_error_to_fitting(error: FennelDefaultLayoutError) -> FennelFittingLayoutError {
    match error {
        FennelDefaultLayoutError::InvalidLineTable => FennelFittingLayoutError::InvalidLineTable,
        FennelDefaultLayoutError::UnsupportedRecordKind { record_index, kind } => {
            FennelFittingLayoutError::UnsupportedRecordKind { record_index, kind }
        }
        FennelDefaultLayoutError::MissingGlyphMetrics {
            record_index,
            glyph_token,
        } => FennelFittingLayoutError::MissingGlyphMetrics {
            record_index,
            glyph_token,
        },
        FennelDefaultLayoutError::NonProgressingZeroHeightWrap { record_index } => {
            FennelFittingLayoutError::HorizontalWrapRequired {
                line_index: 0,
                record_index,
            }
        }
        FennelDefaultLayoutError::NonTerminatingWrapWithoutVerticalCutoff { record_index } => {
            FennelFittingLayoutError::NonTerminatingWrapWithoutVerticalCutoff { record_index }
        }
    }
}

const FENNEL_AUTO_FIT_BIAS: f32 = f32::from_bits(0x3F7F_BE77);
const FENNEL_FLOAT_EQUAL_EPSILON: f32 = f32::from_bits(0x3400_0000);
const FENNEL_FLAG_200_POINT_SCALE: f32 = f32::from_bits(0x3FAA_AAAA);
const FENNEL_HALF: f32 = f32::from_bits(0x3F00_0000);

fn fennel_float_nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= FENNEL_FLOAT_EQUAL_EPSILON
}

fn fennel_flag_200_cell_width_i32(metric: FennelLayoutGlyphMetrics) -> i32 {
    fennel_cvttss2si((metric.point_x as f32) * FENNEL_FLAG_200_POINT_SCALE)
}

fn fennel_visual_width(metric: FennelLayoutGlyphMetrics, textbox_flags: u32) -> f32 {
    if textbox_flags & 0x200 != 0 {
        fennel_flag_200_cell_width_i32(metric) as f32
    } else {
        metric.bearing_x.wrapping_add(metric.width as i32) as f32
    }
}

fn fennel_advance(
    record: &FennelGlyphLayoutRecord,
    metric: FennelLayoutGlyphMetrics,
    scale_x: f32,
    textbox_flags: u32,
) -> f32 {
    let metric_advance = if textbox_flags & 0x200 != 0 {
        fennel_flag_200_cell_width_i32(metric)
    } else {
        metric.advance_x as i32
    };
    (record.field_20 + metric_advance as f32) * scale_x
}

fn fennel_cvttss2si(value: f32) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0f32..2_147_483_648.0f32).contains(&value) {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

fn fennel_cvttss2si_as_f32(value: f32) -> f32 {
    fennel_cvttss2si(value) as f32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FennelPlainRecordError {
    Decode(FennelTextDecodeError),
    Control(UnsupportedFennelControl),
    MissingGlyph { font_slot_id: u16, code: u16 },
    RecordCapacity { capacity: usize, required: usize },
}

impl std::fmt::Display for FennelPlainRecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(error) => error.fmt(formatter),
            Self::Control(error) => error.fmt(formatter),
            Self::MissingGlyph { font_slot_id, code } => {
                write!(
                    formatter,
                    "Fennel font slot {font_slot_id} has no glyph for U+{code:04X}"
                )
            }
            Self::RecordCapacity { capacity, required } => write!(
                formatter,
                "Fennel record capacity {capacity} is smaller than required {required}"
            ),
        }
    }
}

impl std::error::Error for FennelPlainRecordError {}

/// Host-independent result of the global Fennel font-slot lookup
/// `sub_F323B0`. The x86 runtime pointer remains an opaque caller-provided
/// token, while the 128-byte glyph record is copied for deterministic layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelResolvedGlyph {
    pub glyph_token: u32,
    pub glyph: RuhunaRuntimeGlyphRecord,
}

/// Builds the pre-layout record stream produced by the proven plain-token
/// subset of `sub_7C90A0`.
///
/// The caller supplies the iterator-derived placement state and an opaque
/// token for each runtime glyph. Missing glyphs are rejected because the
/// game's `fennel_npc` EmbeddedSprite fallback has not yet been reproduced.
pub fn build_fennel_plain_record_stream<F>(
    bytes: &[u8],
    font: &RuhunaRuntimeFont,
    placement: FennelGlyphPlacementInput,
    record_capacity: usize,
    mut glyph_token: F,
) -> Result<FennelPlainRecordStream, FennelPlainRecordError>
where
    F: FnMut(u16, &RuhunaRuntimeGlyphRecord) -> u32,
{
    build_fennel_plain_record_stream_with_font_slots(
        bytes,
        0,
        placement,
        record_capacity,
        |font_slot_id, code| {
            if font_slot_id != 0 {
                return None;
            }
            let glyph = *font.glyph(code)?;
            Some(FennelResolvedGlyph {
                glyph_token: glyph_token(code, &glyph),
                glyph,
            })
        },
    )
}

/// Builds the same record stream while reproducing the iterator's explicit
/// font-slot state. `$F[n]` selects `n as u16`; bare `$F` restores
/// `initial_font_slot_id`. Each glyph is resolved with the exact `(slot, code)`
/// pair consumed by `sub_F323B0`.
///
/// Slot registration order belongs to the resource loader, so this function
/// requires the caller to provide that mapping explicitly instead of treating
/// a PROJ FONT index or filename order as a global Fennel slot.
pub fn build_fennel_plain_record_stream_with_font_slots<F>(
    bytes: &[u8],
    initial_font_slot_id: u16,
    placement: FennelGlyphPlacementInput,
    record_capacity: usize,
    mut resolve_glyph: F,
) -> Result<FennelPlainRecordStream, FennelPlainRecordError>
where
    F: FnMut(u16, u16) -> Option<FennelResolvedGlyph>,
{
    let units = decode_fennel_game_text(bytes).map_err(FennelPlainRecordError::Decode)?;
    let tokens = tokenize_fennel_plain_text(&units).map_err(FennelPlainRecordError::Control)?;
    let required = tokens
        .iter()
        .filter(|token| {
            !matches!(
                token,
                FennelPlainToken::SetPosition { .. }
                    | FennelPlainToken::ToggleEffect
                    | FennelPlainToken::SetColors(_)
                    | FennelPlainToken::ResetColors
                    | FennelPlainToken::SetFontSlot(_)
                    | FennelPlainToken::ResetFontSlot
            )
        })
        .count()
        .checked_add(2)
        .unwrap_or(usize::MAX);
    if required > record_capacity {
        return Err(FennelPlainRecordError::RecordCapacity {
            capacity: record_capacity,
            required,
        });
    }

    let mut records = Vec::with_capacity(required);
    let mut line_start_indices = vec![0];
    let mut glyph_count = 0usize;
    let mut current_placement = placement;
    let mut current_font_slot_id = initial_font_slot_id;
    for token in tokens {
        match token {
            FennelPlainToken::Glyph(code) => {
                let mut resolved = resolve_glyph(current_font_slot_id, code).ok_or(
                    FennelPlainRecordError::MissingGlyph {
                        font_slot_id: current_font_slot_id,
                        code,
                    },
                )?;
                // Successful `sub_F323B0` writes the selected slot to runtime
                // glyph `+0x04` before returning the same record.
                resolved.glyph.font_slot_id = current_font_slot_id;
                let mut glyph_placement = current_placement;
                glyph_placement.glyph_token = resolved.glyph_token;
                records.push(FennelGlyphLayoutRecord::from_runtime_glyph(
                    &resolved.glyph,
                    glyph_placement,
                ));
                glyph_count += 1;
            }
            FennelPlainToken::NewLine => {
                let kind = if line_start_indices.len() < FENNEL_LINE_START_CAPACITY {
                    FENNEL_RECORD_LINE_END
                } else {
                    FENNEL_RECORD_LINE_TABLE_OVERFLOW
                };
                records.push(FennelGlyphLayoutRecord {
                    kind,
                    ..Default::default()
                });
                if line_start_indices.len() < FENNEL_LINE_START_CAPACITY {
                    line_start_indices.push(records.len());
                }
            }
            FennelPlainToken::SetPosition { x, y } => {
                current_placement.x = x;
                current_placement.y = y;
            }
            FennelPlainToken::ToggleEffect => {
                current_placement.field_0c ^= FENNEL_EFFECT_GLYPH_FLAG;
            }
            FennelPlainToken::SetColors(colors) => {
                current_placement.colors = colors;
            }
            FennelPlainToken::ResetColors => {
                current_placement.colors = placement.colors;
            }
            FennelPlainToken::SetFontSlot(font_slot_id) => {
                current_font_slot_id = font_slot_id;
            }
            FennelPlainToken::ResetFontSlot => {
                current_font_slot_id = initial_font_slot_id;
            }
        }
    }

    records.push(FennelGlyphLayoutRecord {
        kind: FENNEL_RECORD_LINE_END,
        ..Default::default()
    });
    if line_start_indices.len() < FENNEL_LINE_START_CAPACITY {
        line_start_indices.push(records.len());
    }
    records.push(FennelGlyphLayoutRecord {
        kind: FENNEL_RECORD_STREAM_END,
        ..Default::default()
    });

    Ok(FennelPlainRecordStream {
        records,
        line_start_indices,
        glyph_count,
    })
}

/// Host-independent form of the 116-byte glyph item built by
/// `font::TextBoxObject::setTextByWideString` (`sub_7C90A0`). Pointer fields
/// remain opaque 32-bit tokens so the layout is identical on x86 and x64
/// editor builds.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelGlyphLayoutRecord {
    pub kind: i32,
    pub glyph_token: u32,
    pub texture_token: u32,
    pub field_0c: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub field_20: f32,
    pub field_24: f32,
    pub field_28: f32,
    pub field_2c: f32,
    pub field_30: f32,
    pub colors: [u32; 4],
    pub scale_x: f32,
    pub scale_y: f32,
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub uv3: [f32; 2],
    pub field_6c: u32,
    pub field_70: u32,
}

/// Exact 28-byte vertex written by `sub_7C10B0` for Fennel glyph triangles.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FennelRenderVertex {
    pub position: [f32; 3],
    pub primary_color_bgra: [u8; 4],
    pub secondary_color_bgra: [u8; 4],
    pub texture_coordinates: [f32; 2],
}

/// Reproduces the non-clipping branch (`TextBoxObject flags & 0x400 == 0`) of
/// `sub_7C10B0` for one already-positioned glyph record.
pub fn build_fennel_unclipped_vertices(
    record: &FennelGlyphLayoutRecord,
    origin: [f32; 2],
    effective_scale: [f32; 2],
    transform: &Matrix4x4,
    secondary_color: u32,
) -> [FennelRenderVertex; 6] {
    // `sub_7C10B0` receives this pair through its separate `a7` argument.
    // `sub_7C7F90` computes it as record +0x44/+0x48 multiplied by
    // TextBoxObject +0xC4/+0xC8; it does not rewrite the copied record.
    let far_x = record.width * effective_scale[0] + origin[0] - record.field_2c;
    let far_y = record.height * effective_scale[1] + origin[1] - record.field_30;
    let near_x = record.field_24 + origin[0];
    let near_y = record.field_28 + origin[1];
    let positions = [
        transform_fennel_point(transform, near_x, near_y),
        transform_fennel_point(transform, far_x, near_y),
        transform_fennel_point(transform, near_x, far_y),
        transform_fennel_point(transform, far_x, far_y),
    ];
    let uvs = [record.uv0, record.uv1, record.uv2, record.uv3];
    let secondary_color_bgra = fennel_packed_color_to_bgra(secondary_color);

    FENNEL_TRIANGLE_CORNER_INDICES.map(|corner| FennelRenderVertex {
        position: positions[corner],
        primary_color_bgra: fennel_packed_color_to_bgra(record.colors[corner]),
        secondary_color_bgra,
        texture_coordinates: [
            uvs[corner][0] + FENNEL_UV_BIAS,
            uvs[corner][1] + FENNEL_UV_BIAS,
        ],
    })
}

/// Reproduces the clipping branch (`TextBoxObject flags & 0x400 != 0`) of
/// `sub_7C10B0` for one already-positioned glyph record.
///
/// The game clamps local X to `[0, clip_width]`. Local Y is clamped to
/// `[-clip_height * 0.5, clip_height * 1.5]`, except flag `0x4000` changes it
/// to `[0, clip_height]`. UVs are then linearly remapped from the unclipped
/// local rectangle before the same matrix transform and triangle order used by
/// the non-clipping branch.
pub fn build_fennel_clipped_vertices(
    record: &FennelGlyphLayoutRecord,
    origin: [f32; 2],
    effective_scale: [f32; 2],
    transform: &Matrix4x4,
    secondary_color: u32,
    textbox_flags: u32,
    clip_width: f32,
    clip_height: f32,
) -> [FennelRenderVertex; 6] {
    let far_x = record.width * effective_scale[0] + origin[0] - record.field_2c;
    let far_y = record.height * effective_scale[1] + origin[1] - record.field_30;
    let near_x = record.field_24 + origin[0];
    let near_y = record.field_28 + origin[1];
    let original_positions = [
        [near_x, near_y],
        [far_x, near_y],
        [near_x, far_y],
        [far_x, far_y],
    ];

    let y_half = if textbox_flags & FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG != 0 {
        0.0
    } else {
        clip_height * 0.5
    };
    // The binary uses XORPS with four 0x80000000 lanes, so preserve the exact
    // sign-bit operation (including -0.0 and NaN payloads).
    let clip_min_y = f32::from_bits(y_half.to_bits() ^ 0x8000_0000);
    let clip_max_y = y_half + clip_height;
    let clipped_positions = original_positions.map(|[x, y]| {
        [
            fennel_sse_clamp(x, 0.0, clip_width),
            fennel_sse_clamp(y, clip_min_y, clip_max_y),
        ]
    });

    let u_range = record.uv1[0] - record.uv0[0];
    let v_range = record.uv2[1] - record.uv0[1];
    let x_span = far_x - near_x;
    let y_span = far_y - near_y;
    let uvs = clipped_positions.map(|[x, y]| {
        [
            record.uv0[0] + (x - near_x) * u_range / x_span,
            record.uv0[1] + (y - near_y) * v_range / y_span,
        ]
    });
    let positions = clipped_positions.map(|[x, y]| transform_fennel_point(transform, x, y));
    let secondary_color_bgra = fennel_packed_color_to_bgra(secondary_color);

    FENNEL_TRIANGLE_CORNER_INDICES.map(|corner| FennelRenderVertex {
        position: positions[corner],
        primary_color_bgra: fennel_packed_color_to_bgra(record.colors[corner]),
        secondary_color_bgra,
        texture_coordinates: [
            uvs[corner][0] + FENNEL_UV_BIAS,
            uvs[corner][1] + FENNEL_UV_BIAS,
        ],
    })
}

fn fennel_sse_clamp(value: f32, minimum: f32, maximum: f32) -> f32 {
    if value > maximum {
        maximum
    } else {
        fennel_sse_max(value, minimum)
    }
}

/// SSE MAXSS returns its second operand for unordered or equal inputs. That
/// detail matters for the binary's signed-zero clipping bounds.
fn fennel_sse_max(left: f32, right: f32) -> f32 {
    if left.is_nan() || right.is_nan() || left <= right {
        right
    } else {
        left
    }
}

fn transform_fennel_point(transform: &Matrix4x4, x: f32, y: f32) -> [f32; 3] {
    let rows = &transform.rows;
    let transformed_x = rows[0][0] * x + rows[0][1] * y + rows[0][3];
    let transformed_y = rows[1][0] * x + rows[1][1] * y + rows[1][3];
    let transformed_z = rows[2][0] * x + rows[2][1] * y + rows[2][3];
    let transformed_w = rows[3][0] * x + rows[3][1] * y + rows[3][3];
    [
        transformed_x / transformed_w,
        transformed_y / transformed_w,
        transformed_z / transformed_w,
    ]
}

fn fennel_packed_color_to_bgra(color: u32) -> [u8; 4] {
    let bytes = color.to_le_bytes();
    [bytes[2], bytes[1], bytes[0], bytes[3]]
}

/// Reproduces `sub_F2D730` for one packed color. The effect RGB replaces the
/// normal RGB; only alpha is multiplied through the binary's two `/255`, one
/// `*255`, and `CVTTSS2SI` sequence.
fn fennel_effect_color(normal: u32, effect: u32) -> u32 {
    let normal_alpha = normal.to_le_bytes()[3] as f32 / 255.0;
    let effect_alpha = effect.to_le_bytes()[3] as f32 / 255.0;
    let alpha = (effect_alpha * normal_alpha * 255.0).trunc() as u8;
    let mut bytes = effect.to_le_bytes();
    bytes[3] = alpha;
    u32::from_le_bytes(bytes)
}

impl Default for FennelGlyphLayoutRecord {
    fn default() -> Self {
        Self {
            kind: 0,
            glyph_token: 0,
            texture_token: 0,
            field_0c: 0,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            field_20: 0.0,
            field_24: 0.0,
            field_28: 0.0,
            field_2c: 0.0,
            field_30: 0.0,
            colors: [0; 4],
            scale_x: 0.0,
            scale_y: 0.0,
            uv0: [0.0; 2],
            uv1: [0.0; 2],
            uv2: [0.0; 2],
            uv3: [0.0; 2],
            field_6c: 0,
            field_70: 0,
        }
    }
}

impl FennelRenderVertex {
    pub const STRIDE: usize = 28;
}

/// Values supplied by the Fennel token iterator immediately before
/// `sub_7C90A0` converts a 128-byte runtime glyph into a 116-byte layout item.
/// Fields whose semantic names are not yet proven retain their binary offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelGlyphPlacementInput {
    pub glyph_token: u32,
    pub field_0c: u32,
    pub x: i32,
    pub y: i32,
    pub field_20: i32,
    pub colors: [u32; 4],
}

impl FennelGlyphLayoutRecord {
    /// Reproduces the normal-glyph block at `0x7C92CA..0x7C940C`.
    ///
    /// `glyph.enabled` is deliberately kept as the binary field name for now;
    /// this routine proves only that it is added on both sides of the glyph box.
    pub fn from_runtime_glyph(
        glyph: &RuhunaRuntimeGlyphRecord,
        input: FennelGlyphPlacementInput,
    ) -> Self {
        let border = (glyph.enabled as i32).wrapping_mul(2);
        let (kind, width, height) = if (glyph.width as i32) <= 0 {
            (
                FENNEL_RECORD_ZERO_WIDTH_GLYPH,
                input.field_20.wrapping_add(glyph.advance_x as i32) as f32,
                input.field_20.wrapping_add(glyph.line_height as i32) as f32,
            )
        } else {
            (
                0,
                (glyph.width as i32).wrapping_add(border) as f32,
                (glyph.height as i32).wrapping_add(border) as f32,
            )
        };

        Self {
            kind,
            glyph_token: input.glyph_token,
            texture_token: glyph.texture_token,
            field_0c: input.field_0c,
            x: input.x as f32,
            y: input.y as f32,
            width,
            height,
            field_20: input.field_20 as f32,
            colors: input.colors,
            scale_x: 1.0,
            scale_y: 1.0,
            uv0: glyph.uv0,
            uv1: glyph.uv1,
            uv2: glyph.uv2,
            uv3: glyph.uv3,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_slot_registry_reuses_cached_resources_and_lowest_freed_slot() {
        let mut registry = FennelFontSlotRegistry::default();
        let first = registry.request(b"font_a".to_vec());
        let second = registry.request(b"font_b".to_vec());
        let duplicate = registry.request(b"font_a".to_vec());

        assert_eq!((first.font_slot_id, second.font_slot_id), (0, 1));
        assert!(first.first_request && first.registered);
        assert_eq!(duplicate.resource_handle, first.resource_handle);
        assert_eq!(duplicate.font_slot_id, 0);
        assert_eq!(duplicate.lease_count, 2);
        assert_eq!(registry.resource_for_slot(0), Some(&b"font_a".to_vec()));

        assert_eq!(
            registry.release(first.resource_handle).unwrap(),
            FennelFontSlotRelease::Retained {
                resource_handle: first.resource_handle,
                remaining_leases: 1,
            }
        );
        assert_eq!(registry.resource_for_slot(0), Some(&b"font_a".to_vec()));
        assert_eq!(
            registry.release(first.resource_handle).unwrap(),
            FennelFontSlotRelease::Destroyed {
                resource_handle: first.resource_handle,
                font_slot_id: 0,
                was_registered: true,
            }
        );
        assert_eq!(registry.resource_for_slot(0), None);

        let replacement = registry.request(b"font_c".to_vec());
        assert_eq!(replacement.font_slot_id, 0);
        assert_eq!(registry.resource_for_slot(1), Some(&b"font_b".to_vec()));
    }

    #[test]
    fn font_slot_registry_preserves_the_full_table_unregistered_resource() {
        let mut registry = FennelFontSlotRegistry::default();
        for index in 0..FENNEL_FONT_SLOT_COUNT {
            let request = registry.request(format!("font_{index}").into_bytes());
            assert_eq!(request.font_slot_id, index);
            assert!(request.registered);
        }

        let overflow = registry.request(b"overflow".to_vec());
        assert_eq!(overflow.font_slot_id, FENNEL_FONT_SLOT_COUNT);
        assert!(!overflow.registered);
        assert!(overflow.first_request);
        assert_eq!(
            registry.request(b"overflow".to_vec()),
            FennelFontSlotRequest {
                first_request: false,
                lease_count: 2,
                ..overflow
            }
        );
        assert_eq!(registry.resource_for_slot(FENNEL_FONT_SLOT_COUNT), None);

        assert_eq!(
            registry.release(overflow.resource_handle).unwrap(),
            FennelFontSlotRelease::Retained {
                resource_handle: overflow.resource_handle,
                remaining_leases: 1,
            }
        );
        assert_eq!(
            registry.release(overflow.resource_handle).unwrap(),
            FennelFontSlotRelease::Destroyed {
                resource_handle: overflow.resource_handle,
                font_slot_id: FENNEL_FONT_SLOT_COUNT,
                was_registered: false,
            }
        );
    }

    fn input() -> FennelGlyphPlacementInput {
        FennelGlyphPlacementInput {
            glyph_token: 0x1122_3344,
            field_0c: 0x5566_7788,
            x: -12,
            y: 34,
            field_20: 7,
            colors: [1, 2, 3, 4],
        }
    }

    fn fitting_record(glyph_token: u32) -> FennelGlyphLayoutRecord {
        FennelGlyphLayoutRecord {
            glyph_token,
            scale_x: 1.0,
            scale_y: 1.0,
            ..Default::default()
        }
    }

    fn one_line_stream(tokens: &[u32]) -> FennelPlainRecordStream {
        let mut records = tokens
            .iter()
            .copied()
            .map(fitting_record)
            .collect::<Vec<_>>();
        records.push(FennelGlyphLayoutRecord {
            kind: FENNEL_RECORD_LINE_END,
            ..Default::default()
        });
        let stream_end_index = records.len();
        records.push(FennelGlyphLayoutRecord {
            kind: FENNEL_RECORD_STREAM_END,
            ..Default::default()
        });
        FennelPlainRecordStream {
            records,
            line_start_indices: vec![0, stream_end_index],
            glyph_count: tokens.len(),
        }
    }

    fn two_line_stream(first: &[u32], second: &[u32]) -> FennelPlainRecordStream {
        let mut records = first
            .iter()
            .copied()
            .map(fitting_record)
            .collect::<Vec<_>>();
        records.push(FennelGlyphLayoutRecord {
            kind: FENNEL_RECORD_LINE_END,
            ..Default::default()
        });
        let second_start = records.len();
        records.extend(second.iter().copied().map(fitting_record));
        records.push(FennelGlyphLayoutRecord {
            kind: FENNEL_RECORD_LINE_END,
            ..Default::default()
        });
        let stream_end_index = records.len();
        records.push(FennelGlyphLayoutRecord {
            kind: FENNEL_RECORD_STREAM_END,
            ..Default::default()
        });
        FennelPlainRecordStream {
            records,
            line_start_indices: vec![0, second_start, stream_end_index],
            glyph_count: first.len() + second.len(),
        }
    }

    fn metrics(token: u32) -> Option<FennelLayoutGlyphMetrics> {
        match token {
            1 => Some(FennelLayoutGlyphMetrics {
                code: b'A' as u16,
                point_x: 10,
                bearing_x: 1,
                width: 8,
                advance_x: 10,
                em_pixels_y: 12,
            }),
            2 => Some(FennelLayoutGlyphMetrics {
                code: b'B' as u16,
                point_x: 11,
                bearing_x: 2,
                width: 7,
                advance_x: 11,
                em_pixels_y: 12,
            }),
            _ => None,
        }
    }

    #[test]
    fn font_manager_line_break_sets_match_the_binary_tables() {
        assert_eq!(FENNEL_FONT_MANAGER_SET_E0.len(), 45);
        assert_eq!(FENNEL_FONT_MANAGER_SET_E4.len(), 14);
        assert!(FENNEL_FONT_MANAGER_SET_E0.is_sorted());
        assert!(FENNEL_FONT_MANAGER_SET_E4.is_sorted());
        assert!(fennel_font_manager_set_e0_contains(0x3002));
        assert!(!fennel_font_manager_set_e0_contains(0x3008));
        assert!(fennel_font_manager_set_e4_contains(0x3008));
        assert!(!fennel_font_manager_set_e4_contains(0x3002));
    }

    #[test]
    fn layout_record_matches_binary_offsets() {
        assert_eq!(std::mem::size_of::<FennelGlyphLayoutRecord>(), 116);
        assert_eq!(std::mem::offset_of!(FennelGlyphLayoutRecord, kind), 0);
        assert_eq!(
            std::mem::offset_of!(FennelGlyphLayoutRecord, texture_token),
            8
        );
        assert_eq!(std::mem::offset_of!(FennelGlyphLayoutRecord, x), 16);
        assert_eq!(std::mem::offset_of!(FennelGlyphLayoutRecord, colors), 52);
        assert_eq!(std::mem::offset_of!(FennelGlyphLayoutRecord, uv0), 76);
        assert_eq!(std::mem::offset_of!(FennelGlyphLayoutRecord, field_70), 112);
        assert_eq!(std::mem::size_of::<FennelRenderVertex>(), 28);
        assert_eq!(std::mem::offset_of!(FennelRenderVertex, position), 0);
        assert_eq!(
            std::mem::offset_of!(FennelRenderVertex, primary_color_bgra),
            12
        );
        assert_eq!(
            std::mem::offset_of!(FennelRenderVertex, secondary_color_bgra),
            16
        );
        assert_eq!(
            std::mem::offset_of!(FennelRenderVertex, texture_coordinates),
            20
        );
    }

    #[test]
    fn fresh_srd_textbox_mode_flags_match_ac6f50_dispatch() {
        assert_eq!(fennel_fresh_srd_textbox_flags(1, 6), 3);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 0), 7);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 1), 15);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 2), 0x1ca3);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 3), 0x0ca3);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 4), 0x2ca3);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 5), 0x6c03);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 6), 0x7c03);
        assert_eq!(fennel_fresh_srd_textbox_flags(0, 7), 3);
        for mode in 2..=6 {
            assert_ne!(
                fennel_fresh_srd_textbox_flags(0, mode) & FENNEL_TEXTBOX_CLIP_FLAG,
                0
            );
        }
        assert_eq!(
            fennel_fresh_srd_textbox_flags(0, 4) & FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG,
            0
        );
        assert_ne!(
            fennel_fresh_srd_textbox_flags(0, 5) & FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG,
            0
        );
        assert_eq!(
            fennel_layout_dispatch(fennel_fresh_srd_textbox_flags(0, 0)),
            FennelLayoutDispatch::Default
        );
        assert_eq!(
            fennel_layout_dispatch(fennel_fresh_srd_textbox_flags(0, 1)),
            FennelLayoutDispatch::Default
        );
        for mode in 2..=4 {
            assert_eq!(
                fennel_layout_dispatch(fennel_fresh_srd_textbox_flags(0, mode)),
                FennelLayoutDispatch::Flag20
            );
        }
        assert_eq!(
            fennel_layout_dispatch(fennel_fresh_srd_textbox_flags(0, 5)),
            FennelLayoutDispatch::Default
        );
        assert_eq!(
            fennel_layout_dispatch(fennel_fresh_srd_textbox_flags(0, 6)),
            FennelLayoutDispatch::Default
        );
        assert_eq!(fennel_layout_dispatch(0x60), FennelLayoutDispatch::Flag20);
        assert_eq!(fennel_layout_dispatch(0x40), FennelLayoutDispatch::Flag40);
    }

    #[test]
    fn font_param_state_applies_post_mode_flags_and_effect_color_order() {
        let font_param = FontParamData {
            no_wrap_put_mode: 4,
            prohibition: true,
            word_wrap: true,
            monospaced: true,
            ..FontParamData::default()
        };
        assert_eq!(fennel_srd_textbox_flags(0, font_param), 0x2EA3);

        let cleared = FontParamData {
            no_wrap_put_mode: 4,
            ..FontParamData::default()
        };
        assert_eq!(fennel_srd_textbox_flags(1, cleared), 0);
        assert_eq!(fennel_font_param_effect_color(0xAABB_CCDD), 0xAADD_CCBB);
    }

    #[test]
    fn srd_font_style_matches_ac6f50_f2c100_and_rfo_record_flags() {
        assert_eq!(
            fennel_srd_font_style(FontParamData::default()),
            FennelSrdFontStyle {
                point_x: 32,
                point_y: 32,
                record_flags: 1,
                bold_value: -1,
                outline_value: 0,
                italic_value: 0,
            }
        );
        assert_eq!(
            fennel_srd_font_style(FontParamData {
                display_shadow: true,
                point_x: 257,
                point_y: -20,
                outline: -1,
                italic: 6,
                bold: 18,
                ..FontParamData::default()
            }),
            FennelSrdFontStyle {
                point_x: 1,
                point_y: 1,
                record_flags: 1 | 2 | 4 | 8 | FENNEL_EFFECT_GLYPH_FLAG,
                bold_value: 2,
                outline_value: 15,
                italic_value: 2,
            }
        );
        assert_eq!(
            fennel_srd_font_style(FontParamData {
                bold: -2,
                ..FontParamData::default()
            })
            .bold_value,
            -2
        );
    }

    #[test]
    fn srd_font_param_layout_uses_the_final_flag20_state() {
        let mut stream = one_line_stream(&[1]);
        let result = layout_fennel_static_srd_font_param(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 20.0,
                box_height: 20.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            FontParamData {
                no_wrap_put_mode: 2,
                prohibition: true,
                word_wrap: true,
                monospaced: true,
                ..FontParamData::default()
            },
            metrics,
        )
        .unwrap();
        assert_eq!(result.textbox_flags, 0x1EA3);
        assert_eq!(result.field_358, Some(0));

        let mut invalid = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_srd_font_param(
                &mut invalid,
                FennelStaticLayoutInput {
                    text_flags: 0,
                    box_width: 20.0,
                    box_height: 20.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                FontParamData {
                    no_wrap_put_mode: 7,
                    ..FontParamData::default()
                },
                metrics,
            )
            .unwrap_err(),
            FennelSrdLayoutError::UnsupportedMode { mode: 7 }
        );
    }

    #[test]
    fn srd_mode6_control_matches_ada300_and_preserves_values_when_disabled() {
        let mut state = FennelSrdMode6ControlState {
            text_flags: 0x35,
            mode_108: 4,
            field_12c: -1,
            field_130: -2,
            field_134: -3,
        };

        fennel_apply_srd_mode6_control(&mut state, true, 11, 22, 33);
        assert_eq!(
            state,
            FennelSrdMode6ControlState {
                text_flags: 0x34,
                mode_108: 6,
                field_12c: 33,
                field_130: 11,
                field_134: 22,
            }
        );

        fennel_apply_srd_mode6_control(&mut state, false, 101, 202, 303);
        assert_eq!(state.text_flags, 0x35);
        assert_eq!(state.mode_108, 0);
        assert_eq!(state.field_12c, 33);
        assert_eq!(state.field_130, 11);
        assert_eq!(state.field_134, 22);
    }

    #[test]
    fn srd_scroll_clock_and_no_control_outputs_match_ad8d00_ac5740() {
        let mut state = FennelSrdScrollState::initial_without_controls(FontParamData::default());
        assert_eq!(
            state,
            FennelSrdScrollState {
                field_f4: 0.0,
                field_f8: -1,
                field_fc: 40,
                field_130: 2,
                field_134: 2,
            }
        );
        assert_eq!(
            state.outputs(),
            FennelSrdScrollOutputs {
                maximum_glyphs: -1,
                field_2c4: 0.0,
                field_2c8: 80.0,
                field_2cc: 80.0,
            }
        );

        state.advance(false, 2.0, 3.0);
        assert_eq!(state.field_f4, 0.0);
        state.advance(true, 2.0, 3.0);
        assert_eq!(
            state.field_f4.to_bits(),
            ((2.0f32 * 3.0) * FENNEL_SRD_CLOCK_SCALE).to_bits()
        );

        let revealed = FennelSrdScrollState {
            field_f4: 0.25,
            field_f8: 10,
            field_fc: -40,
            field_130: 2,
            field_134: 2,
        }
        .outputs();
        assert_eq!(revealed.maximum_glyphs, 2);
        assert_eq!(revealed.field_2c4, 70.0);
        assert_eq!(revealed.field_2c8, 0.0);
        assert_eq!(revealed.field_2cc, 0.0);
    }

    fn draw_preparation_input(textbox_flags: u32) -> FennelSrdDrawPreparationInput {
        FennelSrdDrawPreparationInput {
            textbox_flags,
            text_size: [50.0, 100.0],
            clip_size: [100.0, 40.0],
            font_point_y: 32,
            scroll: FennelSrdScrollOutputs {
                maximum_glyphs: -1,
                field_2c4: 0.0,
                field_2c8: 20.0,
                field_2cc: 10.0,
            },
            repeated_text_size: None,
        }
    }

    #[test]
    fn srd_draw_preparation_matches_the_800_fit_relayout_guard() {
        assert_eq!(
            prepare_fennel_srd_draw(draw_preparation_input(0x1CA3)).unwrap(),
            FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags: 3 }
        );

        let mut vertical = draw_preparation_input(0x6C03);
        vertical.text_size[1] = 56.0;
        assert_eq!(
            prepare_fennel_srd_draw(vertical).unwrap(),
            FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags: 3 }
        );
    }

    #[test]
    fn srd_draw_preparation_matches_horizontal_7c04f0_branches() {
        let mut entering = draw_preparation_input(0x1020);
        entering.scroll.field_2c4 = 80.0;
        assert_eq!(
            prepare_fennel_srd_draw(entering).unwrap(),
            FennelSrdDrawPreparation::DrawOffset([-70.0, 0.0])
        );

        let mut repeating = draw_preparation_input(0x2020);
        repeating.scroll.field_2c4 = 100.0;
        repeating.repeated_text_size = Some([140.0, 0.0]);
        assert_eq!(
            prepare_fennel_srd_draw(repeating).unwrap(),
            FennelSrdDrawPreparation::DrawOffset([10.0, 0.0])
        );
    }

    #[test]
    fn srd_repeated_text_matches_horizontal_and_vertical_binary_builders() {
        assert_eq!(
            build_fennel_srd_repeated_text(b"text", 0x2020, FENNEL_DEFAULT_REPEAT_SPACE_COUNT,)
                .unwrap()
                .unwrap(),
            b"text\xE3\x80\x80\xE3\x80\x80\xE3\x80\x80text"
        );
        assert_eq!(
            build_fennel_srd_repeated_text(b"text", 0x0020, -7)
                .unwrap()
                .unwrap(),
            b"texttext"
        );
        assert_eq!(
            build_fennel_srd_repeated_text(b"text", 0x4000, 99)
                .unwrap()
                .unwrap(),
            b"text$n$n$ntext"
        );
        assert_eq!(
            build_fennel_srd_repeated_text(b"text", 0x1020, 3).unwrap(),
            None
        );
    }

    #[test]
    fn srd_draw_preparation_matches_vertical_1000_wait_and_tail_segments() {
        let mut input = draw_preparation_input(0x7C03);
        input.repeated_text_size = Some([0.0, 260.0]);
        for (clock_distance, expected) in [(30.0, 30.0), (65.0, 60.0), (100.0, 90.0), (180.0, 0.0)]
        {
            input.scroll.field_2c4 = clock_distance;
            assert_eq!(
                prepare_fennel_srd_draw(input).unwrap(),
                FennelSrdDrawPreparation::DrawOffset([0.0, expected])
            );
        }
    }

    #[test]
    fn srd_draw_preparation_matches_vertical_non_1000_visible_interval() {
        let mut input = draw_preparation_input(0x6C03);
        input.repeated_text_size = Some([0.0, 260.0]);
        input.scroll.field_2c4 = 100.0;
        assert_eq!(
            prepare_fennel_srd_draw(input).unwrap(),
            FennelSrdDrawPreparation::DrawOffset([0.0, 100.0])
        );
        input.scroll.field_2c4 = 200.0;
        assert_eq!(
            prepare_fennel_srd_draw(input).unwrap(),
            FennelSrdDrawPreparation::DrawOffset([0.0, 0.0])
        );
    }

    #[test]
    fn srd_draw_preparation_requires_the_binary_measured_repeated_string() {
        assert_eq!(
            prepare_fennel_srd_draw(draw_preparation_input(0x2020)).unwrap_err(),
            FennelSrdDrawPreparationError::MissingRepeatedTextMeasurement {
                textbox_flags: 0x2020,
            }
        );
    }

    #[test]
    fn srd_runtime_text_substitutions_follow_ad8d50_index_order() {
        let mut substitutions: [&[u8]; 8] = [b""; 8];
        substitutions[0] = b"$[1]";
        substitutions[1] = b"expanded";
        let prepared = prepare_fennel_srd_runtime_text(
            b"$[0]|$[1]",
            substitutions,
            23,
            FontParamData::default(),
        )
        .unwrap();
        assert_eq!(prepared.text, b"expanded|expanded");

        substitutions[0] = b"$[0]";
        substitutions[1] = b"";
        let self_reference =
            prepare_fennel_srd_runtime_text(b"$[0]", substitutions, 23, FontParamData::default())
                .unwrap();
        assert_eq!(self_reference.text, b"$[0]");
    }

    #[test]
    fn srd_runtime_text_controls_prefer_uppercase_and_update_scroll_state() {
        let prepared = prepare_fennel_srd_runtime_text(
            b"a$d[1]b$D[ \t-12junk]c$l[99]d",
            [b""; 8],
            23,
            FontParamData {
                scroll_speed: 61,
                scroll_wait: 3,
                field_34: 4,
                ..FontParamData::default()
            },
        )
        .unwrap();
        assert_eq!(prepared.text, b"a$d[1]bcd");
        assert!(prepared.found_d);
        assert!(prepared.found_l);
        assert_eq!(
            prepared.scroll_state,
            FennelSrdScrollState {
                field_f4: 0.0,
                field_f8: -12,
                field_fc: 0,
                field_130: 3,
                field_134: 4,
            }
        );
    }

    #[test]
    fn srd_runtime_text_controls_match_bracket_and_atoi_edges() {
        let incomplete =
            prepare_fennel_srd_runtime_text(b"$D[12tail", [b""; 8], 37, FontParamData::default())
                .unwrap();
        assert_eq!(incomplete.text, b"[12tail");
        assert_eq!(incomplete.scroll_state.field_f8, 37);

        let no_digits = prepare_fennel_srd_runtime_text(
            b"$D[words]$Ltail",
            [b""; 8],
            37,
            FontParamData::default(),
        )
        .unwrap();
        assert_eq!(no_digits.text, b"tail");
        assert_eq!(no_digits.scroll_state.field_f8, 0);
        assert_eq!(no_digits.scroll_state.field_fc, 0);

        let minimum = prepare_fennel_srd_runtime_text(
            b"$D[-2147483648]",
            [b""; 8],
            37,
            FontParamData::default(),
        )
        .unwrap();
        assert_eq!(minimum.scroll_state.field_f8, i32::MIN);
    }

    #[test]
    fn srd_runtime_text_scans_controls_created_by_substitution() {
        let mut substitutions: [&[u8]; 8] = [b""; 8];
        substitutions[0] = b"$D[7]$L";
        let prepared = prepare_fennel_srd_runtime_text(
            b"before$[0]after",
            substitutions,
            23,
            FontParamData::default(),
        )
        .unwrap();
        assert_eq!(prepared.text, b"beforeafter");
        assert_eq!(prepared.scroll_state.field_f8, 7);
        assert_eq!(prepared.scroll_state.field_fc, 0);
    }

    #[test]
    fn srd_runtime_text_rejects_unproven_atoi_overflow() {
        assert_eq!(
            prepare_fennel_srd_runtime_text(
                b"$D[2147483648]",
                [b""; 8],
                23,
                FontParamData::default(),
            )
            .unwrap_err(),
            FennelRuntimeTextControlError::AtoiOverflow {
                control: b'D',
                value: b"2147483648".to_vec(),
            }
        );
    }

    #[test]
    fn fitting_layout_applies_center_alignment_and_binary_metric_order() {
        let mut stream = one_line_stream(&[1, 2]);
        let result = layout_fennel_static_fitting_lines(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0x15,
                box_width: 100.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            metrics,
        )
        .unwrap();

        assert_eq!(result.effective_scale_x, 1.0);
        assert_eq!(result.effective_scale_y, 1.0);
        assert_eq!(result.total_height, 12.0);
        assert_eq!(result.line_count, 1);
        // Final visual extent is 10 advance + (2 bearing + 7 width) = 19.
        // cvttss2si((100 - 19) / 2) is 40. The vertical center displacement
        // is stored at TextBoxObject +0x108, not folded into record +0x14.
        assert_eq!(result.textbox_vertical_offset, 14.0);
        assert_eq!((stream.records[0].x, stream.records[0].y), (40.0, 12.0));
        assert_eq!((stream.records[1].x, stream.records[1].y), (50.0, 12.0));
    }

    #[test]
    fn fitting_layout_reproduces_first_line_auto_fit_bias() {
        let mut stream = one_line_stream(&[1]);
        let metric = FennelLayoutGlyphMetrics {
            code: b'A' as u16,
            point_x: 20,
            bearing_x: 0,
            width: 20,
            advance_x: 20,
            em_pixels_y: 10,
        };
        let result = layout_fennel_static_fitting_lines(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 10.0,
                box_height: 20.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            |_| Some(metric),
        )
        .unwrap();

        let expected = ((1.0 * 10.0) / 20.0) * FENNEL_AUTO_FIT_BIAS;
        assert_eq!(result.effective_scale_x.to_bits(), expected.to_bits());
        assert_eq!(stream.records[0].scale_x.to_bits(), expected.to_bits());
        assert_eq!((stream.records[0].x, stream.records[0].y), (0.0, 10.0));
    }

    #[test]
    fn fitting_layout_inserts_line_spacing_between_explicit_lines() {
        let mut stream = two_line_stream(&[1], &[2]);
        let result = layout_fennel_static_fitting_lines(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 100.0,
                box_height: 100.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 5,
            },
            metrics,
        )
        .unwrap();

        assert_eq!(result.total_height, 29.0);
        assert_eq!(stream.records[0].y, 12.0);
        assert_eq!(stream.records[2].y, 29.0);
    }

    #[test]
    fn static_text_properties_use_only_binary_proven_sources() {
        let text = TextDefinition {
            field_78: Some(0x24),
            font_index: Some(0),
            text: b"A".to_vec(),
            field_36: Some([0.75, 0.5]),
            field_7b: None,
            field_7c: Some(-2),
            field_41: Some(7),
        };
        let properties =
            FennelStaticTextProperties::from_text_definition(&text, 320.0, 80.0).unwrap();
        assert_eq!((properties.initial_x, properties.initial_y), (0, 0));
        assert_eq!(properties.glyph_spacing, -2);
        assert_eq!(properties.layout.text_flags, 0x24);
        assert_eq!(
            (properties.layout.scale_x, properties.layout.scale_y),
            (0.75, 0.5)
        );
        assert_eq!(properties.layout.line_spacing, 7);
        assert_eq!(
            properties.glyph_placement(9, 11, [1, 2, 3, 4]),
            FennelGlyphPlacementInput {
                glyph_token: 9,
                field_0c: 11,
                x: 0,
                y: 0,
                field_20: -2,
                colors: [1, 2, 3, 4],
            }
        );

        let mut missing = text;
        missing.field_7c = None;
        assert_eq!(
            FennelStaticTextProperties::from_text_definition(&missing, 320.0, 80.0).unwrap_err(),
            FennelStaticTextPropertyError::MissingProperty { code: 0x7C }
        );
    }

    #[test]
    fn fitting_layout_rejects_wrap_atomically() {
        let mut stream = one_line_stream(&[1, 2]);
        stream.records[0].x = 3.0;
        let before = stream.clone();
        assert_eq!(
            layout_fennel_static_fitting_lines(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 1,
                    box_width: 8.0,
                    box_height: 40.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                metrics,
            )
            .unwrap_err(),
            FennelFittingLayoutError::HorizontalWrapRequired {
                line_index: 0,
                record_index: 0,
            }
        );
        assert_eq!(stream, before);
    }

    #[test]
    fn default_layout_wraps_at_the_overflowing_record() {
        let mut stream = one_line_stream(&[1, 2]);
        let result = layout_fennel_static_default(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 1,
                box_width: 10.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            metrics,
        )
        .unwrap();

        assert_eq!(result.positioned_line_count, 2);
        assert_eq!(result.automatic_wrap_count, 1);
        assert_eq!(
            result.first_automatic_wrap,
            Some(FennelAutomaticWrap {
                explicit_line_index: 0,
                record_index: 1,
            })
        );
        assert_eq!(result.vertical_overflow, None);
        assert_eq!(result.record_limit, 2);
        assert_eq!((stream.records[0].x, stream.records[0].y), (0.0, 12.0));
        assert_eq!((stream.records[1].x, stream.records[1].y), (0.0, 24.0));
        assert_eq!(
            result.line_positions,
            vec![
                FennelLinePosition {
                    x_offset: 0.0,
                    y_bottom: 12.0,
                },
                FennelLinePosition {
                    x_offset: 0.0,
                    y_bottom: 24.0,
                },
            ]
        );
        assert_eq!(
            result.line_descriptions,
            vec![
                FennelLineDescription {
                    first_record_index: 0,
                    record_count: 1,
                    advance_width: 10.0,
                    line_height: 12.0,
                },
                FennelLineDescription {
                    first_record_index: 1,
                    record_count: 1,
                    advance_width: 11.0,
                    line_height: 12.0,
                },
            ]
        );
    }

    #[test]
    fn default_line_positions_include_the_separate_vertical_alignment_offset() {
        let mut stream = one_line_stream(&[1]);
        let result = layout_fennel_static_default(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0x14,
                box_width: 20.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            metrics,
        )
        .unwrap();

        assert_eq!(stream.records[0].y, 12.0);
        assert_eq!(result.textbox_vertical_offset, 14.0);
        assert_eq!(
            result.line_positions,
            vec![FennelLinePosition {
                x_offset: 5.0,
                y_bottom: 26.0,
            }]
        );
        assert_eq!(
            result.line_descriptions,
            vec![FennelLineDescription {
                first_record_index: 0,
                record_count: 1,
                advance_width: 10.0,
                line_height: 12.0,
            }]
        );
    }

    #[test]
    fn default_line_metadata_omits_empty_visual_lines() {
        let mut stream = two_line_stream(&[], &[1]);
        let result = layout_fennel_static_default(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 20.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            metrics,
        )
        .unwrap();

        assert_eq!(result.positioned_line_count, 2);
        assert_eq!(stream.records[1].y, 24.0);
        assert_eq!(
            result.line_positions,
            vec![FennelLinePosition {
                x_offset: 0.0,
                y_bottom: 24.0,
            }]
        );
        assert_eq!(
            result.line_descriptions,
            vec![FennelLineDescription {
                first_record_index: 1,
                record_count: 1,
                advance_width: 10.0,
                line_height: 12.0,
            }]
        );
    }

    #[test]
    fn flag20_layout_uses_the_proven_non_auto_fit_flags() {
        let mut stream = one_line_stream(&[1, 2]);
        let result = layout_fennel_static_flag20(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 15.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            0x0CA3,
            metrics,
        )
        .unwrap();

        assert_eq!(result.layout.effective_scale_x, 1.0);
        assert_eq!(result.layout.automatic_wrap_count, 1);
        assert_eq!(result.layout.line_descriptions.len(), 2);
        assert_eq!(result.layout.line_descriptions[0].record_count, 1);
        assert_eq!(result.layout.line_descriptions[1].record_count, 1);
    }

    #[test]
    fn flag20_field_358_tracks_the_first_strict_minimum_advance_line() {
        let input = FennelStaticLayoutInput {
            text_flags: 0,
            box_width: 40.0,
            box_height: 40.0,
            scale_x: 1.0,
            scale_y: 1.0,
            line_spacing: 0,
        };
        let mut decreasing = two_line_stream(&[1, 2], &[1]);
        let result = layout_fennel_static_flag20(&mut decreasing, input, 0x1CA3, metrics).unwrap();
        assert_eq!(
            result
                .layout
                .line_descriptions
                .iter()
                .map(|line| line.advance_width)
                .collect::<Vec<_>>(),
            vec![21.0, 10.0]
        );
        assert_eq!(result.field_358, 1);

        let mut tied = two_line_stream(&[1], &[1]);
        let result = layout_fennel_static_flag20(&mut tied, input, 0x2CA3, metrics).unwrap();
        assert_eq!(result.field_358, 0);
    }

    #[test]
    fn flag20_layout_rejects_unproven_flag_combinations() {
        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_flag20(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 0,
                    box_width: 20.0,
                    box_height: 20.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                0x20,
                metrics,
            )
            .unwrap_err(),
            FennelFlag20LayoutError::UnsupportedTextBoxFlags {
                textbox_flags: 0x20,
            }
        );

        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_flag20(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 1,
                    box_width: 20.0,
                    box_height: 20.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                0x0CA3,
                metrics,
            )
            .unwrap_err(),
            FennelFlag20LayoutError::TextFlagBypassesModeSwitch { text_flags: 1 }
        );
    }

    #[test]
    fn mode56_layout_disables_the_default_vertical_cutoff() {
        let input = FennelStaticLayoutInput {
            text_flags: 0,
            box_width: 40.0,
            box_height: 5.0,
            scale_x: 1.0,
            scale_y: 1.0,
            line_spacing: 0,
        };
        for mode in [5, 6] {
            let mut stream = two_line_stream(&[1], &[2]);
            let result = layout_fennel_static_mode56(&mut stream, input, mode, metrics).unwrap();
            assert_eq!(result.positioned_line_count, 2);
            assert_eq!(result.total_height, 24.0);
            assert_eq!(result.vertical_overflow, None);
            assert_eq!(result.record_limit, 2);
        }
    }

    #[test]
    fn mode56_layout_reports_the_binary_nonterminating_wrap_state() {
        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_mode56(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 0,
                    box_width: 5.0,
                    box_height: 5.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                5,
                metrics,
            )
            .unwrap_err(),
            FennelMode56LayoutError::Layout(
                FennelDefaultLayoutError::NonTerminatingWrapWithoutVerticalCutoff {
                    record_index: 0,
                }
            )
        );
    }

    #[test]
    fn mode56_layout_rejects_other_modes_and_the_text_bypass() {
        let input = FennelStaticLayoutInput {
            text_flags: 0,
            box_width: 20.0,
            box_height: 20.0,
            scale_x: 1.0,
            scale_y: 1.0,
            line_spacing: 0,
        };
        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_mode56(&mut stream, input, 4, metrics).unwrap_err(),
            FennelMode56LayoutError::UnsupportedMode { mode: 4 }
        );

        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_mode56(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 1,
                    ..input
                },
                6,
                metrics,
            )
            .unwrap_err(),
            FennelMode56LayoutError::TextFlagBypassesModeSwitch { text_flags: 1 }
        );
    }

    #[test]
    fn mode1_auto_fit_couples_y_only_when_it_exceeds_the_fitted_x_scale() {
        let metric = |_| {
            Some(FennelLayoutGlyphMetrics {
                code: b'A' as u16,
                point_x: 20,
                bearing_x: 0,
                width: 20,
                advance_x: 20,
                em_pixels_y: 10,
            })
        };
        let expected_scale = ((1.0 * 10.0) / 20.0) * FENNEL_AUTO_FIT_BIAS;

        let mut coupled = one_line_stream(&[1]);
        let result = layout_fennel_static_mode1(
            &mut coupled,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 10.0,
                box_height: 20.0,
                scale_x: 1.0,
                scale_y: 2.0,
                line_spacing: 0,
            },
            metric,
        )
        .unwrap();
        assert_eq!(result.effective_scale_x, expected_scale);
        assert_eq!(result.effective_scale_y, expected_scale);
        assert_eq!(coupled.records[0].scale_x, expected_scale);
        assert_eq!(coupled.records[0].scale_y, expected_scale / 2.0);
        assert_eq!(result.total_height, 10.0 * expected_scale);

        let mut uncoupled = one_line_stream(&[1]);
        let result = layout_fennel_static_mode1(
            &mut uncoupled,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 10.0,
                box_height: 20.0,
                scale_x: 1.0,
                scale_y: 0.4,
                line_spacing: 0,
            },
            metric,
        )
        .unwrap();
        assert_eq!(result.effective_scale_y, 0.4);
        assert_eq!(uncoupled.records[0].scale_y, 1.0);
        assert_eq!(result.total_height, 4.0);
    }

    #[test]
    fn mode1_layout_rejects_the_text_bypass() {
        let mut stream = one_line_stream(&[1]);
        assert_eq!(
            layout_fennel_static_mode1(
                &mut stream,
                FennelStaticLayoutInput {
                    text_flags: 1,
                    box_width: 20.0,
                    box_height: 20.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    line_spacing: 0,
                },
                metrics,
            )
            .unwrap_err(),
            FennelMode1LayoutError::TextFlagBypassesModeSwitch { text_flags: 1 }
        );
    }

    #[test]
    fn flag_200_uses_the_point_cell_and_applies_the_tail_centering_correction() {
        assert_eq!(FENNEL_FLAG_200_POINT_SCALE.to_bits(), 0x3FAA_AAAA);
        let mut stream = one_line_stream(&[1]);
        let result = layout_fennel_static_common(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 0,
                box_width: 7.0,
                box_height: 20.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            0x203,
            |_| {
                Some(FennelLayoutGlyphMetrics {
                    code: b'A' as u16,
                    point_x: 6,
                    bearing_x: 0,
                    width: 9,
                    advance_x: 10,
                    em_pixels_y: 12,
                })
            },
        )
        .unwrap();

        assert_eq!(result.automatic_wrap_count, 0);
        assert_eq!(result.line_descriptions[0].advance_width, 7.0);
        assert_eq!(stream.records[0].x, -1.5);
        assert_eq!(result.total_height, 12.0);
    }

    #[test]
    fn default_layout_uses_the_saved_space_candidate_before_alignment() {
        let mut stream = one_line_stream(&[1, 3, 2]);
        let result = layout_fennel_static_default(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 5,
                box_width: 8.0,
                box_height: 40.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            |token| match token {
                1 => Some(FennelLayoutGlyphMetrics {
                    code: b'A' as u16,
                    point_x: 5,
                    bearing_x: 0,
                    width: 4,
                    advance_x: 5,
                    em_pixels_y: 10,
                }),
                2 => Some(FennelLayoutGlyphMetrics {
                    code: b'B' as u16,
                    point_x: 5,
                    bearing_x: 0,
                    width: 4,
                    advance_x: 5,
                    em_pixels_y: 10,
                }),
                3 => Some(FennelLayoutGlyphMetrics {
                    code: 0x20,
                    point_x: 3,
                    bearing_x: 0,
                    width: 2,
                    advance_x: 3,
                    em_pixels_y: 10,
                }),
                _ => None,
            },
        )
        .unwrap();

        assert_eq!(result.automatic_wrap_count, 1);
        // The saved candidate is after the space, but its visual extent is
        // the pre-space value 4. Centering therefore truncates (8-4)/2 to 2.
        assert_eq!(stream.records[0].x, 2.0);
        assert_eq!(stream.records[1].x, 7.0);
        assert_eq!(stream.records[2].x, 2.0);
    }

    #[test]
    fn default_layout_applies_both_fixed_set_pointer_rules() {
        let metric = |token| match token {
            1 => Some(FennelLayoutGlyphMetrics {
                code: b'A' as u16,
                point_x: 5,
                bearing_x: 0,
                width: 4,
                advance_x: 5,
                em_pixels_y: 10,
            }),
            2 => Some(FennelLayoutGlyphMetrics {
                code: b'B' as u16,
                point_x: 5,
                bearing_x: 0,
                width: 4,
                advance_x: 5,
                em_pixels_y: 10,
            }),
            3 => Some(FennelLayoutGlyphMetrics {
                code: 0x3008,
                point_x: 12,
                bearing_x: 0,
                width: 12,
                advance_x: 12,
                em_pixels_y: 10,
            }),
            4 => Some(FennelLayoutGlyphMetrics {
                code: 0x3002,
                point_x: 5,
                bearing_x: 0,
                width: 4,
                advance_x: 5,
                em_pixels_y: 10,
            }),
            _ => None,
        };
        let input = FennelStaticLayoutInput {
            text_flags: 1,
            box_width: 10.0,
            box_height: 50.0,
            scale_x: 1.0,
            scale_y: 1.0,
            line_spacing: 0,
        };

        let mut set_e4 = one_line_stream(&[3, 1]);
        let result = layout_fennel_static_default(&mut set_e4, input, metric).unwrap();
        assert_eq!(result.automatic_wrap_count, 1);
        // The first over-width +0xE4 member is accepted once, so the break is
        // made before the following glyph rather than before record zero.
        assert_eq!(result.first_automatic_wrap.unwrap().record_index, 1);
        assert_eq!(set_e4.records[0].y, 10.0);
        assert_eq!(set_e4.records[1].y, 20.0);

        let mut set_e0 = one_line_stream(&[1, 4, 2]);
        let result = layout_fennel_static_default(&mut set_e0, input, metric).unwrap();
        assert_eq!(result.automatic_wrap_count, 1);
        // On overflow at B, the previous +0xE0 member is moved to the next
        // logical segment, so only A remains on the first line.
        assert_eq!(set_e0.records[0].y, 10.0);
        assert_eq!((set_e0.records[1].x, set_e0.records[1].y), (0.0, 20.0));
        assert_eq!((set_e0.records[2].x, set_e0.records[2].y), (5.0, 20.0));
    }

    #[test]
    fn default_layout_marks_the_first_record_of_a_vertical_overflow_segment() {
        let mut stream = one_line_stream(&[1]);
        let result = layout_fennel_static_default(
            &mut stream,
            FennelStaticLayoutInput {
                text_flags: 1,
                box_width: 10.0,
                box_height: 6.0,
                scale_x: 1.0,
                scale_y: 1.0,
                line_spacing: 0,
            },
            |_| {
                Some(FennelLayoutGlyphMetrics {
                    code: b'A' as u16,
                    point_x: 20,
                    bearing_x: 0,
                    width: 20,
                    advance_x: 20,
                    em_pixels_y: 5,
                })
            },
        )
        .unwrap();

        assert_eq!(result.total_height, 5.0);
        assert_eq!(result.positioned_line_count, 1);
        assert_eq!(result.automatic_wrap_count, 1);
        assert_eq!(
            result.vertical_overflow,
            Some(FennelVerticalOverflow {
                explicit_line_index: 0,
                positioned_line_count: 1,
                record_index: 0,
            })
        );
        assert_eq!(stream.records[0].kind, FENNEL_RECORD_LINE_TABLE_OVERFLOW);
        assert_eq!(result.record_limit, -1);
        assert_eq!((stream.records[0].x, stream.records[0].y), (0.0, 0.0));
    }

    #[test]
    fn texture_batch_membership_matches_the_initial_hash_forward_list() {
        let mut stream = one_line_stream(&[1, 2, 3, 4]);
        for (record, texture_token) in stream.records[..4].iter_mut().zip([1u32, 2, 16, 2]) {
            record.texture_token = texture_token;
        }
        stream.records[1].field_0c = FENNEL_EFFECT_GLYPH_FLAG;

        let build = build_fennel_texture_batch_membership(&stream, -1).unwrap();
        assert_eq!(FENNEL_INITIAL_TEXTURE_BATCH_BUCKET_COUNT, 17);
        assert_eq!(build.processed_glyph_count, 4);
        assert_eq!(build.stop, FennelTextureBatchStop::EndOfRecordArray);
        // Token 2 opens a new bucket and is inserted at global head. Token 16
        // hashes to the same bucket as token 1 and is inserted before that
        // bucket's existing first node.
        assert_eq!(
            build
                .batches
                .iter()
                .map(|batch| batch.texture_token)
                .collect::<Vec<_>>(),
            vec![2, 16, 1]
        );
        assert_eq!(build.batches[0].normal_glyph_count, 2);
        assert_eq!(build.batches[0].effect_glyph_count, 1);
        assert_eq!(build.batches[0].record_indices, vec![1, 3]);
    }

    #[test]
    fn texture_batch_scan_stops_at_minus_254_and_the_strict_maximum_gate() {
        let mut stream = one_line_stream(&[1, 2, 3]);
        for (record, texture_token) in stream.records[..3].iter_mut().zip([1u32, 2, 3]) {
            record.texture_token = texture_token;
        }
        stream.records[2].kind = FENNEL_RECORD_LINE_TABLE_OVERFLOW;
        let build = build_fennel_texture_batch_membership(&stream, -1).unwrap();
        assert_eq!(build.processed_glyph_count, 2);
        assert_eq!(
            build.stop,
            FennelTextureBatchStop::LineTableOverflow { record_index: 2 }
        );

        stream.records[2].kind = 0;
        let build = build_fennel_texture_batch_membership(&stream, 2).unwrap();
        assert_eq!(build.processed_glyph_count, 1);
        assert_eq!(
            build.stop,
            FennelTextureBatchStop::MaximumGlyphs { record_index: 1 }
        );
    }

    #[test]
    fn texture_batch_builder_rejects_the_unported_rehash_boundary() {
        let tokens = (0..18).collect::<Vec<_>>();
        let mut stream = one_line_stream(&tokens);
        for (index, record) in stream.records[..tokens.len()].iter_mut().enumerate() {
            record.texture_token = index as u32;
        }
        assert_eq!(
            build_fennel_texture_batch_membership(&stream, -1).unwrap_err(),
            FennelTextureBatchRehashRequired {
                unique_texture_count: 18,
            }
        );
    }

    #[test]
    fn static_unclipped_batches_apply_the_binary_glyph_origin_and_2d_matrix_order() {
        let mut stream = one_line_stream(&[99]);
        stream.records[0] = FennelGlyphLayoutRecord {
            glyph_token: 99,
            texture_token: 7,
            x: 10.0,
            y: 20.0,
            width: 4.0,
            height: 6.0,
            scale_x: 1.0,
            scale_y: 1.0,
            colors: [0xFFFF_FFFF; 4],
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let glyph = RuhunaRuntimeGlyphRecord {
            code: 99,
            texture_token: 7,
            enabled: 1,
            bearing_x: 2,
            bearing_y: 10,
            line_height: 12,
            flag_mode: 2,
            ..Default::default()
        };
        let mut textbox_transform = identity_matrix4x4_game();
        textbox_transform.rows[0][0] = 2.0;
        textbox_transform.rows[1][1] = 4.0;
        textbox_transform.rows[0][3] = 7.0;
        textbox_transform.rows[1][3] = 11.0;
        let build = build_fennel_static_unclipped_vertex_batches(
            &stream,
            -1,
            FennelStaticUnclippedDrawInput {
                is_2d: true,
                textbox_position: [100.0, 200.0, 0.0],
                textbox_scale: [2.0, 3.0],
                textbox_vertical_offset: 14.0,
                textbox_transform,
                secondary_color: 0,
            },
            |token| (token == 99).then_some(glyph),
        )
        .unwrap();

        assert_eq!(build.processed_glyph_count, 1);
        assert_eq!(build.stop, FennelTextureBatchStop::EndOfRecordArray);
        assert_eq!(build.batches.len(), 1);
        assert_eq!(build.batches[0].texture_token, 7);
        // correction=(bearing_x, bearing_y-flag_mode-line_height+3)=(2,-1),
        // then both axes subtract enabled=1. Local origin is therefore
        // (12,14). The 2D CPU matrix is TextBoxTransform * local translation:
        // x'=2*x+207 and y'=4*y+867 for this fixture.
        assert_eq!(
            build.batches[0]
                .vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            vec![
                [247.0, 923.0, 0.0],
                [231.0, 923.0, 0.0],
                [231.0, 995.0, 0.0],
                [247.0, 995.0, 0.0],
                [247.0, 923.0, 0.0],
                [231.0, 995.0, 0.0],
            ]
        );
    }

    #[test]
    fn srd_mode0_text_measurement_matches_7bfab0_origin_and_effect_choice() {
        let mut stream = one_line_stream(&[99]);
        stream.records[0] = FennelGlyphLayoutRecord {
            glyph_token: 99,
            texture_token: 7,
            x: 10.0,
            y: 20.0,
            width: 4.0,
            height: 6.0,
            scale_x: 1.0,
            scale_y: 1.0,
            ..Default::default()
        };
        let glyph = RuhunaRuntimeGlyphRecord {
            code: 99,
            texture_token: 7,
            enabled: 1,
            bearing_x: 2,
            bearing_y: 10,
            line_height: 12,
            flag_mode: 2,
            ..Default::default()
        };
        assert_eq!(
            measure_fennel_srd_text_size_mode0(&stream, -1, [2.0, 3.0], [5.0, -4.0], |token| {
                (token == 99).then_some(glyph)
            })
            .unwrap(),
            [20.0, 32.0]
        );

        stream.records[0].field_0c |= FENNEL_EFFECT_GLYPH_FLAG;
        assert_eq!(
            measure_fennel_srd_text_size_mode0(&stream, -1, [2.0, 3.0], [5.0, -4.0], |token| {
                (token == 99).then_some(glyph)
            })
            .unwrap(),
            [25.0, 28.0]
        );
        assert_eq!(
            measure_fennel_srd_text_size_mode0(&stream, 0, [2.0, 3.0], [5.0, -4.0], |token| (token
                == 99)
                .then_some(glyph))
            .unwrap(),
            [0.0, 0.0]
        );
    }

    #[test]
    fn effect_segment_precedes_normal_and_replaces_rgb_while_multiplying_alpha() {
        let mut stream = one_line_stream(&[1]);
        stream.records[0] = FennelGlyphLayoutRecord {
            kind: 1,
            glyph_token: 1,
            texture_token: 7,
            field_0c: FENNEL_EFFECT_GLYPH_FLAG,
            y: -3.0,
            width: 1.0,
            height: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            colors: [0x40FF_FFFF; 4],
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let build = build_fennel_normal_vertex_batches(
            &stream,
            -1,
            FennelNormalDrawInput {
                is_2d: true,
                textbox_position: [0.0; 3],
                textbox_scale: [1.0; 2],
                textbox_vertical_offset: 0.0,
                textbox_transform: identity_matrix4x4_game(),
                secondary_color: 0,
                textbox_flags: 3,
                clip_size: [0.0; 2],
                draw_offset: [3.0, 4.0],
                effect_colors: [0x8040_3020; 4],
                effect_offset: [2.0, 3.0],
            },
            |_| {
                Some(RuhunaRuntimeGlyphRecord {
                    texture_token: 7,
                    ..Default::default()
                })
            },
        );
        let vertices = &build.unwrap().batches[0].vertices;
        assert_eq!(vertices.len(), 12);
        assert_eq!(vertices[0].position, [0.0, -1.0, 0.0]);
        assert_eq!(vertices[6].position, [-2.0, -4.0, 0.0]);
        assert_eq!(vertices[0].primary_color_bgra, [0x40, 0x30, 0x20, 0x20]);
        assert_eq!(vertices[6].primary_color_bgra, [0xFF, 0xFF, 0xFF, 0x40]);
    }

    #[test]
    fn normal_batches_select_the_runtime_clipping_branch() {
        let mut stream = one_line_stream(&[1]);
        stream.records[0] = FennelGlyphLayoutRecord {
            kind: 1,
            glyph_token: 1,
            texture_token: 7,
            x: -1.0,
            y: -4.0,
            width: 4.0,
            height: 4.0,
            scale_x: 1.0,
            scale_y: 1.0,
            colors: [0xFFFF_FFFF; 4],
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let build = build_fennel_normal_vertex_batches(
            &stream,
            -1,
            FennelNormalDrawInput {
                is_2d: true,
                textbox_position: [0.0; 3],
                textbox_scale: [1.0; 2],
                textbox_vertical_offset: 0.0,
                textbox_transform: identity_matrix4x4_game(),
                secondary_color: 0,
                textbox_flags: FENNEL_TEXTBOX_CLIP_FLAG | FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG,
                clip_size: [2.0, 2.0],
                draw_offset: [0.0; 2],
                effect_colors: FENNEL_DEFAULT_EFFECT_COLORS,
                effect_offset: FENNEL_DEFAULT_EFFECT_OFFSET,
            },
            |_| {
                Some(RuhunaRuntimeGlyphRecord {
                    texture_token: 7,
                    ..Default::default()
                })
            },
        )
        .unwrap();

        assert_eq!(build.batches.len(), 1);
        assert_eq!(
            build.batches[0]
                .vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            vec![
                [2.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [2.0, 2.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
            ]
        );
        assert_eq!(
            build.batches[0]
                .vertices
                .iter()
                .map(|vertex| vertex.texture_coordinates)
                .collect::<Vec<_>>(),
            vec![
                [0.75 + FENNEL_UV_BIAS, 0.25 + FENNEL_UV_BIAS],
                [0.25 + FENNEL_UV_BIAS, 0.25 + FENNEL_UV_BIAS],
                [0.25 + FENNEL_UV_BIAS, 0.75 + FENNEL_UV_BIAS],
                [0.75 + FENNEL_UV_BIAS, 0.75 + FENNEL_UV_BIAS],
                [0.75 + FENNEL_UV_BIAS, 0.25 + FENNEL_UV_BIAS],
                [0.25 + FENNEL_UV_BIAS, 0.75 + FENNEL_UV_BIAS],
            ]
        );
    }

    #[test]
    fn builds_unclipped_six_vertex_glyph_triangles() {
        let record = FennelGlyphLayoutRecord {
            width: 4.0,
            height: 6.0,
            field_24: 1.0,
            field_28: 2.0,
            field_2c: 0.5,
            field_30: 1.0,
            colors: [0x4433_2211, 0x8877_6655, 0xCCBB_AA99, 0xFFEE_DDCC],
            scale_x: 2.0,
            scale_y: 3.0,
            uv0: [0.0, 0.1],
            uv1: [0.2, 0.3],
            uv2: [0.4, 0.5],
            uv3: [0.6, 0.7],
            ..Default::default()
        };
        let vertices = build_fennel_unclipped_vertices(
            &record,
            [10.0, 20.0],
            [2.0, 3.0],
            &crate::projection::identity_matrix4x4_game(),
            0xA4A3_A2A1,
        );

        assert_eq!(
            vertices.map(|vertex| vertex.position),
            [
                [17.5, 22.0, 0.0],
                [11.0, 22.0, 0.0],
                [11.0, 37.0, 0.0],
                [17.5, 37.0, 0.0],
                [17.5, 22.0, 0.0],
                [11.0, 37.0, 0.0],
            ]
        );
        assert_eq!(vertices[0].primary_color_bgra, [0x77, 0x66, 0x55, 0x88]);
        assert_eq!(vertices[0].secondary_color_bgra, [0xA3, 0xA2, 0xA1, 0xA4]);
        assert_eq!(
            vertices.map(|vertex| vertex.texture_coordinates),
            [
                [0.2 + FENNEL_UV_BIAS, 0.3 + FENNEL_UV_BIAS],
                [FENNEL_UV_BIAS, 0.1 + FENNEL_UV_BIAS],
                [0.4 + FENNEL_UV_BIAS, 0.5 + FENNEL_UV_BIAS],
                [0.6 + FENNEL_UV_BIAS, 0.7 + FENNEL_UV_BIAS],
                [0.2 + FENNEL_UV_BIAS, 0.3 + FENNEL_UV_BIAS],
                [0.4 + FENNEL_UV_BIAS, 0.5 + FENNEL_UV_BIAS],
            ]
        );
    }

    #[test]
    fn builds_clipped_vertices_and_remaps_uvs_before_transform() {
        let record = FennelGlyphLayoutRecord {
            width: 10.0,
            height: 8.0,
            colors: [0x4433_2211, 0x8877_6655, 0xCCBB_AA99, 0xFFEE_DDCC],
            uv0: [0.2, 0.1],
            uv1: [0.8, 0.1],
            uv2: [0.2, 0.9],
            uv3: [0.8, 0.9],
            ..Default::default()
        };
        let vertices = build_fennel_clipped_vertices(
            &record,
            [-2.0, -1.0],
            [1.0, 1.0],
            &identity_matrix4x4_game(),
            0xA4A3_A2A1,
            FENNEL_TEXTBOX_CLIP_FLAG | FENNEL_TEXTBOX_CLIP_Y_ZERO_BASE_FLAG,
            6.0,
            6.0,
        );

        assert_eq!(
            vertices.map(|vertex| vertex.position),
            [
                [6.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 6.0, 0.0],
                [6.0, 6.0, 0.0],
                [6.0, 0.0, 0.0],
                [0.0, 6.0, 0.0],
            ]
        );
        let expected_uvs = [
            [0.68, 0.2],
            [0.32, 0.2],
            [0.32, 0.8],
            [0.68, 0.8],
            [0.68, 0.2],
            [0.32, 0.8],
        ];
        for (vertex, expected) in vertices.iter().zip(expected_uvs) {
            assert!(
                (vertex.texture_coordinates[0] - (expected[0] + FENNEL_UV_BIAS)).abs() < 1.0e-6
            );
            assert!(
                (vertex.texture_coordinates[1] - (expected[1] + FENNEL_UV_BIAS)).abs() < 1.0e-6
            );
        }
        assert_eq!(vertices[0].primary_color_bgra, [0x77, 0x66, 0x55, 0x88]);
        assert_eq!(vertices[0].secondary_color_bgra, [0xA3, 0xA2, 0xA1, 0xA4]);
    }

    #[test]
    fn clipped_vertices_use_the_expanded_y_interval_without_0x4000() {
        let record = FennelGlyphLayoutRecord {
            width: 1.0,
            height: 25.0,
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let vertices = build_fennel_clipped_vertices(
            &record,
            [0.0, -7.0],
            [1.0, 1.0],
            &identity_matrix4x4_game(),
            0,
            FENNEL_TEXTBOX_CLIP_FLAG,
            1.0,
            10.0,
        );

        assert_eq!(vertices[0].position[1], -5.0);
        assert_eq!(vertices[2].position[1], 15.0);
        assert!((vertices[0].texture_coordinates[1] - (0.08 + FENNEL_UV_BIAS)).abs() < 1.0e-6);
        assert!((vertices[2].texture_coordinates[1] - (0.88 + FENNEL_UV_BIAS)).abs() < 1.0e-6);
    }

    #[test]
    fn decodes_the_games_utf8_mode_to_utf16() {
        assert_eq!(FENNEL_GAME_ENCODING_UTF8, 0);
        assert_eq!(FENNEL_CONTROL_PREFIX, 0x24);
        assert_eq!(
            decode_fennel_game_text("A中😀".as_bytes()).unwrap(),
            vec![0x0041, 0x4E2D, 0xD83D, 0xDE00]
        );
        assert_eq!(
            decode_fennel_game_text(b"before\0ignored").unwrap(),
            "before".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(
            decode_fennel_game_text(&[b'A', 0xFF]).unwrap_err(),
            FennelTextDecodeError {
                valid_prefix_len: 1
            }
        );
    }

    #[test]
    fn tokenizes_proven_plain_glyph_and_newline_forms() {
        let units = "A\r\nB\rC\nD$$E$NF$nG".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap(),
            vec![
                FennelPlainToken::Glyph(b'A' as u16),
                FennelPlainToken::NewLine,
                FennelPlainToken::Glyph(b'B' as u16),
                FennelPlainToken::NewLine,
                FennelPlainToken::Glyph(b'C' as u16),
                FennelPlainToken::NewLine,
                FennelPlainToken::Glyph(b'D' as u16),
                FennelPlainToken::Glyph(b'$' as u16),
                FennelPlainToken::Glyph(b'E' as u16),
                FennelPlainToken::NewLine,
                FennelPlainToken::Glyph(b'F' as u16),
                FennelPlainToken::NewLine,
                FennelPlainToken::Glyph(b'G' as u16),
            ]
        );
    }

    #[test]
    fn rejects_unclosed_control_grammars() {
        let units = "$Z[FFFFFFFF]".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap_err(),
            UnsupportedFennelControl {
                unit_index: 0,
                command: Some(b'Z' as u16),
            }
        );
        assert!(
            tokenize_fennel_plain_text(&[FENNEL_CONTROL_PREFIX])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn tokenizes_proven_signed_position_control() {
        let units = "A$t[-6:0]B$T[12:-34]C".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap(),
            vec![
                FennelPlainToken::Glyph(b'A' as u16),
                FennelPlainToken::SetPosition { x: -6, y: 0 },
                FennelPlainToken::Glyph(b'B' as u16),
                FennelPlainToken::SetPosition { x: 12, y: -34 },
                FennelPlainToken::Glyph(b'C' as u16),
            ]
        );
    }

    #[test]
    fn effect_control_toggles_the_iterator_flag_without_emitting_a_record() {
        let units = "A$sB$SC".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap(),
            vec![
                FennelPlainToken::Glyph(b'A' as u16),
                FennelPlainToken::ToggleEffect,
                FennelPlainToken::Glyph(b'B' as u16),
                FennelPlainToken::ToggleEffect,
                FennelPlainToken::Glyph(b'C' as u16),
            ]
        );

        let font = RuhunaRuntimeFont {
            minimum_code: b'A' as u16,
            maximum_code: b'C' as u16,
            dense_glyph_indices: vec![0, 1, 2],
            glyph_pages: vec![Some(0), Some(0), Some(0)],
            glyphs: (b'A'..=b'C')
                .map(|code| RuhunaRuntimeGlyphRecord {
                    code: u16::from(code),
                    ..Default::default()
                })
                .collect(),
        };
        let mut placement = input();
        placement.field_0c = 0x20;
        let stream =
            build_fennel_plain_record_stream(b"A$sB$SC", &font, placement, 5, |code, _glyph| {
                u32::from(code)
            })
            .unwrap();

        assert_eq!(stream.glyph_count, 3);
        assert_eq!(stream.records[0].field_0c, 0x20);
        assert_eq!(stream.records[1].field_0c, 0x20 | FENNEL_EFFECT_GLYPH_FLAG);
        assert_eq!(stream.records[2].field_0c, 0x20);
    }

    #[test]
    fn color_control_updates_and_restores_the_iterator_colors() {
        let units = "A$C[01020304]B$C[11223344:55667788:99AABBCC:DDEEFF00]C$CD"
            .encode_utf16()
            .collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap(),
            vec![
                FennelPlainToken::Glyph(b'A' as u16),
                FennelPlainToken::SetColors([0x0403_0201; 4]),
                FennelPlainToken::Glyph(b'B' as u16),
                FennelPlainToken::SetColors([0x4433_2211, 0x8877_6655, 0xCCBB_AA99, 0x00FF_EEDD,]),
                FennelPlainToken::Glyph(b'C' as u16),
                FennelPlainToken::ResetColors,
                FennelPlainToken::Glyph(b'D' as u16),
            ]
        );

        let font = RuhunaRuntimeFont {
            minimum_code: b'A' as u16,
            maximum_code: b'D' as u16,
            dense_glyph_indices: vec![0, 1, 2, 3],
            glyph_pages: vec![Some(0), Some(0), Some(0), Some(0)],
            glyphs: (b'A'..=b'D')
                .map(|code| RuhunaRuntimeGlyphRecord {
                    code: u16::from(code),
                    ..Default::default()
                })
                .collect(),
        };
        let placement = input();
        let stream = build_fennel_plain_record_stream(
            b"A$C[01020304]B$C[11223344:55667788:99AABBCC:DDEEFF00]C$CD",
            &font,
            placement,
            6,
            |code, _glyph| u32::from(code),
        )
        .unwrap();

        assert_eq!(stream.glyph_count, 4);
        assert_eq!(stream.records[0].colors, placement.colors);
        assert_eq!(stream.records[1].colors, [0x0403_0201; 4]);
        assert_eq!(
            stream.records[2].colors,
            [0x4433_2211, 0x8877_6655, 0xCCBB_AA99, 0x00FF_EEDD]
        );
        assert_eq!(stream.records[3].colors, placement.colors);
    }

    #[test]
    fn color_control_preserves_the_binary_partial_list_fallbacks() {
        let two = "$C[01020304:11121314]".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&two).unwrap(),
            vec![FennelPlainToken::SetColors([0x0403_0201; 4])]
        );

        let three = "$C[01020304:11121314:21222324]"
            .encode_utf16()
            .collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&three).unwrap(),
            vec![FennelPlainToken::SetColors([
                0x0403_0201,
                0x1413_1211,
                0x0403_0201,
                0x0403_0201,
            ])]
        );
    }

    #[test]
    fn font_control_resolves_each_glyph_with_the_current_u16_slot() {
        let units = "A$F[65543]B$FC".encode_utf16().collect::<Vec<_>>();
        assert_eq!(
            tokenize_fennel_plain_text(&units).unwrap(),
            vec![
                FennelPlainToken::Glyph(b'A' as u16),
                FennelPlainToken::SetFontSlot(7),
                FennelPlainToken::Glyph(b'B' as u16),
                FennelPlainToken::ResetFontSlot,
                FennelPlainToken::Glyph(b'C' as u16),
            ]
        );

        let mut lookups = Vec::new();
        let stream = build_fennel_plain_record_stream_with_font_slots(
            b"A$F[65543]B$FC",
            2,
            input(),
            5,
            |font_slot_id, code| {
                lookups.push((font_slot_id, code));
                matches!((font_slot_id, code), (2, 0x41 | 0x43) | (7, 0x42)).then_some(
                    FennelResolvedGlyph {
                        glyph_token: (u32::from(font_slot_id) << 16) | u32::from(code),
                        glyph: RuhunaRuntimeGlyphRecord {
                            code,
                            texture_token: u32::from(font_slot_id),
                            ..Default::default()
                        },
                    },
                )
            },
        )
        .unwrap();

        assert_eq!(lookups, vec![(2, 0x41), (7, 0x42), (2, 0x43)]);
        assert_eq!(stream.glyph_count, 3);
        assert_eq!(stream.records[0].glyph_token, 0x0002_0041);
        assert_eq!(stream.records[1].glyph_token, 0x0007_0042);
        assert_eq!(stream.records[2].glyph_token, 0x0002_0043);
        assert_eq!(stream.records[1].texture_token, 7);
    }

    #[test]
    fn builds_plain_record_stream_with_binary_markers() {
        let font = RuhunaRuntimeFont {
            minimum_code: b'A' as u16,
            maximum_code: b'B' as u16,
            dense_glyph_indices: vec![0, 1],
            glyph_pages: vec![Some(0), Some(0)],
            glyphs: vec![
                RuhunaRuntimeGlyphRecord {
                    code: b'A' as u16,
                    texture_token: 7,
                    width: 3,
                    height: 4,
                    ..Default::default()
                },
                RuhunaRuntimeGlyphRecord {
                    code: b'B' as u16,
                    texture_token: 8,
                    width: 5,
                    height: 6,
                    ..Default::default()
                },
            ],
        };
        let stream =
            build_fennel_plain_record_stream(b"A\nB", &font, input(), 5, |code, _glyph| {
                u32::from(code)
            })
            .unwrap();

        assert_eq!(stream.glyph_count, 2);
        assert_eq!(stream.line_start_indices, vec![0, 2, 4]);
        assert_eq!(
            stream
                .records
                .iter()
                .map(|record| record.kind)
                .collect::<Vec<_>>(),
            vec![
                0,
                FENNEL_RECORD_LINE_END,
                0,
                FENNEL_RECORD_LINE_END,
                FENNEL_RECORD_STREAM_END
            ]
        );
        assert_eq!(stream.records[0].glyph_token, u32::from(b'A'));
        assert_eq!(stream.records[2].texture_token, 8);
    }

    #[test]
    fn plain_record_stream_refuses_unproven_fallbacks_and_overflow() {
        let font = RuhunaRuntimeFont {
            minimum_code: b'A' as u16,
            maximum_code: b'A' as u16,
            dense_glyph_indices: vec![0],
            glyph_pages: vec![Some(0)],
            glyphs: vec![RuhunaRuntimeGlyphRecord {
                code: b'A' as u16,
                ..Default::default()
            }],
        };
        assert_eq!(
            build_fennel_plain_record_stream(b"B", &font, input(), 3, |_, _| 1).unwrap_err(),
            FennelPlainRecordError::MissingGlyph {
                font_slot_id: 0,
                code: b'B' as u16,
            }
        );
        assert_eq!(
            build_fennel_plain_record_stream(b"A", &font, input(), 2, |_, _| 1).unwrap_err(),
            FennelPlainRecordError::RecordCapacity {
                capacity: 2,
                required: 3,
            }
        );
    }

    #[test]
    fn converts_normal_runtime_glyph_exactly() {
        let glyph = RuhunaRuntimeGlyphRecord {
            texture_token: 9,
            enabled: 1,
            width: 13,
            height: 17,
            uv0: [0.1, 0.2],
            uv1: [0.3, 0.4],
            uv2: [0.5, 0.6],
            uv3: [0.7, 0.8],
            ..Default::default()
        };
        let record = FennelGlyphLayoutRecord::from_runtime_glyph(&glyph, input());

        assert_eq!(record.kind, 0);
        assert_eq!(record.glyph_token, 0x1122_3344);
        assert_eq!(record.texture_token, 9);
        assert_eq!(record.field_0c, 0x5566_7788);
        assert_eq!(record.x, -12.0);
        assert_eq!(record.y, 34.0);
        assert_eq!(record.width, 15.0);
        assert_eq!(record.height, 19.0);
        assert_eq!(record.field_20, 7.0);
        assert_eq!(record.colors, [1, 2, 3, 4]);
        assert_eq!(record.scale_x, 1.0);
        assert_eq!(record.scale_y, 1.0);
        assert_eq!(record.uv0, [0.1, 0.2]);
        assert_eq!(record.uv3, [0.7, 0.8]);
        assert_eq!(record.field_6c, 0);
        assert_eq!(record.field_70, 0);
    }

    #[test]
    fn converts_zero_width_runtime_glyph_to_special_kind() {
        let glyph = RuhunaRuntimeGlyphRecord {
            width: 0,
            line_height: 18,
            advance_x: 6,
            ..Default::default()
        };
        let record = FennelGlyphLayoutRecord::from_runtime_glyph(&glyph, input());

        assert_eq!(record.kind, -2);
        assert_eq!(record.width, 13.0);
        assert_eq!(record.height, 25.0);
    }
}
