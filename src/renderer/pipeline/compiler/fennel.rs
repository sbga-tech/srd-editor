use super::*;
/// Builds the initial RFZ TextCast subset whose layout, texture batching,
/// 2D/3D vertex and matrix paths, world transform and color inputs are all
/// supported. Legacy `.sbfont` stays outside this runtime path.
///
/// `font_registry` is the process-global slot state after this player's
/// requests have been applied. It may already contain earlier host resources;
/// the function uses it for both the TextCast primary slot and explicit
/// `$F[n]` switches. Runtime fonts retain caller-defined opaque texture tokens,
/// allowing the renderer to route batches across different font atlases.
#[derive(Debug, Clone, PartialEq)]
pub struct FennelSrdRuntimeTextInput {
    pub substitutions: [Vec<u8>; 8],
    pub default_d: i32,
    pub repeat_space_count: i32,
    /// Exact current SrTextCast text-state field `+0xF4`. Callers may advance
    /// it with `FennelSrdScrollState::advance`; this API does not infer a game
    /// tick rate from editor animation frames.
    pub field_f4: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FennelRuntimeTextCastKey {
    pub owner: ReferenceLayerParent,
    pub node_index: usize,
}
impl Default for FennelSrdRuntimeTextInput {
    fn default() -> Self {
        // Empty slots are an editor form default, not a claim about an
        // arbitrary game's current SrTextCast host strings. The initial draw
        // wrapper still rejects serialized substitution tokens without an
        // explicit input entry.
        Self {
            substitutions: std::array::from_fn(|_| Vec::new()),
            default_d: FENNEL_DEFAULT_D_VALUE,
            repeat_space_count: FENNEL_DEFAULT_REPEAT_SPACE_COUNT,
            field_f4: 0.0,
        }
    }
}
pub fn build_base_pose_fennel_draws(
    project: &Project,
    scene_index: usize,
    snapshot: WorldSnapshot,
    resources: FennelRenderResources<'_>,
) -> Result<Vec<FennelDraw>, SrdDrawError> {
    build_fennel_draws(project, scene_index, snapshot, resources, None)
}
pub fn build_base_pose_reference_fennel_draws(
    project: &Project,
    scene_index: usize,
    snapshot: WorldSnapshot,
    resources: FennelRenderResources<'_>,
) -> Result<Vec<FennelDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_reference_fennel_draws_from_runtime(project, scene_index, snapshot, &runtime, resources)
}
fn build_reference_fennel_draws_from_runtime(
    project: &Project,
    scene_index: usize,
    snapshot: WorldSnapshot,
    runtime: &ProjectRuntime,
    resources: FennelRenderResources<'_>,
) -> Result<Vec<FennelDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = inverse_camera_view(project, snapshot);
    let worlds = runtime
        .compose_world_states(
            project,
            snapshot.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }
    let camera_bridge = project_camera_bridge(project, snapshot);
    let project_screen = project_screen_matrix(snapshot, camera_bridge)?;
    let geometry = FrameGeometry {
        snapshot,
        camera_bridge,
        project_screen: project_screen.as_ref(),
    };
    let mut draws = Vec::new();
    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, snapshot.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(runtime, entry.owner, entry.node_index)?;
        let site = RuntimeCastSite {
            owner: entry.owner,
            source: entry.source,
            layer,
            node_index: entry.node_index,
            renderer_layer_key: entry.renderer_layer_key,
        };
        let state = RuntimeCastState {
            is_2d: layer_worlds.is_2d,
            world: cast_world,
            image: image_state,
        };
        let cast_resources = FennelCastResources {
            font_slots: resources.font_slots,
            fonts: resources.fonts,
            force_color_update: resources.force_color_update,
            text_input: resources.text_inputs.get(&FennelRuntimeTextCastKey {
                owner: entry.owner,
                node_index: entry.node_index,
            }),
        };
        if let Some(draw) =
            build_fennel_draw_for_runtime_cast(project, site, state, geometry, cast_resources)?
        {
            draws.push(draw);
        }
    }
    Ok(draws)
}
fn build_fennel_draws(
    project: &Project,
    scene_index: usize,
    snapshot: WorldSnapshot,
    resources: FennelRenderResources<'_>,
    runtime_layers: Option<&[ProjectLayerRuntimeState]>,
) -> Result<Vec<FennelDraw>, SrdDrawError> {
    let FennelRenderResources {
        font_slots: font_registry,
        fonts: runtime_fonts,
        force_color_update,
        text_inputs: runtime_text_inputs,
    } = resources;
    validate_world_snapshot(snapshot)?;
    let mut composition_runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    if let Some(runtime_layers) = runtime_layers {
        let destination = composition_runtime
            .project_layers
            .get_mut(scene_index)
            .ok_or_else(|| {
                SrdDrawError(format!("scene index {scene_index} is outside the runtime"))
            })?;
        if destination.len() != runtime_layers.len() {
            return Err(SrdDrawError(format!(
                "SCN[{scene_index}] has {} runtime layers, expected {}",
                runtime_layers.len(),
                destination.len()
            )));
        }
        destination.clone_from_slice(runtime_layers);
    }
    let renderer_inverse_camera_view = inverse_camera_view(project, snapshot);
    let composed_worlds = composition_runtime
        .compose_world_states(
            project,
            snapshot.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let scene = project
        .scenes
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the project")))?;
    let identity = identity_matrix4x4_game();
    let camera_bridge = project_camera_bridge(project, snapshot);
    let project_screen = project_screen_matrix(snapshot, camera_bridge)?;
    let mut draws = Vec::new();

    for (layer_index, layer) in scene.layers.iter().enumerate() {
        let runtime_layer = runtime_layers
            .map(|layers| {
                layers.get(layer_index).ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}] is missing from the runtime"
                    ))
                })
            })
            .transpose()?;
        let layer_enabled = runtime_layer
            .map(|runtime_layer| runtime_layer.enabled)
            .unwrap_or(layer.flags & 0x100 != 0);
        if !layer_enabled {
            continue;
        }
        let is_2d = layer.is_2d();
        let layer_worlds = composed_worlds
            .project_layers
            .get(scene_index)
            .and_then(|layers| layers.get(layer_index))
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}] has no composed runtime world state"
                ))
            })?;
        let cast_layer_keys = layer
            .compose_runtime_cast_layer_keys(snapshot.renderer_layer_key)
            .map_err(|error| SrdDrawError(error.to_string()))?;

        for (node_index, (cast_layer_key, node)) in cast_layer_keys
            .iter()
            .copied()
            .zip(&layer.nodes)
            .enumerate()
        {
            let Some(image) = layer.image_by_node[node_index]
                .as_ref()
                .filter(|image| image.creates_text_cast())
            else {
                continue;
            };
            let Some(text) = image.text.as_ref() else {
                continue;
            };
            let cast_world = layer_worlds.casts[node_index];
            if !cast_world.render_gate {
                continue;
            }
            let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
            SrdDrawError(format!(
                "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] has an invalid RFZ font index"
            ))
        })?;
            let font = project.fonts.get(font_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] RFZ font index {font_index} is outside PROJ"
            ))
        })?;
            if !font
                .name
                .iter()
                .map(u8::to_ascii_lowercase)
                .collect::<Vec<_>>()
                .ends_with(b".rfz")
            {
                continue;
            }
            if !runtime_fonts.contains_key(font.name.as_slice()) {
                return Err(SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] runtime RFZ {:?} is not loaded",
                    String::from_utf8_lossy(&font.name)
                )));
            }
            let primary_slot = font_registry
            .request_for_key(&font.name)
            .filter(|request| request.registered)
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] RFZ {:?} has no registered global Fennel slot",
                    String::from_utf8_lossy(&font.name)
                ))
            })?
            .font_slot_id;
            let image_state = runtime_layer.map_or_else(
                || image.initial_runtime_state(),
                |runtime_layer| runtime_layer.image_states[node_index],
            );
            let positions = image
                .build_quad_with_geometry(image_state.geometry, is_2d)
                .positions
                .map(|point| cast_world.matrix.transform_point_game(point));
            if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen.as_ref())?
            {
                continue;
            }
            let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
            if font_param.vertical {
                return Err(SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] uses the unported FontParamData vertical correction"
                )));
            }
            let properties = FennelStaticTextProperties::from_text_definition(
                text,
                image_state.geometry.size[0],
                image_state.geometry.size[1],
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let source_colors = image_state
                .coordinate_state(ImageReferenceChannel::Cref)
                .vertex_colors;
            let primary_rgba = [0usize, 2, 1, 3].map(|source_index| {
                multiply_color_game(source_colors[source_index], cast_world.multiply_color)
            });
            let secondary_rgba = premultiply_additive_color_game(cast_world.additive_color);
            if !force_color_update
                && !secondary_rgba[..3].iter().any(|component| *component != 0)
                && !primary_rgba.iter().any(|color| color[3] != 0)
            {
                // `sub_AC5740` skips the entire TextBox update/draw block when
                // its external +0x11C bit 0x100 is clear and all effective
                // primary alpha / secondary RGB channels are zero.
                continue;
            }
            let primary_colors = primary_rgba.map(pack_fennel_record_color);
            let secondary_color = pack_fennel_record_color(secondary_rgba);
            let font_style = fennel_srd_font_style(font_param);
            let effect_color = fennel_font_param_effect_color(font_param.shadow_color);
            let effect_offset = [font_param.shadow_x as f32, font_param.shadow_y as f32];
            let runtime_text_input = runtime_text_inputs.get(&FennelRuntimeTextCastKey {
                owner: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                    scene_index,
                    layer_index,
                }),
                node_index,
            });
            if runtime_text_input.is_none() {
                for substitution_index in 0..8u8 {
                    let token = [b'$', b'[', b'0' + substitution_index, b']'];
                    if text
                        .text
                        .windows(token.len())
                        .any(|candidate| candidate == token)
                    {
                        return Err(SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] uses runtime substitution {:?}, but this draw has no SrTextCast substitution host input",
                            String::from_utf8_lossy(&token)
                        )));
                    }
                }
            }
            let substitutions: [&[u8]; 8] = runtime_text_input
                .map_or([b"".as_slice(); 8], |input| {
                    std::array::from_fn(|index| input.substitutions[index].as_slice())
                });
            let default_d =
                runtime_text_input.map_or(FENNEL_DEFAULT_D_VALUE, |input| input.default_d);
            let repeat_space_count = runtime_text_input
                .map_or(FENNEL_DEFAULT_REPEAT_SPACE_COUNT, |input| {
                    input.repeat_space_count
                });
            let mut prepared_text =
                prepare_fennel_srd_runtime_text(&text.text, substitutions, default_d, font_param)
                    .map_err(|error| SrdDrawError(error.to_string()))?;
            if let Some(input) = runtime_text_input {
                prepared_text.scroll_state.field_f4 = input.field_f4;
            }
            let scroll = prepared_text.scroll_state.outputs();
            let build_layout = |source: &[u8], explicit_flags: Option<u32>| {
                let mut stream = build_fennel_plain_record_stream_with_font_slots(
                    source,
                    primary_slot,
                    properties.glyph_placement(0, font_style.record_flags, primary_colors),
                    2048,
                    |font_slot_id, code| {
                        let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                        let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                        Some(FennelResolvedGlyph {
                            glyph_token: fennel_slot_code_token(font_slot_id, code),
                            glyph: *runtime_font.glyph(code)?,
                        })
                    },
                )
                .map_err(|error| SrdDrawError(error.to_string()))?;
                let layout = if let Some(textbox_flags) = explicit_flags {
                    layout_fennel_static_srd_explicit_flags(
                        &mut stream,
                        properties.layout,
                        textbox_flags,
                        |token| {
                            let (font_slot_id, code) = fennel_slot_code_from_token(token);
                            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                            runtime_font.glyph(code).map(Into::into)
                        },
                    )
                } else {
                    layout_fennel_static_srd_font_param(
                        &mut stream,
                        properties.layout,
                        font_param,
                        |token| {
                            let (font_slot_id, code) = fennel_slot_code_from_token(token);
                            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                            runtime_font.glyph(code).map(Into::into)
                        },
                    )
                }
                .map_err(|error| SrdDrawError(error.to_string()))?;
                Ok::<_, SrdDrawError>((stream, layout))
            };
            let measure = |stream: &_| {
                measure_fennel_srd_text_size_mode0(
                    stream,
                    scroll.maximum_glyphs,
                    [properties.layout.scale_x, properties.layout.scale_y],
                    effect_offset,
                    |token| {
                        let (font_slot_id, code) = fennel_slot_code_from_token(token);
                        let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                        let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                        runtime_font.glyph(code).copied()
                    },
                )
                .map_err(|error| SrdDrawError(error.to_string()))
            };

            let (initial_stream, initial_layout) = build_layout(&prepared_text.text, None)?;
            let initial_text_size = measure(&initial_stream)?;
            let repeated_text = build_fennel_srd_repeated_text(
                &prepared_text.text,
                initial_layout.textbox_flags,
                repeat_space_count,
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let repeated = if let Some(repeated_text) = repeated_text {
                let (stream, layout) =
                    build_layout(&repeated_text, Some(initial_layout.textbox_flags))?;
                let size = measure(&stream)?;
                Some((stream, layout, size))
            } else {
                None
            };
            let preparation = prepare_fennel_srd_draw(FennelSrdDrawPreparationInput {
                textbox_flags: initial_layout.textbox_flags,
                text_size: initial_text_size,
                clip_size: [properties.layout.box_width, properties.layout.box_height],
                font_point_y: u16::from(font_style.point_y),
                scroll,
                repeated_text_size: repeated.as_ref().map(|(_, _, size)| *size),
            })
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let draw_textbox_flags = initial_layout.textbox_flags;
            let (stream, layout, draw_offset) = match preparation {
                FennelSrdDrawPreparation::DrawOffset(draw_offset) => {
                    if let Some((stream, layout, _)) = repeated {
                        (stream, layout, draw_offset)
                    } else {
                        (initial_stream, initial_layout, draw_offset)
                    }
                }
                FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags } => {
                    let (stream, layout) = build_layout(&prepared_text.text, Some(layout_flags))?;
                    (stream, layout, [0.0; 2])
                }
            };
            let vertex_build = build_fennel_normal_vertex_batches(
                &stream,
                scroll.maximum_glyphs,
                FennelNormalDrawInput {
                    is_2d,
                    textbox_position: [
                        -image_state.geometry.origin[0],
                        -image_state.geometry.origin[1],
                        0.0,
                    ],
                    textbox_scale: [properties.layout.scale_x, properties.layout.scale_y],
                    textbox_vertical_offset: layout.layout.textbox_vertical_offset,
                    textbox_transform: fennel_textbox_transform(
                        cast_world.matrix,
                        is_2d,
                        camera_bridge,
                    ),
                    secondary_color,
                    // `sub_7C04F0` restores the original word after a fit
                    // guard relayout and draws with that original clip state.
                    textbox_flags: draw_textbox_flags,
                    clip_size: [properties.layout.box_width, properties.layout.box_height],
                    draw_offset,
                    effect_colors: [effect_color; 4],
                    effect_offset,
                },
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).copied()
                },
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            if vertex_build.processed_glyph_count == 0 {
                continue;
            }
            let vertex_transform = if is_2d {
                identity
            } else {
                fennel_textbox_transform(cast_world.matrix, false, camera_bridge)
            };
            let transform = SrdTransform::from_scene(
                vertex_transform,
                snapshot.target_projection_view,
                snapshot.target_screen_size,
            );
            draws.push(FennelDraw {
                origin: DrawOrigin {
                    owner: ReferenceLayerParent::ProjectLayer(crate::scene::ReferenceTarget {
                        scene_index,
                        layer_index,
                    }),
                    scene_index,
                    layer_index,
                    node_index,
                },
                order: DrawOrder {
                    renderer_layer_key: cast_layer_key
                        .wrapping_add(u32::from(node.render_layer_offset())),
                },
                font_name: font.name.clone(),
                state: SrdDrawState::fennel(transform, SimpleTransformMode::from_is_2d(is_2d)),
                batches: vertex_build.batches,
            });
        }
    }
    Ok(draws)
}
pub(super) fn build_fennel_draw_for_runtime_cast(
    project: &Project,
    site: RuntimeCastSite<'_>,
    state: RuntimeCastState,
    geometry: FrameGeometry<'_>,
    resources: FennelCastResources<'_>,
) -> Result<Option<FennelDraw>, SrdDrawError> {
    let RuntimeCastSite {
        owner,
        source,
        layer,
        node_index,
        renderer_layer_key,
    } = site;
    let RuntimeCastState {
        is_2d,
        world,
        image: image_state,
    } = state;
    let FrameGeometry {
        snapshot,
        camera_bridge,
        project_screen,
    } = geometry;
    let FennelCastResources {
        font_slots: font_registry,
        fonts: runtime_fonts,
        force_color_update,
        text_input: runtime_text_input,
    } = resources;
    if !world.visible || !world.render_gate {
        return Ok(None);
    }
    let Some(image) = layer.image_by_node[node_index]
        .as_ref()
        .filter(|image| image.creates_text_cast())
    else {
        return Ok(None);
    };
    let Some(text) = image.text.as_ref() else {
        return Ok(None);
    };
    let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] has an invalid RFZ font index",
            source.scene_index, source.layer_index
        ))
    })?;
    let font = project.fonts.get(font_index).ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] RFZ font index {font_index} is outside PROJ",
            source.scene_index, source.layer_index
        ))
    })?;
    if !font
        .name
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .ends_with(b".rfz")
    {
        return Ok(None);
    }
    if !runtime_fonts.contains_key(font.name.as_slice()) {
        return Err(SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] runtime RFZ {:?} is not loaded",
            source.scene_index,
            source.layer_index,
            String::from_utf8_lossy(&font.name)
        )));
    }
    let primary_slot = font_registry
        .request_for_key(&font.name)
        .filter(|request| request.registered)
        .ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] RFZ {:?} has no registered global Fennel slot",
                source.scene_index,
                source.layer_index,
                String::from_utf8_lossy(&font.name)
            ))
        })?
        .font_slot_id;
    let positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen)? {
        return Ok(None);
    }
    let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
    if font_param.vertical {
        return Err(SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] uses the unported FontParamData vertical correction",
            source.scene_index, source.layer_index
        )));
    }
    let properties = FennelStaticTextProperties::from_text_definition(
        text,
        image_state.geometry.size[0],
        image_state.geometry.size[1],
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let source_colors = image_state
        .coordinate_state(ImageReferenceChannel::Cref)
        .vertex_colors;
    let primary_rgba = [0usize, 2, 1, 3]
        .map(|source_index| multiply_color_game(source_colors[source_index], world.multiply_color));
    let secondary_rgba = premultiply_additive_color_game(world.additive_color);
    if !force_color_update
        && !secondary_rgba[..3].iter().any(|component| *component != 0)
        && !primary_rgba.iter().any(|color| color[3] != 0)
    {
        return Ok(None);
    }
    let primary_colors = primary_rgba.map(pack_fennel_record_color);
    let secondary_color = pack_fennel_record_color(secondary_rgba);
    let font_style = fennel_srd_font_style(font_param);
    let effect_color = fennel_font_param_effect_color(font_param.shadow_color);
    let effect_offset = [font_param.shadow_x as f32, font_param.shadow_y as f32];
    if runtime_text_input.is_none() {
        for substitution_index in 0..8u8 {
            let token = [b'$', b'[', b'0' + substitution_index, b']'];
            if text
                .text
                .windows(token.len())
                .any(|candidate| candidate == token)
            {
                return Err(SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] uses runtime substitution {:?}, but this draw has no SrTextCast substitution host input",
                    source.scene_index,
                    source.layer_index,
                    String::from_utf8_lossy(&token)
                )));
            }
        }
    }
    let substitutions: [&[u8]; 8] = runtime_text_input.map_or([b"".as_slice(); 8], |input| {
        std::array::from_fn(|index| input.substitutions[index].as_slice())
    });
    let default_d = runtime_text_input.map_or(FENNEL_DEFAULT_D_VALUE, |input| input.default_d);
    let repeat_space_count = runtime_text_input
        .map_or(FENNEL_DEFAULT_REPEAT_SPACE_COUNT, |input| {
            input.repeat_space_count
        });
    let mut prepared_text =
        prepare_fennel_srd_runtime_text(&text.text, substitutions, default_d, font_param)
            .map_err(|error| SrdDrawError(error.to_string()))?;
    if let Some(input) = runtime_text_input {
        prepared_text.scroll_state.field_f4 = input.field_f4;
    }
    let scroll = prepared_text.scroll_state.outputs();
    let build_layout = |text_source: &[u8], explicit_flags: Option<u32>| {
        let mut stream = build_fennel_plain_record_stream_with_font_slots(
            text_source,
            primary_slot,
            properties.glyph_placement(0, font_style.record_flags, primary_colors),
            2048,
            |font_slot_id, code| {
                let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                Some(FennelResolvedGlyph {
                    glyph_token: fennel_slot_code_token(font_slot_id, code),
                    glyph: *runtime_font.glyph(code)?,
                })
            },
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
        let layout = if let Some(textbox_flags) = explicit_flags {
            layout_fennel_static_srd_explicit_flags(
                &mut stream,
                properties.layout,
                textbox_flags,
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).map(Into::into)
                },
            )
        } else {
            layout_fennel_static_srd_font_param(
                &mut stream,
                properties.layout,
                font_param,
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).map(Into::into)
                },
            )
        }
        .map_err(|error| SrdDrawError(error.to_string()))?;
        Ok::<_, SrdDrawError>((stream, layout))
    };
    let measure = |stream: &_| {
        measure_fennel_srd_text_size_mode0(
            stream,
            scroll.maximum_glyphs,
            [properties.layout.scale_x, properties.layout.scale_y],
            effect_offset,
            |token| {
                let (font_slot_id, code) = fennel_slot_code_from_token(token);
                let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                runtime_font.glyph(code).copied()
            },
        )
        .map_err(|error| SrdDrawError(error.to_string()))
    };

    let (initial_stream, initial_layout) = build_layout(&prepared_text.text, None)?;
    let initial_text_size = measure(&initial_stream)?;
    let repeated_text = build_fennel_srd_repeated_text(
        &prepared_text.text,
        initial_layout.textbox_flags,
        repeat_space_count,
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let repeated = if let Some(repeated_text) = repeated_text {
        let (stream, layout) = build_layout(&repeated_text, Some(initial_layout.textbox_flags))?;
        let size = measure(&stream)?;
        Some((stream, layout, size))
    } else {
        None
    };
    let preparation = prepare_fennel_srd_draw(FennelSrdDrawPreparationInput {
        textbox_flags: initial_layout.textbox_flags,
        text_size: initial_text_size,
        clip_size: [properties.layout.box_width, properties.layout.box_height],
        font_point_y: u16::from(font_style.point_y),
        scroll,
        repeated_text_size: repeated.as_ref().map(|(_, _, size)| *size),
    })
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let draw_textbox_flags = initial_layout.textbox_flags;
    let (stream, layout, draw_offset) = match preparation {
        FennelSrdDrawPreparation::DrawOffset(draw_offset) => {
            if let Some((stream, layout, _)) = repeated {
                (stream, layout, draw_offset)
            } else {
                (initial_stream, initial_layout, draw_offset)
            }
        }
        FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags } => {
            let (stream, layout) = build_layout(&prepared_text.text, Some(layout_flags))?;
            (stream, layout, [0.0; 2])
        }
    };
    let vertex_build = build_fennel_normal_vertex_batches(
        &stream,
        scroll.maximum_glyphs,
        FennelNormalDrawInput {
            is_2d,
            textbox_position: [
                -image_state.geometry.origin[0],
                -image_state.geometry.origin[1],
                0.0,
            ],
            textbox_scale: [properties.layout.scale_x, properties.layout.scale_y],
            textbox_vertical_offset: layout.layout.textbox_vertical_offset,
            textbox_transform: fennel_textbox_transform(world.matrix, is_2d, camera_bridge),
            secondary_color,
            textbox_flags: draw_textbox_flags,
            clip_size: [properties.layout.box_width, properties.layout.box_height],
            draw_offset,
            effect_colors: [effect_color; 4],
            effect_offset,
        },
        |token| {
            let (font_slot_id, code) = fennel_slot_code_from_token(token);
            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
            runtime_font.glyph(code).copied()
        },
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    if vertex_build.processed_glyph_count == 0 {
        return Ok(None);
    }
    let vertex_transform = if is_2d {
        identity_matrix4x4_game()
    } else {
        fennel_textbox_transform(world.matrix, false, camera_bridge)
    };
    let transform = SrdTransform::from_scene(
        vertex_transform,
        snapshot.target_projection_view,
        snapshot.target_screen_size,
    );
    Ok(Some(FennelDraw {
        origin: DrawOrigin {
            owner,
            scene_index: source.scene_index,
            layer_index: source.layer_index,
            node_index,
        },
        order: DrawOrder { renderer_layer_key },
        font_name: font.name.clone(),
        state: SrdDrawState::fennel(transform, SimpleTransformMode::from_is_2d(is_2d)),
        batches: vertex_build.batches,
    }))
}
const fn fennel_slot_code_token(font_slot_id: u16, code: u16) -> u32 {
    (font_slot_id as u32) << 16 | code as u32
}
const fn fennel_slot_code_from_token(token: u32) -> (u16, u16) {
    ((token >> 16) as u16, token as u16)
}
fn affine_to_matrix4x4(matrix: Affine3x4) -> Matrix4x4 {
    Matrix4x4 {
        rows: [
            matrix.rows[0],
            matrix.rows[1],
            matrix.rows[2],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}
/// Reproduces the matrix written to TextBoxObject `+0x2EC` by `sub_AC5740`.
/// The 2D path copies the CAST world matrix. The 3D path multiplies
/// `SrRenderer+0x08 * CAST world` and then negates the complete Y column.
pub(in crate::renderer::pipeline) fn fennel_textbox_transform(
    world: Affine3x4,
    is_2d: bool,
    camera_bridge: Matrix4x4,
) -> Matrix4x4 {
    let world = affine_to_matrix4x4(world);
    if is_2d {
        return world;
    }
    let mut result = mul_matrix4x4_game(&camera_bridge, &world);
    for row in &mut result.rows {
        row[1] *= -1.0;
    }
    result
}
fn pack_fennel_record_color([red, green, blue, alpha]: [u8; 4]) -> u32 {
    // `sub_AC5740 -> sub_F25DA0` stores record colors as AARRGGBB, whose
    // little-endian bytes are B,G,R,A.
    u32::from_le_bytes([blue, green, red, alpha])
}
