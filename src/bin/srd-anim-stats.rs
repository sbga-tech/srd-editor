use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use srd_editor::animation::{KeyData, Track};
use srd_editor::document::EditorDocument;

const MAX_LIST_ITEMS: usize = 40;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct AnimationKey {
    file: String,
    scene_index: usize,
    layer_index: usize,
    animation_index: usize,
}

#[derive(Clone, Debug)]
struct NodeOutlier {
    count: usize,
    detail: String,
}

#[derive(Clone, Debug)]
struct OutsideFrameRecord {
    count: usize,
    detail: String,
}

#[derive(Clone, Debug)]
struct TrackRecord {
    key_count: usize,
    detail: String,
}

#[derive(Default)]
struct Stats {
    files_found: usize,
    files_loaded: usize,
    scenes: usize,
    discovery_errors: Vec<String>,
    load_failures: Vec<String>,

    scene_set_counts: Vec<i64>,
    total_anms: usize,
    set_slot_less: usize,
    set_slot_equal: usize,
    set_slot_greater: usize,
    set_length_mismatches: Vec<String>,
    anms_slot_total: usize,
    anms_enabled: usize,
    anms_disabled: usize,
    anms_empty_animation_name: usize,
    enabled_empty: usize,
    enabled_named: usize,
    disabled_empty: usize,
    disabled_named: usize,
    dangling_slots: Vec<String>,
    anms_starts: Vec<i64>,
    anms_runtime_durations: Vec<i64>,
    anms_runtime_le_start: usize,
    anms_start_nonzero: usize,
    anms_empty_names: Vec<String>,
    anms_duplicate_name_groups: Vec<String>,

    layer_animation_counts: Vec<i64>,
    total_layers: usize,
    layers_without_animations: usize,
    total_animations: usize,
    animation_keys: Vec<AnimationKey>,
    referenced_animation_keys: BTreeSet<AnimationKey>,
    anim_negative_duration: usize,
    anim_runtime_durations: Vec<i64>,
    anim_empty_names: Vec<String>,
    anim_duplicate_name_groups: Vec<String>,
    anim_motion_counts: Vec<i64>,
    anim_distinct_node_counts: Vec<i64>,
    node_outliers: Vec<NodeOutlier>,
    negative_target_motions: usize,
    negative_target_values: BTreeMap<i32, usize>,
    high_target_motions: usize,
    out_of_range_zero_track_motions: usize,
    out_of_range_nonzero_track_motions: usize,
    out_of_range_nonzero_track_details: Vec<String>,
    zero_track_valid_target_motions: usize,
    out_of_range_motion_targets: Vec<String>,
    motion_track_counts: Vec<i64>,
    channel_counts: BTreeMap<u16, usize>,
    channel_key_data_counts: BTreeMap<u16, BTreeMap<usize, usize>>,
    channel_format_counts: BTreeMap<u16, BTreeMap<u32, usize>>,
    track_key_counts: Vec<i64>,
    largest_track: Option<TrackRecord>,
    frame_pair_counts: Vec<i64>,
    widest_frame: Option<i32>,
    outside_frame_counts: Vec<i64>,
    outside_frame_total: usize,
    animations_with_outside_frames: usize,
    outside_frame_records: Vec<OutsideFrameRecord>,
    worst_frame_pair: Option<(usize, String)>,
}

fn main() {
    let Some(corpus_root) = corpus_root() else {
        eprintln!("GAME_DATA_CORPUS is not set");
        std::process::exit(2);
    };
    let mut paths = Vec::new();
    let mut stats = Stats::default();

    if let Err(error) = collect_srd_files(&corpus_root, &mut paths) {
        stats.discovery_errors.push(error);
    }
    paths.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    stats.files_found = paths.len();

    for path in paths {
        let file_label = relative_path(&corpus_root, &path);
        match EditorDocument::load(&path) {
            Ok(document) => {
                stats.files_loaded += 1;
                inspect_project(&mut stats, &file_label, &document.project);
            }
            Err(error) => stats
                .load_failures
                .push(format!("file={file_label} error={error}")),
        }
    }

    print_report(&stats, &corpus_root);
}

fn corpus_root() -> Option<PathBuf> {
    env::var_os("GAME_DATA_CORPUS").map(PathBuf::from)
}

fn collect_srd_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| format!("failed to discover corpus root {}: {error}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "failed to read corpus directory {}: {error}",
                path.display()
            )
        })?;
    entries.sort_by(|left, right| {
        left.path()
            .to_string_lossy()
            .cmp(&right.path().to_string_lossy())
    });

    for entry in entries {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_srd_files(&entry_path, output)?;
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "srd")
        {
            output.push(entry_path);
        }
    }
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn inspect_project(stats: &mut Stats, file: &str, project: &srd_editor::scene::Project) {
    stats.scenes += project.scenes.len();

    for (scene_index, scene) in project.scenes.iter().enumerate() {
        stats
            .scene_set_counts
            .push(scene.animation_sets.len() as i64);
        stats.total_anms += scene.animation_sets.len();

        let mut anms_names: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
        for (set_index, set) in scene.animation_sets.iter().enumerate() {
            anms_names
                .entry(set.name.clone())
                .or_default()
                .push(set_index);
            if set.name.is_empty() {
                stats.anms_empty_names.push(format!(
                    "file={file} scene[{scene_index}]={} ANMS[{set_index}] name=<EMPTY>",
                    display_name(&scene.name)
                ));
            }

            let slots_len = set.slots.len();
            let layers_len = scene.layers.len();
            match slots_len.cmp(&layers_len) {
                std::cmp::Ordering::Less => stats.set_slot_less += 1,
                std::cmp::Ordering::Equal => stats.set_slot_equal += 1,
                std::cmp::Ordering::Greater => stats.set_slot_greater += 1,
            }
            if slots_len != layers_len {
                stats.set_length_mismatches.push(format!(
                    "file={file} scene[{scene_index}]={} ANMS[{set_index}]={} slots.len()={} layers.len()={}",
                    display_name(&scene.name),
                    display_name(&set.name),
                    slots_len,
                    layers_len
                ));
            }

            stats.anms_starts.push(set.start_frame as i64);
            stats
                .anms_runtime_durations
                .push(set.runtime_duration as i64);
            if set.runtime_duration <= set.start_frame {
                stats.anms_runtime_le_start += 1;
            }
            if set.start_frame != 0 {
                stats.anms_start_nonzero += 1;
            }

            for (slot_index, slot) in set.slots.iter().enumerate() {
                stats.anms_slot_total += 1;
                let enabled = slot.is_enabled();
                let empty = slot.animation_name.is_empty();
                match (enabled, empty) {
                    (true, true) => stats.enabled_empty += 1,
                    (true, false) => stats.enabled_named += 1,
                    (false, true) => stats.disabled_empty += 1,
                    (false, false) => stats.disabled_named += 1,
                }
                if enabled {
                    stats.anms_enabled += 1;
                } else {
                    stats.anms_disabled += 1;
                }
                if empty {
                    stats.anms_empty_animation_name += 1;
                }

                if !empty {
                    match scene.layers.get(slot_index) {
                        Some(layer) => match layer.find_animation(&slot.animation_name) {
                            Some((animation_index, _)) => {
                                stats.referenced_animation_keys.insert(AnimationKey {
                                    file: file.to_owned(),
                                    scene_index,
                                    layer_index: slot_index,
                                    animation_index,
                                });
                            }
                            None => stats.dangling_slots.push(format!(
                                "file={file} scene[{scene_index}]={} ANMS[{set_index}]={} slot[{slot_index}] name={}",
                                display_name(&scene.name),
                                display_name(&set.name),
                                display_name(&slot.animation_name)
                            )),
                        },
                        None => stats.dangling_slots.push(format!(
                            "file={file} scene[{scene_index}]={} ANMS[{set_index}]={} slot[{slot_index}] name={} (no layer at slot index)",
                            display_name(&scene.name),
                            display_name(&set.name),
                            display_name(&slot.animation_name)
                        )),
                    }
                }
            }
        }
        for (name, indexes) in anms_names {
            if indexes.len() > 1 {
                stats.anms_duplicate_name_groups.push(format!(
                    "file={file} scene[{scene_index}]={} name={} ANMS indexes={:?} count={}",
                    display_name(&scene.name),
                    display_name(&name),
                    indexes,
                    indexes.len()
                ));
            }
        }

        stats.total_layers += scene.layers.len();
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            stats
                .layer_animation_counts
                .push(layer.animations.len() as i64);
            if layer.animations.is_empty() {
                stats.layers_without_animations += 1;
            }

            let mut animation_names: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
            for (animation_index, animation) in layer.animations.iter().enumerate() {
                stats.total_animations += 1;
                stats.animation_keys.push(AnimationKey {
                    file: file.to_owned(),
                    scene_index,
                    layer_index,
                    animation_index,
                });
                animation_names
                    .entry(animation.name.clone())
                    .or_default()
                    .push(animation_index);
                let animation_detail = format!(
                    "file={file} scene[{scene_index}]={} layer[{layer_index}]={} ANIM[{animation_index}]={}",
                    display_name(&scene.name),
                    display_name(&layer.name),
                    display_name(&animation.name)
                );

                if animation.name.is_empty() {
                    stats
                        .anim_empty_names
                        .push(format!("{animation_detail} name=<EMPTY>"));
                }
                if animation.duration < 0 {
                    stats.anim_negative_duration += 1;
                }
                let runtime_duration = animation.runtime_duration();
                stats.anim_runtime_durations.push(runtime_duration as i64);
                stats
                    .anim_motion_counts
                    .push(animation.motions.len() as i64);

                let mut distinct_targets = BTreeSet::new();
                let mut pair_frames: BTreeMap<i32, BTreeSet<i32>> = BTreeMap::new();
                let mut outside_for_animation = 0usize;
                for (motion_index, motion) in animation.motions.iter().enumerate() {
                    stats.motion_track_counts.push(motion.tracks.len() as i64);
                    let target_negative = motion.target < 0;
                    let target_high =
                        motion.target >= 0 && (motion.target as usize) >= layer.nodes.len();
                    let valid_target = !target_negative && !target_high;
                    if target_negative {
                        stats.negative_target_motions += 1;
                        *stats
                            .negative_target_values
                            .entry(motion.target)
                            .or_default() += 1;
                    }
                    if target_high {
                        stats.high_target_motions += 1;
                    }
                    if target_negative || target_high {
                        if motion.tracks.is_empty() {
                            stats.out_of_range_zero_track_motions += 1;
                        } else {
                            stats.out_of_range_nonzero_track_motions += 1;
                            stats.out_of_range_nonzero_track_details.push(format!(
                                "{animation_detail} MOT[{motion_index}] target={} nodes.len()={} tracks.len()={}",
                                motion.target,
                                layer.nodes.len(),
                                motion.tracks.len()
                            ));
                        }
                    } else if motion.tracks.is_empty() {
                        stats.zero_track_valid_target_motions += 1;
                    }
                    if valid_target {
                        let node = motion.target as usize;
                        distinct_targets.insert(node);
                        pair_frames.entry(motion.target).or_default();
                    } else {
                        stats.out_of_range_motion_targets.push(format!(
                            "{animation_detail} MOT[{motion_index}] target={} nodes.len()={}",
                            motion.target,
                            layer.nodes.len()
                        ));
                    }

                    for (track_index, track) in motion.tracks.iter().enumerate() {
                        *stats.channel_counts.entry(track.target).or_default() += 1;
                        *stats
                            .channel_key_data_counts
                            .entry(track.target)
                            .or_default()
                            .entry(key_data_variant_index(&track.keys))
                            .or_default() += 1;
                        *stats
                            .channel_format_counts
                            .entry(track.target)
                            .or_default()
                            .entry(track.format)
                            .or_default() += 1;
                        stats.track_key_counts.push(track.key_count as i64);
                        let track_detail = format!(
                            "{animation_detail} node={} track[{track_index}] channel={} key_count={}",
                            motion.target, track.target, track.key_count
                        );
                        let track_record = TrackRecord {
                            key_count: track.key_count as usize,
                            detail: track_detail,
                        };
                        let replace_largest = match &stats.largest_track {
                            None => true,
                            Some(current) => {
                                track_record.key_count > current.key_count
                                    || (track_record.key_count == current.key_count
                                        && track_record.detail < current.detail)
                            }
                        };
                        if replace_largest {
                            stats.largest_track = Some(track_record);
                        }

                        for frame in track_frames(track) {
                            if stats.widest_frame.is_none_or(|current| frame > current) {
                                stats.widest_frame = Some(frame);
                            }
                            if frame < 0 || (frame as f32) > runtime_duration {
                                outside_for_animation += 1;
                            }
                            if valid_target {
                                pair_frames.entry(motion.target).or_default().insert(frame);
                            }
                        }
                    }
                }
                stats
                    .anim_distinct_node_counts
                    .push(distinct_targets.len() as i64);
                stats.node_outliers.push(NodeOutlier {
                    count: distinct_targets.len(),
                    detail: format!(
                        "file={file} scene[{scene_index}]={} layer[{layer_index}]={} ANIM[{animation_index}]={} node_count={}",
                        display_name(&scene.name),
                        display_name(&layer.name),
                        display_name(&animation.name),
                        distinct_targets.len()
                    ),
                });
                stats
                    .outside_frame_counts
                    .push(outside_for_animation as i64);
                if outside_for_animation > 0 {
                    stats.animations_with_outside_frames += 1;
                    stats.outside_frame_total += outside_for_animation;
                    stats.outside_frame_records.push(OutsideFrameRecord {
                        count: outside_for_animation,
                        detail: format!(
                            "{animation_detail} runtime_duration={} outside_keyframes={}",
                            runtime_duration as i64, outside_for_animation
                        ),
                    });
                }
                for (node, frames) in pair_frames {
                    let frame_count = frames.len();
                    stats.frame_pair_counts.push(frame_count as i64);
                    let detail = format!(
                        "{animation_detail} node={} distinct_frame_values={}",
                        node, frame_count
                    );
                    let replace_worst = match &stats.worst_frame_pair {
                        None => true,
                        Some((current_count, current_detail)) => {
                            frame_count > *current_count
                                || (frame_count == *current_count && detail < *current_detail)
                        }
                    };
                    if replace_worst {
                        stats.worst_frame_pair = Some((frame_count, detail));
                    }
                }
            }
            for (name, indexes) in animation_names {
                if indexes.len() > 1 {
                    stats.anim_duplicate_name_groups.push(format!(
                        "file={file} scene[{scene_index}]={} layer[{layer_index}]={} name={} ANIM indexes={:?} count={}",
                        display_name(&scene.name),
                        display_name(&layer.name),
                        display_name(&name),
                        indexes,
                        indexes.len()
                    ));
                }
            }
        }
    }
}

const KEY_DATA_VARIANT_NAMES: [&str; 6] = [
    "Key8F32",
    "Key8I32",
    "Key8Bytes4",
    "Key20F32",
    "Key20I32",
    "Unsupported",
];

fn key_data_variant_index(data: &KeyData) -> usize {
    match data {
        KeyData::Key8F32(_) => 0,
        KeyData::Key8I32(_) => 1,
        KeyData::Key8Bytes4(_) => 2,
        KeyData::Key20F32(_) => 3,
        KeyData::Key20I32(_) => 4,
        KeyData::Unsupported => 5,
    }
}

fn track_frames(track: &Track) -> Vec<i32> {
    match &track.keys {
        KeyData::Key8F32(keys) => keys.iter().map(|key| key.frame).collect(),
        KeyData::Key8I32(keys) => keys.iter().map(|key| key.frame).collect(),
        KeyData::Key8Bytes4(keys) => keys.iter().map(|key| key.frame).collect(),
        KeyData::Key20F32(keys) => keys.iter().map(|key| key.frame).collect(),
        KeyData::Key20I32(keys) => keys.iter().map(|key| key.frame).collect(),
        KeyData::Unsupported => Vec::new(),
    }
}

fn display_name(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "<EMPTY>".to_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn histogram(values: &[i64]) -> BTreeMap<i64, usize> {
    let mut result = BTreeMap::new();
    for value in values {
        *result.entry(*value).or_default() += 1;
    }
    result
}

fn median(values: &[i64]) -> String {
    if values.is_empty() {
        return "n/a".to_owned();
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[middle].to_string()
    } else {
        let sum = sorted[middle - 1] + sorted[middle];
        if sum % 2 == 0 {
            (sum / 2).to_string()
        } else if sum >= 0 {
            format!("{}.5", sum / 2)
        } else {
            format!("-{}.5", (-sum) / 2)
        }
    }
}

fn modes(values: &[i64]) -> String {
    let counts = histogram(values);
    let Some(max_count) = counts.values().copied().max() else {
        return "n/a".to_owned();
    };
    counts
        .into_iter()
        .filter(|(_, count)| *count == max_count)
        .map(|(value, count)| format!("{value} (count={count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_histogram(label: &str, values: &[i64]) {
    println!("{label} (value -> count):");
    let counts = histogram(values);
    if counts.is_empty() {
        println!("  none");
    } else {
        for (value, count) in counts {
            println!("  {value} -> {count}");
        }
    }
}

fn print_summary(label: &str, values: &[i64]) {
    if values.is_empty() {
        println!("{label}: count=0 min=n/a max=n/a median=n/a most_common=n/a");
        return;
    }
    let (min, max) = values
        .iter()
        .copied()
        .fold((i64::MAX, i64::MIN), |(min, max), value| {
            (min.min(value), max.max(value))
        });
    println!(
        "{label}: count={} min={} max={} median={} most_common={}",
        values.len(),
        min,
        max,
        median(values),
        modes(values)
    );
}

fn print_list(label: &str, values: &[String]) {
    println!("{label}: count={}", values.len());
    if values.is_empty() {
        println!("  none");
        return;
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    for value in sorted.iter().take(MAX_LIST_ITEMS) {
        println!("  {value}");
    }
    if sorted.len() > MAX_LIST_ITEMS {
        println!(
            "  ... {} more truncated (showing first {})",
            sorted.len() - MAX_LIST_ITEMS,
            MAX_LIST_ITEMS
        );
    }
}

fn editor_channel_name(target: u16) -> Option<&'static str> {
    match target {
        0 => Some("Position · X"),
        1 => Some("Position · Y"),
        2 => Some("Position · Z"),
        3 => Some("Rotation · X"),
        4 => Some("Rotation · Y"),
        5 => Some("Rotation · Z"),
        6 => Some("Scale · X"),
        7 => Some("Scale · Y"),
        8 => Some("Scale · Z"),
        9 => Some("Multiply color"),
        10 => Some("Visibility"),
        19 => Some("Additive color"),
        21 => Some("Multiply Alpha"),
        22 => Some("Additive Alpha"),
        _ => None,
    }
}

const CHANNEL_MEANINGS: [&str; 24] = [
    "Position X",
    "Position Y",
    "Position Z",
    "Rotation X",
    "Rotation Y",
    "Rotation Z",
    "Scale X",
    "Scale Y",
    "Scale Z",
    "Multiplicative color RGB",
    "Visibility word",
    "Image width (geometry size[0])",
    "Image height (geometry size[1])",
    "CREF vertex color vertex 0",
    "CREF vertex color vertex 2",
    "CREF vertex color vertex 1",
    "CREF vertex color vertex 3",
    "CREF reference selector / explicit image and rectangle",
    "Unestablished; no runtime state consumer found",
    "Additive color RGB",
    "CRE1 reference selector / explicit image and rectangle",
    "Multiplicative opacity (color alpha)",
    "Additive opacity (color alpha)",
    "Reference animation frame request (CRFD / RefCast)",
];

const CHANNEL_EVIDENCE: [&str; 24] = [
    "srd-editor/src/transform.rs:55: `0..=2 => self.translation[usize::from(target)] = ...`",
    "srd-editor/src/transform.rs:55: `0..=2 => self.translation[usize::from(target)] = ...`",
    "srd-editor/src/transform.rs:55: `0..=2 => self.translation[usize::from(target)] = ...`",
    "srd-editor/src/transform.rs:56: `3..=5 => self.rotation[usize::from(target - 3)] = ...`",
    "srd-editor/src/transform.rs:56: `3..=5 => self.rotation[usize::from(target - 3)] = ...`",
    "srd-editor/src/transform.rs:56: `3..=5 => self.rotation[usize::from(target - 3)] = ...`",
    "srd-editor/src/transform.rs:57: `6..=8 => self.scale[usize::from(target - 6)] = ...`",
    "srd-editor/src/transform.rs:57: `6..=8 => self.scale[usize::from(target - 6)] = ...`",
    "srd-editor/src/transform.rs:57: `6..=8 => self.scale[usize::from(target - 6)] = ...`",
    "srd-editor/src/transform.rs:62-75: `9 | 19 => ... color[0..2]`",
    "srd-editor/src/transform.rs:58: `10 => self.visibility_word = bits`",
    "srd-editor/src/image.rs:320-332: `11 => 0`, then `state.size[component] = ...`; dispatch at :582-584",
    "srd-editor/src/image.rs:320-332: `12 => 1`, then `state.size[component] = ...`; dispatch at :582-584",
    "srd-editor/src/image.rs:558-572: `13 => 0`, then `state.vertex_colors[vertex_index] = bytes`",
    "srd-editor/src/image.rs:558-572: `14 => 2`, then `state.vertex_colors[vertex_index] = bytes`",
    "srd-editor/src/image.rs:558-572: `15 => 1`, then `state.vertex_colors[vertex_index] = bytes`",
    "srd-editor/src/image.rs:558-572: `16 => 3`, then `state.vertex_colors[vertex_index] = bytes`",
    "srd-editor/src/image.rs:589-595: `17 => self.apply_coordinate_track(ImageReferenceChannel::Cref, ...)`",
    "No channel-18 consumer in searched src/docs; parsed at srd-editor/src/animation.rs:94-132, then fallbacks at srd-editor/src/transform.rs:88-89 and srd-editor/src/image.rs:602-603; docs/evidence/image-coordinate-animation.md:22 says 18 falls into default",
    "srd-editor/src/transform.rs:62-75: `9 | 19 => ... additive_color`",
    "srd-editor/src/image.rs:596-602: `20 => self.apply_coordinate_track(ImageReferenceChannel::Cre1, ...)`",
    "srd-editor/src/transform.rs:76-85: `if target == 21 { self.multiply_color[3] = alpha }`",
    "srd-editor/src/transform.rs:76-85: `else { self.additive_color[3] = alpha }`",
    "srd-editor/src/reference.rs:72-95: `track.target != 23` gate, then ReferenceAnimationRequest; consumed at srd-editor/src/reference_runtime.rs:606-615",
];

fn channel_semantics(target: u16) -> (&'static str, &'static str, &'static str) {
    let index = usize::from(target);
    if index < CHANNEL_MEANINGS.len() {
        let status = if target == 18 {
            "PARSED-BUT-IGNORED"
        } else {
            "IMPLEMENTED"
        };
        (CHANNEL_MEANINGS[index], CHANNEL_EVIDENCE[index], status)
    } else {
        (
            "Unestablished",
            "No mapping in the searched source/docs; parser target storage is srd-editor/src/animation.rs:94",
            "PARSED-BUT-IGNORED",
        )
    }
}

fn print_report(stats: &Stats, corpus_root: &Path) {
    println!("srd-anim-stats");
    println!("corpus_root={}", corpus_root.display());
    println!(
        "files discovered={} loaded={} failed={}",
        stats.files_found,
        stats.files_loaded,
        stats.load_failures.len()
    );
    print_list("discovery errors", &stats.discovery_errors);
    print_list("load failures", &stats.load_failures);

    println!();
    println!("1. Scene / set level: animation-set counts");
    println!("scenes analyzed={}", stats.scenes);
    print_histogram("animation_sets.len() per scene", &stats.scene_set_counts);
    println!("total ANMS={}", stats.total_anms);

    println!();
    println!("2. Scene / set level: ANMS slot count versus layer count");
    println!(
        "slot/layer relation: less={} equal={} greater={} (total ANMS={})",
        stats.set_slot_less, stats.set_slot_equal, stats.set_slot_greater, stats.total_anms
    );
    print_list("slot/layer mismatches", &stats.set_length_mismatches);

    println!();
    println!("3. Scene / set level: ANMS slot enabled/named cross-tab");
    println!(
        "all ANMS slots={} enabled={} disabled={} empty animation_name={} named={}",
        stats.anms_slot_total,
        stats.anms_enabled,
        stats.anms_disabled,
        stats.anms_empty_animation_name,
        stats.anms_slot_total - stats.anms_empty_animation_name
    );
    println!("cross-tab:");
    println!("  enabled + empty={}", stats.enabled_empty);
    println!("  enabled + named={}", stats.enabled_named);
    println!("  disabled + empty={}", stats.disabled_empty);
    println!("  disabled + named={}", stats.disabled_named);

    println!();
    println!("4. Scene / set level: dangling non-empty ANMS slot names");
    print_list("dangling names", &stats.dangling_slots);

    println!();
    println!("5. Scene / set level: ANMS start_frame and runtime_duration");
    print_summary("start_frame", &stats.anms_starts);
    print_histogram("start_frame distribution", &stats.anms_starts);
    print_summary("runtime_duration", &stats.anms_runtime_durations);
    print_histogram(
        "runtime_duration distribution",
        &stats.anms_runtime_durations,
    );
    println!(
        "runtime_duration <= start_frame={} start_frame != 0={}",
        stats.anms_runtime_le_start, stats.anms_start_nonzero
    );

    println!();
    println!("6. Scene / set level: ANMS names");
    println!(
        "duplicate ANMS name groups={} empty ANMS names={}",
        stats.anms_duplicate_name_groups.len(),
        stats.anms_empty_names.len()
    );
    print_list(
        "duplicate ANMS name groups",
        &stats.anms_duplicate_name_groups,
    );
    print_list("empty ANMS names", &stats.anms_empty_names);

    println!();
    println!("7. Layer / animation level: animations per layer");
    print_histogram(
        "layer.animations.len() per layer",
        &stats.layer_animation_counts,
    );
    println!(
        "layers={} total ANIM={} layers with zero animations={}",
        stats.total_layers, stats.total_animations, stats.layers_without_animations
    );

    println!();
    println!("8. Layer / animation level: ANIM references from ANMS");
    let referenced = stats
        .animation_keys
        .iter()
        .filter(|key| stats.referenced_animation_keys.contains(*key))
        .count();
    println!(
        "ANIMs referenced by at least one ANMS slot={} never referenced by any ANMS in scene={} total ANIM={}",
        referenced,
        stats.total_animations - referenced,
        stats.total_animations
    );

    println!();
    println!("9. Layer / animation level: ANIM duration");
    println!("ANIM duration < 0={}", stats.anim_negative_duration);
    print_summary("runtime_duration", &stats.anim_runtime_durations);
    print_histogram(
        "runtime_duration distribution",
        &stats.anim_runtime_durations,
    );

    println!();
    println!("10. Layer / animation level: ANIM names");
    println!(
        "duplicate ANIM name groups={} empty ANIM names={}",
        stats.anim_duplicate_name_groups.len(),
        stats.anim_empty_names.len()
    );
    print_list(
        "duplicate ANIM name groups",
        &stats.anim_duplicate_name_groups,
    );
    print_list("empty ANIM names", &stats.anim_empty_names);

    println!();
    println!("11. Motion / track level: motions per ANIM");
    print_histogram("motions.len() per ANIM", &stats.anim_motion_counts);
    print_summary("motions.len() per ANIM", &stats.anim_motion_counts);

    println!();
    println!("12. Motion / track level: distinct animated node targets per ANIM");
    print_histogram(
        "distinct valid node targets per ANIM",
        &stats.anim_distinct_node_counts,
    );
    print_summary(
        "distinct valid node targets per ANIM",
        &stats.anim_distinct_node_counts,
    );
    let mut node_outliers = stats.node_outliers.clone();
    node_outliers.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    println!("top 10 ANIM node-count outliers:");
    if node_outliers.is_empty() {
        println!("  none");
    } else {
        for outlier in node_outliers.iter().take(10) {
            println!("  {}", outlier.detail);
        }
    }

    println!();
    println!("13. Motion / track level: out-of-range Motion.target");
    print_list(
        "out-of-range motion targets",
        &stats.out_of_range_motion_targets,
    );

    println!();
    println!("14. Motion / track level: tracks per motion and channel ids");
    print_histogram("tracks.len() per motion", &stats.motion_track_counts);
    println!("Track.target channel histogram (id -> occurrence count):");
    if stats.channel_counts.is_empty() {
        println!("  none");
    } else {
        for (target, count) in &stats.channel_counts {
            match editor_channel_name(*target) {
                Some(name) => println!("  {target} ({name}) -> {count}"),
                None => println!(
                    "  {target} (Channel {target}; UNNAMED by editor/model::animation_target_name) -> {count}"
                ),
            }
        }
    }

    println!();
    println!("15. Motion / track level: Track.key_count");
    print_histogram("Track.key_count", &stats.track_key_counts);
    print_summary("Track.key_count", &stats.track_key_counts);
    match &stats.largest_track {
        Some(track) => println!(
            "largest single track: key_count={} {}",
            track.key_count, track.detail
        ),
        None => println!("largest single track: none"),
    }

    println!();
    println!("16. Motion / track level: distinct keyframe frames per (ANIM,node)");
    print_histogram(
        "distinct frame values per (ANIM,node)",
        &stats.frame_pair_counts,
    );
    print_summary(
        "distinct frame values per (ANIM,node)",
        &stats.frame_pair_counts,
    );
    match &stats.worst_frame_pair {
        Some((count, detail)) => println!("worst (ANIM,node) pair: count={} {}", count, detail),
        None => println!("worst (ANIM,node) pair: none"),
    }

    println!();
    println!("17. Motion / track level: keyframe frame extent and runtime bounds");
    match stats.widest_frame {
        Some(frame) => println!("maximum keyframe frame value seen={frame}"),
        None => println!("maximum keyframe frame value seen=n/a"),
    }
    println!(
        "keyframes outside [0, runtime_duration()] total={} ANIMs with at least one outside keyframe={}",
        stats.outside_frame_total, stats.animations_with_outside_frames
    );
    print_histogram(
        "outside keyframes per ANIM (all ANIMs)",
        &stats.outside_frame_counts,
    );
    print_outside_frame_records(&stats.outside_frame_records);
    println!();
    println!("18. Follow-up: Motion.target versus zero-track placeholders");
    let out_of_range_total = stats.negative_target_motions + stats.high_target_motions;
    println!(
        "motions with target < 0={} target >= nodes.len()={} out-of-range total={}",
        stats.negative_target_motions, stats.high_target_motions, out_of_range_total
    );
    println!("negative target values:");
    for (target, count) in &stats.negative_target_values {
        println!("  {target} -> {count}");
    }
    println!(
        "out-of-range motions: tracks.len()==0={} tracks.len()>=1={}",
        stats.out_of_range_zero_track_motions, stats.out_of_range_nonzero_track_motions
    );
    println!(
        "zero-track motions with valid in-range target={}",
        stats.zero_track_valid_target_motions
    );
    print_list(
        "out-of-range motions carrying at least one track",
        &stats.out_of_range_nonzero_track_details,
    );
    println!();
    println!("19. Follow-up: channel semantics and runtime consumers");
    println!(
        "search scope: srd-editor/src/** and srd-editor/docs/**; targeted consumers include animation.rs, transform.rs, image.rs, reference.rs, reference_runtime.rs, and editor/model.rs"
    );
    for target in 0u16..=23 {
        let count = stats.channel_counts.get(&target).copied().unwrap_or(0);
        let editor_name = editor_channel_name(target).unwrap_or("UNNAMED");
        let (meaning, evidence, status) = channel_semantics(target);
        println!(
            "channel {target}: occurrences={count} editor_name={editor_name} meaning={meaning} status={status} evidence={evidence}"
        );
    }
    println!();
    println!("20. Follow-up: per-channel KeyData variants and Track.format values");
    for target in 0u16..=23 {
        let track_count = stats.channel_counts.get(&target).copied().unwrap_or(0);
        println!("channel {target}: tracks={track_count}");
        print!("  KeyData variants:");
        let counts = stats.channel_key_data_counts.get(&target);
        for (variant_index, variant_name) in KEY_DATA_VARIANT_NAMES.iter().enumerate() {
            let count = counts
                .and_then(|counts| counts.get(&variant_index))
                .copied()
                .unwrap_or(0);
            print!(" {variant_name}={count}");
        }
        println!();
        print!("  Track.format values:");
        match stats.channel_format_counts.get(&target) {
            Some(counts) if !counts.is_empty() => {
                for (format, count) in counts {
                    print!(" 0x{format:X}={count}");
                }
                println!();
            }
            _ => println!(" none"),
        }
    }
}

fn print_outside_frame_records(records: &[OutsideFrameRecord]) {
    println!("per-ANIM outside-keyframe counts: count={}", records.len());
    if records.is_empty() {
        println!("  none");
        return;
    }
    let mut sorted = records.to_vec();
    sorted.sort_by(|left, right| {
        left.detail
            .cmp(&right.detail)
            .then_with(|| left.count.cmp(&right.count))
    });
    for record in sorted.iter().take(MAX_LIST_ITEMS) {
        println!("  {}", record.detail);
    }
    if sorted.len() > MAX_LIST_ITEMS {
        println!(
            "  ... {} more truncated (showing first {})",
            sorted.len() - MAX_LIST_ITEMS,
            MAX_LIST_ITEMS
        );
    }
}
