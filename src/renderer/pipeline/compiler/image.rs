use super::*;
pub(super) fn build_image_draw_for_runtime_cast(
    textures: &TextureList,
    site: RuntimeCastSite<'_>,
    state: RuntimeCastState,
    geometry: FrameGeometry<'_>,
) -> Result<Option<SrdDraw>, SrdDrawError> {
    let RuntimeCastSite {
        owner,
        source,
        layer,
        node_index,
        mut renderer_layer_key,
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
    let identity = identity_matrix4x4_game();
    let Some(image) = layer.image_by_node[node_index]
        .as_ref()
        .filter(|image| !image.creates_text_cast())
    else {
        return Ok(None);
    };
    if !world.visible || !world.render_gate {
        return Ok(None);
    }

    let vertex_transform = if is_2d { identity } else { camera_bridge };
    let transform = SrdTransform::from_scene(
        vertex_transform,
        snapshot.target_projection_view,
        snapshot.target_screen_size,
    );

    let preset =
        select_srd_image_render_preset(image.flags, image_state.render_preset_override, false)
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] does not select a proven render preset",
                    source.scene_index, source.layer_index
                ))
            })?;
    let slots = image
        .resolve_texture_slots(
            &image_state,
            textures,
            ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            [false; 2],
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let mut texture_bindings = [None; 3];
    for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
        match slots.slots[slot_index] {
            Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] texture slot {slot_index} has no proven sampler",
                    source.scene_index, source.layer_index
                ))
            })?;
                *destination = Some(SrdTextureBinding {
                    texture_index,
                    sampler,
                });
            }
            Some(SrdTextureBindingSource::ExplicitOverride) => continue,
            None => {}
        }
    }
    if slots
        .slots
        .iter()
        .any(|slot| matches!(slot, Some(SrdTextureBindingSource::ExplicitOverride)))
    {
        return Ok(None);
    }

    let mut renderer_counter = renderer_layer_key as u8;
    let state = SrdDrawState::surface(
        SrdSurfaceStateInput {
            transform,
            transform_mode: SimpleTransformMode::from_is_2d(is_2d),
            render_preset: preset,
            image_field_0c: image_state.field_0c,
            image_field_10: image_state.field_10,
            image_field_14: image_state.field_14,
            image_field_18: image_state.field_18,
            textures: texture_bindings,
        },
        &mut renderer_counter,
    )
    .map_err(|error| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}]: {error}",
            source.scene_index, source.layer_index
        ))
    })?;
    renderer_layer_key = (renderer_layer_key & !0xff) | u32::from(renderer_counter);

    let local_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions;
    let positions = local_positions.map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen)? {
        return Ok(None);
    }
    let quad = build_image_render_quad(
        image,
        ImageRenderQuadInput {
            positions,
            color_state: image_state.coordinate_state(ImageReferenceChannel::Cref),
            coordinates: slots.channels,
            multiplicative_tint: world.multiply_color,
            additive_tint: world.additive_color,
        },
    );
    Ok(Some(SrdDraw {
        origin: DrawOrigin {
            owner,
            scene_index: source.scene_index,
            layer_index: source.layer_index,
            node_index,
        },
        order: DrawOrder { renderer_layer_key },
        geometry: quad,
        state,
    }))
}
pub(super) fn build_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
    runtime_layers: Option<&[ProjectLayerRuntimeState]>,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
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
    let camera_bridge = project_camera_bridge(project, snapshot);
    let project_screen = project_screen_matrix(snapshot, camera_bridge)?;
    let identity = identity_matrix4x4_game();
    let mut draws = Vec::new();

    for (layer_index, layer) in scene.layers.iter().enumerate() {
        let cast_layer_keys = layer
            .compose_runtime_cast_layer_keys(snapshot.renderer_layer_key)
            .map_err(|error| SrdDrawError(error.to_string()))?;
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

        for (node_index, (cast_layer_key, node)) in cast_layer_keys
            .iter()
            .copied()
            .zip(&layer.nodes)
            .enumerate()
        {
            let Some(image) = layer.image_by_node[node_index]
                .as_ref()
                .filter(|image| !image.creates_text_cast())
            else {
                continue;
            };
            let cast_world = layer_worlds.casts[node_index];
            if !cast_world.render_gate {
                continue;
            }

            let vertex_transform = if is_2d { identity } else { camera_bridge };
            let transform = SrdTransform::from_scene(
                vertex_transform,
                snapshot.target_projection_view,
                snapshot.target_screen_size,
            );

            let image_state = runtime_layer.map_or_else(
                || {
                    let mut image_state = image.initial_runtime_state();
                    if let Some(ext_param) = layer.ext_param_for_node(node_index) {
                        image_state.render_preset_override = ext_param.render_preset_override;
                    }
                    image_state
                },
                |runtime_layer| runtime_layer.image_states[node_index],
            );
            let preset = select_srd_image_render_preset(
            image.flags,
            image_state.render_preset_override,
            false,
        )
        .ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] does not select a proven render preset"
            ))
        })?;
            let slots = image
                .resolve_texture_slots(
                    &image_state,
                    textures,
                    ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                    [false; 2],
                )
                .map_err(|error| SrdDrawError(error.to_string()))?;
            let mut texture_bindings = [None; 3];
            for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
                match slots.slots[slot_index] {
                    Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                        let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                        SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] texture slot {slot_index} has no proven sampler"
                        ))
                    })?;
                        *destination = Some(SrdTextureBinding {
                            texture_index,
                            sampler,
                        });
                    }
                    Some(SrdTextureBindingSource::ExplicitOverride) => continue,
                    None => {}
                }
            }
            if slots
                .slots
                .iter()
                .any(|slot| matches!(slot, Some(SrdTextureBindingSource::ExplicitOverride)))
            {
                continue;
            }

            let mut renderer_layer_key =
                cast_layer_key.wrapping_add(u32::from(node.render_layer_offset()));
            let mut renderer_counter = renderer_layer_key as u8;
            let state = SrdDrawState::surface(
                SrdSurfaceStateInput {
                    transform,
                    transform_mode: SimpleTransformMode::from_is_2d(is_2d),
                    render_preset: preset,
                    image_field_0c: image_state.field_0c,
                    image_field_10: image_state.field_10,
                    image_field_14: image_state.field_14,
                    image_field_18: image_state.field_18,
                    textures: texture_bindings,
                },
                &mut renderer_counter,
            )
            .map_err(|error| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}]: {error}"
                ))
            })?;
            renderer_layer_key = (renderer_layer_key & !0xff) | u32::from(renderer_counter);

            let local_positions = image
                .build_quad_with_geometry(image_state.geometry, is_2d)
                .positions;
            let positions =
                local_positions.map(|point| cast_world.matrix.transform_point_game(point));
            if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen.as_ref())?
            {
                continue;
            }
            let quad = build_image_render_quad(
                image,
                ImageRenderQuadInput {
                    positions,
                    color_state: image_state.coordinate_state(ImageReferenceChannel::Cref),
                    coordinates: slots.channels,
                    multiplicative_tint: cast_world.multiply_color,
                    additive_tint: cast_world.additive_color,
                },
            );
            draws.push(SrdDraw {
                origin: DrawOrigin {
                    owner: ReferenceLayerParent::ProjectLayer(crate::scene::ReferenceTarget {
                        scene_index,
                        layer_index,
                    }),
                    scene_index,
                    layer_index,
                    node_index,
                },
                order: DrawOrder { renderer_layer_key },
                geometry: quad,
                state,
            });
        }
    }
    Ok(draws)
}
