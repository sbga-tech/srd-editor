#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::{CastAttribute, CastAttributeList, CastAttributeValue, ExtParamData};
    use crate::camera::CameraDefinition;
    use crate::csli::{CrefEntry, CsliDefinition, SlicCell};
    use crate::image::ImageDefinition;
    use crate::reference::ReferenceDefinition;
    use crate::ruhuna::RuhunaRuntimeGlyphRecord;
    use crate::scene::{NodeRecord, RawTransform, Scene};
    use crate::text::{FontDefinition, TextDefinition};
    use crate::texture::{TextureCrop, TextureDefinition};
    use crate::transform::SpatialTransform;

    fn text_image(node_index: i32, font_index: i32) -> ImageDefinition {
        ImageDefinition {
            flags: 0x100,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: -1,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index,
            has_text_child: true,
            text: Some(TextDefinition {
                field_78: None,
                font_index: Some(font_index),
                text: Vec::new(),
                field_36: None,
                field_7b: None,
                field_7c: None,
                field_41: None,
            }),
        }
    }

    fn node() -> NodeRecord {
        NodeRecord {
            name: None,
            type_flags: Some(1),
            parent_csli_cell_index: None,
            first_child_index: -1,
            next_sibling_index: -1,
            field_a0: None,
        }
    }

    fn plain_image(node_index: i32) -> ImageDefinition {
        ImageDefinition {
            flags: 0,
            width: 100.0,
            height: 50.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: -1,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index,
            has_text_child: false,
            text: None,
        }
    }

    fn font(name: &[u8]) -> crate::text::FontDefinition {
        FontDefinition {
            name: name.to_vec(),
            flags_70: None,
            field_71: None,
            characters: Vec::new(),
        }
    }

    #[test]
    fn fennel_font_requests_follow_runtime_cast_then_catr_order() {
        let attribute_list = CastAttributeList {
            node_index: Some(0),
            declared_count: 4,
            attributes: vec![
                CastAttribute {
                    name: b"rubyFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"ruby.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"ignoredFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"ignored.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"rfzOutlineFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"outline.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"rfzOutlineRubyFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(Vec::new()),
                },
            ],
        };
        let layer = Layer {
            name: b"layer".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node(), node(), node()],
            transforms: Vec::new(),
            image_by_node: vec![
                Some(text_image(0, 1)),
                Some(text_image(1, 0)),
                Some(text_image(2, 1)),
            ],
            number_by_node: vec![None, None, None],
            reference_by_node: vec![None, None, None],
            csli_by_node: vec![None, None, None],
            cast_attribute_lists: vec![attribute_list],
            cast_attribute_list_by_node: vec![Some(0), None, None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 2,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 1,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![layer],
                animation_sets: Vec::new(),
            }],
            fonts: vec![font(b"primary_a.rfz"), font(b"primary_b.rfz")],
        };

        let requests = collect_fennel_font_resource_requests(&project).unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| (request.node_index, request.role, request.name.as_slice()))
                .collect::<Vec<_>>(),
            vec![
                (0, FennelTextFontRole::Primary, b"primary_b.rfz".as_slice()),
                (0, FennelTextFontRole::Ruby, b"ruby.rfz".as_slice()),
                (0, FennelTextFontRole::Outline, b"outline.rfz".as_slice()),
                (1, FennelTextFontRole::Primary, b"primary_a.rfz".as_slice()),
                (2, FennelTextFontRole::Primary, b"primary_b.rfz".as_slice()),
            ]
        );

        let mut registry = FennelFontSlotRegistry::default();
        let assignments = assign_fennel_font_resource_requests(&mut registry, requests);
        assert_eq!(
            assignments
                .iter()
                .map(|assignment| assignment.slot.font_slot_id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 0]
        );
        assert_eq!(
            assignments[4].slot.resource_handle,
            assignments[0].slot.resource_handle
        );
        assert_eq!(assignments[4].slot.lease_count, 2);
    }

    #[test]
    fn fennel_runtime_text_input_manual_defaults_keep_slots_explicit_and_clock_zero() {
        let input = FennelSrdRuntimeTextInput::default();
        assert!(input.substitutions.iter().all(Vec::is_empty));
        assert_eq!(input.default_d, FENNEL_DEFAULT_D_VALUE);
        assert_eq!(input.repeat_space_count, FENNEL_DEFAULT_REPEAT_SPACE_COUNT);
        assert_eq!(input.field_f4.to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn reference_image_builder_draws_the_independent_copy_at_the_refcast_position() {
        let root = Layer {
            name: b"root".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![NodeRecord {
                type_flags: Some(3),
                ..node()
            }],
            transforms: vec![RawTransform::Trs2(SpatialTransform {
                translation: [10.0, 20.0, 0.0],
                ..SpatialTransform::default()
            })],
            image_by_node: vec![None],
            number_by_node: vec![None],
            reference_by_node: vec![Some(ReferenceDefinition {
                source_name: b"scene".to_vec(),
                layer_name: b"target".to_vec(),
                animation_enabled: 0,
                animation_name: Vec::new(),
                default_frame: 0.0,
                node_index: 0,
            })],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        // The source target is disabled in the original runtime scene table;
        // its copied layer is independently enabled by reference binding.
        let target = Layer {
            name: b"target".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node()],
            transforms: vec![RawTransform::Trs2(SpatialTransform {
                translation: [5.0, 7.0, 0.0],
                ..SpatialTransform::default()
            })],
            image_by_node: vec![Some(plain_image(0))],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 2,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![root, target],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        };
        let draws = build_base_pose_reference_image_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
        )
        .unwrap();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].owner(), ReferenceLayerParent::ReferenceInstance(0));
        assert_eq!((draws[0].origin.scene_index, draws[0].origin.layer_index), (0, 1));
        assert_eq!(draws[0].node_index(), 0);
        assert_eq!(draws[0].geometry.vertices[0].position, [15.0, 27.0, 0.0]);

        let culled = build_base_pose_reference_image_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [10, 10],
                )),
                identity_matrix4x4_game(),
                [10, 10],
            ),
        )
        .unwrap();
        assert!(culled.is_empty());
    }

    #[test]
    fn image_compiler_emits_dynamic_simple_shader_preset() {
        let layer = Layer {
            name: b"layer".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node()],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default())],
            image_by_node: vec![Some(plain_image(0))],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: vec![CastAttributeList {
                node_index: Some(0),
                declared_count: 1,
                attributes: vec![CastAttribute {
                    name: b"ExtParamData".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::ExtParam {
                        source: b"blendMode#1".to_vec(),
                        parsed: ExtParamData {
                            render_preset_override: 34,
                            ..ExtParamData::default()
                        },
                    },
                }],
            }],
            cast_attribute_list_by_node: vec![Some(0)],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 1,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![layer],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        };
        let draws = build_base_pose_image_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
        )
        .unwrap();

        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].state.profile.blend_mode.get(), 34);
        assert!(draws[0].state.profile.requires_backdrop());
        assert!(!draws[0].state.pipeline.blend.enabled);
        assert_eq!(draws[0].state.queue.target_attributes, 0xf7);
    }

    #[test]
    fn copied_fennel_instances_use_independent_runtime_text_inputs() {
        let reference = |node_index| {
            Some(ReferenceDefinition {
                source_name: b"scene".to_vec(),
                layer_name: b"target".to_vec(),
                animation_enabled: 0,
                animation_name: Vec::new(),
                default_frame: 0.0,
                node_index,
            })
        };
        let root = Layer {
            name: b"root".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![
                node(),
                NodeRecord {
                    type_flags: Some(3),
                    ..node()
                },
                node(),
                NodeRecord {
                    type_flags: Some(3),
                    ..node()
                },
            ],
            transforms: vec![
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform {
                    translation: [100.0, 0.0, 0.0],
                    ..SpatialTransform::default()
                }),
            ],
            image_by_node: vec![Some(plain_image(0)), None, Some(plain_image(2)), None],
            number_by_node: vec![None, None, None, None],
            reference_by_node: vec![None, reference(1), None, reference(3)],
            csli_by_node: vec![None, None, None, None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None, None, None, None],
        };
        let mut target_text = text_image(0, 0);
        let text = target_text.text.as_mut().unwrap();
        text.text = b"$[0]".to_vec();
        text.field_78 = Some(0);
        text.field_36 = Some([1.0, 1.0]);
        text.field_7c = Some(0);
        text.field_41 = Some(0);
        let target = Layer {
            name: b"target".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node()],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default())],
            image_by_node: vec![Some(target_text)],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 1,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 2,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![root, target],
                animation_sets: Vec::new(),
            }],
            fonts: vec![font(b"test.rfz")],
        };
        let glyph = |code, texture_token| RuhunaRuntimeGlyphRecord {
            code,
            texture_token,
            enabled: 1,
            point_x: 10,
            point_y: 12,
            width: 10,
            height: 12,
            line_height: 12,
            advance_x: 10,
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let runtime_font = RuhunaRuntimeFont {
            minimum_code: u16::from(b'A'),
            maximum_code: u16::from(b'B'),
            dense_glyph_indices: vec![0, 1],
            glyph_pages: vec![Some(0), Some(0)],
            glyphs: vec![glyph(u16::from(b'A'), 11), glyph(u16::from(b'B'), 22)],
        };
        let mut runtime_fonts = BTreeMap::new();
        runtime_fonts.insert(b"test.rfz".to_vec(), runtime_font);
        let mut font_registry = FennelFontSlotRegistry::default();
        font_registry.request(b"test.rfz".to_vec());
        let mut inputs = BTreeMap::new();
        for (instance_index, substitution) in
            [b"A".as_slice(), b"B".as_slice()].into_iter().enumerate()
        {
            let mut input = FennelSrdRuntimeTextInput::default();
            input.substitutions[0] = substitution.to_vec();
            inputs.insert(
                FennelRuntimeTextCastKey {
                    owner: ReferenceLayerParent::ReferenceInstance(instance_index),
                    node_index: 0,
                },
                input,
            );
        }
        let draws = build_base_pose_reference_fennel_draws(
            &project,
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
            FennelRenderResources {
                font_slots: &font_registry,
                fonts: &runtime_fonts,
                force_color_update: false,
                text_inputs: &inputs,
            },
        )
        .unwrap();
        assert_eq!(draws.len(), 2);
        assert_eq!(
            draws.iter().map(|draw| draw.origin.owner).collect::<Vec<_>>(),
            vec![
                ReferenceLayerParent::ReferenceInstance(0),
                ReferenceLayerParent::ReferenceInstance(1),
            ]
        );
        assert_eq!(
            draws
                .iter()
                .map(|draw| draw.batches[0].texture_token)
                .collect::<Vec<_>>(),
            vec![11, 22]
        );

        let culled = build_base_pose_reference_fennel_draws(
            &project,
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [10, 10],
                )),
                identity_matrix4x4_game(),
                [10, 10],
            ),
            FennelRenderResources {
                font_slots: &font_registry,
                fonts: &runtime_fonts,
                force_color_update: false,
                text_inputs: &inputs,
            },
        )
        .unwrap();
        assert_eq!(culled.len(), 1);
        assert_eq!(culled[0].origin.owner, ReferenceLayerParent::ReferenceInstance(0));

        let runtime_cast_draws = build_base_pose_runtime_cast_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
            FennelRenderResources {
                font_slots: &font_registry,
                fonts: &runtime_fonts,
                force_color_update: false,
                text_inputs: &inputs,
            },
        )
        .unwrap();
        assert_eq!(runtime_cast_draws.len(), 4);
        assert!(matches!(
            &runtime_cast_draws[0],
            RuntimeCastDraw::Image(draw)
                if draw.owner() == ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }) && draw.node_index() == 0
        ));
        assert!(matches!(
            &runtime_cast_draws[1],
            RuntimeCastDraw::Fennel(draw)
                if draw.origin.owner == ReferenceLayerParent::ReferenceInstance(0)
        ));
        assert!(matches!(
            &runtime_cast_draws[2],
            RuntimeCastDraw::Image(draw) if draw.node_index() == 2
        ));
        assert!(matches!(
            &runtime_cast_draws[3],
            RuntimeCastDraw::Fennel(draw)
                if draw.origin.owner == ReferenceLayerParent::ReferenceInstance(1)
        ));

        let expected_target_commands = runtime_cast_draws
            .iter()
            .enumerate()
            .flat_map(|(runtime_draw_index, draw)| match draw {
                RuntimeCastDraw::Image(_) => {
                    vec![RuntimeTargetCommandSource::Image { runtime_draw_index }]
                }
                RuntimeCastDraw::SliceCell(_) => {
                    vec![RuntimeTargetCommandSource::SliceCell { runtime_draw_index }]
                }
                RuntimeCastDraw::NumberGlyph(_) => {
                    vec![RuntimeTargetCommandSource::NumberGlyph { runtime_draw_index }]
                }
                RuntimeCastDraw::Fennel(draw) => (0..draw.batches.len())
                    .map(
                        |batch_index| RuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        },
                    )
                    .collect(),
            })
            .collect::<Vec<_>>();
        let profile = crate::game_host::CHUSAN_MAIN_SCENE
            .scene_pass_profile()
            .unwrap();
        assert_eq!(
            build_runtime_target_submission(&runtime_cast_draws, &profile).unwrap(),
            expected_target_commands
        );
        assert_eq!(
            build_filtered_runtime_target_submission(
                &runtime_cast_draws,
                &profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
            )
            .unwrap(),
            expected_target_commands
        );
        let background_profile = crate::game_host::CHUSAN_BG_SCENE
            .scene_pass_profile()
            .unwrap();
        assert!(
            build_filtered_runtime_target_submission(
                &runtime_cast_draws,
                &background_profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_BG_SCENE),
            )
            .unwrap()
            .is_empty()
        );
        let merged = build_runtime_target_commands(
            &runtime_cast_draws,
            &profile,
            crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
        )
        .unwrap();
        assert_eq!(
            merged
                .iter()
                .flat_map(|group| group.sources.iter().copied())
                .collect::<Vec<_>>(),
            expected_target_commands
        );
        for group in &merged {
            let source_vertex_count = group
                .sources
                .iter()
                .map(|source| match *source {
                    RuntimeTargetCommandSource::Image { .. } => 4,
                    RuntimeTargetCommandSource::SliceCell { .. } => 4,
                    RuntimeTargetCommandSource::NumberGlyph { .. } => 4,
                    RuntimeTargetCommandSource::FennelBatch {
                        runtime_draw_index,
                        batch_index,
                    } => match &runtime_cast_draws[runtime_draw_index] {
                        RuntimeCastDraw::Fennel(draw) => {
                            draw.batches[batch_index].vertices.len()
                        }
                        RuntimeCastDraw::Image(_)
                        | RuntimeCastDraw::SliceCell(_)
                        | RuntimeCastDraw::NumberGlyph(_) => unreachable!(),
                    },
                })
                .sum::<usize>();
            let connector_count = if group.topology == DrawTopology::TriangleStrip {
                (group.sources.len() - 1) * 2
            } else {
                0
            };
            assert_eq!(group.vertex_count, source_vertex_count + connector_count);
        }
        assert!(
            build_runtime_target_commands(
                &runtime_cast_draws,
                &background_profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_BG_SCENE),
            )
            .unwrap()
            .is_empty()
        );
        for draw in &runtime_cast_draws {
            let queue = match draw {
                RuntimeCastDraw::Image(draw) => draw.state.queue,
                RuntimeCastDraw::SliceCell(draw) => draw.draw.state.queue,
                RuntimeCastDraw::NumberGlyph(draw) => draw.draw.state.queue,
                RuntimeCastDraw::Fennel(draw) => draw.state.queue,
            };
            assert_eq!(queue.command_class, 3);
            assert_eq!(queue.attribute_group, 0);
            assert!(!queue.rejected);
        }
    }

    #[test]
    fn slice_cast_emits_one_runtime_draw_per_active_cell() {
        let cell = |cref_index| SlicCell {
            flags: 0x100,
            explicit_width: 0.0,
            explicit_height: 0.0,
            field_3a: Some([0xff; 4]),
            field_33: None,
            field_44: vec![[0xff; 4]; 4],
            cref_index,
        };
        let csli = CsliDefinition {
            field_80: 0,
            width: 20.0,
            height: 10.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 2,
            rows: 1,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            node_index: 0,
            cells: vec![cell(0), cell(-1)],
        };
        let layer = Layer {
            name: b"slice".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![NodeRecord {
                type_flags: Some(2),
                ..node()
            }],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default())],
            image_by_node: vec![None],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![Some(csli)],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 1,
                declared_animation_set_count: 0,
                width: 20.0,
                height: 10.0,
                layers: vec![layer],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        };
        let textures = TextureList {
            declared_count: 1,
            textures: vec![TextureDefinition {
                filename: b"slice".to_vec(),
                width: 32,
                height: 32,
                field_62: 0,
                crop_count: 1,
                crops: vec![TextureCrop {
                    normalized_rectangle: [0.25, 0.5, 0.75, 1.0],
                }],
            }],
        };
        let draws = build_base_pose_runtime_cast_draws(
            &project,
            &textures,
            0,
            WorldSnapshot::new(
                Affine3x4::IDENTITY,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(ProjectTargetSnapshot::new(
                    identity_matrix4x4_game(),
                    [20, 10],
                )),
                identity_matrix4x4_game(),
                [20, 10],
            ),
            FennelRenderResources {
                font_slots: &FennelFontSlotRegistry::default(),
                fonts: &BTreeMap::new(),
                force_color_update: false,
                text_inputs: &BTreeMap::new(),
            },
        )
        .unwrap();

        assert_eq!(draws.len(), 2);
        let cells = draws
            .iter()
            .map(|draw| match draw {
                RuntimeCastDraw::SliceCell(cell) => cell,
                _ => panic!("SliceCast emitted a non-slice runtime draw"),
            })
            .collect::<Vec<_>>();
        assert_eq!([cells[0].cell_index, cells[1].cell_index], [0, 1]);
        assert_eq!(
            cells[0].draw.geometry.vertices.map(|vertex| vertex.position),
            [
                [0.0, 0.0, 0.0],
                [0.0, 10.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 0.0],
            ]
        );
        assert_eq!(
            cells[0]
                .draw
                .geometry
                .vertices
                .map(|vertex| vertex.texture_coordinates),
            [
                [[0.25, 0.5]; 2],
                [[0.25, 1.0]; 2],
                [[0.75, 0.5]; 2],
                [[0.75, 1.0]; 2],
            ]
        );
        assert!(cells[0].draw.state.material.textures[0].is_some());
        assert_eq!(cells[1].draw.state.material.textures, [None; 3]);
        assert!(
            cells[1]
                .draw
                .geometry
                .vertices
                .iter()
                .all(|vertex| vertex.texture_coordinates == [[0.0; 2]; 2])
        );

        let profile = crate::game_host::CHUSAN_MAIN_SCENE
            .scene_pass_profile()
            .unwrap();
        assert_eq!(
            build_runtime_target_submission(&draws, &profile).unwrap(),
            vec![
                RuntimeTargetCommandSource::SliceCell {
                    runtime_draw_index: 0,
                },
                RuntimeTargetCommandSource::SliceCell {
                    runtime_draw_index: 1,
                },
            ]
        );
        assert_eq!(
            build_runtime_target_commands(
                &draws,
                &profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
            )
            .unwrap()
            .len(),
            2
        );
    }

    #[test]
    fn adjacent_merge_adds_strip_connectors_but_not_triangle_list_vertices() {
        let strip_key = AdjacentMergeKey {
            state: SrdDrawState::fennel(
                SrdTransform::identity_2d([1, 1]),
                SimpleTransformMode::TwoDimensional,
            ),
            order: DrawOrder {
                renderer_layer_key: crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            },
            topology: DrawTopology::TriangleStrip,
            textures: [None; 3],
        };
        let list_key = AdjacentMergeKey {
            state: SrdDrawState::fennel(
                SrdTransform::identity_2d([1, 1]),
                SimpleTransformMode::TwoDimensional,
            ),
            order: DrawOrder {
                renderer_layer_key: crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            },
            topology: DrawTopology::TriangleList,
            textures: [None; 3],
        };
        let commands = vec![
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::Image {
                    runtime_draw_index: 0,
                },
                key: strip_key.clone(),
                vertex_count: 4,
            },
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::Image {
                    runtime_draw_index: 1,
                },
                key: strip_key,
                vertex_count: 4,
            },
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index: 2,
                    batch_index: 0,
                },
                key: list_key.clone(),
                vertex_count: 6,
            },
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index: 3,
                    batch_index: 0,
                },
                key: list_key,
                vertex_count: 12,
            },
        ];

        let groups = merge_adjacent_commands(commands);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].vertex_count, 10);
        assert_eq!(groups[0].sources.len(), 2);
        assert_eq!(groups[1].vertex_count, 18);
        assert_eq!(groups[1].sources.len(), 2);
    }

    #[test]
    fn materialized_strip_duplicates_previous_last_then_next_first() {
        let vertex = |x| SrdVertex {
            position: [x, 0.0, 0.0],
            primary_color: [0; 4],
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        };
        let first = [vertex(0.0), vertex(1.0), vertex(2.0), vertex(3.0)];
        let second = [vertex(4.0), vertex(5.0), vertex(6.0), vertex(7.0)];
        let mut vertices = Vec::new();
        append_srd_quad_strip_vertices(&mut vertices, &first);
        append_srd_quad_strip_vertices(&mut vertices, &second);
        assert_eq!(
            vertices
                .iter()
                .map(|vertex| vertex.position[0])
                .collect::<Vec<_>>(),
            [0.0, 1.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 6.0, 7.0]
        );
    }

    #[test]
    fn adjacent_merge_keeps_different_renderer_layer_keys_separate() {
        let base_key = AdjacentMergeKey {
            state: SrdDrawState::fennel(
                SrdTransform::identity_2d([1, 1]),
                SimpleTransformMode::TwoDimensional,
            ),
            order: DrawOrder {
                renderer_layer_key: 0x8580,
            },
            topology: DrawTopology::TriangleStrip,
            textures: [None; 3],
        };
        let mut different_key = base_key.clone();
        different_key.order.renderer_layer_key = 0x8680;
        let groups = merge_adjacent_commands(vec![
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::Image {
                    runtime_draw_index: 0,
                },
                key: base_key,
                vertex_count: 4,
            },
            AdjacentMergeCommand {
                source: RuntimeTargetCommandSource::Image {
                    runtime_draw_index: 1,
                },
                key: different_key,
                vertex_count: 4,
            },
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].order.renderer_layer_key, 0x8580);
        assert_eq!(groups[1].order.renderer_layer_key, 0x8680);
    }

    #[test]
    fn three_dimensional_fennel_textbox_matrix_multiplies_then_negates_y_column() {
        let world = Affine3x4 {
            rows: [
                [2.0, 0.0, 0.0, 1.0],
                [0.0, 3.0, 0.0, 2.0],
                [0.0, 0.0, 4.0, 3.0],
            ],
        };
        let camera_bridge = Matrix4x4 {
            rows: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };
        assert_eq!(
            fennel_textbox_transform(world, true, camera_bridge).rows,
            [
                [2.0, 0.0, 0.0, 1.0],
                [0.0, 3.0, 0.0, 2.0],
                [0.0, 0.0, 4.0, 3.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
        assert_eq!(
            fennel_textbox_transform(world, false, camera_bridge).rows,
            [
                [2.0, -6.0, 12.0, 18.0],
                [10.0, -18.0, 28.0, 46.0],
                [18.0, -30.0, 44.0, 74.0],
                [26.0, -42.0, 60.0, 102.0],
            ]
        );
    }
}
