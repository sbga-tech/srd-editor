use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use srd_editor::document::{EditorDocument, display_srd_name};
use srd_editor::image::{ImageDefinition, SrdTextureBindingSource};
use srd_editor::number::NumberDefinition;
use srd_editor::render::{
    CeylonDrawPacketPresetState, apply_srd_image_field_0c_shader_bits,
    select_srd_image_render_preset,
};
use srd_editor::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let game_data_root = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: srd-render-audit <game-data-root> <file.srd>")?;
    let srd_path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: srd-render-audit <game-data-root> <file.srd>")?;
    if arguments.next().is_some() {
        return Err("usage: srd-render-audit <game-data-root> <file.srd>".into());
    }

    let collection = load_simple_collection(&game_data_root)?;
    let document = EditorDocument::load(&srd_path)?;
    let mut candidate_count = 0usize;
    let mut rejected_count = 0usize;

    let camera = document.project.camera;
    println!(
        "camera position={:?} target={:?} angle_units={} angle_degrees={} near={} far={}",
        camera.position,
        camera.target,
        camera.angle_units,
        camera.angle_degrees(),
        camera.near,
        camera.far,
    );
    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        println!(
            "scene={scene_index}:{} composition={}x{}",
            display_srd_name(&scene.name),
            scene.width,
            scene.height,
        );
    }

    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            for node_index in 0..layer.nodes.len() {
                let Some(image) = (match layer.nodes[node_index].cast_type() {
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
                let Some(preset) = select_srd_image_render_preset(
                    image.flags,
                    state.render_preset_override,
                    false,
                ) else {
                    continue;
                };
                let slots = image.resolve_texture_slots(
                    &state,
                    &document.textures,
                    ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                    [false; 2],
                )?;

                let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
                packet.set_render_preset_id(preset);
                packet.set_srd_quad_is_2d(layer.is_2d());
                apply_srd_image_field_0c_shader_bits(&mut packet, state.field_0c as i32);
                let shape_key = packet.srd_quad_shader_key(slots.texture_present());
                let simple_key = shape_key
                    .srd_simple_shader_direct_contributions()
                    .map_err(|error| format!("SRD packet has no direct Simple mapping: {error:?}"))?
                    .compact_key();
                if !collection.contains(&simple_key) {
                    rejected_count += 1;
                    continue;
                }

                candidate_count += 1;
                println!(
                    "candidate={candidate_count} scene={scene_index}:{} layer={layer_index}:{} node={node_index}:{} cast={} shape={:08X}:{:08X} simple={}",
                    display_srd_name(&scene.name),
                    display_srd_name(&layer.name),
                    display_srd_name(layer.nodes[node_index].name.as_deref().unwrap_or_default()),
                    layer.nodes[node_index].cast_type().unwrap_or(0),
                    shape_key.high,
                    shape_key.low,
                    String::from_utf8_lossy(&simple_key),
                );
                for (slot_index, source) in slots.slots.iter().enumerate() {
                    match source {
                        Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                            let texture = &document.textures.textures[*texture_index];
                            println!(
                                "  slot={slot_index} texture={texture_index} base={} declared={}x{}",
                                display_srd_name(&texture.filename),
                                texture.width,
                                texture.height,
                            );
                        }
                        Some(SrdTextureBindingSource::ExplicitOverride) => {
                            println!("  slot={slot_index} source=explicit-override");
                        }
                        None => {}
                    }
                }
            }
        }
    }

    eprintln!(
        "{}: exact direct-key candidates={candidate_count}, collection-rejected image nodes={rejected_count}",
        srd_path.display()
    );
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
