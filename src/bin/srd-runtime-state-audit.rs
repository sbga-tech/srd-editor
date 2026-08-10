use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use srd_editor::csli::multiply_color_game;
use srd_editor::document::EditorDocument;
use srd_editor::game_host::{
    CHUSAN_ADVERTISE_LOGO_PLAYER, CHUSAN_COMMON_BACKGROUND_PLAYER, CHUSAN_MAIN_SCENE,
};
use srd_editor::image::{ImageDefinition, premultiply_additive_color_game};
use srd_editor::number::NumberDefinition;
use srd_editor::projection::{
    cast_overlaps_render_target_game, compose_screen_matrix_game, identity_matrix4x4_game,
    inverse_matrix4x4_game, mul_matrix4x4_game,
};
use srd_editor::reference_runtime::{ProjectRuntime, ReferenceLayerParent};
use srd_editor::render::{
    CeylonDrawPacketPresetState, SRD_RENDERER_INITIAL_LAYER_KEY,
    apply_srd_image_alpha_stencil_packet_fields, apply_srd_image_field_0c_shader_bits,
    ceylon_d3d9_blend_preset, select_srd_image_render_preset,
};
use srd_editor::scene::{AnimationSetDefinition, ReferenceTarget, Scene};
use srd_editor::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use srd_editor::shader_bytecode::embedded_simple_shader_pair;
use srd_editor::srd_draw::{
    EvidenceCompleteRuntimeCastDraw, EvidenceCompleteSrdDraw, SrdHostDrawContext,
    SrdRendererProjectTargetContext, build_evidence_complete_runtime_srd_draws_from_runtime,
};
use srd_editor::transform::Affine3x4;

#[derive(Debug, Clone)]
struct FirstOccurrence {
    path: PathBuf,
    scene_index: usize,
    animation_set_index: usize,
    frame: i32,
    layer_index: usize,
    node_index: usize,
    kind: &'static str,
}

#[derive(Debug, Default)]
struct StaticAuditSummary {
    node_context_count: usize,
    type_counts: [usize; 3],
    initial_key_counts: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], usize>,
    initial_unpacked: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>,
    initial_outside_collection: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>,
    alpha_test_count: usize,
    alpha_test_keys: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], (usize, String)>,
    alpha_test_outside_collection: usize,
    alpha_test_first: Option<String>,
    stencil_count: usize,
    stencil_first: Option<String>,
}

#[derive(Debug, Default)]
struct SpecialMatrixAuditSummary {
    layer_count: usize,
    flagged_node_count: usize,
    flag_counts: BTreeMap<u32, usize>,
    full_flag_counts: BTreeMap<u32, usize>,
    billboard_modifier_count: usize,
    billboard_modifier_mode_counts: BTreeMap<&'static str, usize>,
    layer_mode_counts: BTreeMap<&'static str, usize>,
    affected_cast_type_counts: BTreeMap<u8, usize>,
    first: Option<String>,
}

#[derive(Debug, Default)]
struct TextCastAuditSummary {
    definition_count: usize,
    definition_mode_counts: BTreeMap<&'static str, usize>,
    runtime_context_count: usize,
    runtime_mode_counts: BTreeMap<&'static str, usize>,
    copied_context_count: usize,
    copied_mode_override_count: usize,
    rfz_context_count: usize,
    vertical_context_count: usize,
    vertical_mode_counts: BTreeMap<&'static str, usize>,
    runtime_substitution_context_count: usize,
    three_d_definitions: Vec<String>,
    first_3d: Option<String>,
    first_vertical: Option<String>,
    first_copied_mode_override: Option<String>,
}

#[derive(Debug, Default)]
struct Runtime3dTextReachabilitySummary {
    sampled_states: usize,
    contexts: usize,
    world_enabled: usize,
    color_enabled: usize,
    target_visible: usize,
    first_world_enabled: Option<String>,
    first_color_enabled: Option<String>,
    first_target_visible: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: srd-runtime-state-audit <game-data-root>")?;
    let mut exhaustive_integer_frames = false;
    let mut text_only = false;
    let mut file_filter = None;
    let mut host_profile = None;
    for argument in arguments {
        let argument = argument.to_string_lossy();
        if argument == "--integer-frames" {
            exhaustive_integer_frames = true;
        } else if argument == "--text-only" {
            text_only = true;
        } else if let Some(value) = argument.strip_prefix("--file=")
            && !value.is_empty()
        {
            file_filter = Some(value.to_ascii_lowercase());
        } else if let Some(value) = argument.strip_prefix("--host=")
            && matches!(value, "advertise-logo" | "common-background")
        {
            host_profile = Some(value.to_owned());
        } else {
            return Err(
                "usage: srd-runtime-state-audit <game-data-root> [--integer-frames] [--text-only] [--file=<path-substring>] [--host=advertise-logo|common-background]"
                    .into(),
            );
        }
    }
    let collection = load_simple_collection(&root)?;
    let mut files = Vec::new();
    collect_srd_files(&root.join("surfboard"), &mut files)?;
    files.sort();
    if let Some(filter) = &file_filter {
        files.retain(|path| path.to_string_lossy().to_ascii_lowercase().contains(filter));
    }
    if files.is_empty() {
        return Err("no SRD files matched the requested filter".into());
    }

    let host = match host_profile.as_deref() {
        Some("advertise-logo") => CHUSAN_ADVERTISE_LOGO_PLAYER.host_context_for_target(
            CHUSAN_MAIN_SCENE,
            1080,
            1920,
            [1920, 1080],
        )?,
        Some("common-background") => CHUSAN_COMMON_BACKGROUND_PLAYER.host_context_for_target(
            CHUSAN_MAIN_SCENE,
            1080,
            1920,
            [1920, 1080],
        )?,
        None => SrdHostDrawContext::new(
            Affine3x4::IDENTITY,
            SRD_RENDERER_INITIAL_LAYER_KEY,
            Some(SrdRendererProjectTargetContext::new(
                identity_matrix4x4_game(),
                [1920, 1080],
            )),
            identity_matrix4x4_game(),
            [1920, 1080],
        ),
        Some(_) => unreachable!(),
    };
    let mut animation_set_count = 0usize;
    let mut sampled_frame_count = 0usize;
    let mut draw_count = 0usize;
    let mut type_counts = [0usize; 3];
    let mut shader_key_counts = BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], usize>::new();
    let mut texture_mask_counts = BTreeMap::<u8, usize>::new();
    let mut texture_mask_first = BTreeMap::<u8, FirstOccurrence>::new();
    let mut unpackaged = BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], FirstOccurrence>::new();
    let mut outside_collection =
        BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], FirstOccurrence>::new();
    let mut stencil_count = 0usize;
    let mut stencil_first = None::<FirstOccurrence>;
    let mut alpha_test_count = 0usize;
    let mut alpha_test_first = None::<FirstOccurrence>;
    let mut alpha_test_key_counts = BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], usize>::new();
    let mut potential_key_counts = BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], usize>::new();
    let mut potential_unpacked = BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>::new();
    let mut potential_outside_collection =
        BTreeMap::<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>::new();
    let mut static_summary = StaticAuditSummary::default();
    let mut special_matrix_summary = SpecialMatrixAuditSummary::default();
    let mut text_cast_summary = TextCastAuditSummary::default();
    let mut runtime_3d_text_summary = Runtime3dTextReachabilitySummary::default();
    let mut initial_runtime_draw_count = 0usize;
    let mut initial_runtime_special_layer_draw_count = 0usize;
    let mut initial_runtime_flagged_node_draw_count = 0usize;

    for (file_index, path) in files.iter().enumerate() {
        let document = EditorDocument::load(path)?;
        audit_special_matrix_layers(path, &document, &mut special_matrix_summary);
        let base_runtime = ProjectRuntime::new(&document.project)?;
        audit_text_casts(path, &document, &base_runtime, &mut text_cast_summary);
        for scene_index in 0..document.project.scenes.len() {
            audit_runtime_3d_text_reachability(
                path,
                &document,
                scene_index,
                "initial",
                host,
                &base_runtime,
                &mut runtime_3d_text_summary,
            )?;
        }
        if text_only && !exhaustive_integer_frames {
            continue;
        }
        if !text_only {
            audit_potential_shader_keys(
                path,
                &document,
                &base_runtime,
                &collection,
                &mut potential_key_counts,
                &mut potential_unpacked,
                &mut potential_outside_collection,
                &mut static_summary,
            )?;
            for scene_index in 0..document.project.scenes.len() {
                let draws = build_evidence_complete_runtime_srd_draws_from_runtime(
                    &document.project,
                    &document.textures,
                    scene_index,
                    host,
                    &base_runtime,
                )?;
                for draw in &draws {
                    let (state, _, _) = srd_state(draw);
                    initial_runtime_draw_count += 1;
                    let layer =
                        &document.project.scenes[state.scene_index].layers[state.layer_index];
                    if layer
                        .nodes
                        .iter()
                        .any(|node| node.type_flags.unwrap_or(0) & 0x0007_0000 != 0)
                    {
                        initial_runtime_special_layer_draw_count += 1;
                    }
                    if layer.nodes[state.node_index].type_flags.unwrap_or(0) & 0x0007_0000 != 0 {
                        initial_runtime_flagged_node_draw_count += 1;
                    }
                }
            }
        }
        if !exhaustive_integer_frames {
            continue;
        }
        for (scene_index, scene) in document.project.scenes.iter().enumerate() {
            for (animation_set_index, animation_set) in scene.animation_sets.iter().enumerate() {
                animation_set_count += 1;
                let start = animation_set.start_frame;
                let end = integer_state_coverage_end(scene, animation_set);
                for frame in start..=end {
                    sampled_frame_count += 1;
                    let mut runtime = base_runtime.clone();
                    runtime.apply_animation_set(
                        &document.project,
                        &document.textures,
                        scene_index,
                        animation_set_index,
                        frame as f32,
                    )?;
                    audit_runtime_3d_text_reachability(
                        path,
                        &document,
                        scene_index,
                        &format!("ANMS[{animation_set_index}] frame={frame}"),
                        host,
                        &runtime,
                        &mut runtime_3d_text_summary,
                    )?;
                    if text_only {
                        continue;
                    }
                    let draws = build_evidence_complete_runtime_srd_draws_from_runtime(
                        &document.project,
                        &document.textures,
                        scene_index,
                        host,
                        &runtime,
                    )?;
                    for draw in &draws {
                        let (state, kind, type_index) = srd_state(draw);
                        draw_count += 1;
                        type_counts[type_index] += 1;
                        *shader_key_counts.entry(state.shader_key).or_insert(0) += 1;
                        let texture_mask = state
                            .texture_bindings
                            .iter()
                            .enumerate()
                            .fold(0u8, |mask, (slot, binding)| {
                                mask | (u8::from(binding.is_some()) << slot)
                            });
                        *texture_mask_counts.entry(texture_mask).or_insert(0) += 1;
                        let occurrence = || FirstOccurrence {
                            path: path.clone(),
                            scene_index,
                            animation_set_index,
                            frame,
                            layer_index: state.layer_index,
                            node_index: state.node_index,
                            kind,
                        };
                        texture_mask_first
                            .entry(texture_mask)
                            .or_insert_with(occurrence);
                        if embedded_simple_shader_pair(&state.shader_key).is_none() {
                            unpackaged
                                .entry(state.shader_key)
                                .or_insert_with(occurrence);
                        }
                        if !collection.contains(&state.shader_key) {
                            outside_collection
                                .entry(state.shader_key)
                                .or_insert_with(occurrence);
                        }
                        if state.packet.flags_0c & 0x100 != 0 {
                            stencil_count += 1;
                            stencil_first.get_or_insert_with(occurrence);
                        }
                        if state.blend.alpha_test_enabled {
                            alpha_test_count += 1;
                            *alpha_test_key_counts.entry(state.shader_key).or_insert(0) += 1;
                            alpha_test_first.get_or_insert_with(occurrence);
                        }
                    }
                }
            }
        }
        eprintln!(
            "audited {}/{}: {}",
            file_index + 1,
            files.len(),
            path.display()
        );
    }

    println!("files={}", files.len());
    println!(
        "special_matrix_layers={} flagged_nodes={} flags={:?} full_flags={:?} modifier_01000000={} modifier_modes={:?} layer_modes={:?} affected_cast_types={:?} first={}",
        special_matrix_summary.layer_count,
        special_matrix_summary.flagged_node_count,
        special_matrix_summary.flag_counts,
        special_matrix_summary.full_flag_counts,
        special_matrix_summary.billboard_modifier_count,
        special_matrix_summary.billboard_modifier_mode_counts,
        special_matrix_summary.layer_mode_counts,
        special_matrix_summary.affected_cast_type_counts,
        special_matrix_summary.first.as_deref().unwrap_or("none")
    );
    println!(
        "initial_node_contexts={} image/slice/number={:?}",
        static_summary.node_context_count, static_summary.type_counts
    );
    println!(
        "text_cast_definitions={} definition_modes={:?} runtime_contexts={} runtime_modes={:?} copied_contexts={} copied_mode_overrides={} rfz_contexts={} vertical_contexts={} vertical_modes={:?} runtime_substitution_contexts={}",
        text_cast_summary.definition_count,
        text_cast_summary.definition_mode_counts,
        text_cast_summary.runtime_context_count,
        text_cast_summary.runtime_mode_counts,
        text_cast_summary.copied_context_count,
        text_cast_summary.copied_mode_override_count,
        text_cast_summary.rfz_context_count,
        text_cast_summary.vertical_context_count,
        text_cast_summary.vertical_mode_counts,
        text_cast_summary.runtime_substitution_context_count,
    );
    println!(
        "text_cast_first_3d={} first_vertical={} first_copied_mode_override={}",
        text_cast_summary.first_3d.as_deref().unwrap_or("none"),
        text_cast_summary
            .first_vertical
            .as_deref()
            .unwrap_or("none"),
        text_cast_summary
            .first_copied_mode_override
            .as_deref()
            .unwrap_or("none"),
    );
    for location in &text_cast_summary.three_d_definitions {
        println!("  text_cast_3d_definition={location}");
    }
    println!(
        "runtime_3d_text_states={} contexts={} world_enabled={} color_enabled={} target_visible={} first_world_enabled={} first_color_enabled={} first_target_visible={}",
        runtime_3d_text_summary.sampled_states,
        runtime_3d_text_summary.contexts,
        runtime_3d_text_summary.world_enabled,
        runtime_3d_text_summary.color_enabled,
        runtime_3d_text_summary.target_visible,
        runtime_3d_text_summary
            .first_world_enabled
            .as_deref()
            .unwrap_or("none"),
        runtime_3d_text_summary
            .first_color_enabled
            .as_deref()
            .unwrap_or("none"),
        runtime_3d_text_summary
            .first_target_visible
            .as_deref()
            .unwrap_or("none"),
    );
    println!(
        "initial_runtime_draws={} draws_in_special_layers={} draws_from_flagged_nodes={}",
        initial_runtime_draw_count,
        initial_runtime_special_layer_draw_count,
        initial_runtime_flagged_node_draw_count
    );
    println!(
        "initial_direct_shader_keys={} unpacked={} outside_game_collection={}",
        static_summary.initial_key_counts.len(),
        static_summary.initial_unpacked.len(),
        static_summary.initial_outside_collection.len()
    );
    println!(
        "initial_alpha_test={} first={}",
        static_summary.alpha_test_count,
        static_summary.alpha_test_first.as_deref().unwrap_or("none")
    );
    println!(
        "initial_alpha_test_keys={} outside_game_collection={}",
        static_summary.alpha_test_keys.len(),
        static_summary.alpha_test_outside_collection
    );
    for (key, (count, first)) in &static_summary.alpha_test_keys {
        println!(
            "  alpha_test_key={} count={} in_collection={} first={}",
            String::from_utf8_lossy(key),
            count,
            collection.contains(key),
            first
        );
    }
    println!(
        "initial_stencil={} first={}",
        static_summary.stencil_count,
        static_summary.stencil_first.as_deref().unwrap_or("none")
    );
    println!(
        "potential_upper_bound_shader_keys={}",
        potential_key_counts.len()
    );
    println!(
        "potential_upper_bound_unpacked_keys={}",
        potential_unpacked.len()
    );
    for (key, first) in &potential_unpacked {
        println!("  {} first={first}", String::from_utf8_lossy(key));
    }
    println!(
        "potential_upper_bound_keys_outside_game_collection={}",
        potential_outside_collection.len()
    );
    for (key, first) in &potential_outside_collection {
        println!("  {} first={first}", String::from_utf8_lossy(key));
    }
    if !exhaustive_integer_frames || text_only {
        return Ok(());
    }

    println!("animation_sets={animation_set_count}");
    println!("sampled_integer_frames={sampled_frame_count}");
    println!("draws={draw_count} image/slice/number={type_counts:?}");
    println!("distinct_runtime_shader_keys={}", shader_key_counts.len());
    println!("runtime_texture_masks={texture_mask_counts:?}");
    for (mask, first) in &texture_mask_first {
        println!("  texture_mask={mask:#05b} first={}", describe(first));
    }
    println!("unpackaged_shader_keys={}", unpackaged.len());
    for (key, first) in &unpackaged {
        println!(
            "  {} first={}",
            String::from_utf8_lossy(key),
            describe(first)
        );
    }
    println!("keys_outside_game_collection={}", outside_collection.len());
    for (key, first) in &outside_collection {
        println!(
            "  {} first={}",
            String::from_utf8_lossy(key),
            describe(first)
        );
    }
    println!("stencil_draws={stencil_count}");
    println!(
        "stencil_first={}",
        stencil_first.as_ref().map_or("none".into(), describe)
    );
    println!(
        "alpha_test_draws={} keys={:?}",
        alpha_test_count,
        alpha_test_key_counts
            .iter()
            .map(|(key, count)| (String::from_utf8_lossy(key), count))
            .collect::<Vec<_>>()
    );
    println!(
        "alpha_test_first={}",
        alpha_test_first.as_ref().map_or("none".into(), describe)
    );
    Ok(())
}

fn audit_text_casts(
    path: &Path,
    document: &EditorDocument,
    runtime: &ProjectRuntime,
    summary: &mut TextCastAuditSummary,
) {
    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            for (node_index, image) in layer.image_by_node.iter().enumerate() {
                if image
                    .as_ref()
                    .is_some_and(ImageDefinition::creates_text_cast)
                {
                    summary.definition_count += 1;
                    *summary
                        .definition_mode_counts
                        .entry(if layer.is_2d() { "2d" } else { "3d" })
                        .or_insert(0) += 1;
                    if !layer.is_2d() {
                        summary.three_d_definitions.push(format!(
                            "{} SCN[{}]/LAYR[{}]/NODE[{}]",
                            path.display(),
                            scene_index,
                            layer_index,
                            node_index,
                        ));
                    }
                }
            }

            audit_text_cast_context(
                path,
                document,
                ReferenceTarget {
                    scene_index,
                    layer_index,
                },
                layer.is_2d(),
                false,
                summary,
            );
        }
    }

    for instance in &runtime.references.plan.instances {
        audit_text_cast_context(
            path,
            document,
            instance.target,
            instance.is_2d,
            true,
            summary,
        );
    }
}

fn audit_text_cast_context(
    path: &Path,
    document: &EditorDocument,
    target: ReferenceTarget,
    effective_is_2d: bool,
    copied: bool,
    summary: &mut TextCastAuditSummary,
) {
    let layer = &document.project.scenes[target.scene_index].layers[target.layer_index];
    let mode = if effective_is_2d { "2d" } else { "3d" };
    for (node_index, image) in layer.image_by_node.iter().enumerate() {
        let Some(image) = image.as_ref().filter(|image| image.creates_text_cast()) else {
            continue;
        };
        summary.runtime_context_count += 1;
        *summary.runtime_mode_counts.entry(mode).or_insert(0) += 1;
        if copied {
            summary.copied_context_count += 1;
        }
        if copied && effective_is_2d != layer.is_2d() {
            summary.copied_mode_override_count += 1;
            summary.first_copied_mode_override.get_or_insert_with(|| {
                format!(
                    "{} SCN[{}]/LAYR[{}]/NODE[{}] source_mode={} effective_mode={}",
                    path.display(),
                    target.scene_index,
                    target.layer_index,
                    node_index,
                    if layer.is_2d() { "2d" } else { "3d" },
                    mode,
                )
            });
        }
        if !effective_is_2d {
            summary.first_3d.get_or_insert_with(|| {
                format!(
                    "{} SCN[{}]/LAYR[{}]/NODE[{}] copied={}",
                    path.display(),
                    target.scene_index,
                    target.layer_index,
                    node_index,
                    copied,
                )
            });
        }
        let Some(text) = image.text.as_ref() else {
            continue;
        };
        if document
            .project
            .resolve_text_font(text)
            .is_some_and(|font| font.name.to_ascii_lowercase().ends_with(b".rfz"))
        {
            summary.rfz_context_count += 1;
        }
        let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
        if font_param.vertical {
            summary.vertical_context_count += 1;
            *summary.vertical_mode_counts.entry(mode).or_insert(0) += 1;
            summary.first_vertical.get_or_insert_with(|| {
                format!(
                    "{} SCN[{}]/LAYR[{}]/NODE[{}] copied={} effective_mode={}",
                    path.display(),
                    target.scene_index,
                    target.layer_index,
                    node_index,
                    copied,
                    mode,
                )
            });
        }
        if (0..8u8).any(|substitution_index| {
            let token = [b'$', b'[', b'0' + substitution_index, b']'];
            text.text
                .windows(token.len())
                .any(|candidate| candidate == token)
        }) {
            summary.runtime_substitution_context_count += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn audit_runtime_3d_text_reachability(
    path: &Path,
    document: &EditorDocument,
    scene_index: usize,
    state_name: &str,
    host: SrdHostDrawContext,
    runtime: &ProjectRuntime,
    summary: &mut Runtime3dTextReachabilitySummary,
) -> Result<(), Box<dyn Error>> {
    summary.sampled_states += 1;
    let renderer_inverse_camera_view = if let Some(target) = host.renderer_project_target {
        let view = document
            .project
            .camera
            .runtime_matrices(target.render_size[0] as f32)
            .view;
        Affine3x4 {
            rows: [view.rows[0], view.rows[1], view.rows[2]],
        }
        .inverse_game()
    } else {
        Affine3x4::IDENTITY
    };
    let worlds = runtime.compose_world_states(
        &document.project,
        host.first_calc_matrix,
        renderer_inverse_camera_view,
    )?;
    let project_screen = host
        .renderer_project_target
        .map(|target| -> Result<_, Box<dyn Error>> {
            let external_inverse = inverse_matrix4x4_game(&target.projection_view);
            let srd_projection_view = document
                .project
                .camera
                .runtime_matrices(target.render_size[0] as f32)
                .projection_view;
            let camera_bridge = mul_matrix4x4_game(&external_inverse, &srd_projection_view);
            let width = i32::try_from(target.render_size[0])?;
            let height = i32::try_from(target.render_size[1])?;
            Ok((
                compose_screen_matrix_game(width, height, &target.projection_view, &camera_bridge),
                [width, height],
            ))
        })
        .transpose()?;

    for entry in runtime.references.plan.structural_cast_draw_order(
        &document.project,
        scene_index,
        host.renderer_layer_key,
    )? {
        let layer =
            &document.project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds
            .layer(entry.owner)
            .ok_or_else(|| format!("runtime owner {:?} has no world state", entry.owner))?;
        if layer_worlds.is_2d {
            continue;
        }
        let Some(image) = layer.image_by_node[entry.node_index]
            .as_ref()
            .filter(|image| image.creates_text_cast())
        else {
            continue;
        };
        summary.contexts += 1;
        let world = layer_worlds.casts[entry.node_index];
        let location = || {
            format!(
                "{} {} owner={:?} SCN[{}]/LAYR[{}]/NODE[{}]",
                path.display(),
                state_name,
                entry.owner,
                entry.source.scene_index,
                entry.source.layer_index,
                entry.node_index,
            )
        };
        if !world.visible || !world.render_gate {
            continue;
        }
        summary.world_enabled += 1;
        summary.first_world_enabled.get_or_insert_with(location);

        let image_states = match entry.owner {
            ReferenceLayerParent::ProjectLayer(target) => {
                &runtime.project_layers[target.scene_index][target.layer_index].image_states
            }
            ReferenceLayerParent::ReferenceInstance(instance_index) => {
                &runtime.references.layers[instance_index].image_states
            }
        };
        let image_state = image_states[entry.node_index];
        let source_colors = image_state
            .coordinate_state(srd_editor::image::ImageReferenceChannel::Cref)
            .vertex_colors;
        let primary_rgba = [0usize, 2, 1, 3].map(|source_index| {
            multiply_color_game(source_colors[source_index], world.multiply_color)
        });
        let secondary_rgba = premultiply_additive_color_game(world.additive_color);
        if !secondary_rgba[..3].iter().any(|component| *component != 0)
            && !primary_rgba.iter().any(|color| color[3] != 0)
        {
            continue;
        }
        summary.color_enabled += 1;
        summary.first_color_enabled.get_or_insert_with(location);

        let positions = image
            .build_quad_with_geometry(image_state.geometry, false)
            .positions
            .map(|point| world.matrix.transform_point_game(point));
        if let Some((screen_matrix, [width, height])) = &project_screen
            && !cast_overlaps_render_target_game(
                &positions,
                false,
                Some(screen_matrix),
                *width,
                *height,
            )?
        {
            continue;
        }
        summary.target_visible += 1;
        summary.first_target_visible.get_or_insert_with(location);
    }
    Ok(())
}

fn audit_special_matrix_layers(
    path: &Path,
    document: &EditorDocument,
    summary: &mut SpecialMatrixAuditSummary,
) {
    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            let flagged = layer
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(node_index, node)| {
                    let full_flags = node.type_flags.unwrap_or(0);
                    let matrix_flags = full_flags & 0x0007_0000;
                    (matrix_flags != 0).then_some((node_index, matrix_flags, full_flags))
                })
                .collect::<Vec<_>>();
            if flagged.is_empty() {
                continue;
            }
            summary.layer_count += 1;
            summary.flagged_node_count += flagged.len();
            *summary
                .layer_mode_counts
                .entry(if layer.is_2d() { "2d" } else { "3d" })
                .or_insert(0) += 1;
            for (_, matrix_flags, full_flags) in &flagged {
                *summary.flag_counts.entry(*matrix_flags).or_insert(0) += 1;
                *summary.full_flag_counts.entry(*full_flags).or_insert(0) += 1;
                if full_flags & 0x0100_0000 != 0 {
                    summary.billboard_modifier_count += 1;
                    *summary
                        .billboard_modifier_mode_counts
                        .entry(if layer.is_2d() { "2d" } else { "3d" })
                        .or_insert(0) += 1;
                }
            }
            for node in &layer.nodes {
                if let Some(cast_type) = node.cast_type() {
                    *summary
                        .affected_cast_type_counts
                        .entry(cast_type)
                        .or_insert(0) += 1;
                }
            }
            summary.first.get_or_insert_with(|| {
                format!(
                    "{} SCN[{}]/LAYR[{}] flagged={:?}",
                    path.display(),
                    scene_index,
                    layer_index,
                    flagged
                )
            });
        }
    }
}

/// Returns the last integer frame needed to cover every distinct combination
/// of selected animation-track evaluations in one ANMS. Non-wrapped tracks
/// are constant after their final range end. Wrapped tracks are periodic with
/// their integer range span, so after the non-wrapped prefix the joint state
/// repeats after the least common multiple of all selected wrapped spans.
fn integer_state_coverage_end(scene: &Scene, animation_set: &AnimationSetDefinition) -> i32 {
    let start = animation_set.start_frame;
    let duration_end = animation_set.runtime_duration.max(start);
    let available = i64::from(duration_end) - i64::from(start) + 1;
    let mut non_wrapped_end = start;
    let mut joint_period = 1i64;
    let mut has_wrapped_track = false;

    for (layer, slot) in scene.layers.iter().zip(&animation_set.slots) {
        if !slot.is_enabled() || slot.animation_name.is_empty() {
            continue;
        }
        let Some((_, animation)) = layer.find_animation(&slot.animation_name) else {
            continue;
        };
        for track in animation.motions.iter().flat_map(|motion| &motion.tracks) {
            if track.format & 0x300 == 0 {
                non_wrapped_end = non_wrapped_end.max(track.range_end);
                continue;
            }
            has_wrapped_track = true;
            let span = i64::from(track.range_end) - i64::from(track.range_start);
            let span = span.max(1).min(available);
            joint_period = lcm_capped(joint_period, span, available);
        }
    }

    let prefix_end = i64::from(non_wrapped_end.max(start));
    let coverage_end = if has_wrapped_track {
        prefix_end.saturating_add(joint_period.saturating_sub(1))
    } else {
        prefix_end
    };
    i32::try_from(coverage_end.min(i64::from(duration_end))).unwrap_or(duration_end)
}

fn lcm_capped(left: i64, right: i64, cap: i64) -> i64 {
    let divisor = gcd(left, right);
    left.checked_div(divisor)
        .and_then(|value| value.checked_mul(right))
        .unwrap_or(cap)
        .min(cap)
}

fn gcd(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left.abs().max(1)
}

#[allow(clippy::too_many_arguments)]
fn audit_potential_shader_keys(
    path: &Path,
    document: &EditorDocument,
    runtime: &ProjectRuntime,
    collection: &BTreeSet<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH]>,
    key_counts: &mut BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], usize>,
    unpacked: &mut BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>,
    outside_collection: &mut BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], String>,
    static_summary: &mut StaticAuditSummary,
) -> Result<(), Box<dyn Error>> {
    let mut contexts = BTreeSet::<(ReferenceTarget, bool)>::new();
    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            contexts.insert((
                ReferenceTarget {
                    scene_index,
                    layer_index,
                },
                layer.is_2d(),
            ));
        }
    }
    contexts.extend(
        runtime
            .references
            .plan
            .instances
            .iter()
            .map(|instance| (instance.target, instance.is_2d)),
    );

    for (target, is_2d) in contexts {
        let layer = &document.project.scenes[target.scene_index].layers[target.layer_index];
        for node_index in 0..layer.nodes.len() {
            let cast_type = layer.nodes[node_index].cast_type();
            let Some(image) = (match cast_type {
                Some(1) => layer.image_by_node[node_index]
                    .clone()
                    .filter(|image| !image.creates_text_cast()),
                Some(2) => layer.csli_by_node[node_index]
                    .as_ref()
                    .map(ImageDefinition::from_csli_runtime_base),
                Some(4) => layer.number_by_node[node_index]
                    .as_ref()
                    .map(NumberDefinition::image_base),
                _ => None,
            }) else {
                continue;
            };
            let mut state = image.initial_runtime_state();
            if let Some(ext_param) = layer.ext_param_for_node(node_index) {
                state.render_preset_override = ext_param.render_preset_override;
            }
            let Some(preset) =
                select_srd_image_render_preset(image.flags, state.render_preset_override, false)
            else {
                continue;
            };
            let initial_slots = image.resolve_texture_slots(
                &state,
                &document.textures,
                ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                [false; 2],
            )?;
            let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
            packet.set_render_preset_id(preset);
            packet.set_srd_quad_is_2d(is_2d);
            apply_srd_image_field_0c_shader_bits(&mut packet, state.field_0c as i32);
            let mut renderer_counter = SRD_RENDERER_INITIAL_LAYER_KEY as u8;
            apply_srd_image_alpha_stencil_packet_fields(
                &mut packet,
                state.field_10,
                state.field_14,
                state.field_18,
                0,
                &mut renderer_counter,
            );
            let initial_key = packet
                .srd_quad_shader_key(initial_slots.texture_present())
                .srd_simple_shader_direct_contributions()
                .map_err(|error| format!("unsupported Simple mapping: {error:?}"))?
                .compact_key();
            let initial_location = format!(
                "{} SCN[{}]/LAYR[{}]/NODE[{}] is_2d={} presence={:?}",
                path.display(),
                target.scene_index,
                target.layer_index,
                node_index,
                is_2d,
                initial_slots.texture_present()
            );
            static_summary.node_context_count += 1;
            static_summary.type_counts[match cast_type {
                Some(1) => 0,
                Some(2) => 1,
                Some(4) => 2,
                _ => unreachable!(),
            }] += 1;
            *static_summary
                .initial_key_counts
                .entry(initial_key)
                .or_insert(0) += 1;
            if embedded_simple_shader_pair(&initial_key).is_none() {
                static_summary
                    .initial_unpacked
                    .entry(initial_key)
                    .or_insert_with(|| initial_location.clone());
            }
            if !collection.contains(&initial_key) {
                static_summary
                    .initial_outside_collection
                    .entry(initial_key)
                    .or_insert_with(|| initial_location.clone());
            }
            if ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id())).alpha_test_enabled {
                static_summary.alpha_test_count += 1;
                static_summary
                    .alpha_test_keys
                    .entry(initial_key)
                    .and_modify(|(count, _)| *count += 1)
                    .or_insert_with(|| (1, initial_location.clone()));
                static_summary.alpha_test_outside_collection +=
                    usize::from(!collection.contains(&initial_key));
                static_summary
                    .alpha_test_first
                    .get_or_insert_with(|| initial_location.clone());
            }
            if packet.flags_0c & 0x100 != 0 {
                static_summary.stencil_count += 1;
                static_summary
                    .stencil_first
                    .get_or_insert_with(|| initial_location.clone());
            }
            let animated_channels = layer
                .animations
                .iter()
                .flat_map(|animation| &animation.motions)
                .filter(|motion| motion.target == node_index as i32)
                .flat_map(|motion| &motion.tracks)
                .fold([false; 2], |mut channels, track| {
                    if track.target == 17 {
                        channels[0] = true;
                    } else if track.target == 20 {
                        channels[1] = true;
                    }
                    channels
                });
            let reference_has_texture =
                [image.crefs.as_slice(), image.cre1s.as_slice()].map(|references| {
                    references.iter().any(|reference| {
                        usize::try_from(reference.image_index)
                            .is_ok_and(|index| index < document.textures.textures.len())
                    })
                });
            let possible_presence = std::array::from_fn::<_, 2, _>(|slot| {
                let initial = initial_slots.slots[slot].is_some();
                if animated_channels[slot] {
                    [true, reference_has_texture[slot]]
                } else {
                    [!initial, initial]
                }
            });
            for slot_0 in [false, true]
                .into_iter()
                .filter(|present| possible_presence[0][usize::from(*present)])
            {
                for slot_1 in [false, true]
                    .into_iter()
                    .filter(|present| possible_presence[1][usize::from(*present)])
                {
                    let key = packet
                        .srd_quad_shader_key([slot_0, slot_1, false])
                        .srd_simple_shader_direct_contributions()
                        .map_err(|error| format!("unsupported Simple mapping: {error:?}"))?
                        .compact_key();
                    *key_counts.entry(key).or_insert(0) += 1;
                    let location = || {
                        format!(
                            "{} SCN[{}]/LAYR[{}]/NODE[{}] is_2d={} presence={:?}",
                            path.display(),
                            target.scene_index,
                            target.layer_index,
                            node_index,
                            is_2d,
                            [slot_0, slot_1, false]
                        )
                    };
                    if embedded_simple_shader_pair(&key).is_none() {
                        unpacked.entry(key).or_insert_with(location);
                    }
                    if !collection.contains(&key) {
                        outside_collection.entry(key).or_insert_with(location);
                    }
                }
            }
        }
    }
    Ok(())
}

fn srd_state(
    draw: &EvidenceCompleteRuntimeCastDraw,
) -> (&EvidenceCompleteSrdDraw, &'static str, usize) {
    match draw {
        EvidenceCompleteRuntimeCastDraw::Image(state) => (state, "Image", 0),
        EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => (&cell.draw, "Slice", 1),
        EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) => (&glyph.draw, "Number", 2),
        EvidenceCompleteRuntimeCastDraw::Fennel(_) => unreachable!("SRD-only builder omits Fennel"),
    }
}

fn describe(first: &FirstOccurrence) -> String {
    format!(
        "{} SCN[{}]/ANMS[{}] frame={} LAYR[{}]/NODE[{}] {}",
        first.path.display(),
        first.scene_index,
        first.animation_set_index,
        first.frame,
        first.layer_index,
        first.node_index,
        first.kind
    )
}

fn collect_srd_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_srd_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "srd") {
            output.push(path);
        }
    }
    Ok(())
}

fn load_simple_collection(
    game_data_root: &Path,
) -> Result<BTreeSet<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH]>, Box<dyn Error>> {
    let xml = fs::read_to_string(game_data_root.join("A000/shader/shadercollect.xml"))?;
    let mut inside_simple_group = false;
    let mut keys = BTreeSet::new();
    for line in xml.lines().map(str::trim) {
        if line.starts_with("<SimpleShaderVSSimpleShaderPS_") {
            inside_simple_group = true;
            continue;
        }
        if line.starts_with("</SimpleShaderVSSimpleShaderPS_") {
            inside_simple_group = false;
            continue;
        }
        if !inside_simple_group || !line.starts_with('<') || !line.ends_with("/>") {
            continue;
        }
        let key = line.as_bytes()[1..line.len() - 2].try_into()?;
        keys.insert(key);
    }
    if keys.len() != 82 {
        return Err(format!("expected 82 Simple keys, found {}", keys.len()).into());
    }
    Ok(keys)
}
