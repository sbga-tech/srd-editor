use std::env;
use std::error::Error;
use std::path::PathBuf;

use srd_editor::animation::{KeyData, Track};
use srd_editor::document::{EditorDocument, display_srd_name};
use srd_editor::game_host::{ProjectTargetSnapshot, WorldSnapshot};
use srd_editor::projection::identity_matrix4x4_game;
use srd_editor::renderer::{build_animation_set_image_draws, build_base_pose_image_draws};
use srd_editor::transform::Affine3x4;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: srd-inspect <file.srd> [--initial-draws=WIDTHxHEIGHT|--animation-set-draws=INDEX[:FRAME]@WIDTHxHEIGHT]")?;
    let draw_selection = arguments
        .next()
        .map(|argument| parse_draw_selection(&argument.to_string_lossy()))
        .transpose()?;
    if arguments.next().is_some() {
        return Err("usage: srd-inspect <file.srd> [--initial-draws=WIDTHxHEIGHT|--animation-set-draws=INDEX[:FRAME]@WIDTHxHEIGHT]".into());
    }
    let document = EditorDocument::load(&path)?;

    println!(
        "file={}",
        document.path().expect("loaded document path").display()
    );
    println!(
        "project={:?} scenes={} fonts={} textures={}",
        display_srd_name(&document.project.name),
        document.project.scenes.len(),
        document.project.fonts.len(),
        document.textures.textures.len()
    );
    for (font_index, font) in document.project.fonts.iter().enumerate() {
        println!(
            "FONT[{font_index}] name={:?} flags_70={:?} field_71={:?} characters={}",
            display_srd_name(&font.name),
            font.flags_70,
            font.field_71,
            font.characters.len()
        );
    }

    for (scene_index, scene) in document.project.scenes.iter().enumerate() {
        println!(
            "SCN[{scene_index}] name={:?} size={}x{} layers={} animation_sets={}",
            display_srd_name(&scene.name),
            scene.width,
            scene.height,
            scene.layers.len(),
            scene.animation_sets.len()
        );
        for (set_index, set) in scene.animation_sets.iter().enumerate() {
            println!(
                "  ANMS[{set_index}] name={:?} start_frame={} runtime_duration={} slots={}",
                display_srd_name(&set.name),
                set.start_frame,
                set.runtime_duration,
                set.slots.len()
            );
            for (layer_index, slot) in set.slots.iter().enumerate() {
                println!(
                    "    SANM[{layer_index}] enabled={} animation={:?}",
                    slot.is_enabled(),
                    display_srd_name(&slot.animation_name)
                );
            }
        }
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            println!(
                "  LAYR[{layer_index}] name={:?} flags={:#010x} nodes={} animations={}",
                display_srd_name(&layer.name),
                layer.flags,
                layer.nodes.len(),
                layer.animations.len()
            );
            for (node_index, (node, transform)) in
                layer.nodes.iter().zip(&layer.transforms).enumerate()
            {
                let transform = transform.spatial();
                println!(
                    "    NODE[{node_index}] name={:?} type={:?} child={} sibling={} translation={:?} rotation={:?} scale={:?} visible={}",
                    node.name.as_deref().map(display_srd_name),
                    node.cast_type(),
                    node.first_child_index,
                    node.next_sibling_index,
                    transform.translation,
                    transform.rotation,
                    transform.scale,
                    transform.is_visible()
                );
                if let Some(text) = layer.image_by_node[node_index]
                    .as_ref()
                    .and_then(|image| image.text.as_ref())
                {
                    println!(
                        "      TEXT font_index={:?} text={:?} field_78={:?} field_36={:?} field_7b={:?} field_7c={:?} field_41={:?}",
                        text.font_index,
                        display_srd_name(&text.text),
                        text.field_78,
                        text.field_36,
                        text.field_7b,
                        text.field_7c,
                        text.field_41
                    );
                }
            }
            for (animation_index, animation) in layer.animations.iter().enumerate() {
                println!(
                    "    ANIM[{animation_index}] name={:?} flags={:#010x} duration={} runtime_duration={} motions={}",
                    display_srd_name(&animation.name),
                    animation.flags,
                    animation.duration,
                    animation.runtime_duration(),
                    animation.motions.len()
                );
                for (motion_index, motion) in animation.motions.iter().enumerate() {
                    if motion.target < 0 && motion.tracks.is_empty() {
                        continue;
                    }
                    println!(
                        "      MOT[{motion_index}] target={} tracks={}",
                        motion.target,
                        motion.tracks.len()
                    );
                    for (track_index, track) in motion.tracks.iter().enumerate() {
                        println!(
                            "        TRK[{track_index}] target={} format={:#010x} range={}..{} keys={} {}",
                            track.target,
                            track.format,
                            track.range_start,
                            track.range_end,
                            track.key_count,
                            key_summary(track)
                        );
                    }
                }
            }
        }
    }

    if let Some(draw_selection) = draw_selection {
        let screen_size = draw_selection.screen_size();
        let host = WorldSnapshot::new(
            Affine3x4::IDENTITY,
            srd_editor::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            Some(ProjectTargetSnapshot::new(
                identity_matrix4x4_game(),
                screen_size,
            )),
            identity_matrix4x4_game(),
            screen_size,
        );
        let draws = match draw_selection {
            DrawSelection::Initial { .. } => {
                build_base_pose_image_draws(&document.project, &document.textures, 0, host)?
            }
            DrawSelection::AnimationSet { index, frame, .. } => {
                let animation_set = document.project.scenes[0]
                    .animation_sets
                    .get(index)
                    .ok_or("animation-set draw index is outside SCN[0]")?;
                build_animation_set_image_draws(
                    &document.project,
                    &document.textures,
                    0,
                    index,
                    frame.unwrap_or(animation_set.start_frame) as f32,
                    host,
                )?
            }
        };
        println!(
            "INITIAL_DRAWS selection={} diagnostic_first_calc=identity diagnostic_projection_view=identity screen={}x{} count={}",
            draw_selection.label(),
            screen_size[0],
            screen_size[1],
            draws.len()
        );
        for (draw_index, draw) in draws.iter().enumerate() {
            let layer =
                &document.project.scenes[draw.origin.scene_index].layers[draw.origin.layer_index];
            let node = &layer.nodes[draw.origin.node_index];
            let min_x = draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position[0])
                .fold(f32::INFINITY, f32::min);
            let min_y = draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::INFINITY, f32::min);
            let max_x = draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position[0])
                .fold(f32::NEG_INFINITY, f32::max);
            let max_y = draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            let textures = draw.state.material.textures.map(|binding| {
                binding
                    .map(|binding| binding.texture_index as isize)
                    .unwrap_or(-1)
            });
            println!(
                "  DRAW[{draw_index}] layer={}:{} node={}:{} renderer_key={:#010x} profile={:?} pipeline={:?} bbox=({min_x},{min_y})..({max_x},{max_y}) color0={:02X?} color1={:02X?} textures={textures:?}",
                draw.origin.layer_index,
                display_srd_name(&layer.name),
                draw.origin.node_index,
                display_srd_name(node.name.as_deref().unwrap_or_default()),
                draw.order.renderer_layer_key,
                draw.state.profile,
                draw.state.pipeline,
                draw.geometry.vertices[0].primary_color,
                draw.geometry.vertices[0].secondary_color,
            );
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum DrawSelection {
    Initial {
        screen_size: [u32; 2],
    },
    AnimationSet {
        index: usize,
        frame: Option<i32>,
        screen_size: [u32; 2],
    },
}

impl DrawSelection {
    fn screen_size(self) -> [u32; 2] {
        match self {
            Self::Initial { screen_size } | Self::AnimationSet { screen_size, .. } => screen_size,
        }
    }

    fn label(self) -> String {
        match self {
            Self::Initial { .. } => "initial-runtime-state".to_string(),
            Self::AnimationSet { index, frame, .. } => frame.map_or_else(
                || format!("ANMS[{index}]-start-frame"),
                |frame| format!("ANMS[{index}]-frame-{frame}"),
            ),
        }
    }
}

fn parse_draw_selection(argument: &str) -> Result<DrawSelection, Box<dyn Error>> {
    if let Some(value) = argument.strip_prefix("--initial-draws=") {
        return Ok(DrawSelection::Initial {
            screen_size: parse_screen_size(value)?,
        });
    }
    let value = argument.strip_prefix("--animation-set-draws=").ok_or(
        "expected --initial-draws=WIDTHxHEIGHT or --animation-set-draws=INDEX[:FRAME]@WIDTHxHEIGHT",
    )?;
    let (index_and_frame, screen_size) = value
        .split_once('@')
        .ok_or("animation-set draws must use INDEX[:FRAME]@WIDTHxHEIGHT")?;
    let (index, frame) = index_and_frame
        .split_once(':')
        .map_or((index_and_frame, None), |(index, frame)| {
            (index, Some(frame))
        });
    Ok(DrawSelection::AnimationSet {
        index: index.parse()?,
        frame: frame.map(str::parse).transpose()?,
        screen_size: parse_screen_size(screen_size)?,
    })
}

fn parse_screen_size(value: &str) -> Result<[u32; 2], Box<dyn Error>> {
    let (width, height) = value
        .split_once('x')
        .ok_or("draw screen size must use WIDTHxHEIGHT")?;
    let size = [width.parse::<u32>()?, height.parse::<u32>()?];
    if size.contains(&0) {
        return Err("draw screen size must be non-zero".into());
    }
    Ok(size)
}

fn key_summary(track: &Track) -> String {
    match &track.keys {
        KeyData::Key8F32(keys) => endpoint_summary(keys, |key| key.frame, |key| key.value),
        KeyData::Key8I32(keys) => endpoint_summary(keys, |key| key.frame, |key| key.value),
        KeyData::Key8Bytes4(keys) => endpoint_summary(keys, |key| key.frame, |key| key.value),
        KeyData::Key20F32(keys) => endpoint_summary(keys, |key| key.frame, |key| key.value),
        KeyData::Key20I32(keys) => endpoint_summary(keys, |key| key.frame, |key| key.value),
        KeyData::Unsupported => "unsupported-key-layout".into(),
    }
}

fn endpoint_summary<T, K>(keys: &[K], frame: impl Fn(&K) -> i32, value: impl Fn(&K) -> T) -> String
where
    T: std::fmt::Debug,
{
    let Some(first) = keys.first() else {
        return "no-keys".into();
    };
    let last = keys.last().unwrap();
    format!(
        "first=({}, {:?}) last=({}, {:?})",
        frame(first),
        value(first),
        frame(last),
        value(last)
    )
}
