use super::*;
impl ReferenceRuntimePlan {
    /// Finds the original ProjectLayer whose RefCast expansion owns `owner`.
    ///
    /// A copied reference draw keeps its source layer identity for assets but
    /// inherits visibility and layer controls from this root ProjectLayer.
    pub fn root_project_layer(
        &self,
        mut owner: ReferenceLayerParent,
    ) -> Result<ReferenceTarget, ReferenceRuntimeAncestryError> {
        let mut visited_instances = BTreeSet::new();
        loop {
            match owner {
                ReferenceLayerParent::ProjectLayer(target) => return Ok(target),
                ReferenceLayerParent::ReferenceInstance(instance_index) => {
                    if !visited_instances.insert(instance_index) {
                        return Err(ReferenceRuntimeAncestryError::ReferenceCycle {
                            instance_index,
                        });
                    }
                    owner = self
                        .instances
                        .get(instance_index)
                        .ok_or(ReferenceRuntimeAncestryError::InvalidReferenceInstance {
                            instance_index,
                        })?
                        .parent;
                }
            }
        }
    }
    pub fn structural_cast_draw_order(
        &self,
        project: &Project,
        scene_index: usize,
        root_layer_key: u32,
    ) -> Result<Vec<RuntimeCastDrawOrderEntry>, SceneError> {
        let mut output = Vec::new();
        let Some(scene) = project.scenes.get(scene_index) else {
            return Ok(output);
        };
        for layer_index in 0..scene.layers.len() {
            let target = ReferenceTarget {
                scene_index,
                layer_index,
            };
            self.append_structural_cast_draw_order(
                project,
                ReferenceLayerParent::ProjectLayer(target),
                target,
                root_layer_key,
                &mut output,
            )?;
        }
        Ok(output)
    }

    fn append_structural_cast_draw_order(
        &self,
        project: &Project,
        owner: ReferenceLayerParent,
        source: ReferenceTarget,
        inherited_layer_key: u32,
        output: &mut Vec<RuntimeCastDrawOrderEntry>,
    ) -> Result<(), SceneError> {
        let layer = &project.scenes[source.scene_index].layers[source.layer_index];
        let cast_layer_keys = layer.compose_runtime_cast_layer_keys(inherited_layer_key)?;
        for (node_index, (cast_layer_key, node)) in cast_layer_keys
            .iter()
            .copied()
            .zip(&layer.nodes)
            .enumerate()
        {
            let node_offset = node.render_layer_offset();
            if layer
                .reference_by_node
                .get(node_index)
                .and_then(Option::as_ref)
                .is_some()
            {
                if let Some((instance_index, instance)) =
                    self.instances.iter().enumerate().find(|(_, instance)| {
                        instance.parent == owner && instance.reference_node_index == node_index
                    })
                {
                    self.append_structural_cast_draw_order(
                        project,
                        ReferenceLayerParent::ReferenceInstance(instance_index),
                        instance.target,
                        replace_low_byte_with_wrapping_sum(cast_layer_key, node_offset),
                        output,
                    )?;
                }
                // A RefCast itself submits no primitive. An unresolved
                // reference has a null copied-layer pointer and contributes
                // no recursive entries.
                continue;
            }
            output.push(RuntimeCastDrawOrderEntry {
                owner,
                source,
                node_index,
                renderer_layer_key: cast_layer_key.wrapping_add(u32::from(node_offset)),
            });
        }
        Ok(())
    }
}
/// `srd_update_cast_tree` passes a RefCast's key into its copied layer by
/// replacing only the low byte with the wrapping sum. Unlike the later render
/// wrapper, a carry from that byte is deliberately discarded here.
const fn replace_low_byte_with_wrapping_sum(key: u32, offset: u8) -> u32 {
    (key & !0xff) | ((key as u8).wrapping_add(offset) as u32)
}
impl Project {
    pub fn build_reference_runtime_plan(
        &self,
    ) -> Result<ReferenceRuntimePlan, ReferenceRuntimeError> {
        let mut plan = ReferenceRuntimePlan::default();
        let mut lineages = Vec::new();

        // The binary resolves all CASTs in the original runtime scene table before it
        // drains the queue of newly constructed reference layers.
        for (scene_index, scene) in self.scenes.iter().enumerate() {
            for (layer_index, layer) in scene.layers.iter().enumerate() {
                let source = ReferenceTarget {
                    scene_index,
                    layer_index,
                };
                append_layer_references(
                    self,
                    source,
                    ReferenceLayerParent::ProjectLayer(source),
                    layer.is_2d(),
                    &[source],
                    &mut plan,
                    &mut lineages,
                )?;
            }
        }

        // Each appended instance corresponds to one fresh runtime layer. Processing
        // the growing vector in order reproduces the binary's successive queue waves.
        let mut instance_index = 0usize;
        while instance_index < plan.instances.len() {
            let instance = plan.instances[instance_index].clone();
            let lineage = lineages[instance_index].clone();
            append_layer_references(
                self,
                instance.target,
                ReferenceLayerParent::ReferenceInstance(instance_index),
                instance.is_2d,
                &lineage,
                &mut plan,
                &mut lineages,
            )?;
            instance_index += 1;
        }

        Ok(plan)
    }
}
fn append_layer_references(
    project: &Project,
    source: ReferenceTarget,
    parent: ReferenceLayerParent,
    is_2d: bool,
    lineage: &[ReferenceTarget],
    plan: &mut ReferenceRuntimePlan,
    lineages: &mut Vec<Vec<ReferenceTarget>>,
) -> Result<(), ReferenceRuntimeError> {
    let layer = &project.scenes[source.scene_index].layers[source.layer_index];
    for (reference_node_index, reference) in layer.reference_by_node.iter().enumerate() {
        let Some(reference) = reference else {
            continue;
        };
        let Some(target) = project.resolve_reference(reference) else {
            plan.unresolved.push(UnresolvedReference {
                parent,
                reference_node_index,
                source_name: reference.source_name.clone(),
                layer_name: reference.layer_name.clone(),
            });
            continue;
        };

        if let Some(repeated_at) = lineage.iter().position(|entry| *entry == target) {
            let mut repeating_layers = lineage[repeated_at..].to_vec();
            repeating_layers.push(target);
            return Err(ReferenceRuntimeError { repeating_layers });
        }

        plan.instances.push(ReferenceLayerInstance {
            parent,
            reference_node_index,
            target,
            is_2d,
            flip_y: project.scenes[target.scene_index].layers[target.layer_index].is_2d() && !is_2d,
        });
        let mut child_lineage = lineage.to_vec();
        child_lineage.push(target);
        lineages.push(child_lineage);
    }
    Ok(())
}
