use super::*;
use crate::reference_runtime::ReferenceLayerInstance;
use crate::renderer::backend::{CompositionReadback, RenderBackendError, SimpleDraw};
use crate::renderer::pipeline::{
    DrawOrder, DrawOrigin, DrawTopology, SimpleTransformMode, SrdDrawState, SrdQuad,
    SrdSurfaceStateInput, SrdTransform, SrdVertex,
};
use crate::scene::{Layer, NodeRecord, Project, RawTransform, ReferenceTarget, Scene};
use crate::transform::SpatialTransform;

#[derive(Default)]
struct RecordingBackend {
    composition_size: Option<[u32; 2]>,
    rgba: Vec<u8>,
    draw_calls: usize,
}

impl RecordingBackend {
    fn mark_draw(&mut self) {
        self.draw_calls += 1;
        if let Some(pixel) = self.rgba.get_mut(..4) {
            pixel.copy_from_slice(&[0xE1, 0x43, 0x27, 0xFF]);
        }
    }
}

impl SrdRenderBackend for RecordingBackend {
    fn upload_srd_textures(
        &mut self,
        _sources: SrdTextureSourceSet,
    ) -> Result<SrdTextureSetHandle, RenderBackendError> {
        Ok(SrdTextureSetHandle::new(1))
    }

    fn upload_fennel_atlas(
        &mut self,
        _sources: FennelAtlasSourceSet,
    ) -> Result<FennelAtlasHandle, RenderBackendError> {
        Ok(FennelAtlasHandle::new(2))
    }

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError> {
        self.composition_size = Some(size);
        Ok(())
    }

    fn composition_size(&self) -> Option<[u32; 2]> {
        self.composition_size
    }

    fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError> {
        let [width, height] = self
            .composition_size
            .ok_or_else(|| RenderBackendError("composition is not configured".into()))?;
        let pixel_count = usize::try_from(width).unwrap() * usize::try_from(height).unwrap();
        let clear = [
            (clear_argb >> 16) as u8,
            (clear_argb >> 8) as u8,
            clear_argb as u8,
            (clear_argb >> 24) as u8,
        ];
        self.rgba.resize(pixel_count * 4, 0);
        for pixel in self.rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&clear);
        }
        Ok(())
    }

    fn end_composition(&mut self) -> Result<(), RenderBackendError> {
        Ok(())
    }

    fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
        let [width, height] = self
            .composition_size
            .ok_or_else(|| RenderBackendError("composition is not configured".into()))?;
        Ok(CompositionReadback {
            width,
            height,
            rgba: self.rgba.clone(),
        })
    }

    fn render_simple(&mut self, _draw: SimpleDraw<'_>) -> Result<(), RenderBackendError> {
        self.mark_draw();
        Ok(())
    }
}

impl ManagedSrdRenderBackend for RecordingBackend {
    fn release_srd_textures(&mut self, _handle: SrdTextureSetHandle) {}

    fn release_fennel_atlas(&mut self, _handle: FennelAtlasHandle) {}
}

#[test]
fn layerless_first_scene_renders_the_clear_composition() {
    let mut document = EditorDocument::empty();
    document.project.declared_scene_count = 1;
    document.project.scenes.push(Scene {
        name: b"Scene 1".to_vec(),
        declared_layer_count: 0,
        declared_animation_set_count: 0,
        width: 1920.0,
        height: 1080.0,
        layers: Vec::new(),
        animation_sets: Vec::new(),
    });
    let empty_layers = BTreeSet::new();
    let empty_casts = BTreeSet::new();
    let runtime_text_inputs = BTreeMap::new();
    let mut state = GpuPreviewState::new(RecordingBackend::default());
    let mut phases = Vec::new();

    let frame = state
        .render(
            PreviewRequest {
                foreground: PreviewLayerRequest {
                    document: &document,
                    document_revision: 1,
                    scene_index: 0,
                    animation_assignment: None,
                    frame: 0,
                    hidden_layers: &empty_layers,
                    solo_layers: &empty_layers,
                    hidden_casts: &empty_casts,
                    solo_casts: &empty_casts,
                    runtime_text_inputs: &runtime_text_inputs,
                },
                background: None,
                profile: PreviewProfile::AdvertiseLogoMain,
                selection: None,
                highlights: &[],
            },
            |phase| {
                phases.push(phase);
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(phases, [GpuRenderPhase::Begin, GpuRenderPhase::End]);
    assert_eq!(state.backend.draw_calls, 0);
    assert_eq!([frame.width, frame.height], [1920, 1080]);
    assert_eq!(&frame.rgba[..4], &[0x00, 0x00, 0x00, 0xFF]);
    assert_eq!(frame.selection_bounds, None);
    assert!(frame.highlight_bounds.is_empty());
}

#[test]
fn base_pose_preview_renders_submitted_fixture_geometry_without_an_animation_set() {
    let Some(path) =
        crate::test_support::game_data_path("surfboard/advertise/chu_ui_advertise_00_v10.srd")
    else {
        return;
    };
    let document = EditorDocument::load(path).unwrap();
    let hidden_layers = BTreeSet::new();
    let runtime_text_inputs = BTreeMap::new();
    let hidden_casts = BTreeSet::new();
    let solo_casts = BTreeSet::new();
    let mut state = GpuPreviewState::new(RecordingBackend::default());
    let mut phases = Vec::new();

    let frame = state
        .render(
            PreviewRequest {
                foreground: PreviewLayerRequest {
                    document: &document,
                    document_revision: 0,
                    scene_index: 0,
                    animation_assignment: None,
                    frame: 0,
                    hidden_layers: &hidden_layers,
                    solo_layers: &hidden_layers,
                    hidden_casts: &hidden_casts,
                    solo_casts: &solo_casts,
                    runtime_text_inputs: &runtime_text_inputs,
                },
                background: None,
                profile: PreviewProfile::AdvertiseLogoMain,
                selection: Some(PreviewCastSelection {
                    scene_index: 0,
                    layer_index: 0,
                    node_index: 0,
                }),
                highlights: &[],
            },
            |phase| {
                phases.push(phase);
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(phases, [GpuRenderPhase::Begin, GpuRenderPhase::End]);
    assert!(state.backend.draw_calls > 0);
    assert_eq!([frame.width, frame.height], [1920, 1080]);
    assert_eq!(&frame.rgba[..4], &[0xE1, 0x43, 0x27, 0xFF]);
    let [left, top, width, height] = frame
        .selection_bounds
        .expect("the selected NullCast includes its submitted C_black child");
    assert!(width > 0 && height > 0);
    assert!(left + width <= frame.width && top + height <= frame.height);
}

#[test]
fn advertise_mask_nodes_select_native_secondary_red_alpha_mode() {
    let Some(path) =
        crate::test_support::game_data_path("surfboard/advertise/chu_ui_advertise_00_v10.srd")
    else {
        return;
    };
    let document = EditorDocument::load(path).unwrap();
    let animation_set_index = 10;
    let frame = 30.0;
    let (host, _, _) = preview_snapshot_for_profile(PreviewProfile::AdvertiseLogoMain).unwrap();
    let snapshot = preview_snapshot_with_project_camera(
        host,
        document.project.camera,
        checked_scene_size(
            document.project.scenes[0].width,
            document.project.scenes[0].height,
        )
        .unwrap(),
    );
    let draws = crate::renderer::build_animation_set_runtime_srd_draws(
        &document.project,
        &document.textures,
        0,
        animation_set_index,
        frame,
        snapshot,
    )
    .unwrap();
    let expected = [
        (47, b"C_mask_CHUNITHM".as_slice()),
        (52, b"C_Mate_mask_typeA_01".as_slice()),
        (54, b"C_mask_M".as_slice()),
        (55, b"C_mask_a".as_slice()),
        (56, b"C_mask_t".as_slice()),
        (57, b"C_mask_e".as_slice()),
    ];
    let layer = &document.project.scenes[0].layers[3];
    let mut actual = Vec::new();

    for draw in draws {
        let RuntimeCastDraw::Image(draw) = draw else {
            continue;
        };
        let node_index = draw.node_index();
        if !expected
            .iter()
            .any(|(expected_index, _)| *expected_index == node_index)
        {
            continue;
        }
        actual.push((node_index, layer.nodes[node_index].name.as_deref().unwrap()));
        assert_eq!(draw.state.profile.texture_count.get(), 2);
        assert_eq!(
            draw.state.profile.multi_texture_mode,
            crate::renderer::pipeline::SrdMultiTextureMode::MultiplyAlphaBySecondaryRed,
        );
        assert_eq!(draw.state.profile.blend_mode.get(), 0);
        assert!(draw.state.profile.alpha_blend);
        assert!(draw.state.material.textures[0].is_some());
        assert!(draw.state.material.textures[1].is_some());
    }

    assert_eq!(actual, expected);
}

fn test_node(first_child_index: i16, next_sibling_index: i16) -> NodeRecord {
    NodeRecord {
        name: None,
        type_flags: Some(0x101),
        parent_csli_cell_index: None,
        first_child_index,
        next_sibling_index,
        field_a0: None,
    }
}

fn test_layer(nodes: Vec<NodeRecord>) -> Layer {
    let node_count = nodes.len();
    Layer {
        name: Vec::new(),
        flags: 0x100,
        animation_count: 0,
        animations: Vec::new(),
        field_23: Vec::new(),
        nodes,
        transforms: vec![RawTransform::Trs2(SpatialTransform::default()); node_count],
        image_by_node: vec![None; node_count],
        number_by_node: vec![None; node_count],
        reference_by_node: vec![None; node_count],
        csli_by_node: vec![None; node_count],
        cast_attribute_lists: Vec::new(),
        cast_attribute_list_by_node: vec![None; node_count],
    }
}

fn selection_project() -> Project {
    Project {
        name: Vec::new(),
        declared_scene_count: 1,
        declared_font_count: 0,
        camera: CameraDefinition::ZERO_FALLBACK,
        scenes: vec![Scene {
            name: Vec::new(),
            declared_layer_count: 2,
            declared_animation_set_count: 0,
            width: 100.0,
            height: 100.0,
            layers: vec![
                test_layer(vec![test_node(1, -1), test_node(-1, 2), test_node(-1, -1)]),
                test_layer(vec![test_node(-1, -1)]),
            ],
            animation_sets: Vec::new(),
        }],
        fonts: Vec::new(),
    }
}

fn test_vertex(x: f32, y: f32) -> SrdVertex {
    SrdVertex {
        position: [x, y, 0.0],
        primary_color: [255; 4],
        secondary_color: [0; 4],
        texture_coordinates: [[0.0; 2]; 2],
    }
}

fn test_draw(
    owner: ReferenceLayerParent,
    node_index: usize,
    rectangle: [f32; 4],
) -> RuntimeCastDraw {
    let [left, top, right, bottom] = rectangle;
    let mut renderer_counter = 0;
    RuntimeCastDraw::Image(SrdDraw {
        origin: DrawOrigin {
            owner,
            scene_index: 0,
            layer_index: 0,
            node_index,
        },
        order: DrawOrder {
            renderer_layer_key: 0,
        },
        geometry: SrdQuad::new([
            test_vertex(left, top),
            test_vertex(left, bottom),
            test_vertex(right, top),
            test_vertex(right, bottom),
        ]),
        state: SrdDrawState::surface(
            SrdSurfaceStateInput {
                transform: SrdTransform::identity_2d([100, 100]),
                transform_mode: SimpleTransformMode::TwoDimensional,
                render_preset: 0,
                image_field_0c: 0,
                image_field_10: 0,
                image_field_14: 0,
                image_field_18: 0,
                textures: [None; 3],
            },
            &mut renderer_counter,
        )
        .unwrap(),
    })
}

fn submitted(runtime_draw_index: usize) -> RuntimeTargetCommand {
    RuntimeTargetCommand {
        sources: vec![RuntimeTargetCommandSource::Image { runtime_draw_index }],
        state: SrdDrawState::fennel(
            SrdTransform::identity_2d([100, 100]),
            SimpleTransformMode::TwoDimensional,
        ),
        order: DrawOrder {
            renderer_layer_key: 0,
        },
        topology: DrawTopology::TriangleStrip,
        vertex_count: 4,
    }
}

#[test]
fn cast_filters_apply_hidden_precedence_solo_unions_and_reference_descendants() {
    let root = ReferenceTarget {
        scene_index: 0,
        layer_index: 0,
    };
    let referenced = ReferenceTarget {
        scene_index: 0,
        layer_index: 1,
    };
    let plan = ReferenceRuntimePlan {
        instances: vec![ReferenceLayerInstance {
            parent: ReferenceLayerParent::ProjectLayer(root),
            reference_node_index: 1,
            target: referenced,
            is_2d: true,
            flip_y: false,
        }],
        unresolved: Vec::new(),
    };
    let draws = vec![
        test_draw(
            ReferenceLayerParent::ProjectLayer(root),
            0,
            [0.0, 0.0, 1.0, 1.0],
        ),
        test_draw(
            ReferenceLayerParent::ProjectLayer(root),
            2,
            [0.0, 0.0, 1.0, 1.0],
        ),
        test_draw(
            ReferenceLayerParent::ReferenceInstance(0),
            0,
            [0.0, 0.0, 1.0, 1.0],
        ),
    ];
    let mut hidden_casts = BTreeSet::new();
    let mut solo_casts = BTreeSet::new();
    let no_layers = BTreeSet::new();
    let parent = PreviewCastSelection {
        scene_index: 0,
        layer_index: 0,
        node_index: 0,
    };

    hidden_casts.insert(parent);
    solo_casts.insert(parent);
    assert!(
        filter_editor_preview_draws(
            draws.clone(),
            &selection_project(),
            &plan,
            PreviewVisibilityFilter {
                selected_scene_index: 0,
                hidden_layers: &no_layers,
                solo_layers: &no_layers,
                hidden_casts: &hidden_casts,
                solo_casts: &solo_casts,
            },
        )
        .unwrap()
        .is_empty()
    );

    hidden_casts.clear();
    solo_casts.clear();
    solo_casts.insert(PreviewCastSelection {
        scene_index: 0,
        layer_index: 0,
        node_index: 1,
    });
    solo_casts.insert(PreviewCastSelection {
        scene_index: 0,
        layer_index: 0,
        node_index: 2,
    });
    let soloed = filter_editor_preview_draws(
        draws.clone(),
        &selection_project(),
        &plan,
        PreviewVisibilityFilter {
            selected_scene_index: 0,
            hidden_layers: &no_layers,
            solo_layers: &no_layers,
            hidden_casts: &hidden_casts,
            solo_casts: &solo_casts,
        },
    )
    .unwrap();
    assert_eq!(soloed.len(), 2);
    assert!(soloed.iter().any(|draw| {
        draw.owner() == ReferenceLayerParent::ProjectLayer(root) && draw.node_index() == 2
    }));
    assert!(
        soloed
            .iter()
            .any(|draw| { draw.owner() == ReferenceLayerParent::ReferenceInstance(0) })
    );

    hidden_casts.insert(PreviewCastSelection {
        scene_index: 0,
        layer_index: 0,
        node_index: 1,
    });
    solo_casts.clear();
    solo_casts.insert(parent);
    let reference_hidden = filter_editor_preview_draws(
        draws,
        &selection_project(),
        &plan,
        PreviewVisibilityFilter {
            selected_scene_index: 0,
            hidden_layers: &no_layers,
            solo_layers: &no_layers,
            hidden_casts: &hidden_casts,
            solo_casts: &solo_casts,
        },
    )
    .unwrap();
    assert_eq!(reference_hidden.len(), 2);
    assert!(
        reference_hidden
            .iter()
            .all(|draw| { draw.owner() == ReferenceLayerParent::ProjectLayer(root) })
    );
}

#[test]
fn selection_and_highlight_bounds_include_descendants_and_omit_no_geometry() {
    let selected_layer = ReferenceTarget {
        scene_index: 0,
        layer_index: 0,
    };
    let other_layer = ReferenceTarget {
        scene_index: 0,
        layer_index: 1,
    };
    let plan = ReferenceRuntimePlan {
        instances: vec![
            ReferenceLayerInstance {
                parent: ReferenceLayerParent::ProjectLayer(selected_layer),
                reference_node_index: 1,
                target: other_layer,
                is_2d: true,
                flip_y: false,
            },
            ReferenceLayerInstance {
                parent: ReferenceLayerParent::ReferenceInstance(0),
                reference_node_index: 0,
                target: other_layer,
                is_2d: true,
                flip_y: false,
            },
            ReferenceLayerInstance {
                parent: ReferenceLayerParent::ProjectLayer(other_layer),
                reference_node_index: 0,
                target: selected_layer,
                is_2d: true,
                flip_y: false,
            },
        ],
        unresolved: Vec::new(),
    };
    let draws = vec![
        test_draw(
            ReferenceLayerParent::ProjectLayer(selected_layer),
            0,
            [10.0, 20.0, 30.0, 40.0],
        ),
        test_draw(
            ReferenceLayerParent::ProjectLayer(selected_layer),
            1,
            [40.0, 50.0, 60.0, 70.0],
        ),
        test_draw(
            ReferenceLayerParent::ReferenceInstance(0),
            0,
            [70.0, 80.0, 90.0, 95.0],
        ),
        test_draw(
            ReferenceLayerParent::ReferenceInstance(1),
            0,
            [5.0, 15.0, 20.0, 25.0],
        ),
        test_draw(
            ReferenceLayerParent::ReferenceInstance(2),
            0,
            [0.0, 0.0, 100.0, 100.0],
        ),
    ];
    let submission = (0..draws.len()).map(submitted).collect::<Vec<_>>();
    let selection = PreviewCastSelection {
        scene_index: 0,
        layer_index: 0,
        node_index: 0,
    };
    let highlights = [
        PreviewHighlightRequest {
            selection,
            kind: crate::renderer::PreviewHighlightKind::Direct,
        },
        PreviewHighlightRequest {
            selection: PreviewCastSelection {
                scene_index: 0,
                layer_index: 1,
                node_index: 0,
            },
            kind: crate::renderer::PreviewHighlightKind::Inherited,
        },
        PreviewHighlightRequest {
            selection: PreviewCastSelection {
                scene_index: 0,
                layer_index: 0,
                node_index: 2,
            },
            kind: crate::renderer::PreviewHighlightKind::Direct,
        },
    ];

    assert_eq!(
        runtime_cast_bounds(
            &selection_project(),
            &plan,
            [(draws.as_slice(), submission.as_slice())],
            Some(selection),
            &highlights,
            [100, 100],
        ),
        (
            Some([4, 14, 86, 81]),
            vec![
                PreviewHighlightBounds {
                    kind: crate::renderer::PreviewHighlightKind::Direct,
                    bounds: [4, 14, 86, 81],
                },
                PreviewHighlightBounds {
                    kind: crate::renderer::PreviewHighlightKind::Inherited,
                    bounds: [0, 0, 100, 100],
                },
            ],
        ),
    );
}

#[test]
fn selection_bounds_ignore_unsubmitted_and_unrelated_draws_then_clip() {
    let selected_layer = ReferenceTarget {
        scene_index: 0,
        layer_index: 0,
    };
    let other_layer = ReferenceTarget {
        scene_index: 0,
        layer_index: 1,
    };
    let draws = vec![
        test_draw(
            ReferenceLayerParent::ProjectLayer(selected_layer),
            0,
            [10.0, 20.0, 120.0, 80.0],
        ),
        test_draw(
            ReferenceLayerParent::ProjectLayer(selected_layer),
            1,
            [-50.0, -50.0, 200.0, 200.0],
        ),
        test_draw(
            ReferenceLayerParent::ProjectLayer(other_layer),
            0,
            [-25.0, -25.0, 125.0, 125.0],
        ),
    ];
    let submission = vec![submitted(0), submitted(2)];

    assert_eq!(
        runtime_cast_bounds(
            &selection_project(),
            &ReferenceRuntimePlan::default(),
            [(draws.as_slice(), submission.as_slice())],
            Some(PreviewCastSelection {
                scene_index: 0,
                layer_index: 0,
                node_index: 0,
            }),
            &[],
            [100, 100],
        )
        .0,
        Some([9, 19, 91, 61]),
    );
}

#[test]
fn embedded_camera_replaces_only_the_host_projection_view() {
    let (host, _, _) = preview_snapshot_for_profile(PreviewProfile::CommonBackgroundMain).unwrap();
    let camera = CameraDefinition {
        position: [0.0, 0.0, 1000.0],
        target: [0.0; 3],
        angle_units: 8191,
        near: 10.0,
        far: 100_000.0,
    };
    let mut expected = host;
    expected.target_projection_view = camera.runtime_matrices(1920.0 / 1080.0).projection_view;

    assert_eq!(
        preview_snapshot_with_project_camera(host, camera, [1920, 1080]),
        expected,
    );
}

#[test]
fn missing_camera_zero_fallback_is_applied_without_a_host_matrix_substitute() {
    let (host, _, _) = preview_snapshot_for_profile(PreviewProfile::AdvertiseLogoMain).unwrap();
    let result =
        preview_snapshot_with_project_camera(host, CameraDefinition::ZERO_FALLBACK, [1920, 1080]);

    let expected = CameraDefinition::ZERO_FALLBACK
        .runtime_matrices(1920.0 / 1080.0)
        .projection_view;
    assert_eq!(
        result
            .target_projection_view
            .rows
            .map(|row| row.map(f32::to_bits)),
        expected.rows.map(|row| row.map(f32::to_bits)),
    );
    assert_ne!(
        result
            .target_projection_view
            .rows
            .map(|row| row.map(f32::to_bits)),
        host.target_projection_view
            .rows
            .map(|row| row.map(f32::to_bits)),
    );
}

#[test]
fn advertise_logo_animation_uses_native_half_and_whole_frame_samples() {
    assert_eq!(
        preview_animation_frame_samples(PreviewProfile::AdvertiseLogoMain, true),
        (0.5, Some(1.0)),
    );
    assert_eq!(
        preview_animation_frame_samples(PreviewProfile::AdvertiseLogoMain, false),
        (0.0, None),
    );
    assert_eq!(
        preview_animation_frame_samples(PreviewProfile::CommonBackgroundMain, true),
        (0.0, None),
    );
}

#[test]
fn selection_bounds_union_all_submitted_frame_samples() {
    let owner = ReferenceLayerParent::ProjectLayer(ReferenceTarget {
        scene_index: 0,
        layer_index: 0,
    });
    let first_draws = vec![test_draw(owner, 0, [10.0, 20.0, 30.0, 40.0])];
    let trailing_draws = vec![test_draw(owner, 0, [60.0, 70.0, 90.0, 95.0])];
    let submission = vec![submitted(0)];

    assert_eq!(
        runtime_cast_bounds(
            &selection_project(),
            &ReferenceRuntimePlan::default(),
            [
                (first_draws.as_slice(), submission.as_slice()),
                (trailing_draws.as_slice(), submission.as_slice()),
            ],
            Some(PreviewCastSelection {
                scene_index: 0,
                layer_index: 0,
                node_index: 0,
            }),
            &[],
            [100, 100],
        )
        .0,
        Some([9, 19, 81, 76]),
    );
}
