use std::fmt;

/// The target renderer walks exactly 32 EntryInfo records when it flushes a
/// scene model module.
pub const SCENE_TARGET_ENTRY_COUNT: usize = 32;

/// Low byte carried by packet `+0x84` on the proven SRD submission path:
/// constructor `0x00`, SrRenderer `| 0x10`, submit `| 0x01`.
pub const SRD_COMMAND_PACKET_84_LOW: u8 = 0x11;

/// `sea::BasePass` property values used to build one active scene-pass rule.
/// `pass_index` keeps the binary property's original name: it indexes the
/// target's 32-entry `EntryInfo` table, while classification uses the compact
/// active-rule index produced after disabled entries are skipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BasePassProfile {
    pub name: &'static str,
    pub pass_index: u32,
    pub entry: bool,
    pub rule: ScenePassRule,
}

/// Five `sea::PassBasic` instances installed by the common `air::Scene`
/// constructor at `sub_6CF9A0`. Chusan's MainScene and BgScene both use this
/// constructor and do not replace these values in their concrete setup path.
pub const AIR_SCENE_BASE_PASSES: [BasePassProfile; 5] = [
    BasePassProfile {
        name: "Back2DPass",
        pass_index: 4,
        entry: true,
        rule: ScenePassRule {
            class_selector: 3,
            attribute_group: 0,
            condition_mode: 4,
            depth_store_selector: 5,
            depth_threshold: 0.0,
            order_threshold: 8_388_608,
        },
    },
    BasePassProfile {
        name: "OpaquePass",
        pass_index: 8,
        entry: true,
        rule: ScenePassRule {
            class_selector: 0,
            attribute_group: 0,
            condition_mode: 0,
            depth_store_selector: 3,
            depth_threshold: 0.0,
            order_threshold: 0,
        },
    },
    BasePassProfile {
        name: "PunchPass",
        pass_index: 12,
        entry: true,
        rule: ScenePassRule {
            class_selector: 1,
            attribute_group: 0,
            condition_mode: 0,
            depth_store_selector: 3,
            depth_threshold: 0.0,
            order_threshold: 0,
        },
    },
    BasePassProfile {
        // The spelling is copied exactly from the binary string table.
        name: "TrancePass",
        pass_index: 16,
        entry: true,
        rule: ScenePassRule {
            class_selector: 2,
            attribute_group: 0,
            condition_mode: 0,
            depth_store_selector: 6,
            depth_threshold: 0.0,
            order_threshold: 0,
        },
    },
    BasePassProfile {
        name: "Front2DPass",
        pass_index: 24,
        entry: true,
        rule: ScenePassRule {
            class_selector: 3,
            attribute_group: 0,
            condition_mode: 3,
            depth_store_selector: 5,
            depth_threshold: 0.0,
            order_threshold: 8_388_608,
        },
    },
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePassRule {
    pub class_selector: u32,
    pub attribute_group: u32,
    pub condition_mode: u32,
    pub depth_store_selector: i32,
    pub depth_threshold: f32,
    pub order_threshold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePassClassificationInput {
    pub command_class: u32,
    pub attribute_group: u32,
    pub depth: f32,
    pub order: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedScenePass {
    pub pass_index: usize,
    pub stores_depth: bool,
}

/// Reproduces the type-1 command classifier at `sub_63E760`.
pub const fn classify_type1_command(flags_60: u32, packet_84_low: u8) -> u32 {
    let mut class = if flags_60 & 0x20 != 0 {
        2
    } else if flags_60 & 0x40 != 0 {
        1
    } else {
        0
    };
    if flags_60 & 0x80 != 0 || packet_84_low & 0x10 != 0 {
        class = 3;
    }
    if flags_60 & 0x2000 != 0 {
        class = 4;
    }
    class
}

pub const fn classify_srd_type1_command(flags_60: u32) -> u32 {
    classify_type1_command(flags_60, SRD_COMMAND_PACKET_84_LOW)
}

pub const fn type1_attribute_group(packet_64: u32) -> u32 {
    (packet_64 >> 25) & 0x0f
}

/// Reproduces the dispatch gate in `sea::BasicScene` virtual `+0x44`
/// (`sub_601C60`). A disabled scene is not asked to filter the global queue;
/// DrawIndex 31 also produces no dispatch because the sign bit is cleared.
pub const fn scene_target_dispatch_mask(enabled: bool, draw_index: u8) -> Option<u32> {
    if !enabled {
        return None;
    }

    let mask = 1u32.wrapping_shl((draw_index as u32) & 31) & 0x7fff_ffff;
    if mask == 0 { None } else { Some(mask) }
}

/// Inputs consumed by the type-1 branch of the target filter at
/// `sub_63EA50`. The three packet fields are the exact values read from
/// packet `+0x58`, `+0x60`, and command copy `+0x10`/packet `+0x84`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type1TargetFilterInput {
    pub target_mask: u32,
    pub target_attribute: u8,
    pub command_draw_mask: u32,
    pub packet_58_low: u8,
    pub packet_flags_60: u32,
    pub packet_84_low: u8,
}

pub const fn type1_target_filter_accepts(input: Type1TargetFilterInput) -> bool {
    if input.target_mask != 0 && input.target_mask & input.command_draw_mask == 0 {
        return false;
    }
    if input.packet_flags_60 & 0x100 != 0 {
        return false;
    }

    // The binary uses x86 SHL and then tests only the low byte. Scene
    // Attribute is registered with the bounded range 0..7, but preserving the
    // masked shift count keeps this helper faithful for raw diagnostic input.
    let attribute_bit = 1u32.wrapping_shl((input.target_attribute as u32) & 31) as u8;
    if input.packet_58_low & attribute_bit == 0 {
        return false;
    }
    if input.target_attribute == 1 && input.packet_flags_60 & 0x04 == 0 {
        return false;
    }

    input.packet_84_low & 0x02 == 0
}

/// Backend-neutral fields used to admit and classify one editor draw in the
/// recovered scene target queue. Raw Ceylon packet offsets do not cross this
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SrdQueueState {
    pub target_attributes: u8,
    pub command_class: u8,
    pub attribute_group: u8,
    pub target_attribute_one_enabled: bool,
    pub rejected: bool,
}

impl SrdQueueState {
    pub const fn surface(alpha_blend_enabled: bool) -> Self {
        Self {
            target_attributes: if alpha_blend_enabled { 0xf7 } else { 0xff },
            command_class: 3,
            attribute_group: 0,
            target_attribute_one_enabled: false,
            rejected: false,
        }
    }

    pub const fn fennel() -> Self {
        Self {
            target_attributes: 0xff,
            command_class: 3,
            attribute_group: 0,
            target_attribute_one_enabled: false,
            rejected: false,
        }
    }
}

/// Proven SRD type-1 specialization. Global SRD queue construction copies
/// packet `+0x84` into command `+0x10`; the normal submit path leaves its low
/// byte at `0x11`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrdType1TargetFilter {
    pub target_dispatch_mask: Option<u32>,
    pub target_attribute: u8,
    pub command_draw_mask: u32,
}

impl SrdType1TargetFilter {
    pub const fn accepts(self, state: SrdQueueState) -> bool {
        let Some(target_mask) = self.target_dispatch_mask else {
            return false;
        };
        if target_mask != 0 && target_mask & self.command_draw_mask == 0 {
            return false;
        }
        if state.rejected {
            return false;
        }
        let attribute_bit = 1u32.wrapping_shl((self.target_attribute as u32) & 31) as u8;
        if state.target_attributes & attribute_bit == 0 {
            return false;
        }
        if self.target_attribute == 1 && !state.target_attribute_one_enabled {
            return false;
        }
        true
    }
}

/// Reproduces `sub_64BAB0`'s forward, first-match rule scan.
pub fn select_scene_pass(
    rules: &[ScenePassRule],
    input: ScenePassClassificationInput,
) -> Option<SelectedScenePass> {
    if input.command_class >= 8 {
        return None;
    }

    rules.iter().enumerate().find_map(|(pass_index, rule)| {
        if rule.attribute_group != input.attribute_group
            || !class_selector_matches(rule.class_selector, input.command_class)
            || !condition_matches(*rule, input)
        {
            return None;
        }
        Some(SelectedScenePass {
            pass_index,
            stores_depth: rule.depth_store_selector < 8,
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePassInvariantError(pub String);

impl fmt::Display for ScenePassInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScenePassInvariantError {}

/// Selects a pass only when the forward rule scan has a provably identical
/// result for every possible command depth and every `u16` order value.
///
/// This is intentionally conservative. A depth-dependent condition, or an
/// order condition that divides the `u16` domain, stops the proof instead of
/// substituting an editor-owned value. Rules that are impossible over the
/// complete domain are skipped; the first universally true rule is selected.
pub fn select_scene_pass_without_depth_or_order(
    rules: &[ScenePassRule],
    command_class: u32,
    attribute_group: u32,
) -> Result<Option<SelectedScenePass>, ScenePassInvariantError> {
    if command_class >= 8 {
        return Ok(None);
    }

    for (pass_index, rule) in rules.iter().copied().enumerate() {
        if rule.attribute_group != attribute_group
            || !class_selector_matches(rule.class_selector, command_class)
        {
            continue;
        }

        match condition_domain(rule) {
            ConditionDomain::Never => continue,
            ConditionDomain::Always => {
                return Ok(Some(SelectedScenePass {
                    pass_index,
                    stores_depth: rule.depth_store_selector < 8,
                }));
            }
            ConditionDomain::InputDependent => {
                return Err(ScenePassInvariantError(format!(
                    "matching rule {pass_index} depends on unprovided depth/order input"
                )));
            }
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConditionDomain {
    Never,
    Always,
    InputDependent,
}

fn condition_domain(rule: ScenePassRule) -> ConditionDomain {
    match rule.condition_mode {
        0 => ConditionDomain::Always,
        // A NaN threshold makes either comparison false for every f32 input.
        // All other depth thresholds retain at least one input-dependent edge,
        // including infinities and NaN command depths.
        1 | 2 if rule.depth_threshold.is_nan() => ConditionDomain::Never,
        1 | 2 => ConditionDomain::InputDependent,
        3 if rule.order_threshold == 0 => ConditionDomain::Always,
        3 if rule.order_threshold > u32::from(u16::MAX) => ConditionDomain::Never,
        3 => ConditionDomain::InputDependent,
        4 if rule.order_threshold == 0 => ConditionDomain::Never,
        4 if rule.order_threshold > u32::from(u16::MAX) => ConditionDomain::Always,
        4 => ConditionDomain::InputDependent,
        _ => ConditionDomain::Never,
    }
}

const fn class_selector_matches(selector: u32, command_class: u32) -> bool {
    selector == command_class
        || selector == 6
        || selector == 7 && command_class < 2
        || selector == 5 && command_class <= 2
}

fn condition_matches(rule: ScenePassRule, input: ScenePassClassificationInput) -> bool {
    match rule.condition_mode {
        0 => true,
        1 => rule.depth_threshold > input.depth,
        2 => input.depth >= rule.depth_threshold,
        3 => u32::from(input.order) >= rule.order_threshold,
        4 => u32::from(input.order) < rule.order_threshold,
        _ => false,
    }
}

/// Inclusive SceneModelModule pass-index range stored by one target EntryInfo.
/// The binary initializes unused entries to `(-1, -1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenePassRange {
    pub first: i32,
    pub last: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenePassProfile {
    pub rules: Vec<ScenePassRule>,
    pub target_entries: [ScenePassRange; SCENE_TARGET_ENTRY_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePassProfileError(pub String);

impl fmt::Display for ScenePassProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScenePassProfileError {}

/// Reproduces the `sub_64BDA0 -> sub_64F970` profile build:
///
/// - disabled BasePass entries produce neither a rule nor an EntryInfo update;
/// - enabled rules receive compact indices in registration order;
/// - the BasePass `PassIndex` property selects one of 32 EntryInfo slots;
/// - the slot keeps the first compact rule index and always updates the last.
pub fn build_scene_pass_profile(
    base_passes: &[BasePassProfile],
) -> Result<ScenePassProfile, ScenePassProfileError> {
    let mut rules = Vec::new();
    let mut target_entries = [ScenePassRange::DISABLED; SCENE_TARGET_ENTRY_COUNT];

    for base_pass in base_passes {
        if !base_pass.entry {
            continue;
        }
        let compact_rule_index = i32::try_from(rules.len()).map_err(|_| {
            ScenePassProfileError("active BasePass rule count exceeds i32".to_string())
        })?;
        let entry_index = usize::try_from(base_pass.pass_index).map_err(|_| {
            ScenePassProfileError(format!(
                "BasePass {:?} PassIndex {} does not fit usize",
                base_pass.name, base_pass.pass_index
            ))
        })?;
        let Some(entry) = target_entries.get_mut(entry_index) else {
            return Err(ScenePassProfileError(format!(
                "BasePass {:?} PassIndex {} is outside the binary's 32 EntryInfo slots",
                base_pass.name, base_pass.pass_index
            )));
        };

        rules.push(base_pass.rule);
        if entry.first == -1 {
            entry.first = compact_rule_index;
        }
        entry.last = compact_rule_index;
    }

    Ok(ScenePassProfile {
        rules,
        target_entries,
    })
}

impl ScenePassRange {
    pub const DISABLED: Self = Self {
        first: -1,
        last: -1,
    };

    pub const fn inclusive(first: i32, last: i32) -> Self {
        Self { first, last }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePassOrderError(pub String);

impl fmt::Display for ScenePassOrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScenePassOrderError {}

/// Reproduces the proven SceneModelModule queue/flush ordering after every
/// command has already been classified to a pass-rule index.
///
/// `classified_pass_indices` is in target classification order. Each item is
/// appended to its pass vector, retaining that order within the pass. The 32
/// target EntryInfo ranges are then visited in array order; every enabled
/// inclusive range visits pass indices in ascending order. Repeated or
/// overlapping ranges therefore repeat the same queued item indices.
///
/// This deliberately does not classify packets or invent a target profile.
pub fn build_scene_model_submission_indices(
    classified_pass_indices: &[usize],
    pass_count: usize,
    target_entries: &[ScenePassRange; SCENE_TARGET_ENTRY_COUNT],
) -> Result<Vec<usize>, ScenePassOrderError> {
    let mut pass_items = vec![Vec::new(); pass_count];
    for (item_index, &pass_index) in classified_pass_indices.iter().enumerate() {
        let Some(pass) = pass_items.get_mut(pass_index) else {
            return Err(ScenePassOrderError(format!(
                "classified item {item_index} uses pass {pass_index}, but the SceneModelModule has only {pass_count} passes"
            )));
        };
        pass.push(item_index);
    }

    let mut submission = Vec::new();
    for entry in target_entries {
        if entry.first < 0 || entry.first > entry.last {
            continue;
        }

        let first = entry.first as usize;
        if first >= pass_count {
            continue;
        }
        let last = (entry.last as usize).min(pass_count.saturating_sub(1));
        for pass_index in first..=last {
            submission.extend_from_slice(&pass_items[pass_index]);
        }
    }
    Ok(submission)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrdSceneSubmissionError(pub String);

impl fmt::Display for SrdSceneSubmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SrdSceneSubmissionError {}

/// Classifies already target-admitted normal SRD/Fennel commands and applies
/// the target's exact SceneModelModule flush order without inventing depth or
/// order values. Each packet must reach a pass invariant over those two inputs.
///
/// Target-filter activation and adjacent packet merging happen outside this
/// helper; the returned indices preserve the logical command order on either
/// side of a merge but do not claim a final GPU packet count.
pub fn build_srd_scene_submission_indices(
    states: &[SrdQueueState],
    profile: &ScenePassProfile,
) -> Result<Vec<usize>, SrdSceneSubmissionError> {
    let mut classified_pass_indices = Vec::with_capacity(states.len());
    for (command_index, state) in states.iter().copied().enumerate() {
        let command_class = u32::from(state.command_class);
        let attribute_group = u32::from(state.attribute_group);
        let selected = select_scene_pass_without_depth_or_order(
            &profile.rules,
            command_class,
            attribute_group,
        )
        .map_err(|error| {
            SrdSceneSubmissionError(format!(
                "SRD command {command_index} cannot be classified without guessing: {error}"
            ))
        })?
        .ok_or_else(|| {
            SrdSceneSubmissionError(format!(
                "SRD command {command_index} with class {command_class} and attribute group {attribute_group} matches no scene pass"
            ))
        })?;
        classified_pass_indices.push(selected.pass_index);
    }

    build_scene_model_submission_indices(
        &classified_pass_indices,
        profile.rules.len(),
        &profile.target_entries,
    )
    .map_err(|error| SrdSceneSubmissionError(error.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type1_classifier_preserves_binary_override_order() {
        assert_eq!(classify_type1_command(0, 0), 0);
        assert_eq!(classify_type1_command(0x40, 0), 1);
        assert_eq!(classify_type1_command(0x20 | 0x40, 0), 2);
        assert_eq!(classify_type1_command(0x20, 0x10), 3);
        assert_eq!(classify_type1_command(0x80 | 0x2000, 0), 4);
        assert_eq!(classify_srd_type1_command(0x4000), 3);
        assert_eq!(classify_srd_type1_command(0x6000), 4);
    }

    #[test]
    fn packet_64_attribute_group_uses_only_bits_25_through_28() {
        assert_eq!(type1_attribute_group(0), 0);
        assert_eq!(type1_attribute_group(0x1e00_0000), 0x0f);
        assert_eq!(type1_attribute_group(0xe1ff_ffff), 0);
    }

    #[test]
    fn basic_scene_dispatch_gate_uses_enable_and_signed_bit_suppression() {
        assert_eq!(scene_target_dispatch_mask(false, 0), None);
        assert_eq!(scene_target_dispatch_mask(true, 0), Some(1));
        assert_eq!(scene_target_dispatch_mask(true, 16), Some(0x1_0000));
        assert_eq!(scene_target_dispatch_mask(true, 31), None);
    }

    #[test]
    fn type1_target_filter_preserves_every_binary_rejection_gate() {
        let accepted = Type1TargetFilterInput {
            target_mask: 1,
            target_attribute: 0,
            command_draw_mask: 0xffff,
            packet_58_low: 0xff,
            packet_flags_60: 0x4000,
            packet_84_low: SRD_COMMAND_PACKET_84_LOW,
        };
        assert!(type1_target_filter_accepts(accepted));
        assert!(!type1_target_filter_accepts(Type1TargetFilterInput {
            target_mask: 0x1_0000,
            ..accepted
        }));
        assert!(!type1_target_filter_accepts(Type1TargetFilterInput {
            packet_flags_60: accepted.packet_flags_60 | 0x100,
            ..accepted
        }));
        assert!(!type1_target_filter_accepts(Type1TargetFilterInput {
            target_attribute: 2,
            packet_58_low: 0xfb,
            ..accepted
        }));
        assert!(!type1_target_filter_accepts(Type1TargetFilterInput {
            target_attribute: 1,
            ..accepted
        }));
        assert!(type1_target_filter_accepts(Type1TargetFilterInput {
            target_attribute: 1,
            packet_flags_60: accepted.packet_flags_60 | 0x04,
            ..accepted
        }));
        assert!(!type1_target_filter_accepts(Type1TargetFilterInput {
            packet_84_low: accepted.packet_84_low | 0x02,
            ..accepted
        }));
    }

    #[test]
    fn rule_scan_uses_first_matching_selector_and_attribute_group() {
        let rules = [
            ScenePassRule {
                class_selector: 6,
                attribute_group: 1,
                condition_mode: 0,
                depth_store_selector: 0,
                depth_threshold: 0.0,
                order_threshold: 0,
            },
            ScenePassRule {
                class_selector: 6,
                attribute_group: 0,
                condition_mode: 0,
                depth_store_selector: 8,
                depth_threshold: 0.0,
                order_threshold: 0,
            },
            ScenePassRule {
                class_selector: 3,
                attribute_group: 0,
                condition_mode: 0,
                depth_store_selector: 0,
                depth_threshold: 0.0,
                order_threshold: 0,
            },
        ];

        assert_eq!(
            select_scene_pass(
                &rules,
                ScenePassClassificationInput {
                    command_class: 3,
                    attribute_group: 0,
                    depth: 0.0,
                    order: 0,
                }
            ),
            Some(SelectedScenePass {
                pass_index: 1,
                stores_depth: false,
            })
        );
    }

    #[test]
    fn rule_class_selectors_five_and_seven_keep_their_exact_ranges() {
        let rule = |class_selector| ScenePassRule {
            class_selector,
            attribute_group: 0,
            condition_mode: 0,
            depth_store_selector: 0,
            depth_threshold: 0.0,
            order_threshold: 0,
        };
        let input = |command_class| ScenePassClassificationInput {
            command_class,
            attribute_group: 0,
            depth: 0.0,
            order: 0,
        };

        assert!(select_scene_pass(&[rule(7)], input(1)).is_some());
        assert!(select_scene_pass(&[rule(7)], input(2)).is_none());
        assert!(select_scene_pass(&[rule(5)], input(2)).is_some());
        assert!(select_scene_pass(&[rule(5)], input(3)).is_none());
    }

    #[test]
    fn rule_condition_modes_match_depth_order_and_nan_edges() {
        let input = ScenePassClassificationInput {
            command_class: 3,
            attribute_group: 0,
            depth: 5.0,
            order: 10,
        };
        let rule = |condition_mode, depth_threshold, order_threshold| ScenePassRule {
            class_selector: 3,
            attribute_group: 0,
            condition_mode,
            depth_store_selector: 0,
            depth_threshold,
            order_threshold,
        };

        assert!(select_scene_pass(&[rule(1, 6.0, 0)], input).is_some());
        assert!(select_scene_pass(&[rule(1, 5.0, 0)], input).is_none());
        assert!(select_scene_pass(&[rule(2, 5.0, 0)], input).is_some());
        assert!(select_scene_pass(&[rule(2, 6.0, 0)], input).is_none());
        assert!(select_scene_pass(&[rule(3, 0.0, 10)], input).is_some());
        assert!(select_scene_pass(&[rule(3, 0.0, 11)], input).is_none());
        assert!(select_scene_pass(&[rule(4, 0.0, 11)], input).is_some());
        assert!(select_scene_pass(&[rule(4, 0.0, 10)], input).is_none());
        assert!(select_scene_pass(&[rule(1, f32::NAN, 0)], input).is_none());
        assert!(
            select_scene_pass(
                &[rule(2, 0.0, 0)],
                ScenePassClassificationInput {
                    depth: f32::NAN,
                    ..input
                }
            )
            .is_none()
        );
    }

    #[test]
    fn air_scene_default_base_pass_table_matches_the_binary_records() {
        assert_eq!(
            AIR_SCENE_BASE_PASSES
                .iter()
                .map(|pass| (pass.name, pass.pass_index))
                .collect::<Vec<_>>(),
            vec![
                ("Back2DPass", 4),
                ("OpaquePass", 8),
                ("PunchPass", 12),
                ("TrancePass", 16),
                ("Front2DPass", 24),
            ]
        );
        assert!(
            AIR_SCENE_BASE_PASSES
                .iter()
                .all(|pass| pass.entry && pass.rule.attribute_group == 0)
        );

        let back = AIR_SCENE_BASE_PASSES[0].rule;
        assert_eq!((back.class_selector, back.condition_mode), (3, 4));
        assert_eq!(back.depth_store_selector, 5);
        assert_eq!(back.order_threshold, 8_388_608);

        let front = AIR_SCENE_BASE_PASSES[4].rule;
        assert_eq!((front.class_selector, front.condition_mode), (3, 3));
        assert_eq!(front.depth_store_selector, 5);
        assert_eq!(front.order_threshold, 8_388_608);
    }

    #[test]
    fn air_scene_profile_uses_pass_index_as_entry_slot_not_rule_index() {
        let profile = build_scene_pass_profile(&AIR_SCENE_BASE_PASSES)
            .expect("binary PassIndex values are within 0..31");

        assert_eq!(profile.rules.len(), 5);
        for (entry_index, compact_rule_index) in [(4, 0), (8, 1), (12, 2), (16, 3), (24, 4)] {
            assert_eq!(
                profile.target_entries[entry_index],
                ScenePassRange::inclusive(compact_rule_index, compact_rule_index)
            );
        }
        for (entry_index, entry) in profile.target_entries.iter().enumerate() {
            if ![4, 8, 12, 16, 24].contains(&entry_index) {
                assert_eq!(*entry, ScenePassRange::DISABLED);
            }
        }
    }

    #[test]
    fn profile_builder_skips_disabled_entries_and_extends_duplicate_slots() {
        let mut passes = AIR_SCENE_BASE_PASSES;
        passes[1].entry = false;
        passes[2].pass_index = 4;

        let profile = build_scene_pass_profile(&passes).unwrap();
        assert_eq!(profile.rules.len(), 4);
        assert_eq!(profile.target_entries[4], ScenePassRange::inclusive(0, 1));
        assert_eq!(profile.target_entries[8], ScenePassRange::DISABLED);
        assert_eq!(profile.target_entries[16], ScenePassRange::inclusive(2, 2));
        assert_eq!(profile.target_entries[24], ScenePassRange::inclusive(3, 3));
    }

    #[test]
    fn profile_builder_rejects_pass_index_outside_the_proven_property_range() {
        let invalid = BasePassProfile {
            pass_index: SCENE_TARGET_ENTRY_COUNT as u32,
            ..AIR_SCENE_BASE_PASSES[0]
        };
        let error = build_scene_pass_profile(&[invalid]).unwrap_err();
        assert!(error.0.contains("outside the binary's 32 EntryInfo slots"));
    }

    #[test]
    fn default_srd_class_three_routes_to_back_2d_first() {
        let profile = build_scene_pass_profile(&AIR_SCENE_BASE_PASSES).unwrap();
        for order in [0, u16::MAX] {
            assert_eq!(
                select_scene_pass(
                    &profile.rules,
                    ScenePassClassificationInput {
                        command_class: 3,
                        attribute_group: 0,
                        depth: 0.0,
                        order,
                    }
                ),
                Some(SelectedScenePass {
                    pass_index: 0,
                    stores_depth: true,
                })
            );
        }
    }

    #[test]
    fn default_air_scene_selection_is_proven_without_depth_or_order_inputs() {
        let profile = build_scene_pass_profile(&AIR_SCENE_BASE_PASSES).unwrap();
        assert_eq!(
            select_scene_pass_without_depth_or_order(&profile.rules, 3, 0),
            Ok(Some(SelectedScenePass {
                pass_index: 0,
                stores_depth: true,
            }))
        );
    }

    #[test]
    fn invariant_selector_rejects_a_rule_that_splits_the_order_domain() {
        let rule = ScenePassRule {
            class_selector: 3,
            attribute_group: 0,
            condition_mode: 4,
            depth_store_selector: 5,
            depth_threshold: 0.0,
            order_threshold: 100,
        };
        let error = select_scene_pass_without_depth_or_order(&[rule], 3, 0).unwrap_err();
        assert!(error.0.contains("depends on unprovided depth/order input"));
    }

    #[test]
    fn normal_surface_and_fennel_states_share_the_default_air_pass() {
        let profile = build_scene_pass_profile(&AIR_SCENE_BASE_PASSES).unwrap();
        let image = SrdQueueState::surface(true);
        let fennel = SrdQueueState::fennel();

        assert_eq!(
            build_srd_scene_submission_indices(&[image, fennel], &profile).unwrap(),
            vec![0, 1]
        );
    }

    #[test]
    fn class_four_state_is_not_forced_into_an_unproven_default_pass() {
        let profile = build_scene_pass_profile(&AIR_SCENE_BASE_PASSES).unwrap();
        let state = SrdQueueState {
            command_class: 4,
            ..SrdQueueState::surface(false)
        };
        let error = build_srd_scene_submission_indices(&[state], &profile).unwrap_err();
        assert!(error.0.contains("matches no scene pass"));
    }

    #[test]
    fn scene_model_passes_keep_stable_insertion_and_ascending_range_order() {
        let mut entries = [ScenePassRange::DISABLED; SCENE_TARGET_ENTRY_COUNT];
        entries[0] = ScenePassRange::inclusive(1, 2);
        entries[1] = ScenePassRange::inclusive(0, 0);

        let order = build_scene_model_submission_indices(&[2, 0, 2, 1], 3, &entries)
            .expect("valid classified passes");

        assert_eq!(order, vec![3, 0, 2, 1]);
    }

    #[test]
    fn overlapping_target_ranges_repeat_the_same_pass_records() {
        let mut entries = [ScenePassRange::DISABLED; SCENE_TARGET_ENTRY_COUNT];
        entries[0] = ScenePassRange::inclusive(2, 2);
        entries[1] = ScenePassRange::inclusive(1, 2);

        let order = build_scene_model_submission_indices(&[2, 1, 2], 3, &entries)
            .expect("valid classified passes");

        assert_eq!(order, vec![0, 2, 1, 0, 2]);
    }

    #[test]
    fn disabled_reversed_and_out_of_range_entry_passes_are_no_ops() {
        let mut entries = [ScenePassRange::DISABLED; SCENE_TARGET_ENTRY_COUNT];
        entries[0] = ScenePassRange::inclusive(2, 1);
        entries[1] = ScenePassRange::inclusive(8, 12);
        entries[2] = ScenePassRange::inclusive(1, 12);

        let order = build_scene_model_submission_indices(&[0, 1, 2], 3, &entries)
            .expect("valid classified passes");

        assert_eq!(order, vec![1, 2]);
    }

    #[test]
    fn invalid_classification_is_rejected_instead_of_being_guessed() {
        let entries = [ScenePassRange::DISABLED; SCENE_TARGET_ENTRY_COUNT];
        let error = build_scene_model_submission_indices(&[0, 3], 3, &entries)
            .expect_err("pass index equal to pass count must fail");

        assert!(error.0.contains("classified item 1 uses pass 3"));
    }
}
