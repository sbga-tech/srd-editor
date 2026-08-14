use super::*;
impl RuntimeAnimationTreeApplication {
    fn include_layer(&mut self, application: RuntimeAnimationApplication) {
        self.animated_layers += 1;
        self.common_channels += application.common_channels;
        self.image_channels += application.image_channels;
    }

    fn include_tree(&mut self, child: Self) {
        self.animated_layers += child.animated_layers;
        self.common_channels += child.common_channels;
        self.image_channels += child.image_channels;
        self.reference_requests += child.reference_requests;
    }
}
impl ReferenceLayerRuntimeState {
    pub fn new(
        project: &Project,
        plan: &ReferenceRuntimePlan,
        instance_index: usize,
    ) -> Option<Self> {
        let instance = plan.instances.get(instance_index)?;
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        let image_bases = runtime_image_bases(layer);
        let image_states = runtime_image_states(layer, &image_bases);
        Some(Self {
            instance_index,
            local: ReferenceLayerLocalState::default(),
            cast_transforms: layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect(),
            image_bases,
            image_states,
            animations: layer
                .animations
                .iter()
                .map(|animation| animation.initial_runtime_state())
                .collect(),
        })
    }

    pub fn apply_animation_common_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<usize>, crate::animation::AnimationError> {
        let instance = &plan.instances[self.instance_index];
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        let Some((animation_index, animation)) = layer.find_animation(animation_name) else {
            return Ok(None);
        };
        self.animations[animation_index].frame = frame;
        animation
            .apply_common_channels(&mut self.cast_transforms, frame)
            .map(Some)
    }

    pub fn apply_reference_request_common_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        request: ReferenceAnimationRequest<'_>,
    ) -> Result<Option<usize>, crate::animation::AnimationError> {
        self.apply_animation_common_channels(project, plan, request.animation_name, request.frame)
    }

    pub fn apply_animation_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        textures: &TextureList,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
        let instance = &plan.instances[self.instance_index];
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        apply_runtime_layer_channels(
            RuntimeLayerAnimation {
                layer,
                textures,
                animation_name,
                frame,
            },
            RuntimeLayerBuffers {
                cast_transforms: &mut self.cast_transforms,
                image_bases: &self.image_bases,
                image_states: &mut self.image_states,
                animations: &mut self.animations,
            },
        )
    }
}
fn runtime_image_bases(layer: &Layer) -> Vec<ImageDefinition> {
    layer
        .nodes
        .iter()
        .enumerate()
        .map(|(node_index, node)| match node.cast_type() {
            Some(1) => layer.image_by_node[node_index]
                .clone()
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            Some(2) => layer.csli_by_node[node_index]
                .as_ref()
                .map(ImageDefinition::from_csli_runtime_base)
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            Some(4) => layer.number_by_node[node_index]
                .as_ref()
                .map(crate::number::NumberDefinition::image_base)
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            _ => ImageDefinition::srimage_constructor_base(),
        })
        .collect()
}
fn runtime_image_states(layer: &Layer, image_bases: &[ImageDefinition]) -> Vec<RuntimeImageState> {
    image_bases
        .iter()
        .enumerate()
        .map(|(node_index, image)| {
            let mut state = image.initial_runtime_state();
            if let Some(ext_param) = layer.ext_param_for_node(node_index) {
                state.render_preset_override = ext_param.render_preset_override;
            }
            state
        })
        .collect()
}
struct RuntimeLayerAnimation<'a> {
    layer: &'a Layer,
    textures: &'a TextureList,
    animation_name: &'a [u8],
    frame: f32,
}

struct RuntimeLayerBuffers<'a> {
    cast_transforms: &'a mut [SpatialTransform],
    image_bases: &'a [ImageDefinition],
    image_states: &'a mut [RuntimeImageState],
    animations: &'a mut [RuntimeAnimationState],
}

fn apply_runtime_layer_channels(
    input: RuntimeLayerAnimation<'_>,
    buffers: RuntimeLayerBuffers<'_>,
) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
    let RuntimeLayerAnimation {
        layer,
        textures,
        animation_name,
        frame,
    } = input;
    let RuntimeLayerBuffers {
        cast_transforms,
        image_bases,
        image_states,
        animations,
    } = buffers;
    let Some((animation_index, animation)) = layer.find_animation(animation_name) else {
        return Ok(None);
    };
    animations[animation_index].frame = frame;

    let mut application = RuntimeAnimationApplication::default();
    let cast_count = cast_transforms.len();
    for motion in &animation.motions {
        if motion.target < 0 {
            continue;
        }
        let node_index = usize::try_from(motion.target).map_err(|_| {
            crate::animation::AnimationError("MOT target does not fit usize".into())
        })?;
        if node_index >= cast_count {
            return Err(crate::animation::AnimationError(format!(
                "MOT target {node_index} is outside {cast_count} runtime CASTs"
            )));
        }
        application.common_channels +=
            motion.apply_proven_common_channels(&mut cast_transforms[node_index], frame);
        let base = &image_bases[node_index];
        let state = &mut image_states[node_index];
        for track in &motion.tracks {
            if base
                .apply_runtime_track(state, track, frame, textures)
                .map_err(|error| crate::animation::AnimationError(error.to_string()))?
            {
                application.image_channels += 1;
            }
        }
    }
    Ok(Some(application))
}
fn child_animation_requests(
    layer: &Layer,
    animation_name: &[u8],
    frame: f32,
    plan: &ReferenceRuntimePlan,
    parent: ReferenceLayerParent,
) -> Vec<(usize, Vec<u8>, f32)> {
    let Some((_, animation)) = layer.find_animation(animation_name) else {
        return Vec::new();
    };
    let mut requests = Vec::new();
    for motion in &animation.motions {
        let Ok(node_index) = usize::try_from(motion.target) else {
            continue;
        };
        let Some(reference) = layer
            .reference_by_node
            .get(node_index)
            .and_then(|definition| definition.as_ref())
        else {
            continue;
        };
        let child_instance = plan.instances.iter().position(|instance| {
            instance.parent == parent && instance.reference_node_index == node_index
        });
        let Some(child_instance) = child_instance else {
            continue;
        };
        for track in &motion.tracks {
            let Some(request) = reference.animation_request(track, frame) else {
                continue;
            };
            requests.push((
                child_instance,
                request.animation_name.to_vec(),
                request.frame,
            ));
        }
    }
    requests
}
impl ReferenceRuntime {
    pub fn new(project: &Project) -> Result<Self, ReferenceRuntimeError> {
        let plan = project.build_reference_runtime_plan()?;
        let layers = (0..plan.instances.len())
            .map(|instance_index| {
                ReferenceLayerRuntimeState::new(project, &plan, instance_index)
                    .expect("instance index came from the same reference runtime plan")
            })
            .collect();
        Ok(Self { plan, layers })
    }

    pub fn apply_instance_animation(
        &mut self,
        project: &Project,
        textures: &TextureList,
        instance_index: usize,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationTreeApplication>, crate::animation::AnimationError> {
        if instance_index >= self.layers.len() {
            return Err(crate::animation::AnimationError(format!(
                "reference instance {instance_index} is outside {} runtime layers",
                self.layers.len()
            )));
        }

        let Some(application) = self.layers[instance_index].apply_animation_channels(
            project,
            &self.plan,
            textures,
            animation_name,
            frame,
        )?
        else {
            return Ok(None);
        };
        let mut tree = RuntimeAnimationTreeApplication::default();
        tree.include_layer(application);

        let target = self.plan.instances[instance_index].target;
        let layer = &project.scenes[target.scene_index].layers[target.layer_index];
        let child_requests = child_animation_requests(
            layer,
            animation_name,
            frame,
            &self.plan,
            ReferenceLayerParent::ReferenceInstance(instance_index),
        );

        for (child_instance, child_animation_name, child_frame) in child_requests {
            tree.reference_requests += 1;
            if let Some(child) = self.apply_instance_animation(
                project,
                textures,
                child_instance,
                &child_animation_name,
                child_frame,
            )? {
                tree.include_tree(child);
            }
        }
        Ok(Some(tree))
    }
}
impl ProjectLayerRuntimeState {
    fn new(target: ReferenceTarget, layer: &Layer) -> Self {
        let image_bases = runtime_image_bases(layer);
        let image_states = runtime_image_states(layer, &image_bases);
        Self {
            target,
            enabled: layer.flags & 0x100 != 0,
            cast_transforms: layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect(),
            image_bases,
            image_states,
            animations: layer
                .animations
                .iter()
                .map(|animation| animation.initial_runtime_state())
                .collect(),
        }
    }

    fn apply_animation_channels(
        &mut self,
        layer: &Layer,
        textures: &TextureList,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
        apply_runtime_layer_channels(
            RuntimeLayerAnimation {
                layer,
                textures,
                animation_name,
                frame,
            },
            RuntimeLayerBuffers {
                cast_transforms: &mut self.cast_transforms,
                image_bases: &self.image_bases,
                image_states: &mut self.image_states,
                animations: &mut self.animations,
            },
        )
    }
}
impl ProjectRuntime {
    pub fn new(project: &Project) -> Result<Self, ReferenceRuntimeError> {
        let project_layers = project
            .scenes
            .iter()
            .enumerate()
            .map(|(scene_index, scene)| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(layer_index, layer)| {
                        ProjectLayerRuntimeState::new(
                            ReferenceTarget {
                                scene_index,
                                layer_index,
                            },
                            layer,
                        )
                    })
                    .collect()
            })
            .collect();
        Ok(Self {
            project_layers,
            references: ReferenceRuntime::new(project)?,
        })
    }

    pub fn apply_layer_animation(
        &mut self,
        project: &Project,
        textures: &TextureList,
        target: ReferenceTarget,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationTreeApplication>, crate::animation::AnimationError> {
        let layer = project
            .scenes
            .get(target.scene_index)
            .and_then(|scene| scene.layers.get(target.layer_index))
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{}]/LAYR[{}] is outside the project runtime",
                    target.scene_index, target.layer_index
                ))
            })?;
        let runtime_layer = self
            .project_layers
            .get_mut(target.scene_index)
            .and_then(|scene| scene.get_mut(target.layer_index))
            .expect("project runtime layers mirror the parsed project");
        let Some(application) =
            runtime_layer.apply_animation_channels(layer, textures, animation_name, frame)?
        else {
            return Ok(None);
        };
        let mut tree = RuntimeAnimationTreeApplication::default();
        tree.include_layer(application);

        let child_requests = child_animation_requests(
            layer,
            animation_name,
            frame,
            &self.references.plan,
            ReferenceLayerParent::ProjectLayer(target),
        );

        for (child_instance, child_animation_name, child_frame) in child_requests {
            tree.reference_requests += 1;
            if let Some(child) = self.references.apply_instance_animation(
                project,
                textures,
                child_instance,
                &child_animation_name,
                child_frame,
            )? {
                tree.include_tree(child);
            }
        }
        Ok(Some(tree))
    }

    pub fn apply_animation_set(
        &mut self,
        project: &Project,
        textures: &TextureList,
        scene_index: usize,
        animation_set_index: usize,
        frame: f32,
    ) -> Result<RuntimeAnimationTreeApplication, crate::animation::AnimationError> {
        let scene = project.scenes.get(scene_index).ok_or_else(|| {
            crate::animation::AnimationError(format!(
                "SCN[{scene_index}] is outside the project runtime"
            ))
        })?;
        let animation_set = scene
            .animation_sets
            .get(animation_set_index)
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{scene_index}]/ANMS[{animation_set_index}] is outside the scene"
                ))
            })?;
        self.apply_animation_set_definition(project, textures, scene_index, animation_set, frame)
    }

    /// Applies an arbitrary dense ANMS assignment to one scene's runtime state.
    ///
    /// Slot `n` controls layer `n`: it always overwrites that runtime layer's
    /// enabled flag, and a non-empty animation name evaluates that layer's
    /// animation at `frame`. Slots beyond the scene's layers are ignored. On a
    /// freshly constructed runtime, layers beyond the assignment retain base state.
    pub fn apply_animation_set_definition(
        &mut self,
        project: &Project,
        textures: &TextureList,
        scene_index: usize,
        animation_set: &AnimationSetDefinition,
        frame: f32,
    ) -> Result<RuntimeAnimationTreeApplication, crate::animation::AnimationError> {
        let layer_count = self
            .project_layers
            .get(scene_index)
            .map(Vec::len)
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{scene_index}] is outside the project runtime"
                ))
            })?;
        let mut result = RuntimeAnimationTreeApplication::default();

        for (layer_index, slot) in animation_set.slots.iter().take(layer_count).enumerate() {
            self.project_layers[scene_index][layer_index].enabled = slot.is_enabled();
            if slot.animation_name.is_empty() {
                continue;
            }
            if let Some(application) = self.apply_layer_animation(
                project,
                textures,
                ReferenceTarget {
                    scene_index,
                    layer_index,
                },
                &slot.animation_name,
                frame,
            )? {
                result.include_tree(application);
            }
        }
        Ok(result)
    }
}
