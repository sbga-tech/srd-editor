use crate::animation::{AnimationDefinition, KeyData, Motion, Track};
use crate::scene::{AnimationSetDefinition, Layer, Project, Scene, SceneAnimationSlot};
use crate::vtbf::{OwnedBlock, OwnedProperty, OwnedSrdFile};

/// Recomputes parser-derived animation counts from the authoring vectors.
///
/// The project model retains these wire fields so it exactly represents a
/// parsed file. They are redundant when authoring, however, and must not make
/// a direct `EditorDocument.project` mutation unsaveable.
pub fn normalize_project_animation_counts(project: &mut Project) -> Result<(), String> {
    for (scene_index, scene) in project.scenes.iter_mut().enumerate() {
        scene.declared_animation_set_count = checked_u32(
            scene.animation_sets.len(),
            &format!("scene[{scene_index}] animation-set count"),
        )?;

        for (set_index, set) in scene.animation_sets.iter_mut().enumerate() {
            set.declared_slot_count = checked_i32(
                set.slots.len(),
                &format!("scene[{scene_index}] ANMS[{set_index}] slot count"),
            )?;
        }

        for (layer_index, layer) in scene.layers.iter_mut().enumerate() {
            layer.animation_count = checked_u32(
                layer.animations.len(),
                &format!("scene[{scene_index}] layer[{layer_index}] animation count"),
            )?;
            for (animation_index, animation) in layer.animations.iter_mut().enumerate() {
                animation.declared_motion_count = checked_u32(
                    animation.motions.len(),
                    &format!(
                        "scene[{scene_index}] layer[{layer_index}] ANIM[{animation_index}] motion count"
                    ),
                )?;
                for (motion_index, motion) in animation.motions.iter_mut().enumerate() {
                    for (track_index, track) in motion.tracks.iter_mut().enumerate() {
                        if let Some(key_count) = key_data_len(&track.keys) {
                            track.key_count = checked_u16(
                                key_count,
                                &format!(
                                    "scene[{scene_index}] layer[{layer_index}] ANIM[{animation_index}] MOT[{motion_index}] TRK[{track_index}] key count"
                                ),
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validates animation values newly authored relative to the parsed source.
/// Existing loadable records are grandfathered when they survive unchanged.
pub fn validate_animation_authoring(
    source: &Project,
    project: &Project,
    scene_origins: &[Option<usize>],
) -> Result<(), String> {
    if scene_origins.len() != project.scenes.len() {
        return Err("scene provenance does not match the edited scene count".into());
    }
    for (scene_index, scene) in project.scenes.iter().enumerate() {
        checked_u16(
            scene.animation_sets.len(),
            &format!("scene[{scene_index}] SCN ANMS count"),
        )?;
        let source_scene =
            scene_origins[scene_index].and_then(|source_index| source.scenes.get(source_index));
        let animation_set_sources = source_scene.map_or_else(Vec::new, |source_scene| {
            record_matches(
                &source_scene.animation_sets,
                &scene.animation_sets,
                |source, edited| source == edited,
            )
        });
        for (set_index, set) in scene.animation_sets.iter().enumerate() {
            let source_set = animation_set_sources.get(set_index).and_then(|index| {
                index.and_then(|index| source_scene.map(|scene| &scene.animation_sets[index]))
            });
            let set_context = format!("scene[{scene_index}] ANMS[{set_index}]");
            if source_set.is_some_and(|source_set| source_set == set) {
                for (slot_index, slot) in set.slots.iter().enumerate() {
                    if !slot.animation_name.is_empty()
                        && layer_animations_changed(source_scene, scene, slot_index)
                    {
                        validate_animation_slot_reference(scene, &set_context, slot_index, slot)?;
                    }
                }
                continue;
            }
            checked_i32(
                set.slots.len(),
                &format!("scene[{scene_index}] ANMS[{set_index}] SANM count"),
            )?;
            validate_name(&set.name, false, &format!("{set_context} name"))?;
            if scene
                .animation_sets
                .iter()
                .enumerate()
                .any(|(other_index, other)| other_index != set_index && other.name == set.name)
            {
                return Err(format!(
                    "{set_context} duplicates ANMS name {} within scene[{scene_index}]",
                    display_name(&set.name)
                ));
            }
            if set.slots.len() != scene.layers.len() {
                return Err(format!(
                    "{set_context} has {} SANM slots, but scene[{scene_index}] has {} layers",
                    set.slots.len(),
                    scene.layers.len()
                ));
            }
            for (slot_index, slot) in set.slots.iter().enumerate() {
                validate_name(
                    &slot.animation_name,
                    true,
                    &format!("{set_context} SANM[{slot_index}] animation name"),
                )?;
                if !(0..=u8::MAX as i32).contains(&slot.enabled) {
                    return Err(format!(
                        "{set_context} SANM[{slot_index}] enabled value {} does not fit the audited type-1 property",
                        slot.enabled
                    ));
                }
                if slot.animation_name.is_empty() {
                    continue;
                }
                let layer = scene.layers.get(slot_index).ok_or_else(|| {
                    format!(
                        "{set_context} SANM[{slot_index}] names {} but has no corresponding layer",
                        display_name(&slot.animation_name)
                    )
                })?;
                if !layer
                    .animations
                    .iter()
                    .any(|animation| animation.name == slot.animation_name)
                {
                    return Err(format!(
                        "{set_context} SANM[{slot_index}] names {} which does not resolve in layer[{slot_index}] {}",
                        display_name(&slot.animation_name),
                        display_name(&layer.name)
                    ));
                }
            }
        }

        for (layer_index, layer) in scene.layers.iter().enumerate() {
            validate_layer_authoring(
                scene_index,
                layer_index,
                source_scene.and_then(|source_scene| source_scene.layers.get(layer_index)),
                layer,
            )?;
        }
    }
    Ok(())
}

fn layer_animations_changed(source: Option<&Scene>, edited: &Scene, layer_index: usize) -> bool {
    match (
        source.and_then(|scene| scene.layers.get(layer_index)),
        edited.layers.get(layer_index),
    ) {
        (Some(source), Some(edited)) => {
            !animation_lists_equal(&source.animations, &edited.animations)
        }
        (None, None) => false,
        _ => true,
    }
}

fn validate_animation_slot_reference(
    scene: &Scene,
    set_context: &str,
    slot_index: usize,
    slot: &SceneAnimationSlot,
) -> Result<(), String> {
    let layer = scene.layers.get(slot_index).ok_or_else(|| {
        format!(
            "{set_context} SANM[{slot_index}] names {} but has no corresponding layer",
            display_name(&slot.animation_name)
        )
    })?;
    if !layer
        .animations
        .iter()
        .any(|animation| animation.name == slot.animation_name)
    {
        return Err(format!(
            "{set_context} SANM[{slot_index}] names {} which does not resolve in layer[{slot_index}] {}",
            display_name(&slot.animation_name),
            display_name(&layer.name)
        ));
    }
    Ok(())
}

fn validate_layer_authoring(
    scene_index: usize,
    layer_index: usize,
    source: Option<&Layer>,
    layer: &Layer,
) -> Result<(), String> {
    checked_u16(
        layer.animations.len(),
        &format!("scene[{scene_index}] layer[{layer_index}] ANIM count"),
    )?;
    let animation_sources = source.map_or_else(Vec::new, |source| {
        record_matches(
            &source.animations,
            &layer.animations,
            animation_definition_equal,
        )
    });
    for (animation_index, animation) in layer.animations.iter().enumerate() {
        let source_animation = animation_sources.get(animation_index).and_then(|index| {
            index.and_then(|index| source.map(|source| &source.animations[index]))
        });
        if source_animation
            .is_some_and(|source_animation| animation_definition_equal(source_animation, animation))
        {
            continue;
        }
        let animation_context =
            format!("scene[{scene_index}] layer[{layer_index}] ANIM[{animation_index}]");
        validate_name(&animation.name, false, &format!("{animation_context} name"))?;
        if layer
            .animations
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                other_index != animation_index && other.name == animation.name
            })
        {
            return Err(format!(
                "{animation_context} duplicates ANIM name {} within layer[{layer_index}] {}",
                display_name(&animation.name),
                display_name(&layer.name)
            ));
        }
        if i16::try_from(animation.duration).is_err() {
            return Err(format!(
                "{animation_context} duration {} does not fit the audited type-5 property",
                animation.duration
            ));
        }
        checked_u16(
            animation.motions.len(),
            &format!("{animation_context} MOT count"),
        )?;
        let motion_sources = source_animation.map_or_else(Vec::new, |source_animation| {
            record_matches(&source_animation.motions, &animation.motions, motion_equal)
        });
        for (motion_index, motion) in animation.motions.iter().enumerate() {
            let motion_context = format!("{animation_context} MOT[{motion_index}]");
            if i16::try_from(motion.target).is_err() {
                return Err(format!(
                    "{motion_context} target {} does not fit the audited type-5 property",
                    motion.target
                ));
            }
            let source_motion = motion_sources.get(motion_index).and_then(|index| {
                index.and_then(|index| source_animation.map(|source| &source.motions[index]))
            });
            if !is_valid_authored_motion_target(motion, layer.nodes.len())
                && !motion_target_is_grandfathered(source_motion, motion)
            {
                return Err(format!(
                    "{motion_context} target {} must be -1 with zero tracks or name a CAST in 0..{}",
                    motion.target,
                    layer.nodes.len()
                ));
            }
            if motion.tracks.len() > u16::MAX as usize {
                return Err(format!(
                    "{motion_context} has {} tracks, exceeding the audited u16 count",
                    motion.tracks.len()
                ));
            }
            for (track_index, track) in motion.tracks.iter().enumerate() {
                let track_context = format!("{motion_context} TRK[{track_index}]");
                match &track.keys {
                    KeyData::Unsupported => {}
                    keys => {
                        let count = key_data_len(keys).expect("supported KeyData has a key vector");
                        if count > u16::MAX as usize {
                            return Err(format!(
                                "{track_context} has {count} keys, exceeding the audited u16 count"
                            ));
                        }
                        if !format_supports_key_data(track.format, keys) {
                            return Err(format!(
                                "{track_context} format {:#010x} does not encode its {} keys",
                                track.format,
                                key_kind_name(keys)
                            ));
                        }
                        let source_has_same_keys = key_data_is_grandfathered(source, layer, keys);
                        validate_key_values(keys, source_has_same_keys, &track_context)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn is_valid_authored_motion_target(motion: &Motion, node_count: usize) -> bool {
    (motion.target == -1 && motion.tracks.is_empty())
        || (motion.target >= 0
            && usize::try_from(motion.target).is_ok_and(|target| target < node_count))
}

fn motion_target_is_grandfathered(source: Option<&Motion>, edited: &Motion) -> bool {
    source.is_some_and(|source| {
        if edited.target == -1 {
            motion_equal(source, edited)
        } else {
            source.target == edited.target
        }
    })
}

fn key_data_is_grandfathered(source: Option<&Layer>, edited_layer: &Layer, keys: &KeyData) -> bool {
    let Some(source) = source else {
        return false;
    };
    let source_count = source
        .animations
        .iter()
        .flat_map(|animation| &animation.motions)
        .flat_map(|motion| &motion.tracks)
        .filter(|track| key_data_equal(&track.keys, keys))
        .count();
    source_count != 0
        && edited_layer
            .animations
            .iter()
            .flat_map(|animation| &animation.motions)
            .flat_map(|motion| &motion.tracks)
            .filter(|track| key_data_equal(&track.keys, keys))
            .count()
            <= source_count
}

fn validate_key_values(
    keys: &KeyData,
    source_has_same_keys: bool,
    context: &str,
) -> Result<(), String> {
    if key_data_has_non_finite(keys) && !source_has_same_keys {
        return Err(format!(
            "{context} cannot author non-finite KEY values or slopes"
        ));
    }
    match keys {
        KeyData::Key20F32(values) => {
            for (key_index, key) in values.iter().enumerate() {
                if u16::try_from(key.mode).is_err() {
                    return Err(format!(
                        "{context} KEY[{key_index}] mode {} does not fit the audited type-6 property",
                        key.mode
                    ));
                }
            }
        }
        KeyData::Key20I32(values) => {
            for (key_index, key) in values.iter().enumerate() {
                if u16::try_from(key.mode).is_err() {
                    return Err(format!(
                        "{context} KEY[{key_index}] mode {} does not fit the audited type-6 property",
                        key.mode
                    ));
                }
            }
        }
        KeyData::Key8F32(_) | KeyData::Key8I32(_) | KeyData::Key8Bytes4(_) => {}
        KeyData::Unsupported => return Err(format!("{context} has unsupported KEY data")),
    }
    Ok(())
}

/// Reconciles SCN/ANMS/SANM and LAYR/ANIM/MOT/TRK/KEY portions of an owned
/// tree. Retained SCN blocks keep their opaque children and properties; newly
/// authored scenes use the audited empty-scene schema.
pub fn reconcile_project_animations(
    file: &mut OwnedSrdFile,
    source: &Project,
    edited: &Project,
    scene_origins: &[Option<usize>],
) -> Result<(), String> {
    if scene_origins.len() != edited.scenes.len() {
        return Err("scene provenance does not match the edited scene count".into());
    }
    let mut seen = vec![false; source.scenes.len()];
    for source_index in scene_origins.iter().flatten().copied() {
        let Some(used) = seen.get_mut(source_index) else {
            return Err(format!(
                "scene provenance references missing source SCN {source_index}"
            ));
        };
        if std::mem::replace(used, true) {
            return Err(format!("scene provenance reuses source SCN {source_index}"));
        }
    }

    let project = owned_project_block_mut(file)?;
    let context = "PROJ scene reconciliation";
    if source.declared_scene_count != edited.declared_scene_count {
        set_existing_u32_property(project, 0x00, edited.declared_scene_count, context)?;
    }
    let source_blocks = tagged_child_blocks(project, b"SCN ");
    if source_blocks.len() != source.scenes.len() {
        return Err(format!(
            "owned PROJ has {} SCN children but the source project has {} scenes",
            source_blocks.len(),
            source.scenes.len()
        ));
    }
    let mut desired = Vec::with_capacity(edited.scenes.len());
    for (edited_index, (scene, origin)) in edited
        .scenes
        .iter()
        .zip(scene_origins.iter().copied())
        .enumerate()
    {
        if let Some(source_index) = origin {
            let source_scene = &source.scenes[source_index];
            let mut block = source_blocks[source_index].clone();
            if source_scene.name != scene.name {
                set_existing_property(
                    &mut block,
                    string_property(0x03, &scene.name)?,
                    &format!("scene[{edited_index}] SCN"),
                )?;
            }
            reconcile_scene(&mut block, edited_index, source_scene, scene)?;
            desired.push(block);
        } else {
            desired.push(new_scene(scene)?);
        }
    }
    let insertion = project.children.len();
    replace_tagged_children(project, b"SCN ", desired, scene_origins, insertion);
    Ok(())
}

fn new_scene(scene: &Scene) -> Result<OwnedBlock, String> {
    let mut block = OwnedBlock::new(*b"SCN ");
    block.properties = vec![
        string_property(0x03, &scene.name)?,
        OwnedProperty::u16(0x10, checked_u16(scene.layers.len(), "new SCN LAYR count")?),
        OwnedProperty::u16(
            0x17,
            checked_u16(scene.animation_sets.len(), "new SCN ANMS count")?,
        ),
        OwnedProperty::f32(0x40, scene.width),
        OwnedProperty::f32(0x41, scene.height),
    ];
    block.children = scene
        .animation_sets
        .iter()
        .map(new_animation_set)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block)
}

fn reconcile_scene(
    block: &mut OwnedBlock,
    scene_index: usize,
    source: &Scene,
    edited: &Scene,
) -> Result<(), String> {
    if block.tag != *b"SCN " {
        return Err(format!("scene[{scene_index}] tree block is not SCN "));
    }
    let layers = child_indices(block, b"LAYR");
    if layers.len() != source.layers.len() {
        return Err(format!(
            "scene[{scene_index}] tree has {} LAYR children but the source project has {}",
            layers.len(),
            source.layers.len()
        ));
    }
    for (layer_index, block_index) in layers.into_iter().enumerate() {
        reconcile_layer_animations(
            &mut block.children[block_index],
            scene_index,
            layer_index,
            &source.layers[layer_index],
            &edited.layers[layer_index],
        )?;
    }
    if source.animation_sets == edited.animation_sets {
        return Ok(());
    }
    let context = format!("scene[{scene_index}] SCN");
    if source.animation_sets.len() != edited.animation_sets.len() {
        set_existing_property(
            block,
            OwnedProperty::u16(
                0x17,
                checked_u16(edited.animation_sets.len(), "SCN ANMS count")?,
            ),
            &context,
        )?;
    }
    let insertion = after_last_child_with_tag(block, b"LAYR");
    reconcile_tagged_records(
        block,
        b"ANMS",
        &source.animation_sets,
        &edited.animation_sets,
        |source, edited| source == edited,
        |block, index, source, edited| {
            reconcile_animation_set(block, scene_index, index, source, edited)
        },
        new_animation_set,
        insertion,
        &context,
    )
}

fn reconcile_animation_set(
    block: &mut OwnedBlock,
    scene_index: usize,
    animation_set_index: usize,
    source: &AnimationSetDefinition,
    edited: &AnimationSetDefinition,
) -> Result<(), String> {
    if block.tag != *b"ANMS" {
        return Err(format!(
            "scene[{scene_index}] ANMS[{animation_set_index}] tree block is not ANMS"
        ));
    }
    if source == edited {
        return Ok(());
    }
    let context = format!("scene[{scene_index}] ANMS[{animation_set_index}]");
    if source.name != edited.name {
        set_or_insert_property(block, string_property(0x03, &edited.name)?, &context)?;
    }
    if source.start_frame != edited.start_frame {
        set_or_insert_property(
            block,
            OwnedProperty::i32(0x18, edited.start_frame),
            &context,
        )?;
    }
    if source.runtime_duration != edited.runtime_duration {
        set_or_insert_property(
            block,
            OwnedProperty::i32(0x19, edited.runtime_duration),
            &context,
        )?;
    }
    if source.slots.len() != edited.slots.len() {
        set_or_insert_property(
            block,
            OwnedProperty::i32(
                0x0e,
                checked_i32(edited.slots.len(), &format!("{context} SANM count"))?,
            ),
            &context,
        )?;
    }
    let insertion = block.children.len();
    reconcile_tagged_records(
        block,
        b"SANM",
        &source.slots,
        &edited.slots,
        |source, edited| source == edited,
        |block, index, source, edited| {
            reconcile_scene_animation_slot(
                block,
                &format!("{context} SANM[{index}]"),
                source,
                edited,
            )
        },
        new_scene_animation_slot,
        insertion,
        &context,
    )
}

fn reconcile_scene_animation_slot(
    block: &mut OwnedBlock,
    context: &str,
    source: &SceneAnimationSlot,
    edited: &SceneAnimationSlot,
) -> Result<(), String> {
    if block.tag != *b"SANM" {
        return Err(format!("{context} tree block is not SANM"));
    }
    if source == edited {
        return Ok(());
    }
    if source.animation_name != edited.animation_name {
        set_or_insert_property(
            block,
            string_property(0x03, &edited.animation_name)?,
            context,
        )?;
    }
    if source.enabled != edited.enabled {
        set_or_insert_property(
            block,
            OwnedProperty::u8(0x0f, edited.enabled as u8),
            context,
        )?;
    }
    Ok(())
}

fn reconcile_layer_animations(
    block: &mut OwnedBlock,
    scene_index: usize,
    layer_index: usize,
    source: &Layer,
    edited: &Layer,
) -> Result<(), String> {
    if block.tag != *b"LAYR" {
        return Err(format!(
            "scene[{scene_index}] layer[{layer_index}] tree block is not LAYR"
        ));
    }
    if animation_lists_equal(&source.animations, &edited.animations) {
        return Ok(());
    }
    let context = format!("scene[{scene_index}] layer[{layer_index}]");
    if source.animations.len() != edited.animations.len() {
        set_existing_property(
            block,
            OwnedProperty::u16(
                0x22,
                checked_u16(edited.animations.len(), "LAYR ANIM count")?,
            ),
            &context,
        )?;
    }
    let insertion = after_last_child_with_tag(block, b"CAST");
    reconcile_tagged_records(
        block,
        b"ANIM",
        &source.animations,
        &edited.animations,
        animation_definition_equal,
        |block, index, source, edited| {
            reconcile_animation(block, scene_index, layer_index, index, source, edited)
        },
        new_animation,
        insertion,
        &context,
    )
}

fn reconcile_animation(
    block: &mut OwnedBlock,
    scene_index: usize,
    layer_index: usize,
    animation_index: usize,
    source: &AnimationDefinition,
    edited: &AnimationDefinition,
) -> Result<(), String> {
    if block.tag != *b"ANIM" {
        return Err(format!(
            "scene[{scene_index}] layer[{layer_index}] ANIM[{animation_index}] tree block is not ANIM"
        ));
    }
    if animation_definition_equal(source, edited) {
        return Ok(());
    }
    let context = format!("scene[{scene_index}] layer[{layer_index}] ANIM[{animation_index}]");
    if source.motions.len() != edited.motions.len() {
        set_existing_property(
            block,
            OwnedProperty::u16(
                0x50,
                checked_u16(edited.motions.len(), &format!("{context} MOT count"))?,
            ),
            &context,
        )?;
    }
    if source.duration != edited.duration {
        set_existing_property(
            block,
            OwnedProperty::i16(
                0x56,
                i16::try_from(edited.duration).map_err(|_| {
                    format!(
                        "{context} duration {} does not fit the audited type-5 property",
                        edited.duration
                    )
                })?,
            ),
            &context,
        )?;
    }
    if source.name != edited.name {
        set_existing_property(block, string_property(0x03, &edited.name)?, &context)?;
    }
    if source.flags != edited.flags {
        set_existing_property(block, OwnedProperty::u32(0x5f, edited.flags), &context)?;
    }

    let source_blocks = tagged_child_blocks(block, b"MOT ");
    if source_blocks.len() > source.motions.len() {
        return Err(format!(
            "{context} tree has {} direct MOT blocks but the source model has only {} slots",
            source_blocks.len(),
            source.motions.len()
        ));
    }
    let source_motions = &source.motions[..source_blocks.len()];
    let desired_direct =
        desired_direct_motion_count(source_blocks.len(), &source.motions, &edited.motions);
    let insertion = source_blocks.last().map_or_else(
        || first_child_with_tag(block, b"CATR").unwrap_or(block.children.len()),
        |_| 0,
    );
    reconcile_tagged_records(
        block,
        b"MOT ",
        source_motions,
        &edited.motions[..desired_direct],
        motion_equal,
        |block, index, source, edited| {
            reconcile_motion(block, &format!("{context} MOT[{index}]"), source, edited)
        },
        new_motion,
        insertion,
        &context,
    )
}

fn reconcile_motion(
    block: &mut OwnedBlock,
    context: &str,
    source: &Motion,
    edited: &Motion,
) -> Result<(), String> {
    if block.tag != *b"MOT " {
        return Err(format!("{context} tree block is not MOT "));
    }
    if motion_equal(source, edited) {
        return Ok(());
    }
    if source.target != edited.target {
        set_existing_property(
            block,
            OwnedProperty::i16(
                0x51,
                i16::try_from(edited.target).map_err(|_| {
                    format!(
                        "{context} target {} does not fit the audited type-5 property",
                        edited.target
                    )
                })?,
            ),
            context,
        )?;
    }
    if source.tracks.len() != edited.tracks.len() {
        set_existing_property(
            block,
            OwnedProperty::u16(
                0x52,
                checked_u16(edited.tracks.len(), &format!("{context} TRK count"))?,
            ),
            context,
        )?;
    }
    let insertion = block.children.len();
    reconcile_tagged_records(
        block,
        b"TRK ",
        &source.tracks,
        &edited.tracks,
        track_equal,
        |block, index, source, edited| {
            reconcile_track(block, &format!("{context} TRK[{index}]"), source, edited)
        },
        new_track,
        insertion,
        context,
    )
}

fn reconcile_track(
    block: &mut OwnedBlock,
    context: &str,
    source: &Track,
    edited: &Track,
) -> Result<(), String> {
    if block.tag != *b"TRK " {
        return Err(format!("{context} tree block is not TRK "));
    }
    if track_equal(source, edited) {
        return Ok(());
    }
    if matches!(source.keys, KeyData::Unsupported) {
        return Err(format!(
            "{context} has unsupported source KEY data and cannot be mutated"
        ));
    }
    if matches!(edited.keys, KeyData::Unsupported) {
        return Err(format!("{context} cannot synthesize unsupported KEY data"));
    }
    if !format_supports_key_data(edited.format, &edited.keys) {
        return Err(format!(
            "{context} format {:#010x} does not encode its {} keys",
            edited.format,
            key_kind_name(&edited.keys)
        ));
    }
    if source.target != edited.target {
        set_existing_property(block, OwnedProperty::u16(0x53, edited.target), context)?;
    }
    if !key_data_equal(&source.keys, &edited.keys) {
        set_existing_property(
            block,
            OwnedProperty::u16(
                0x57,
                checked_u16(
                    key_data_len(&edited.keys).expect("supported KEY data"),
                    &format!("{context} key count"),
                )?,
            ),
            context,
        )?;
    }
    if source.format != edited.format {
        set_existing_property(block, OwnedProperty::u32(0x54, edited.format), context)?;
    }
    if source.range_start != edited.range_start {
        set_existing_property(block, OwnedProperty::i32(0x58, edited.range_start), context)?;
    }
    if source.range_end != edited.range_end {
        set_existing_property(block, OwnedProperty::i32(0x59, edited.range_end), context)?;
    }
    if key_data_equal(&source.keys, &edited.keys) {
        return Ok(());
    }
    let key_index = child_indices(block, b"KEY ")
        .into_iter()
        .last()
        .ok_or_else(|| format!("{context} has supported source keys but no KEY child"))?;
    reconcile_key_block(
        &mut block.children[key_index],
        context,
        &source.keys,
        &edited.keys,
    )
}

fn reconcile_key_block(
    block: &mut OwnedBlock,
    context: &str,
    source: &KeyData,
    edited: &KeyData,
) -> Result<(), String> {
    if block.tag != *b"KEY " {
        return Err(format!("{context} selected child is not KEY "));
    }
    let source_kind =
        key_kind(source).ok_or_else(|| format!("{context} has unsupported source KEY data"))?;
    let edited_kind = key_kind(edited)
        .ok_or_else(|| format!("{context} cannot synthesize unsupported KEY data"))?;
    let source_records = canonical_key_records(source, context)?;
    let edited_records = canonical_key_records(edited, context)?;
    let source_owned = split_owned_key_records(&block.properties, source_records.len(), context)?;
    let matches = record_matches(&source_records, &edited_records, |source, edited| {
        source == edited
    });
    let mut output = source_owned.leading;
    for (edited_index, source_index) in matches.into_iter().enumerate() {
        if let Some(source_index) = source_index {
            output.extend(reconcile_key_record(
                source_owned.records[source_index].clone(),
                &source_records[source_index],
                &edited_records[edited_index],
                source_kind == edited_kind,
                edited_kind,
                context,
            )?);
        } else {
            output.extend(edited_records[edited_index].iter().cloned());
        }
    }
    block.properties = output;
    Ok(())
}

struct OwnedKeyRecords {
    leading: Vec<OwnedProperty>,
    records: Vec<Vec<OwnedProperty>>,
}

fn split_owned_key_records(
    properties: &[OwnedProperty],
    expected: usize,
    context: &str,
) -> Result<OwnedKeyRecords, String> {
    let mut leading = Vec::new();
    let mut records = Vec::new();
    for property in properties {
        if property.code == 0x5a {
            records.push(vec![property.clone()]);
        } else if key_field_index(property.code).is_some() {
            let record = records.last_mut().ok_or_else(|| {
                format!(
                    "{context} KEY field {:#04x} precedes a frame",
                    property.code
                )
            })?;
            record.push(property.clone());
        } else if let Some(record) = records.last_mut() {
            record.push(property.clone());
        } else {
            leading.push(property.clone());
        }
    }
    if records.len() != expected {
        return Err(format!(
            "{context} KEY tree has {} key records but the source model has {expected}",
            records.len()
        ));
    }
    Ok(OwnedKeyRecords { leading, records })
}

fn reconcile_key_record(
    original: Vec<OwnedProperty>,
    source: &[OwnedProperty],
    edited: &[OwnedProperty],
    preserve_existing_type: bool,
    kind: KeyKind,
    context: &str,
) -> Result<Vec<OwnedProperty>, String> {
    let mut output = Vec::with_capacity(original.len().max(edited.len()));
    let mut emitted = [false; 5];
    for property in original {
        let Some(field_index) = key_field_index(property.code) else {
            output.push(property);
            continue;
        };
        if emitted[field_index] {
            return Err(format!(
                "{context} KEY has duplicate audited property {:#04x}",
                property.code
            ));
        }
        emitted[field_index] = true;
        let source_value = source
            .iter()
            .find(|candidate| candidate.code == property.code);
        let replacement = edited
            .iter()
            .find(|candidate| candidate.code == property.code);
        match replacement {
            None => {}
            Some(replacement) if source_value == Some(replacement) => output.push(property),
            Some(replacement) => output.push(adapt_key_property(
                property,
                replacement,
                kind,
                preserve_existing_type,
                context,
            )?),
        }
    }
    for replacement in edited {
        let field_index = key_field_index(replacement.code).expect("canonical key property");
        if !emitted[field_index] {
            output.push(replacement.clone());
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn reconcile_tagged_records<T>(
    block: &mut OwnedBlock,
    tag: &[u8; 4],
    source: &[T],
    edited: &[T],
    same: fn(&T, &T) -> bool,
    mut reconcile: impl FnMut(&mut OwnedBlock, usize, &T, &T) -> Result<(), String>,
    mut build: impl FnMut(&T) -> Result<OwnedBlock, String>,
    insertion_when_empty: usize,
    context: &str,
) -> Result<(), String> {
    let source_blocks = tagged_child_blocks(block, tag);
    if source_blocks.len() != source.len() {
        return Err(format!(
            "{context} tree has {} {} children but the source model has {}",
            source_blocks.len(),
            String::from_utf8_lossy(tag),
            source.len()
        ));
    }
    let matches = record_matches(source, edited, same);
    let mut desired = Vec::with_capacity(edited.len());
    for (edited_index, source_index) in matches.iter().copied().enumerate() {
        if let Some(source_index) = source_index {
            let mut child = source_blocks[source_index].clone();
            reconcile(
                &mut child,
                edited_index,
                &source[source_index],
                &edited[edited_index],
            )?;
            desired.push(child);
        } else {
            desired.push(build(&edited[edited_index])?);
        }
    }
    replace_tagged_children(block, tag, desired, &matches, insertion_when_empty);
    Ok(())
}
fn adapt_key_property(
    existing: OwnedProperty,
    replacement: &OwnedProperty,
    kind: KeyKind,
    preserve_existing_type: bool,
    context: &str,
) -> Result<OwnedProperty, String> {
    if !preserve_existing_type {
        return Ok(replacement.clone());
    }
    let type_matches =
        if matches!(kind, KeyKind::Key8I32 | KeyKind::Key20I32) && replacement.code == 0x5b {
            matches!(existing.type_code, 8 | 11 | 12)
        } else {
            existing.type_code == replacement.type_code
        };
    if !type_matches || existing.count != 1 || existing.multiplier != 1 {
        return Err(format!(
            "{context} KEY property {:#04x} has un-audited VTBF shape type={} count={} multiplier={}",
            existing.code, existing.type_code, existing.count, existing.multiplier
        ));
    }
    let mut updated = existing;
    updated.value.clone_from(&replacement.value);
    Ok(updated)
}

fn new_animation_set(animation_set: &AnimationSetDefinition) -> Result<OwnedBlock, String> {
    let mut block = OwnedBlock::new(*b"ANMS");
    block.properties = vec![
        string_property(0x03, &animation_set.name)?,
        OwnedProperty::i32(0x18, animation_set.start_frame),
        OwnedProperty::i32(0x19, animation_set.runtime_duration),
        OwnedProperty::i32(
            0x0e,
            checked_i32(animation_set.slots.len(), "new ANMS SANM count")?,
        ),
    ];
    block.children = animation_set
        .slots
        .iter()
        .map(new_scene_animation_slot)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block)
}

fn new_scene_animation_slot(slot: &SceneAnimationSlot) -> Result<OwnedBlock, String> {
    let mut block = OwnedBlock::new(*b"SANM");
    block.properties = vec![
        string_property(0x03, &slot.animation_name)?,
        OwnedProperty::u8(
            0x0f,
            u8::try_from(slot.enabled).map_err(|_| {
                format!(
                    "new SANM enabled value {} does not fit the audited type-1 property",
                    slot.enabled
                )
            })?,
        ),
    ];
    Ok(block)
}

fn new_animation(animation: &AnimationDefinition) -> Result<OwnedBlock, String> {
    let mut block = OwnedBlock::new(*b"ANIM");
    block.properties = vec![
        OwnedProperty::u16(
            0x50,
            checked_u16(animation.motions.len(), "new ANIM MOT count")?,
        ),
        OwnedProperty::i16(
            0x56,
            i16::try_from(animation.duration).map_err(|_| {
                format!(
                    "new ANIM duration {} does not fit the audited type-5 property",
                    animation.duration
                )
            })?,
        ),
        string_property(0x03, &animation.name)?,
        OwnedProperty::u32(0x5f, animation.flags),
    ];
    block.children = animation.motions[..direct_motion_count(&animation.motions)]
        .iter()
        .map(new_motion)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block)
}

fn new_motion(motion: &Motion) -> Result<OwnedBlock, String> {
    let mut block = OwnedBlock::new(*b"MOT ");
    block.properties = vec![
        OwnedProperty::i16(
            0x51,
            i16::try_from(motion.target).map_err(|_| {
                format!(
                    "new MOT target {} does not fit the audited type-5 property",
                    motion.target
                )
            })?,
        ),
        OwnedProperty::u16(0x52, checked_u16(motion.tracks.len(), "new MOT TRK count")?),
    ];
    block.children = motion
        .tracks
        .iter()
        .map(new_track)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(block)
}

fn new_track(track: &Track) -> Result<OwnedBlock, String> {
    if matches!(track.keys, KeyData::Unsupported) {
        return Err("cannot synthesize a TRK with unsupported KEY data".into());
    }
    if !format_supports_key_data(track.format, &track.keys) {
        return Err(format!(
            "new TRK format {:#010x} does not encode its {} keys",
            track.format,
            key_kind_name(&track.keys)
        ));
    }
    let mut block = OwnedBlock::new(*b"TRK ");
    block.properties = vec![
        OwnedProperty::u16(0x53, track.target),
        OwnedProperty::u16(
            0x57,
            checked_u16(
                key_data_len(&track.keys).expect("supported key data"),
                "new TRK key count",
            )?,
        ),
        OwnedProperty::u32(0x54, track.format),
        OwnedProperty::i32(0x58, track.range_start),
        OwnedProperty::i32(0x59, track.range_end),
    ];
    let mut key = OwnedBlock::new(*b"KEY ");
    for record in canonical_key_records(&track.keys, "new TRK")? {
        key.properties.extend(record);
    }
    block.children.push(key);
    Ok(block)
}

fn canonical_key_records(data: &KeyData, context: &str) -> Result<Vec<Vec<OwnedProperty>>, String> {
    match data {
        KeyData::Key8F32(keys) => Ok(keys
            .iter()
            .map(|key| {
                vec![
                    OwnedProperty::i32(0x5a, key.frame),
                    OwnedProperty::f32(0x5b, key.value),
                ]
            })
            .collect()),
        KeyData::Key8I32(keys) => Ok(keys
            .iter()
            .map(|key| {
                vec![
                    OwnedProperty::i32(0x5a, key.frame),
                    OwnedProperty::i32(0x5b, key.value),
                ]
            })
            .collect()),
        KeyData::Key8Bytes4(keys) => Ok(keys
            .iter()
            .map(|key| {
                vec![
                    OwnedProperty::i32(0x5a, key.frame),
                    raw_scalar_property(
                        0x5b,
                        12,
                        vec![key.value[3], key.value[0], key.value[1], key.value[2]],
                    ),
                ]
            })
            .collect()),
        KeyData::Key20F32(keys) => keys
            .iter()
            .map(|key| {
                Ok(vec![
                    OwnedProperty::i32(0x5a, key.frame),
                    OwnedProperty::f32(0x5b, key.value),
                    OwnedProperty::u16(
                        0x5c,
                        u16::try_from(key.mode).map_err(|_| {
                            format!(
                                "{context} KEY mode {} does not fit the audited type-6 property",
                                key.mode
                            )
                        })?,
                    ),
                    OwnedProperty::f32(0x5d, key.slope_in),
                    OwnedProperty::f32(0x5e, key.slope_out),
                ])
            })
            .collect(),
        KeyData::Key20I32(keys) => keys
            .iter()
            .map(|key| {
                Ok(vec![
                    OwnedProperty::i32(0x5a, key.frame),
                    OwnedProperty::i32(0x5b, key.value),
                    OwnedProperty::u16(
                        0x5c,
                        u16::try_from(key.mode).map_err(|_| {
                            format!(
                                "{context} KEY mode {} does not fit the audited type-6 property",
                                key.mode
                            )
                        })?,
                    ),
                    OwnedProperty::f32(0x5d, key.slope_in),
                    OwnedProperty::f32(0x5e, key.slope_out),
                ])
            })
            .collect(),
        KeyData::Unsupported => Err(format!("{context} cannot synthesize unsupported KEY data")),
    }
}

fn raw_scalar_property(code: u8, type_code: u8, value: Vec<u8>) -> OwnedProperty {
    OwnedProperty {
        code,
        type_code,
        count: 1,
        multiplier: 1,
        value,
    }
}

fn string_property(code: u8, value: &[u8]) -> Result<OwnedProperty, String> {
    OwnedProperty::string(code, value.to_vec()).map_err(|error| error.to_string())
}

fn set_existing_property(
    block: &mut OwnedBlock,
    replacement: OwnedProperty,
    context: &str,
) -> Result<(), String> {
    let property = block
        .properties
        .iter_mut()
        .rev()
        .find(|property| property.code == replacement.code)
        .ok_or_else(|| {
            format!(
                "{context} is missing audited property {:#04x}",
                replacement.code
            )
        })?;
    if property.type_code != replacement.type_code
        || property.count != replacement.count
        || property.multiplier != replacement.multiplier
    {
        return Err(format!(
            "{context} property {:#04x} has un-audited VTBF shape type={} count={} multiplier={}",
            replacement.code, property.type_code, property.count, property.multiplier
        ));
    }
    property.value = replacement.value;
    Ok(())
}

fn set_existing_u32_property(
    block: &mut OwnedBlock,
    code: u8,
    value: u32,
    context: &str,
) -> Result<(), String> {
    let property = block
        .properties
        .iter_mut()
        .rev()
        .find(|property| property.code == code)
        .ok_or_else(|| format!("{context} is missing audited property {code:#04x}"))?;
    if property.count != 1 || property.multiplier != 1 {
        return Err(format!(
            "{context} property {code:#04x} has un-audited VTBF shape type={} count={} multiplier={}",
            property.type_code, property.count, property.multiplier
        ));
    }
    property.value =
        match property.type_code {
            1 | 4 => vec![u8::try_from(value).map_err(|_| {
                format!("{context} value {value} does not fit property {code:#04x}")
            })?],
            3 => {
                vec![i8::try_from(value).map_err(|_| {
                    format!("{context} value {value} does not fit property {code:#04x}")
                })? as u8]
            }
            5 | 7 => i16::try_from(value)
                .map_err(|_| format!("{context} value {value} does not fit property {code:#04x}"))?
                .to_le_bytes()
                .to_vec(),
            6 => u16::try_from(value)
                .map_err(|_| format!("{context} value {value} does not fit property {code:#04x}"))?
                .to_le_bytes()
                .to_vec(),
            8 | 9 | 11 | 12 => value.to_le_bytes().to_vec(),
            type_code => {
                return Err(format!(
                    "{context} property {code:#04x} uses unsupported integer type {type_code}"
                ));
            }
        };
    Ok(())
}

fn set_or_insert_property(
    block: &mut OwnedBlock,
    replacement: OwnedProperty,
    context: &str,
) -> Result<(), String> {
    if block
        .properties
        .iter()
        .any(|property| property.code == replacement.code)
    {
        return set_existing_property(block, replacement, context);
    }
    let rank = canonical_property_rank(block.tag, replacement.code).ok_or_else(|| {
        format!(
            "{context} cannot synthesize unaudited property {:#04x}",
            replacement.code
        )
    })?;
    let insertion = block
        .properties
        .iter()
        .position(|property| {
            canonical_property_rank(block.tag, property.code)
                .is_some_and(|property_rank| property_rank > rank)
        })
        .unwrap_or(block.properties.len());
    block.properties.insert(insertion, replacement);
    Ok(())
}

fn canonical_property_rank(tag: [u8; 4], code: u8) -> Option<u8> {
    match (tag, code) {
        (tag, 0x03) if tag == *b"ANMS" => Some(0),
        (tag, 0x18) if tag == *b"ANMS" => Some(1),
        (tag, 0x19) if tag == *b"ANMS" => Some(2),
        (tag, 0x0e) if tag == *b"ANMS" => Some(3),
        (tag, 0x03) if tag == *b"SANM" => Some(0),
        (tag, 0x0f) if tag == *b"SANM" => Some(1),
        _ => None,
    }
}

fn owned_project_block_mut(file: &mut OwnedSrdFile) -> Result<&mut OwnedBlock, String> {
    file.blocks
        .iter_mut()
        .filter(|block| block.tag == *b"SRCK")
        .filter_map(|srck| {
            srck.children
                .iter_mut()
                .rev()
                .find(|child| child.tag == *b"PROJ")
        })
        .last()
        .ok_or_else(|| "owned SRFF has no SRCK/PROJ block".into())
}

fn child_indices(block: &OwnedBlock, tag: &[u8; 4]) -> Vec<usize> {
    block
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| (child.tag == *tag).then_some(index))
        .collect()
}

fn tagged_child_blocks(block: &OwnedBlock, tag: &[u8; 4]) -> Vec<OwnedBlock> {
    block
        .children
        .iter()
        .filter(|child| child.tag == *tag)
        .cloned()
        .collect()
}

fn replace_tagged_children(
    block: &mut OwnedBlock,
    tag: &[u8; 4],
    desired: Vec<OwnedBlock>,
    matches: &[Option<usize>],
    insertion_when_empty: usize,
) {
    let original = std::mem::take(&mut block.children);
    let source_tag_count = original.iter().filter(|child| child.tag == *tag).count();
    if source_tag_count == 0 {
        let insertion = insertion_when_empty.min(original.len());
        let mut children = original;
        children.splice(insertion..insertion, desired);
        block.children = children;
        return;
    }

    if matches.iter().all(Option::is_none) {
        replace_tagged_children_in_order(block, tag, original, desired);
        return;
    }

    let mut desired = desired.into_iter().map(Some).collect::<Vec<_>>();
    let mut surviving_source_slots = vec![false; source_tag_count];
    for source_index in matches.iter().flatten() {
        surviving_source_slots[*source_index] = true;
    }
    let last_surviving_source_slot = surviving_source_slots
        .iter()
        .rposition(|survives| *survives)
        .expect("at least one source record survives");
    let mut mapped_edited_indices = matches
        .iter()
        .enumerate()
        .filter_map(|(edited_index, source_index)| source_index.is_some().then_some(edited_index));
    let mut next_edited = 0;
    let mut source_index = 0;
    let mut children = Vec::with_capacity(original.len() + desired.len());
    for child in original {
        if child.tag != *tag {
            children.push(child);
            continue;
        }
        if surviving_source_slots[source_index] {
            let edited_index = mapped_edited_indices
                .next()
                .expect("surviving source record");
            while next_edited < edited_index {
                children.push(desired[next_edited].take().expect("new desired record"));
                next_edited += 1;
            }
            children.push(desired[edited_index].take().expect("mapped desired record"));
            next_edited = edited_index + 1;
            if source_index == last_surviving_source_slot {
                while next_edited < desired.len() {
                    children.push(
                        desired[next_edited]
                            .take()
                            .expect("trailing new desired record"),
                    );
                    next_edited += 1;
                }
            }
        }
        source_index += 1;
    }
    block.children = children;
}

fn replace_tagged_children_in_order(
    block: &mut OwnedBlock,
    tag: &[u8; 4],
    original: Vec<OwnedBlock>,
    desired: Vec<OwnedBlock>,
) {
    let last_tag = original
        .iter()
        .rposition(|child| child.tag == *tag)
        .expect("source tag exists");
    let mut desired = desired.into_iter();
    let mut children = Vec::with_capacity(original.len());
    for (index, child) in original.into_iter().enumerate() {
        if child.tag == *tag {
            if let Some(replacement) = desired.next() {
                children.push(replacement);
            }
            if index == last_tag {
                children.extend(desired.by_ref());
            }
        } else {
            children.push(child);
        }
    }
    block.children = children;
}

fn record_matches<T>(source: &[T], edited: &[T], same: fn(&T, &T) -> bool) -> Vec<Option<usize>> {
    let mut used = vec![false; source.len()];
    let mut matches = vec![None; edited.len()];
    for (edited_index, edited_record) in edited.iter().enumerate() {
        if let Some((source_index, _)) =
            source
                .iter()
                .enumerate()
                .find(|(source_index, source_record)| {
                    !used[*source_index] && same(source_record, edited_record)
                })
        {
            used[source_index] = true;
            matches[edited_index] = Some(source_index);
        }
    }
    let mut unmatched_source = source
        .iter()
        .enumerate()
        .filter_map(|(index, _)| (!used[index]).then_some(index));
    for matched in &mut matches {
        if matched.is_none() {
            *matched = unmatched_source.next();
        }
    }
    matches
}

fn first_child_with_tag(block: &OwnedBlock, tag: &[u8; 4]) -> Option<usize> {
    block.children.iter().position(|child| child.tag == *tag)
}

fn after_last_child_with_tag(block: &OwnedBlock, tag: &[u8; 4]) -> usize {
    block
        .children
        .iter()
        .rposition(|child| child.tag == *tag)
        .map_or(block.children.len(), |index| index + 1)
}

fn is_implicit_motion_padding(motion: &Motion) -> bool {
    motion.target == -1 && motion.tracks.is_empty()
}

fn desired_direct_motion_count(
    source_direct_count: usize,
    source: &[Motion],
    edited: &[Motion],
) -> usize {
    let matched_direct = record_matches(source, edited, motion_equal)
        .iter()
        .enumerate()
        .filter_map(|(edited_index, source_index)| {
            source_index
                .is_some_and(|source_index| source_index < source_direct_count)
                .then_some(edited_index)
        })
        .max();
    let authored_direct = edited
        .iter()
        .rposition(|motion| !is_implicit_motion_padding(motion));
    matched_direct
        .into_iter()
        .chain(authored_direct)
        .max()
        .map_or(0, |index| index + 1)
}

fn direct_motion_count(motions: &[Motion]) -> usize {
    motions
        .iter()
        .rposition(|motion| !is_implicit_motion_padding(motion))
        .map_or(0, |index| index + 1)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    Key8F32,
    Key8I32,
    Key8Bytes4,
    Key20F32,
    Key20I32,
}

fn key_kind(data: &KeyData) -> Option<KeyKind> {
    match data {
        KeyData::Key8F32(_) => Some(KeyKind::Key8F32),
        KeyData::Key8I32(_) => Some(KeyKind::Key8I32),
        KeyData::Key8Bytes4(_) => Some(KeyKind::Key8Bytes4),
        KeyData::Key20F32(_) => Some(KeyKind::Key20F32),
        KeyData::Key20I32(_) => Some(KeyKind::Key20I32),
        KeyData::Unsupported => None,
    }
}

fn key_kind_name(data: &KeyData) -> &'static str {
    match key_kind(data) {
        Some(KeyKind::Key8F32) => "Key8F32",
        Some(KeyKind::Key8I32) => "Key8I32",
        Some(KeyKind::Key8Bytes4) => "Key8Bytes4",
        Some(KeyKind::Key20F32) => "Key20F32",
        Some(KeyKind::Key20I32) => "Key20I32",
        None => "Unsupported",
    }
}

fn key_data_len(data: &KeyData) -> Option<usize> {
    match data {
        KeyData::Key8F32(keys) => Some(keys.len()),
        KeyData::Key8I32(keys) => Some(keys.len()),
        KeyData::Key8Bytes4(keys) => Some(keys.len()),
        KeyData::Key20F32(keys) => Some(keys.len()),
        KeyData::Key20I32(keys) => Some(keys.len()),
        KeyData::Unsupported => None,
    }
}

fn key_field_index(code: u8) -> Option<usize> {
    match code {
        0x5a => Some(0),
        0x5b => Some(1),
        0x5c => Some(2),
        0x5d => Some(3),
        0x5e => Some(4),
        _ => None,
    }
}

pub fn projects_equal_for_persistence(left: &Project, right: &Project) -> bool {
    if left.scenes.len() != right.scenes.len() {
        return false;
    }
    let mut non_animation_left = left.clone();
    let mut non_animation_right = right.clone();
    for (left_scene, right_scene) in non_animation_left
        .scenes
        .iter_mut()
        .zip(&mut non_animation_right.scenes)
    {
        if left_scene.animation_sets != right_scene.animation_sets
            || left_scene.layers.len() != right_scene.layers.len()
        {
            return false;
        }
        left_scene.animation_sets.clear();
        right_scene.animation_sets.clear();
        for (left_layer, right_layer) in left_scene.layers.iter_mut().zip(&mut right_scene.layers) {
            if !animation_lists_equal(&left_layer.animations, &right_layer.animations) {
                return false;
            }
            left_layer.animations.clear();
            right_layer.animations.clear();
        }
    }
    non_animation_left == non_animation_right
}

fn animation_lists_equal(left: &[AnimationDefinition], right: &[AnimationDefinition]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| animation_definition_equal(left, right))
}

fn animation_definition_equal(left: &AnimationDefinition, right: &AnimationDefinition) -> bool {
    left.name == right.name
        && left.flags == right.flags
        && left.declared_motion_count == right.declared_motion_count
        && left.duration == right.duration
        && left.motions.len() == right.motions.len()
        && left
            .motions
            .iter()
            .zip(&right.motions)
            .all(|(left, right)| motion_equal(left, right))
}

fn motion_equal(left: &Motion, right: &Motion) -> bool {
    left.target == right.target
        && left.tracks.len() == right.tracks.len()
        && left
            .tracks
            .iter()
            .zip(&right.tracks)
            .all(|(left, right)| track_equal(left, right))
}

fn track_equal(left: &Track, right: &Track) -> bool {
    left.target == right.target
        && left.key_count == right.key_count
        && left.format == right.format
        && left.range_start == right.range_start
        && left.range_end == right.range_end
        && key_data_equal(&left.keys, &right.keys)
}

fn key_data_equal(left: &KeyData, right: &KeyData) -> bool {
    match (left, right) {
        (KeyData::Key8F32(left), KeyData::Key8F32(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| {
                    left.frame == right.frame && left.value.to_bits() == right.value.to_bits()
                })
        }
        (KeyData::Key8I32(left), KeyData::Key8I32(right)) => left == right,
        (KeyData::Key8Bytes4(left), KeyData::Key8Bytes4(right)) => left == right,
        (KeyData::Key20F32(left), KeyData::Key20F32(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| {
                    left.frame == right.frame
                        && left.value.to_bits() == right.value.to_bits()
                        && left.mode == right.mode
                        && left.slope_in.to_bits() == right.slope_in.to_bits()
                        && left.slope_out.to_bits() == right.slope_out.to_bits()
                })
        }
        (KeyData::Key20I32(left), KeyData::Key20I32(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left, right)| {
                    left.frame == right.frame
                        && left.value == right.value
                        && left.mode == right.mode
                        && left.slope_in.to_bits() == right.slope_in.to_bits()
                        && left.slope_out.to_bits() == right.slope_out.to_bits()
                })
        }
        (KeyData::Unsupported, KeyData::Unsupported) => true,
        _ => false,
    }
}

fn key_data_has_non_finite(data: &KeyData) -> bool {
    match data {
        KeyData::Key8F32(keys) => keys.iter().any(|key| !key.value.is_finite()),
        KeyData::Key20F32(keys) => keys.iter().any(|key| {
            !key.value.is_finite() || !key.slope_in.is_finite() || !key.slope_out.is_finite()
        }),
        KeyData::Key20I32(keys) => keys
            .iter()
            .any(|key| !key.slope_in.is_finite() || !key.slope_out.is_finite()),
        KeyData::Key8I32(_) | KeyData::Key8Bytes4(_) | KeyData::Unsupported => false,
    }
}

fn format_supports_key_data(format: u32, data: &KeyData) -> bool {
    let low = format & 3;
    let family = format & 0x70;
    match data {
        KeyData::Key8F32(_) => matches!(low, 0 | 1) && family == 0x10,
        KeyData::Key8I32(_) => matches!(low, 0 | 1) && family == 0x40,
        KeyData::Key8Bytes4(_) => matches!(low, 0 | 1) && family == 0x50,
        KeyData::Key20F32(_) => low == 3 && family == 0x10,
        KeyData::Key20I32(_) => low == 3 && matches!(family, 0x20 | 0x40),
        KeyData::Unsupported => false,
    }
}

fn validate_name(value: &[u8], allow_empty: bool, context: &str) -> Result<(), String> {
    if !allow_empty && value.is_empty() {
        return Err(format!("{context} must not be empty"));
    }
    if value.len() > 64 {
        return Err(format!(
            "{context} is {} bytes, exceeding the audited 64-byte parser limit",
            value.len()
        ));
    }
    Ok(())
}

fn display_name(value: &[u8]) -> String {
    let value = &value[..value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len())];
    if value.is_empty() {
        "<empty>".into()
    } else {
        String::from_utf8_lossy(value).into_owned()
    }
}

fn checked_u16(value: usize, context: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{context} {value} exceeds u16"))
}

fn checked_u32(value: usize, context: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{context} {value} exceeds u32"))
}

fn checked_i32(value: usize, context: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("{context} {value} exceeds i32"))
}
