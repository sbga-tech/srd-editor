use super::*;
pub(super) fn validate_world_snapshot(snapshot: WorldSnapshot) -> Result<(), SrdDrawError> {
    if let Some(target) = snapshot.project_target
        && target.render_size.contains(&0)
    {
        return Err(SrdDrawError(format!(
            "project target size must be non-zero, got {}x{}",
            target.render_size[0], target.render_size[1]
        )));
    }
    if snapshot.target_screen_size.contains(&0) {
        return Err(SrdDrawError(format!(
            "target screen size must be non-zero, got {}x{}",
            snapshot.target_screen_size[0], snapshot.target_screen_size[1]
        )));
    }
    Ok(())
}
pub(super) fn project_camera_bridge(project: &Project, snapshot: WorldSnapshot) -> Matrix4x4 {
    let Some(target) = snapshot.project_target else {
        return identity_matrix4x4_game();
    };
    let external_inverse = inverse_matrix4x4_game(&target.projection_view);
    let project_projection_view = project
        .camera
        .runtime_matrices(target.render_size[0] as f32)
        .projection_view;
    mul_matrix4x4_game(&external_inverse, &project_projection_view)
}
pub(super) fn inverse_camera_view(project: &Project, snapshot: WorldSnapshot) -> Affine3x4 {
    let Some(target) = snapshot.project_target else {
        return Affine3x4::IDENTITY;
    };
    let view = project
        .camera
        .runtime_matrices(target.render_size[0] as f32)
        .view;
    Affine3x4 {
        rows: [view.rows[0], view.rows[1], view.rows[2]],
    }
    .inverse_game()
}
pub(super) fn project_screen_matrix(
    snapshot: WorldSnapshot,
    camera_bridge: Matrix4x4,
) -> Result<Option<(Matrix4x4, [i32; 2])>, SrdDrawError> {
    let Some(target) = snapshot.project_target else {
        return Ok(None);
    };
    let width = i32::try_from(target.render_size[0]).map_err(|_| {
        SrdDrawError(format!(
            "project target width {} exceeds the game's signed i32 domain",
            target.render_size[0]
        ))
    })?;
    let height = i32::try_from(target.render_size[1]).map_err(|_| {
        SrdDrawError(format!(
            "project target height {} exceeds the game's signed i32 domain",
            target.render_size[1]
        ))
    })?;
    Ok(Some((
        compose_screen_matrix_game(width, height, &target.projection_view, &camera_bridge),
        [width, height],
    )))
}
pub(super) fn runtime_cast_passes_renderer_visibility(
    positions: [[f32; 3]; 4],
    is_2d: bool,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
) -> Result<bool, SrdDrawError> {
    let Some((screen_matrix, [width, height])) = project_screen else {
        // The binary reads SrRenderer+0x24C even on this proven null-target
        // path, but those four rectangle floats were never initialized. Keep
        // the editor deterministic and memory-safe instead of inventing a
        // zero/present/receiving-target rectangle.
        return Ok(true);
    };
    cast_overlaps_render_target_game(
        &positions,
        is_2d,
        (!is_2d).then_some(screen_matrix),
        *width,
        *height,
    )
    .map_err(|error| SrdDrawError(error.to_string()))
}
pub(super) fn runtime_image_state_for_owner(
    runtime: &ProjectRuntime,
    owner: ReferenceLayerParent,
    node_index: usize,
) -> Result<crate::image::RuntimeImageState, SrdDrawError> {
    let states = match owner {
        ReferenceLayerParent::ProjectLayer(target) => runtime
            .project_layers
            .get(target.scene_index)
            .and_then(|scene| scene.get(target.layer_index))
            .map(|layer| layer.image_states.as_slice()),
        ReferenceLayerParent::ReferenceInstance(instance_index) => runtime
            .references
            .layers
            .get(instance_index)
            .map(|layer| layer.image_states.as_slice()),
    }
    .ok_or_else(|| SrdDrawError(format!("runtime draw owner {owner:?} is unavailable")))?;
    states.get(node_index).copied().ok_or_else(|| {
        SrdDrawError(format!(
            "runtime draw owner {owner:?} has no image state for NODE[{node_index}]"
        ))
    })
}
