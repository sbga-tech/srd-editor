mod tests {
    use crate::animation::{AnimationDefinition, Key8, KeyData, Motion, Track};
    use crate::attribute::{CastAttribute, CastAttributeList, CastAttributeValue, ExtParamData};
    use crate::reference::ReferenceDefinition;
    use crate::scene::{
        AnimationSetDefinition, Layer, NodeRecord, RawTransform, Scene, SceneAnimationSlot,
    };
    use crate::transform::SpatialTransform;

    use super::*;

    fn reference(source_name: &[u8], layer_name: &[u8], node_index: i32) -> ReferenceDefinition {
        ReferenceDefinition {
            source_name: source_name.to_vec(),
            layer_name: layer_name.to_vec(),
            animation_enabled: 0,
            animation_name: Vec::new(),
            default_frame: 0.0,
            node_index,
        }
    }

    fn layer(name: &[u8], is_2d: bool, references: Vec<Option<ReferenceDefinition>>) -> Layer {
        let count = references.len();
        Layer {
            name: name.to_vec(),
            flags: u32::from(!is_2d),
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![
                NodeRecord {
                    name: None,
                    type_flags: Some(3),
                    parent_csli_cell_index: None,
                    first_child_index: -1,
                    next_sibling_index: -1,
                    field_a0: None,
                };
                count
            ],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default()); count],
            image_by_node: vec![None; count],
            number_by_node: vec![None; count],
            reference_by_node: references,
            csli_by_node: vec![None; count],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None; count],
        }
    }

    fn project(layers: Vec<Layer>) -> Project {
        Project {
            name: Vec::new(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: crate::camera::CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: layers.len() as u32,
                declared_animation_set_count: 0,
                width: 0.0,
                height: 0.0,
                layers,
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        }
    }

    fn scratch_assignment(slots: Vec<SceneAnimationSlot>) -> AnimationSetDefinition {
        AnimationSetDefinition {
            name: b"scratch".to_vec(),
            start_frame: 0,
            runtime_duration: 10,
            declared_slot_count: slots.len() as i32,
            slots,
        }
    }

    fn animation_layer(base_x: f32) -> Layer {
        let mut result = layer(b"animated", true, vec![None]);
        result.flags |= 0x100;
        result.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [base_x, 0.0, 0.0],
            ..SpatialTransform::default()
        });
        result.animation_count = 1;
        result.animations.push(AnimationDefinition {
            name: b"move".to_vec(),
            flags: 0,
            declared_motion_count: 1,
            duration: 10,
            motions: vec![Motion {
                target: 0,
                tracks: vec![Track {
                    target: 0,
                    key_count: 1,
                    format: 0x10,
                    range_start: 0,
                    range_end: 10,
                    keys: KeyData::Key8F32(vec![Key8 {
                        frame: 0,
                        value: 25.0,
                    }]),
                }],
            }],
        });
        result
    }

    #[test]
    fn scratch_assignment_overwrites_dense_enablement_and_leaves_uncovered_layers_at_base_state() {
        let mut first = layer(b"first", true, vec![None]);
        first.flags |= 0x100;
        let mut second = layer(b"second", true, vec![None]);
        second.flags |= 0x100;
        let mut third = layer(b"third", true, vec![None]);
        third.flags |= 0x100;
        let project = project(vec![first, second, third]);
        let assignment = scratch_assignment(vec![
            SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: 0,
            },
            SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: 1,
            },
        ]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 0.0)
            .unwrap();

        assert!(project.scenes[0].animation_sets.is_empty());
        assert_eq!(
            runtime.project_layers[0]
                .iter()
                .map(|layer| layer.enabled)
                .collect::<Vec<_>>(),
            vec![false, true, true],
        );
        assert!(
            !runtime
                .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
                .unwrap()
                .project_layers[0][0]
                .layer
                .render_gate
        );
    }

    #[test]
    fn enabled_empty_assignment_slot_keeps_the_layer_base_pose() {
        let project = project(vec![animation_layer(7.0)]);
        let assignment = scratch_assignment(vec![SceneAnimationSlot {
            animation_name: Vec::new(),
            enabled: 1,
        }]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        let application = runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 4.0)
            .unwrap();

        assert_eq!(application, RuntimeAnimationTreeApplication::default());
        assert!(runtime.project_layers[0][0].enabled);
        assert_eq!(
            runtime.project_layers[0][0].cast_transforms[0].translation[0],
            7.0
        );
    }

    #[test]
    fn named_assignment_slot_applies_the_layer_animation() {
        let project = project(vec![animation_layer(7.0)]);
        let assignment = scratch_assignment(vec![SceneAnimationSlot {
            animation_name: b"move".to_vec(),
            enabled: 1,
        }]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        let application = runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 4.0)
            .unwrap();

        assert_eq!(application.animated_layers, 1);
        assert!(runtime.project_layers[0][0].enabled);
        assert_eq!(
            runtime.project_layers[0][0].cast_transforms[0].translation[0],
            25.0
        );
    }

    #[test]
    fn matrix_kind_10000_removes_parent_basis_and_keeps_world_translation() {
        let parent = Affine3x4 {
            rows: [
                [2.0, 0.0, 0.0, 10.0],
                [0.0, 3.0, 0.0, 20.0],
                [0.0, 0.0, 1.0, 30.0],
            ],
        };
        let local = Affine3x4 {
            rows: [
                [4.0, 0.0, 0.0, 1.0],
                [0.0, 5.0, 0.0, 2.0],
                [0.0, 0.0, 1.0, 3.0],
            ],
        };
        let renderer_inverse_camera_view = Affine3x4 {
            rows: [
                [7.0, 0.0, 0.0, 100.0],
                [0.0, 11.0, 0.0, 200.0],
                [0.0, 0.0, 1.0, 300.0],
            ],
        };
        assert_eq!(
            compose_cast_matrix_game(
                parent,
                local,
                0x0001_0000,
                true,
                renderer_inverse_camera_view,
            ),
            Affine3x4 {
                rows: [
                    [28.0, 0.0, 0.0, 12.0],
                    [0.0, 55.0, 0.0, 26.0],
                    [0.0, 0.0, 1.0, 33.0],
                ]
            }
        );
    }

    #[test]
    fn matrix_modifier_keeps_parent_axis_lengths_and_zeroes_only_own_final_z() {
        let mut special = layer(b"special", true, vec![None, None]);
        special.nodes[0].type_flags = Some(3 | 0x0001_0000 | 0x0100_0000);
        special.nodes[0].first_child_index = 1;
        special.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 0.0, 5.0],
            ..SpatialTransform::default()
        });
        special.transforms[1] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 0.0, 2.0],
            ..SpatialTransform::default()
        });
        let project = project(vec![special]);
        let runtime = ProjectRuntime::new(&project).unwrap();
        let worlds = runtime
            .compose_world_states(
                &project,
                Affine3x4 {
                    rows: [
                        [2.0, 0.0, 0.0, 10.0],
                        [0.0, 3.0, 0.0, 20.0],
                        [0.0, 0.0, 1.0, 30.0],
                    ],
                },
                Affine3x4::IDENTITY,
            )
            .unwrap();
        let casts = &worlds.project_layers[0][0].casts;
        assert_eq!(casts[0].matrix.rows[0][0], 2.0);
        assert_eq!(casts[0].matrix.rows[1][1], 3.0);
        assert_eq!(casts[0].matrix.rows[2][3], 0.0);
        assert_eq!(casts[1].matrix.rows[2][3], 37.0);
    }

    #[test]
    fn repeated_targets_create_distinct_instances_and_nested_copies() {
        let project = project(vec![
            layer(
                b"root",
                true,
                vec![
                    Some(reference(b"scene", b"middle", 0)),
                    Some(reference(b"scene", b"middle", 1)),
                ],
            ),
            layer(
                b"middle",
                false,
                vec![Some(reference(b"scene", b"leaf", 0))],
            ),
            layer(b"leaf", true, Vec::new()),
        ]);

        let plan = project.build_reference_runtime_plan().unwrap();
        assert_eq!(plan.unresolved, Vec::new());
        assert_eq!(plan.instances.len(), 5);
        assert_eq!(plan.instances[0].target.layer_index, 1);
        assert_eq!(plan.instances[1].target.layer_index, 1);
        assert_eq!(plan.instances[2].target.layer_index, 2);
        assert_eq!(
            plan.instances[2].parent,
            ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: 0,
                layer_index: 1,
            })
        );
        assert_eq!(
            plan.instances[3].parent,
            ReferenceLayerParent::ReferenceInstance(0)
        );
        assert_eq!(
            plan.instances[4].parent,
            ReferenceLayerParent::ReferenceInstance(1)
        );
        assert!(plan.instances[0].is_2d);
        assert!(plan.instances[1].is_2d);
        assert!(!plan.instances[2].is_2d);
        assert!(plan.instances[3].is_2d);
        assert!(plan.instances[4].is_2d);
        assert!(!plan.instances[0].flip_y);
        assert!(!plan.instances[1].flip_y);
        assert!(plan.instances[2].flip_y);
        assert!(!plan.instances[3].flip_y);
        assert!(!plan.instances[4].flip_y);
    }

    #[test]
    fn structural_draw_order_expands_reference_layers_at_the_refcast_position() {
        let project = project(vec![
            layer(
                b"root",
                true,
                vec![None, Some(reference(b"scene", b"middle", 1)), None],
            ),
            layer(
                b"middle",
                true,
                vec![None, Some(reference(b"scene", b"leaf", 1)), None],
            ),
            layer(b"leaf", true, vec![None, None]),
        ]);
        let plan = project.build_reference_runtime_plan().unwrap();
        assert_eq!(
            plan.structural_cast_draw_order(
                &project,
                0,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            )
            .unwrap()
            .into_iter()
            .map(|entry| (entry.owner, entry.source.layer_index, entry.node_index))
            .collect::<Vec<_>>(),
            vec![
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 0,
                    }),
                    0,
                    0,
                ),
                (ReferenceLayerParent::ReferenceInstance(0), 1, 0),
                (ReferenceLayerParent::ReferenceInstance(2), 2, 0),
                (ReferenceLayerParent::ReferenceInstance(2), 2, 1),
                (ReferenceLayerParent::ReferenceInstance(0), 1, 2),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 0,
                    }),
                    0,
                    2,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    }),
                    1,
                    0,
                ),
                (ReferenceLayerParent::ReferenceInstance(1), 2, 0),
                (ReferenceLayerParent::ReferenceInstance(1), 2, 1),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    }),
                    1,
                    2,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 2,
                    }),
                    2,
                    0,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 2,
                    }),
                    2,
                    1,
                ),
            ]
        );
    }

    #[test]
    fn cast_layer_keys_inherit_ext_params_and_apply_node_offsets_at_render() {
        let mut tested = layer(b"tested", true, vec![None, None]);
        tested.nodes[0].first_child_index = 1;
        tested.cast_attribute_lists = vec![
            CastAttributeList {
                node_index: Some(0),
                declared_count: 1,
                attributes: vec![CastAttribute {
                    name: b"ExtParamData".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::ExtParam {
                        source: Vec::new(),
                        parsed: ExtParamData {
                            layer: 6,
                            flags: ExtParamData::LAYER_KIND | ExtParamData::ENABLE_LAYER,
                            ..ExtParamData::default()
                        },
                    },
                }],
            },
            CastAttributeList {
                node_index: Some(1),
                declared_count: 1,
                attributes: vec![CastAttribute {
                    name: b"ExtParamData".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::ExtParam {
                        source: Vec::new(),
                        parsed: ExtParamData {
                            layer_level: 0xf0,
                            flags: ExtParamData::LAYER_KIND | ExtParamData::ENABLE_LEVEL,
                            ..ExtParamData::default()
                        },
                    },
                }],
            },
        ];
        tested.cast_attribute_list_by_node = vec![Some(0), Some(1)];
        tested.nodes[0].field_a0 = Some(0x90);
        tested.nodes[1].field_a0 = Some(0x20);
        let project = project(vec![tested]);
        let plan = project.build_reference_runtime_plan().unwrap();
        let entries = plan
            .structural_cast_draw_order(
                &project,
                0,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            )
            .unwrap();
        assert_eq!(entries[0].renderer_layer_key, 0x8710);
        assert_eq!(entries[1].renderer_layer_key, 0x8710);
    }

    #[test]
    fn reference_layer_seed_wraps_only_the_low_byte_before_recursion() {
        let mut root = layer(b"root", true, vec![Some(reference(b"scene", b"child", 0))]);
        root.nodes[0].field_a0 = Some(0x90);
        let mut child = layer(b"child", true, vec![None]);
        child.nodes[0].field_a0 = Some(0x20);
        let project = project(vec![root, child]);
        let plan = project.build_reference_runtime_plan().unwrap();
        let entries = plan
            .structural_cast_draw_order(
                &project,
                0,
                crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            )
            .unwrap();
        let copied = entries
            .iter()
            .find(|entry| entry.owner == ReferenceLayerParent::ReferenceInstance(0))
            .unwrap();
        assert_eq!(copied.renderer_layer_key, 0x8530);
    }

    #[test]
    fn unresolved_references_do_not_create_runtime_layers() {
        let project = project(vec![layer(
            b"root",
            true,
            vec![Some(reference(b"missing", b"layer", 0))],
        )]);
        let plan = project.build_reference_runtime_plan().unwrap();
        assert!(plan.instances.is_empty());
        assert_eq!(plan.unresolved.len(), 1);
    }

    #[test]
    fn cycles_are_reported_without_inventing_a_depth_limit() {
        let project = project(vec![layer(
            b"root",
            true,
            vec![Some(reference(b"scene", b"root", 0))],
        )]);
        let error = project.build_reference_runtime_plan().unwrap_err();
        assert_eq!(
            error.repeating_layers,
            vec![
                ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                },
                ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }
            ]
        );
    }

    #[test]
    fn copied_layer_world_state_uses_the_owning_refcast_as_parent() {
        let instance = ReferenceLayerInstance {
            parent: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: 0,
                layer_index: 0,
            }),
            reference_node_index: 0,
            target: ReferenceTarget {
                scene_index: 0,
                layer_index: 1,
            },
            is_2d: true,
            flip_y: false,
        };
        let parent = RuntimeWorldState {
            matrix: Affine3x4 {
                rows: [
                    [1.0, 0.0, 0.0, 10.0],
                    [0.0, 1.0, 0.0, 20.0],
                    [0.0, 0.0, 1.0, 30.0],
                ],
            },
            multiply_color: [200, 100, 50, 255],
            additive_color: [250, 20, 30, 40],
            visible: true,
            render_gate: true,
        };
        let mut local = ReferenceLayerLocalState::default();
        local.transform.translation = [2.0, 3.0, 4.0];
        local.transform.multiply_color = [128, 255, 0, 200];
        local.transform.additive_color = [10, 240, 20, 220];

        let world = instance.compose_world_state(parent, local);
        assert_eq!(world.matrix.rows[0][3], 12.0);
        assert_eq!(world.matrix.rows[1][3], 23.0);
        assert_eq!(world.matrix.rows[2][3], 34.0);
        assert_eq!(world.multiply_color, [100, 100, 0, 200]);
        assert_eq!(world.additive_color, [255, 255, 50, 255]);
        assert!(world.visible);
        assert!(world.render_gate);

        local.enabled = false;
        let disabled = instance.compose_world_state(parent, local);
        assert!(!disabled.visible);
        assert!(!disabled.render_gate);

        local.enabled = true;
        let gated_parent = RuntimeWorldState {
            render_gate: false,
            ..parent
        };
        let gated = instance.compose_world_state(gated_parent, local);
        assert!(gated.visible);
        assert!(!gated.render_gate);
    }

    #[test]
    fn project_runtime_world_states_follow_refcast_parent_mode_color_and_gate() {
        let mut root = layer(b"root", true, vec![Some(reference(b"scene", b"target", 0))]);
        root.flags |= 0x100;
        root.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [10.0, 20.0, 0.0],
            multiply_color: [200, 100, 50, 255],
            additive_color: [10, 20, 30, 40],
            ..SpatialTransform::default()
        });

        // The parsed target is 3D, but binding propagates the owning 2D
        // RefCast mode to every CAST in this independent copy.
        let mut target = layer(b"target", false, vec![None]);
        target.flags |= 0x100;
        target.nodes[0].type_flags = Some(3 | 0x200 | 0x0008_0000);
        target.transforms[0] = RawTransform::Trs3(SpatialTransform {
            translation: [4.0, 5.0, 0.0],
            rotation: [0, 0, 0x4000],
            multiply_color: [128, 255, 128, 200],
            additive_color: [1, 2, 3, 4],
            ..SpatialTransform::default()
        });

        let project = project(vec![root, target]);
        let mut runtime = ProjectRuntime::new(&project).unwrap();
        runtime.references.layers[0].local.transform.translation = [2.0, 3.0, 0.0];
        runtime.references.layers[0].local.transform.multiply_color = [128, 255, 0, 128];
        runtime.references.layers[0].local.transform.additive_color = [250, 240, 230, 220];

        let worlds = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap();
        let copied = &worlds.references[0];
        assert!(copied.is_2d);
        assert_eq!(copied.layer.matrix.rows[0][3], 12.0);
        assert_eq!(copied.layer.matrix.rows[1][3], 23.0);
        assert_eq!(copied.layer.multiply_color, [100, 100, 0, 128]);
        assert_eq!(copied.layer.additive_color, [255, 255, 255, 255]);
        assert!(copied.layer.visible);
        assert!(copied.layer.render_gate);

        let expected_local = build_local_matrix(
            &runtime.references.layers[0].cast_transforms[0],
            true,
            false,
            [0.0, 0.0],
        );
        assert_eq!(
            copied.casts[0].matrix,
            copied.layer.matrix.mul_game(expected_local)
        );
        assert_eq!(copied.casts[0].multiply_color, [50, 100, 0, 100]);
        assert_eq!(copied.casts[0].additive_color, [255, 255, 255, 255]);
        assert!(copied.casts[0].visible);
        assert!(copied.casts[0].render_gate);

        runtime.project_layers[0][0].enabled = false;
        let gated = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap();
        assert!(gated.references[0].layer.visible);
        assert!(!gated.references[0].layer.render_gate);
        assert!(gated.references[0].casts[0].visible);
        assert!(!gated.references[0].casts[0].render_gate);
    }

    #[test]
    fn copied_layer_flip_y_applies_once_at_the_layer_local_matrix() {
        let mut root = layer(
            b"root",
            false,
            vec![Some(reference(b"scene", b"target", 0))],
        );
        root.flags |= 0x100;
        let mut target = layer(b"target", true, vec![None]);
        target.flags |= 0x100;
        target.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 5.0, 0.0],
            ..SpatialTransform::default()
        });
        let project = project(vec![root, target]);
        let mut runtime = ProjectRuntime::new(&project).unwrap();
        assert!(runtime.references.plan.instances[0].flip_y);
        assert!(!runtime.references.plan.instances[0].is_2d);
        runtime.references.layers[0].local.transform.translation[1] = 3.0;

        let copied = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap()
            .references
            .remove(0);
        assert_eq!(copied.layer.matrix.rows[1][3], -3.0);
        // CAST local Y is not flipped a second time; the propagated runtime
        // mode is 3D and the copied-layer root already contains the flip.
        assert_eq!(copied.casts[0].matrix.rows[1][3], 2.0);
    }

    #[test]
    fn copied_layers_keep_independent_animation_frames_and_cast_transforms() {
        let mut project = project(vec![
            layer(
                b"root",
                true,
                vec![
                    Some(reference(b"scene", b"target", 0)),
                    Some(reference(b"scene", b"target", 1)),
                ],
            ),
            layer(b"target", true, vec![None]),
        ]);
        let animation = AnimationDefinition {
            name: b"move".to_vec(),
            flags: 0,
            declared_motion_count: 1,
            duration: 10,
            motions: vec![Motion {
                target: 0,
                tracks: vec![Track {
                    target: 0,
                    key_count: 1,
                    format: 0x10,
                    range_start: 0,
                    range_end: 10,
                    keys: KeyData::Key8F32(vec![Key8 {
                        frame: 0,
                        value: 25.0,
                    }]),
                }],
            }],
        };
        project.scenes[0].layers[1].animation_count = 1;
        project.scenes[0].layers[1].animations.push(animation);

        let plan = project.build_reference_runtime_plan().unwrap();
        let mut first = ReferenceLayerRuntimeState::new(&project, &plan, 0).unwrap();
        let second = ReferenceLayerRuntimeState::new(&project, &plan, 1).unwrap();
        assert_eq!(
            first
                .apply_animation_common_channels(&project, &plan, b"move", 7.0)
                .unwrap(),
            Some(1)
        );
        assert_eq!(first.animations[0].frame, 7.0);
        assert_eq!(first.animations[0].duration, 10.0);
        assert_eq!(first.animations[0].flags, 9);
        assert_eq!(first.cast_transforms[0].translation[0], 25.0);
        assert_eq!(second.animations[0].frame, 0.0);
        assert_eq!(second.cast_transforms[0].translation[0], 0.0);
        assert_eq!(
            first
                .apply_animation_common_channels(&project, &plan, b"missing", 4.0)
                .unwrap(),
            None
        );
    }

    #[test]
    fn channel_23_recursively_applies_the_named_child_instance_animation() {
        let mut middle_reference = reference(b"scene", b"leaf", 0);
        middle_reference.animation_enabled = 1;
        middle_reference.animation_name = b"leaf_animation".to_vec();
        middle_reference.default_frame = 8.0;
        let mut project = project(vec![
            layer(b"root", true, vec![Some(reference(b"scene", b"middle", 0))]),
            layer(b"middle", true, vec![Some(middle_reference)]),
            layer(b"leaf", true, vec![None]),
        ]);
        project.scenes[0].layers[1].animation_count = 1;
        project.scenes[0].layers[1]
            .animations
            .push(AnimationDefinition {
                name: b"drive_reference".to_vec(),
                flags: 0,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 23,
                        key_count: 1,
                        format: 0x13,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key20F32(vec![crate::animation::Key20 {
                            frame: 0,
                            value: 4.0,
                            mode: 0,
                            slope_in: 0.0,
                            slope_out: 0.0,
                        }]),
                    }],
                }],
            });
        project.scenes[0].layers[2].animation_count = 1;
        project.scenes[0].layers[2]
            .animations
            .push(AnimationDefinition {
                name: b"leaf_animation".to_vec(),
                flags: 0,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 0,
                        key_count: 1,
                        format: 0x10,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8F32(vec![Key8 {
                            frame: 0,
                            value: 99.0,
                        }]),
                    }],
                }],
            });

        let mut runtime = ReferenceRuntime::new(&project).unwrap();
        let nested_leaf = runtime
            .plan
            .instances
            .iter()
            .position(|instance| {
                instance.parent == ReferenceLayerParent::ReferenceInstance(0)
                    && instance.target.layer_index == 2
            })
            .unwrap();
        let top_level_leaf = runtime
            .plan
            .instances
            .iter()
            .position(|instance| {
                instance.parent
                    == ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    })
                    && instance.target.layer_index == 2
            })
            .unwrap();
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let application = runtime
            .apply_instance_animation(&project, &textures, 0, b"drive_reference", 3.0)
            .unwrap()
            .unwrap();
        assert_eq!(application.animated_layers, 2);
        assert_eq!(application.reference_requests, 1);
        assert_eq!(runtime.layers[0].animations[0].frame, 3.0);
        assert_eq!(runtime.layers[nested_leaf].animations[0].frame, 4.0);
        assert_eq!(
            runtime.layers[nested_leaf].cast_transforms[0].translation[0],
            99.0
        );
        assert_eq!(
            runtime.layers[top_level_leaf].cast_transforms[0].translation[0],
            0.0
        );
    }
    #[test]
    fn root_project_layer_follows_nested_refcast_ancestry() {
        let root = ReferenceTarget {
            scene_index: 0,
            layer_index: 4,
        };
        let plan = ReferenceRuntimePlan {
            instances: vec![
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ProjectLayer(root),
                    reference_node_index: 2,
                    target: ReferenceTarget {
                        scene_index: 1,
                        layer_index: 3,
                    },
                    is_2d: true,
                    flip_y: false,
                },
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ReferenceInstance(0),
                    reference_node_index: 5,
                    target: ReferenceTarget {
                        scene_index: 2,
                        layer_index: 1,
                    },
                    is_2d: true,
                    flip_y: false,
                },
            ],
            unresolved: Vec::new(),
        };

        assert_eq!(
            plan.root_project_layer(ReferenceLayerParent::ReferenceInstance(1)),
            Ok(root)
        );
    }

    #[test]
    fn root_project_layer_rejects_invalid_or_cyclic_instances() {
        let invalid = ReferenceRuntimePlan::default();
        assert_eq!(
            invalid.root_project_layer(ReferenceLayerParent::ReferenceInstance(0)),
            Err(ReferenceRuntimeAncestryError::InvalidReferenceInstance { instance_index: 0 })
        );

        let cyclic = ReferenceRuntimePlan {
            instances: vec![ReferenceLayerInstance {
                parent: ReferenceLayerParent::ReferenceInstance(0),
                reference_node_index: 0,
                target: ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                },
                is_2d: true,
                flip_y: false,
            }],
            unresolved: Vec::new(),
        };
        assert_eq!(
            cyclic.root_project_layer(ReferenceLayerParent::ReferenceInstance(0)),
            Err(ReferenceRuntimeAncestryError::ReferenceCycle { instance_index: 0 })
        );
    }
}
