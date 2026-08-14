use super::*;
pub(super) fn build_number_glyph_draws_for_runtime_cast(
    textures: &TextureList,
    site: RuntimeCastSite<'_>,
    state: RuntimeCastState,
    geometry: FrameGeometry<'_>,
) -> Result<Vec<NumberGlyphDraw>, SrdDrawError> {
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
    let Some(definition) = layer.number_by_node[node_index].as_ref() else {
        return Ok(Vec::new());
    };
    if !world.visible || !world.render_gate {
        return Ok(Vec::new());
    }

    let image = definition.image_base();
    let cast_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(cast_positions, is_2d, project_screen)? {
        return Ok(Vec::new());
    }

    let preset = select_srd_image_render_preset(
        image.flags,
        image_state.render_preset_override,
        false,
    )
    .ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast does not select a proven render preset",
            source.scene_index, source.layer_index
        ))
    })?;

    // `srd_render_number_cast` selects the texture pair once from the current
    // two SrImage descriptors before `srd_render_number_glyph` replaces only
    // channel 0's selector for per-glyph UV construction.
    let slots = image
        .resolve_texture_slots(
            &image_state,
            textures,
            crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            [false; 2],
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let mut texture_bindings = [None; 3];
    for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
        match slots.slots[slot_index] {
            Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast texture slot {slot_index} has no proven sampler",
                    source.scene_index, source.layer_index
                ))
            })?;
                *destination = Some(SrdTextureBinding {
                    texture_index,
                    sampler,
                });
            }
            Some(SrdTextureBindingSource::ExplicitOverride) => {
                return Err(SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast uses an unrecorded explicit texture override",
                    source.scene_index, source.layer_index
                )));
            }
            None => {}
        }
    }

    let second_coordinates = image
        .resolve_coordinates(
            ImageReferenceChannel::Cre1,
            image_state.coordinate_state(ImageReferenceChannel::Cre1),
            textures,
            crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let formatted = definition.initial_formatted_text();
    let records = definition.first_history_render_records(&formatted, is_2d);
    let vertex_transform = if is_2d { identity } else { camera_bridge };
    let transform = SrdTransform::from_scene(
        vertex_transform,
        snapshot.target_projection_view,
        snapshot.target_screen_size,
    );

    let renderer_layer_prefix = renderer_layer_key & !0xff;
    let mut renderer_counter = renderer_layer_key as u8;
    let mut draws = Vec::with_capacity(records.len());
    for NumberGlyphRecord {
        glyph_index,
        is_digit: _,
        segment,
        mut quad,
    } in records
    {
        if glyph_index < 0 {
            continue;
        }

        let mut first_state = image_state.coordinate_state(ImageReferenceChannel::Cref);
        first_state.reference_index = glyph_index;
        first_state.uses_explicit_rectangle = false;
        let first_coordinates = image
            .resolve_coordinates(
                ImageReferenceChannel::Cref,
                first_state,
                textures,
                crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
        quad.positions = quad
            .positions
            .map(|point| world.matrix.transform_point_game(point));
        let render_quad = build_image_render_quad(
            &definition.image_base(),
            ImageRenderQuadInput {
                positions: quad.positions,
                color_state: first_state,
                coordinates: [first_coordinates, second_coordinates],
                multiplicative_tint: world.multiply_color,
                additive_tint: world.additive_color,
            },
        );

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
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast: {error}",
                source.scene_index, source.layer_index
            ))
        })?;
        let glyph_renderer_layer_key = renderer_layer_prefix | u32::from(renderer_counter);
        draws.push(NumberGlyphDraw {
            glyph_index,
            segment,
            draw: SrdDraw {
                origin: DrawOrigin {
                    owner,
                    scene_index: source.scene_index,
                    layer_index: source.layer_index,
                    node_index,
                },
                order: DrawOrder {
                    renderer_layer_key: glyph_renderer_layer_key,
                },
                geometry: render_quad,
                state,
            },
        });
    }
    Ok(draws)
}
