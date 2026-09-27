use super::*;
impl ReferenceLayerInstance {
    pub fn compose_world_state(
        &self,
        parent_cast: RuntimeWorldState,
        local: ReferenceLayerLocalState,
    ) -> RuntimeWorldState {
        let local_matrix =
            build_local_matrix(&local.transform, self.is_2d, self.flip_y, [0.0, 0.0]);
        RuntimeWorldState {
            matrix: parent_cast.matrix.mul_game(local_matrix),
            multiply_color: multiply_color_game(
                parent_cast.multiply_color,
                local.transform.multiply_color,
            ),
            additive_color: add_color_saturating_game(
                parent_cast.additive_color,
                local.transform.additive_color,
            ),
            visible: parent_cast.visible && local.enabled,
            render_gate: parent_cast.render_gate && local.enabled,
        }
    }
}
impl ProjectRuntimeWorldStates {
    pub fn layer(&self, owner: ReferenceLayerParent) -> Option<&RuntimeLayerWorldStates> {
        match owner {
            ReferenceLayerParent::ProjectLayer(target) => self
                .project_layers
                .get(target.scene_index)?
                .get(target.layer_index),
            ReferenceLayerParent::ReferenceInstance(instance_index) => {
                self.references.get(instance_index)
            }
        }
    }
}
impl ProjectRuntime {
    /// Reproduces the runtime update chain before render traversal: original
    /// layers use the host FirstCalcMatrix as their root, while every copied
    /// layer composes its independent local state under the owning RefCast.
    /// CAST colors and visibility follow `srd_compose_cast_world_state`; the
    /// render gate remains separate from transform visibility.
    pub fn compose_world_states(
        &self,
        project: &Project,
        first_calc_matrix: Affine3x4,
        renderer_inverse_camera_view: Affine3x4,
    ) -> Result<ProjectRuntimeWorldStates, SceneError> {
        let mut project_worlds = Vec::with_capacity(project.scenes.len());
        for (scene_index, scene) in project.scenes.iter().enumerate() {
            let runtime_scene = self.project_layers.get(scene_index).ok_or_else(|| {
                SceneError(format!(
                    "SCN[{scene_index}] is missing from the project runtime"
                ))
            })?;
            if runtime_scene.len() != scene.layers.len() {
                return Err(SceneError(format!(
                    "SCN[{scene_index}] has {} parsed layers but {} runtime layers",
                    scene.layers.len(),
                    runtime_scene.len()
                )));
            }
            let mut scene_worlds = Vec::with_capacity(scene.layers.len());
            for (layer_index, (layer, runtime_layer)) in
                scene.layers.iter().zip(runtime_scene).enumerate()
            {
                let source = ReferenceTarget {
                    scene_index,
                    layer_index,
                };
                let layer_world = RuntimeWorldState {
                    matrix: first_calc_matrix,
                    multiply_color: [255; 4],
                    additive_color: [0; 4],
                    visible: true,
                    render_gate: runtime_layer.enabled,
                };
                let casts = compose_runtime_cast_world_states(
                    layer,
                    &runtime_layer.cast_transforms,
                    layer_world,
                    layer.is_2d(),
                    renderer_inverse_camera_view,
                )?;
                scene_worlds.push(RuntimeLayerWorldStates {
                    owner: ReferenceLayerParent::ProjectLayer(source),
                    source,
                    is_2d: layer.is_2d(),
                    layer: layer_world,
                    casts,
                });
            }
            project_worlds.push(scene_worlds);
        }

        let mut result = ProjectRuntimeWorldStates {
            project_layers: project_worlds,
            references: Vec::with_capacity(self.references.layers.len()),
        };
        for (instance_index, runtime_layer) in self.references.layers.iter().enumerate() {
            let instance = self
                .references
                .plan
                .instances
                .get(instance_index)
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference runtime layer {instance_index} has no construction plan entry"
                    ))
                })?;
            let parent_layer = result.layer(instance.parent).ok_or_else(|| {
                SceneError(format!(
                    "reference instance {instance_index} has an unavailable parent {:?}",
                    instance.parent
                ))
            })?;
            let parent_cast = *parent_layer
                .casts
                .get(instance.reference_node_index)
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference instance {instance_index} owning NODE {} is outside its parent CAST vector",
                        instance.reference_node_index
                    ))
                })?;
            let layer_world = instance.compose_world_state(parent_cast, runtime_layer.local);
            let layer = project
                .scenes
                .get(instance.target.scene_index)
                .and_then(|scene| scene.layers.get(instance.target.layer_index))
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference instance {instance_index} target SCN[{}]/LAYR[{}] is outside the project",
                        instance.target.scene_index, instance.target.layer_index
                    ))
                })?;
            let casts = compose_runtime_cast_world_states(
                layer,
                &runtime_layer.cast_transforms,
                layer_world,
                instance.is_2d,
                renderer_inverse_camera_view,
            )?;
            result.references.push(RuntimeLayerWorldStates {
                owner: ReferenceLayerParent::ReferenceInstance(instance_index),
                source: instance.target,
                is_2d: instance.is_2d,
                layer: layer_world,
                casts,
            });
        }
        Ok(result)
    }
}
pub(super) fn compose_runtime_cast_world_states(
    layer: &Layer,
    transforms: &[SpatialTransform],
    layer_world: RuntimeWorldState,
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
) -> Result<Vec<RuntimeWorldState>, SceneError> {
    if transforms.len() != layer.nodes.len() {
        return Err(SceneError(format!(
            "world composition needs {} transforms, got {}",
            layer.nodes.len(),
            transforms.len()
        )));
    }
    let offsets = layer.compute_parent_csli_offsets()?;
    let hierarchy = layer.build_hierarchy()?;
    let source = CastWorldSource {
        layer,
        transforms,
        offsets: &offsets,
        children: &hierarchy.children,
        is_2d,
        renderer_inverse_camera_view,
    };
    let mut result = vec![RuntimeWorldState::default(); layer.nodes.len()];
    for &root in &hierarchy.roots {
        compose_runtime_cast_world_state_node(&source, root, layer_world, layer_world, &mut result);
    }
    Ok(result)
}
struct CastWorldSource<'a> {
    layer: &'a Layer,
    transforms: &'a [SpatialTransform],
    offsets: &'a [[f32; 2]],
    children: &'a [Vec<usize>],
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
}

fn compose_runtime_cast_world_state_node(
    source: &CastWorldSource<'_>,
    index: usize,
    layer_world: RuntimeWorldState,
    parent: RuntimeWorldState,
    output: &mut [RuntimeWorldState],
) {
    let CastWorldSource {
        layer,
        transforms,
        offsets,
        children,
        is_2d,
        renderer_inverse_camera_view,
    } = source;
    let flags = layer.nodes[index].type_flags.unwrap_or(0);
    let local = transforms[index];
    let local_matrix = build_local_matrix(&local, *is_2d, false, offsets[index]);
    let matrix = compose_cast_matrix_game(
        parent.matrix,
        local_matrix,
        flags,
        *is_2d,
        *renderer_inverse_camera_view,
    );
    let visible =
        local.is_visible() && layer_world.visible && (flags & 0x400 == 0 || parent.visible);
    let world = RuntimeWorldState {
        matrix,
        multiply_color: if flags & 0x200 != 0 {
            multiply_color_game(parent.multiply_color, local.multiply_color)
        } else {
            local.multiply_color
        },
        additive_color: if flags & 0x0008_0000 != 0 {
            add_color_saturating_game(parent.additive_color, local.additive_color)
        } else {
            local.additive_color
        },
        visible,
        render_gate: visible && layer_world.render_gate && layer.nodes[index].active(),
    };
    output[index] = world;
    for &child in &children[index] {
        compose_runtime_cast_world_state_node(source, child, layer_world, world, output);
    }
    if flags & 0x0100_0000 != 0 && flags & 0x0007_0000 == 0x0001_0000 {
        output[index].matrix.rows[2][3] = 0.0;
    }
}
pub(super) fn compose_cast_matrix_game(
    parent: Affine3x4,
    local: Affine3x4,
    flags: u32,
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
) -> Affine3x4 {
    let mut world = parent.mul_game(local);
    let matrix_kind = flags & 0x0007_0000;
    if matrix_kind == 0 {
        return world;
    }

    let translation = [world.rows[0][3], world.rows[1][3], world.rows[2][3]];
    let inverse_parent = inverse_affine_via_matrix4x4_game(parent);
    if is_2d {
        world = inverse_parent.mul_game(world);
        if flags & 0x0100_0000 != 0 {
            let scale_x = (((parent.rows[0][0] * parent.rows[0][0])
                + (parent.rows[1][0] * parent.rows[1][0]))
                + parent.rows[2][0] * parent.rows[2][0])
                .sqrt();
            let scale_y = (((parent.rows[0][1] * parent.rows[0][1])
                + (parent.rows[1][1] * parent.rows[1][1]))
                + parent.rows[2][1] * parent.rows[2][1])
                .sqrt();
            let scale = Affine3x4 {
                rows: [
                    [scale_x, 0.0, 0.0, 0.0],
                    [0.0, scale_y, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                ],
            };
            world = scale.mul_game(world);
        }
        world = renderer_inverse_camera_view.mul_game(world);
    } else {
        world = match matrix_kind {
            0x0002_0000 => world.mul_game(inverse_parent),
            0x0003_0000 | 0x0004_0000 | 0x0005_0000 => world,
            _ => renderer_inverse_camera_view.mul_game(inverse_parent.mul_game(world)),
        };
    }
    world.rows[0][3] = translation[0];
    world.rows[1][3] = translation[1];
    world.rows[2][3] = translation[2];
    world
}
fn inverse_affine_via_matrix4x4_game(matrix: Affine3x4) -> Affine3x4 {
    let inverse = inverse_matrix4x4_game(&Matrix4x4 {
        rows: [
            matrix.rows[0],
            matrix.rows[1],
            matrix.rows[2],
            [0.0, 0.0, 0.0, 1.0],
        ],
    });
    Affine3x4 {
        rows: [inverse.rows[0], inverse.rows[1], inverse.rows[2]],
    }
}
