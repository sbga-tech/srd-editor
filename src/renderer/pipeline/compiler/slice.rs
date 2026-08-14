use super::*;
pub(super) fn build_slice_cell_draws_for_runtime_cast(
    textures: &TextureList,
    site: RuntimeCastSite<'_>,
    state: RuntimeCastState,
    geometry: FrameGeometry<'_>,
) -> Result<Vec<SliceCellDraw>, SrdDrawError> {
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
    let identity = identity_matrix4x4_game();
    let Some(definition) = layer.csli_by_node[node_index].as_ref() else {
        return Ok(Vec::new());
    };
    if !world.visible || !world.render_gate {
        return Ok(Vec::new());
    }

    let image = ImageDefinition::from_csli_runtime_base(definition);
    let cast_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(cast_positions, is_2d, project_screen)? {
        return Ok(Vec::new());
    }

    let mut quads = definition
        .generate_active_quads_with_geometry(
            is_2d,
            image_state.geometry.size,
            image_state.geometry.origin,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let preset = select_srd_image_render_preset(
        image.flags,
        image_state.render_preset_override,
        false,
    )
    .ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] SliceCast does not select a proven render preset",
            source.scene_index, source.layer_index
        ))
    })?;

    let vertex_transform = if is_2d { identity } else { camera_bridge };
    let transform = SrdTransform::from_scene(
        vertex_transform,
        snapshot.target_projection_view,
        snapshot.target_screen_size,
    );

    let renderer_layer_prefix = renderer_layer_key & !0xff;
    let mut renderer_counter = renderer_layer_key as u8;
    let mut draws = Vec::with_capacity(quads.len());
    for mut slice_quad in quads.drain(..) {
        let cell_index = slice_quad.cell_index;
        let cell = definition.cells.get(cell_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] SliceCast cell {cell_index} is outside SLIC",
                source.scene_index, source.layer_index
            ))
        })?;

        let resolved_texture = textures.resolve_slice_cell(definition, cell_index);
        let (texture_coordinates, texture_binding) = match resolved_texture {
            Some(resolved) => (
                resolved.coordinates,
                Some(SrdTextureBinding {
                    texture_index: resolved.image_index,
                    sampler: resolved.samplers.select(image.point_sampled()),
                }),
            ),
            None => ([[0.0; 2]; 4], None),
        };

        let colors = slice_vertex_colors(
            definition,
            cell,
            slice_quad.normalized_cell_coordinates,
            world.multiply_color,
            world.additive_color,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
        slice_quad.positions = slice_quad
            .positions
            .map(|point| world.matrix.transform_point_game(point));
        let quad = build_slice_render_quad(
            SliceQuad {
                cell_index,
                ..slice_quad
            },
            colors,
            texture_coordinates,
        );

        let texture_bindings = [texture_binding, None, None];
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
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] SliceCast: {error}",
                source.scene_index, source.layer_index
            ))
        })?;
        let cell_renderer_layer_key = renderer_layer_prefix | u32::from(renderer_counter);
        draws.push(SliceCellDraw {
            cell_index,
            draw: SrdDraw {
                origin: DrawOrigin {
                    owner,
                    scene_index: source.scene_index,
                    layer_index: source.layer_index,
                    node_index,
                },
                order: DrawOrder {
                    renderer_layer_key: cell_renderer_layer_key,
                },
                geometry: quad,
                state,
            },
        });
    }
    Ok(draws)
}
