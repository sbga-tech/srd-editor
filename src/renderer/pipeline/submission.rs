use super::*;
/// Materializes one merged format-14/type-4 command using the exact connector
/// copies at `ceylon_enqueue_draw_packet` `0x670E19..0x670E3D`. At every
/// boundary the previous last vertex and next first vertex are duplicated,
/// producing `last,last,first,first` around the join.
pub fn materialize_runtime_srd_strip<'a>(
    draws: &'a [RuntimeCastDraw],
    command: &RuntimeTargetCommand,
    mut include: impl FnMut(RuntimeTargetCommandSource) -> bool,
) -> Result<Option<RuntimeSrdStrip<'a>>, SrdDrawError> {
    if command.topology != DrawTopology::TriangleStrip {
        return Err(SrdDrawError(
            "runtime target command is not an SRD triangle strip".to_string(),
        ));
    }

    let mut state = None;
    let mut vertices = Vec::new();
    let mut included_source_count = 0usize;
    for source in command
        .sources
        .iter()
        .copied()
        .filter(|source| include(*source))
    {
        let (runtime_draw_index, draw) = match source {
            RuntimeTargetCommandSource::Image { runtime_draw_index } => {
                let Some(RuntimeCastDraw::Image(draw)) = draws.get(runtime_draw_index) else {
                    return Err(SrdDrawError(format!(
                        "runtime target Image source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, draw)
            }
            RuntimeTargetCommandSource::SliceCell { runtime_draw_index } => {
                let Some(RuntimeCastDraw::SliceCell(cell)) = draws.get(runtime_draw_index) else {
                    return Err(SrdDrawError(format!(
                        "runtime target SliceCell source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, &cell.draw)
            }
            RuntimeTargetCommandSource::NumberGlyph { runtime_draw_index } => {
                let Some(RuntimeCastDraw::NumberGlyph(glyph)) = draws.get(runtime_draw_index)
                else {
                    return Err(SrdDrawError(format!(
                        "runtime target NumberGlyph source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, &glyph.draw)
            }
            RuntimeTargetCommandSource::FennelBatch { .. } => {
                return Err(SrdDrawError(
                    "SRD triangle strip contains a Fennel batch source".to_string(),
                ));
            }
        };
        if draw.state != command.state || draw.order != command.order {
            return Err(SrdDrawError(format!(
                "runtime draw {runtime_draw_index} no longer matches its merged command state"
            )));
        }
        if vertices.is_empty() {
            state = Some(draw);
        }
        append_srd_quad_strip_vertices(&mut vertices, &draw.geometry.vertices);
        included_source_count += 1;
    }

    let Some(state) = state else {
        return Ok(None);
    };
    if included_source_count == command.sources.len() && vertices.len() != command.vertex_count {
        return Err(SrdDrawError(format!(
            "materialized SRD strip has {} vertices but planner recorded {}",
            vertices.len(),
            command.vertex_count
        )));
    }
    Ok(Some(RuntimeSrdStrip { state, vertices }))
}
pub(super) fn append_srd_quad_strip_vertices(
    destination: &mut Vec<SrdVertex>,
    quad: &[SrdVertex; 4],
) {
    if let Some(previous_last) = destination.last().copied() {
        destination.push(previous_last);
        destination.push(quad[0]);
    }
    destination.extend_from_slice(quad);
}
/// Materializes one format-13/type-3 command. The binary merge branch at
/// `0x670E66` only adds the new vertex count because batch vertices are already
/// contiguous; no connector or reordering is introduced.
pub fn materialize_runtime_fennel_list<'a>(
    draws: &'a [RuntimeCastDraw],
    command: &RuntimeTargetCommand,
) -> Result<RuntimeFennelList<'a>, SrdDrawError> {
    if command.topology != DrawTopology::TriangleList {
        return Err(SrdDrawError(
            "runtime target command is not a Fennel triangle list".to_string(),
        ));
    }

    let mut state = None;
    let mut texture_token = None;
    let mut vertices = Vec::with_capacity(command.vertex_count);
    for source in command.sources.iter().copied() {
        let RuntimeTargetCommandSource::FennelBatch {
            runtime_draw_index,
            batch_index,
        } = source
        else {
            return Err(SrdDrawError(
                "Fennel triangle list contains an SRD quad source".to_string(),
            ));
        };
        let Some(RuntimeCastDraw::Fennel(draw)) = draws.get(runtime_draw_index) else {
            return Err(SrdDrawError(format!(
                "runtime target Fennel source {runtime_draw_index} does not match its draw"
            )));
        };
        let batch = draw.batches.get(batch_index).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime target Fennel batch {runtime_draw_index}/{batch_index} is outside its draw"
            ))
        })?;
        if draw.state != command.state || draw.order != command.order {
            return Err(SrdDrawError(format!(
                "runtime Fennel draw {runtime_draw_index} no longer matches its merged command state"
            )));
        }
        if let Some(expected) = texture_token {
            if batch.texture_token != expected {
                return Err(SrdDrawError(format!(
                    "merged Fennel command changes texture token from {expected:#010x} to {:#010x}",
                    batch.texture_token
                )));
            }
        } else {
            state = Some(draw);
            texture_token = Some(batch.texture_token);
        }
        vertices.extend_from_slice(&batch.vertices);
    }

    let state = state.ok_or_else(|| SrdDrawError("merged Fennel command is empty".to_string()))?;
    let texture_token = texture_token.expect("non-empty Fennel command has a texture token");
    if vertices.len() != command.vertex_count {
        return Err(SrdDrawError(format!(
            "materialized Fennel list has {} vertices but planner recorded {}",
            vertices.len(),
            command.vertex_count
        )));
    }
    Ok(RuntimeFennelList {
        state,
        texture_token,
        vertices,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeTextureIdentity {
    Srd(SrdTextureBinding),
    Fennel(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AdjacentMergeKey {
    pub(super) state: SrdDrawState,
    pub(super) order: DrawOrder,
    pub(super) topology: DrawTopology,
    pub(super) textures: [Option<MergeTextureIdentity>; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct AdjacentMergeCommand {
    pub(super) source: RuntimeTargetCommandSource,
    pub(super) key: AdjacentMergeKey,
    pub(super) vertex_count: usize,
}

/// Applies an already selected target's ScenePass/SceneModel profile to the
/// runtime CAST stream. The caller remains responsible for target admission.
pub fn build_runtime_target_submission(
    draws: &[RuntimeCastDraw],
    profile: &ScenePassProfile,
) -> Result<Vec<RuntimeTargetCommandSource>, SrdDrawError> {
    build_runtime_target_submission_impl(draws, profile, |_| true)
}

pub fn build_filtered_runtime_target_submission(
    draws: &[RuntimeCastDraw],
    profile: &ScenePassProfile,
    filter: SrdType1TargetFilter,
) -> Result<Vec<RuntimeTargetCommandSource>, SrdDrawError> {
    build_runtime_target_submission_impl(draws, profile, |state| filter.accepts(state))
}

/// Merges adjacent commands by canonical program, pipeline, material,
/// transform, and ordering state, then applies target admission and pass order.
pub fn build_runtime_target_commands(
    draws: &[RuntimeCastDraw],
    profile: &ScenePassProfile,
    filter: SrdType1TargetFilter,
) -> Result<Vec<RuntimeTargetCommand>, SrdDrawError> {
    let groups = merge_adjacent_commands(build_adjacent_merge_commands(draws));
    let mut admitted_groups = Vec::new();
    let mut queue_states = Vec::new();
    for group in groups {
        if filter.accepts(group.state.queue) {
            queue_states.push(group.state.queue);
            admitted_groups.push(group);
        }
    }

    let submission_indices = build_srd_scene_submission_indices(&queue_states, profile)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    Ok(submission_indices
        .into_iter()
        .map(|command_index| admitted_groups[command_index].clone())
        .collect())
}

fn build_adjacent_merge_commands(draws: &[RuntimeCastDraw]) -> Vec<AdjacentMergeCommand> {
    let mut commands = Vec::new();
    for (runtime_draw_index, draw) in draws.iter().enumerate() {
        match draw {
            RuntimeCastDraw::Image(draw) => commands.push(srd_merge_command(
                RuntimeTargetCommandSource::Image { runtime_draw_index },
                draw,
            )),
            RuntimeCastDraw::SliceCell(cell) => commands.push(srd_merge_command(
                RuntimeTargetCommandSource::SliceCell { runtime_draw_index },
                &cell.draw,
            )),
            RuntimeCastDraw::NumberGlyph(glyph) => commands.push(srd_merge_command(
                RuntimeTargetCommandSource::NumberGlyph { runtime_draw_index },
                &glyph.draw,
            )),
            RuntimeCastDraw::Fennel(draw) => {
                for (batch_index, batch) in draw.batches.iter().enumerate() {
                    commands.push(AdjacentMergeCommand {
                        source: RuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        },
                        key: AdjacentMergeKey {
                            state: draw.state,
                            order: draw.order,
                            topology: DrawTopology::TriangleList,
                            textures: [
                                Some(MergeTextureIdentity::Fennel(batch.texture_token)),
                                None,
                                None,
                            ],
                        },
                        vertex_count: batch.vertices.len(),
                    });
                }
            }
        }
    }
    commands
}

fn srd_merge_command(source: RuntimeTargetCommandSource, draw: &SrdDraw) -> AdjacentMergeCommand {
    AdjacentMergeCommand {
        source,
        key: AdjacentMergeKey {
            state: draw.state,
            order: draw.order,
            topology: DrawTopology::TriangleStrip,
            textures: draw
                .state
                .material
                .textures
                .map(|binding| binding.map(MergeTextureIdentity::Srd)),
        },
        vertex_count: 4,
    }
}

pub(super) fn merge_adjacent_commands(
    commands: Vec<AdjacentMergeCommand>,
) -> Vec<RuntimeTargetCommand> {
    let mut groups: Vec<(AdjacentMergeKey, RuntimeTargetCommand)> = Vec::new();
    for command in commands {
        if let Some((previous_key, previous)) = groups.last_mut()
            && *previous_key == command.key
        {
            previous.sources.push(command.source);
            previous.vertex_count += match previous.topology {
                DrawTopology::TriangleStrip => command.vertex_count + 2,
                DrawTopology::TriangleList => command.vertex_count,
            };
            continue;
        }

        let state = command.key.state;
        let order = command.key.order;
        let topology = command.key.topology;
        groups.push((
            command.key,
            RuntimeTargetCommand {
                sources: vec![command.source],
                state,
                order,
                topology,
                vertex_count: command.vertex_count,
            },
        ));
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn build_runtime_target_submission_impl(
    draws: &[RuntimeCastDraw],
    profile: &ScenePassProfile,
    mut admits: impl FnMut(SrdQueueState) -> bool,
) -> Result<Vec<RuntimeTargetCommandSource>, SrdDrawError> {
    let mut sources = Vec::new();
    let mut queue_states = Vec::new();
    for (runtime_draw_index, draw) in draws.iter().enumerate() {
        match draw {
            RuntimeCastDraw::Image(draw) => {
                if admits(draw.state.queue) {
                    sources.push(RuntimeTargetCommandSource::Image { runtime_draw_index });
                    queue_states.push(draw.state.queue);
                }
            }
            RuntimeCastDraw::SliceCell(cell) => {
                if admits(cell.draw.state.queue) {
                    sources.push(RuntimeTargetCommandSource::SliceCell { runtime_draw_index });
                    queue_states.push(cell.draw.state.queue);
                }
            }
            RuntimeCastDraw::NumberGlyph(glyph) => {
                if admits(glyph.draw.state.queue) {
                    sources.push(RuntimeTargetCommandSource::NumberGlyph { runtime_draw_index });
                    queue_states.push(glyph.draw.state.queue);
                }
            }
            RuntimeCastDraw::Fennel(draw) => {
                for batch_index in 0..draw.batches.len() {
                    if admits(draw.state.queue) {
                        sources.push(RuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        });
                        queue_states.push(draw.state.queue);
                    }
                }
            }
        }
    }

    let submission_indices = build_srd_scene_submission_indices(&queue_states, profile)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    Ok(submission_indices
        .into_iter()
        .map(|command_index| sources[command_index])
        .collect())
}
