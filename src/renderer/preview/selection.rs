use super::*;
use std::collections::btree_map::Entry;
#[derive(Clone, Copy)]
struct RuntimeBoundsRequest {
    selection: PreviewCastSelection,
    kind: Option<PreviewHighlightKind>,
}

pub(super) fn runtime_cast_bounds<'a>(
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    layers: impl IntoIterator<Item = (&'a [RuntimeCastDraw], &'a [RuntimeTargetCommand])>,
    selection: Option<PreviewCastSelection>,
    highlights: &[PreviewHighlightRequest],
    composition_size: [u32; 2],
) -> (Option<[u32; 4]>, Vec<PreviewHighlightBounds>) {
    let mut requests = Vec::with_capacity(highlights.len() + selection.is_some() as usize);
    let selection_index = selection.map(|selection| {
        let index = requests.len();
        requests.push(RuntimeBoundsRequest {
            selection,
            kind: None,
        });
        index
    });
    requests.extend(highlights.iter().map(|highlight| RuntimeBoundsRequest {
        selection: highlight.selection,
        kind: Some(highlight.kind),
    }));

    let mut nodes_by_project_owner =
        BTreeMap::<(crate::scene::ReferenceTarget, usize), Vec<usize>>::new();
    let mut requests_by_reference_owner = BTreeMap::<usize, Vec<usize>>::new();
    for (request_index, request) in requests.iter().enumerate() {
        let target = crate::scene::ReferenceTarget {
            scene_index: request.selection.scene_index,
            layer_index: request.selection.layer_index,
        };
        let Some(layer) = project
            .scenes
            .get(request.selection.scene_index)
            .and_then(|scene| scene.layers.get(request.selection.layer_index))
        else {
            continue;
        };
        if request.selection.node_index >= layer.nodes.len() {
            continue;
        }

        let mut descendants = vec![false; layer.nodes.len()];
        descendants[request.selection.node_index] = true;
        if let Ok(hierarchy) = layer.build_hierarchy() {
            let mut stack = vec![request.selection.node_index];
            while let Some(node_index) = stack.pop() {
                for &child in hierarchy
                    .children
                    .get(node_index)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    if !descendants[child] {
                        descendants[child] = true;
                        stack.push(child);
                    }
                }
            }
        }
        for (node_index, included) in descendants.iter().copied().enumerate() {
            if included {
                nodes_by_project_owner
                    .entry((target, node_index))
                    .or_default()
                    .push(request_index);
            }
        }

        let mut included_instances = vec![false; reference_plan.instances.len()];
        for (instance_index, instance) in reference_plan.instances.iter().enumerate() {
            included_instances[instance_index] = match instance.parent {
                ReferenceLayerParent::ProjectLayer(parent) => {
                    parent == target
                        && descendants
                            .get(instance.reference_node_index)
                            .copied()
                            .unwrap_or(false)
                }
                ReferenceLayerParent::ReferenceInstance(parent_index) => included_instances
                    .get(parent_index)
                    .copied()
                    .unwrap_or(false),
            };
            if included_instances[instance_index] {
                requests_by_reference_owner
                    .entry(instance_index)
                    .or_default()
                    .push(request_index);
            }
        }
    }

    let mut bounds = vec![
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        requests.len()
    ];
    for (draws, submission) in layers {
        let mut admitted = vec![false; draws.len()];
        for source in submission
            .iter()
            .flat_map(|command| command.sources.iter().copied())
        {
            let runtime_draw_index = match source {
                RuntimeTargetCommandSource::Image { runtime_draw_index }
                | RuntimeTargetCommandSource::SliceCell { runtime_draw_index }
                | RuntimeTargetCommandSource::NumberGlyph { runtime_draw_index }
                | RuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index, ..
                } => runtime_draw_index,
            };
            if let Some(admitted) = admitted.get_mut(runtime_draw_index) {
                *admitted = true;
            }
        }

        for (runtime_draw_index, draw) in draws.iter().enumerate() {
            if !admitted[runtime_draw_index] {
                continue;
            }
            let request_indices = match draw.owner() {
                ReferenceLayerParent::ProjectLayer(owner) => nodes_by_project_owner
                    .get(&(owner, draw.node_index()))
                    .map(Vec::as_slice),
                ReferenceLayerParent::ReferenceInstance(instance_index) => {
                    requests_by_reference_owner
                        .get(&instance_index)
                        .map(Vec::as_slice)
                }
            };
            let Some(request_indices) = request_indices else {
                continue;
            };
            let Some(draw_bounds) = runtime_draw_bounds(draw, composition_size) else {
                continue;
            };
            for &request_index in request_indices {
                let request_bounds = &mut bounds[request_index];
                request_bounds[0] = request_bounds[0].min(draw_bounds[0]);
                request_bounds[1] = request_bounds[1].min(draw_bounds[1]);
                request_bounds[2] = request_bounds[2].max(draw_bounds[2]);
                request_bounds[3] = request_bounds[3].max(draw_bounds[3]);
            }
        }
    }

    let selection_bounds =
        selection_index.and_then(|index| clip_bounds(bounds[index], composition_size));
    let highlight_bounds = requests
        .iter()
        .zip(bounds)
        .filter_map(|(request, bounds)| {
            Some(PreviewHighlightBounds {
                kind: request.kind?,
                bounds: clip_bounds(bounds, composition_size)?,
            })
        })
        .collect();
    (selection_bounds, highlight_bounds)
}

fn runtime_draw_bounds(draw: &RuntimeCastDraw, composition_size: [u32; 2]) -> Option<[f32; 4]> {
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    match draw {
        RuntimeCastDraw::Image(draw) => accumulate_vertex_bounds(
            draw.geometry.vertices.iter().map(|vertex| vertex.position),
            draw.state
                .transform
                .clip_from_vertex(draw.state.profile.transform_mode),
            composition_size,
            &mut bounds,
        ),
        RuntimeCastDraw::SliceCell(cell) => accumulate_vertex_bounds(
            cell.draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position),
            cell.draw
                .state
                .transform
                .clip_from_vertex(cell.draw.state.profile.transform_mode),
            composition_size,
            &mut bounds,
        ),
        RuntimeCastDraw::NumberGlyph(glyph) => accumulate_vertex_bounds(
            glyph
                .draw
                .geometry
                .vertices
                .iter()
                .map(|vertex| vertex.position),
            glyph
                .draw
                .state
                .transform
                .clip_from_vertex(glyph.draw.state.profile.transform_mode),
            composition_size,
            &mut bounds,
        ),
        RuntimeCastDraw::Fennel(draw) => accumulate_vertex_bounds(
            draw.batches
                .iter()
                .flat_map(|batch| batch.vertices.iter())
                .map(|vertex| vertex.position),
            draw.state
                .transform
                .clip_from_vertex(draw.state.profile.transform_mode),
            composition_size,
            &mut bounds,
        ),
    }
    bounds
        .iter()
        .all(|value| value.is_finite())
        .then_some(bounds)
}

fn clip_bounds(bounds: [f32; 4], composition_size: [u32; 2]) -> Option<[u32; 4]> {
    if !bounds.iter().all(|value| value.is_finite()) {
        return None;
    }
    let left = bounds[0].floor().clamp(0.0, composition_size[0] as f32) as u32;
    let top = bounds[1].floor().clamp(0.0, composition_size[1] as f32) as u32;
    let right = bounds[2].ceil().clamp(0.0, composition_size[0] as f32) as u32;
    let bottom = bounds[3].ceil().clamp(0.0, composition_size[1] as f32) as u32;
    (right > left && bottom > top).then_some([left, top, right - left, bottom - top])
}

fn accumulate_vertex_bounds(
    positions: impl IntoIterator<Item = [f32; 3]>,
    clip_from_vertex: Matrix4x4,
    composition_size: [u32; 2],
    bounds: &mut [f32; 4],
) {
    for [x, y, z] in positions {
        let input = [x, y, z, 1.0];
        let clip = clip_from_vertex.rows.map(|row| {
            row[0] * input[0] + row[1] * input[1] + row[2] * input[2] + row[3] * input[3]
        });
        if clip[3] == 0.0 {
            continue;
        }
        let point = [
            (clip[0] / clip[3] + 1.0) * 0.5 * composition_size[0] as f32,
            (1.0 - clip[1] / clip[3]) * 0.5 * composition_size[1] as f32,
        ];
        if !point.iter().all(|value| value.is_finite()) {
            continue;
        }
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    }
}
pub(super) struct PreviewVisibilityFilter<'a> {
    pub(super) selected_scene_index: usize,
    pub(super) hidden_layers: &'a BTreeSet<usize>,
    pub(super) solo_layers: &'a BTreeSet<usize>,
    pub(super) hidden_casts: &'a BTreeSet<PreviewCastSelection>,
    pub(super) solo_casts: &'a BTreeSet<PreviewCastSelection>,
}

pub(super) fn filter_editor_preview_draws(
    draws: Vec<RuntimeCastDraw>,
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    filter: PreviewVisibilityFilter<'_>,
) -> Result<Vec<RuntimeCastDraw>, String> {
    let PreviewVisibilityFilter {
        selected_scene_index,
        hidden_layers,
        solo_layers,
        hidden_casts,
        solo_casts,
    } = filter;
    let mut visible = Vec::with_capacity(draws.len());
    let mut hierarchy_parents = BTreeMap::new();
    for draw in draws {
        let root = reference_plan
            .root_project_layer(draw.owner())
            .map_err(|error| format!("failed to resolve runtime draw owner ancestry: {error}"))?;
        if root.scene_index != selected_scene_index {
            return Err(format!(
                "runtime draw owner {:?} roots in scene {}, not selected scene {selected_scene_index}",
                draw.owner(),
                root.scene_index
            ));
        }
        if hidden_layers.contains(&root.layer_index)
            || (!solo_layers.is_empty() && !solo_layers.contains(&root.layer_index))
            || draw_matches_cast_selection(
                project,
                reference_plan,
                &draw,
                hidden_casts,
                &mut hierarchy_parents,
            )?
        {
            continue;
        }
        if !solo_casts.is_empty()
            && !draw_matches_cast_selection(
                project,
                reference_plan,
                &draw,
                solo_casts,
                &mut hierarchy_parents,
            )?
        {
            continue;
        }
        visible.push(draw);
    }
    Ok(visible)
}

fn draw_matches_cast_selection(
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    draw: &RuntimeCastDraw,
    selections: &BTreeSet<PreviewCastSelection>,
    hierarchy_parents: &mut BTreeMap<crate::scene::ReferenceTarget, Vec<Option<usize>>>,
) -> Result<bool, String> {
    let mut owner = draw.owner();
    let mut node_index = draw.node_index();
    let mut visited_instances = BTreeSet::new();
    loop {
        let (source, parent) = match owner {
            ReferenceLayerParent::ProjectLayer(source) => (source, None),
            ReferenceLayerParent::ReferenceInstance(instance_index) => {
                if !visited_instances.insert(instance_index) {
                    return Err(format!(
                        "reference instance ancestry cycles at {instance_index} while filtering CAST controls"
                    ));
                }
                let instance = reference_plan.instances.get(instance_index).ok_or_else(|| {
                    format!(
                        "reference instance {instance_index} is outside the runtime plan while filtering CAST controls"
                    )
                })?;
                (
                    instance.target,
                    Some((instance.parent, instance.reference_node_index)),
                )
            }
        };
        if selections.iter().any(|selection| {
            selection.scene_index == source.scene_index
                && selection.layer_index == source.layer_index
        }) {
            let parents = hierarchy_parent_indices(project, source, hierarchy_parents)?;
            if selections
                .iter()
                .filter(|selection| {
                    selection.scene_index == source.scene_index
                        && selection.layer_index == source.layer_index
                })
                .any(|selection| node_is_in_subtree(parents, selection.node_index, node_index))
            {
                return Ok(true);
            }
        }
        let Some((parent_owner, parent_node_index)) = parent else {
            return Ok(false);
        };
        owner = parent_owner;
        node_index = parent_node_index;
    }
}

fn hierarchy_parent_indices<'a>(
    project: &crate::scene::Project,
    target: crate::scene::ReferenceTarget,
    cache: &'a mut BTreeMap<crate::scene::ReferenceTarget, Vec<Option<usize>>>,
) -> Result<&'a [Option<usize>], String> {
    let parents = match cache.entry(target) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            let layer = project
                .scenes
                .get(target.scene_index)
                .and_then(|scene| scene.layers.get(target.layer_index))
                .ok_or_else(|| {
                    format!(
                        "runtime draw source layer ({}, {}) is missing while filtering CAST controls",
                        target.scene_index, target.layer_index
                    )
                })?;
            let parents = layer
                .build_hierarchy()
                .map_err(|error| {
                    format!("failed to build CAST hierarchy for preview filtering: {error}")
                })?
                .parents;
            entry.insert(parents)
        }
    };
    Ok(parents.as_slice())
}

fn node_is_in_subtree(parents: &[Option<usize>], ancestor: usize, mut node_index: usize) -> bool {
    for _ in 0..=parents.len() {
        if node_index == ancestor {
            return true;
        }
        let Some(parent) = parents.get(node_index).copied().flatten() else {
            return false;
        };
        node_index = parent;
    }
    false
}

pub(super) fn runtime_srd_draw(draw: &RuntimeCastDraw) -> Option<&SrdDraw> {
    match draw {
        RuntimeCastDraw::Image(draw) => Some(draw),
        RuntimeCastDraw::SliceCell(cell) => Some(&cell.draw),
        RuntimeCastDraw::NumberGlyph(glyph) => Some(&glyph.draw),
        RuntimeCastDraw::Fennel(_) => None,
    }
}
