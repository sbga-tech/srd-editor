use std::fmt;

use crate::camera::{build_look_at_rh_game, build_perspective_fov_rh_game};
use crate::projection::{Matrix4x4, mul_matrix4x4_game};
use crate::target_pass::{
    AIR_SCENE_BASE_PASSES, BasePassProfile, ScenePassProfile, ScenePassProfileError,
    SrdType1TargetFilter, build_scene_pass_profile, scene_target_dispatch_mask,
};
use crate::transform::Affine3x4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHostProfileError(pub String);

impl fmt::Display for GameHostProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for GameHostProfileError {}

/// Chusan-owned `air::Scene` configuration recovered from the concrete
/// MainScene/BgScene construction path. Present dimensions remain explicit:
/// the game copies them from the selected runtime present buffer rather than
/// using a scene-local fixed resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChusanAirSceneTargetProfile {
    pub name: &'static str,
    pub registration_order: i32,
    /// The third argument to `sub_602210`: after successful registration it
    /// writes this target to render-manager slot `+0x11C`. It is independent
    /// from the Scene `Enable` property at target `+0x10`.
    pub registration_sets_manager_current_target: bool,
    /// Value of Scene property 0 immediately after the common constructor and
    /// the concrete Chusan initialization path.
    pub initial_enable: bool,
    pub present_index: u8,
    pub draw_index: u8,
    pub attribute: u8,
    pub present_mode: u8,
    pub rotation_mode: u8,
    pub shader_on_demand: bool,
    pub request_color_offscreen: bool,
    pub request_depth_offscreen: bool,
    pub clear: bool,
}

pub const CHUSAN_MAIN_SCENE: ChusanAirSceneTargetProfile = ChusanAirSceneTargetProfile {
    name: "MainScene",
    registration_order: 10_000,
    registration_sets_manager_current_target: true,
    initial_enable: true,
    present_index: 0,
    draw_index: 0,
    attribute: 0,
    present_mode: 1,
    rotation_mode: 0,
    shader_on_demand: false,
    request_color_offscreen: true,
    request_depth_offscreen: true,
    clear: false,
};

pub const CHUSAN_BG_SCENE: ChusanAirSceneTargetProfile = ChusanAirSceneTargetProfile {
    name: "BgScene",
    registration_order: 9_900,
    registration_sets_manager_current_target: false,
    initial_enable: true,
    present_index: 0,
    draw_index: 16,
    attribute: 0,
    present_mode: 0,
    rotation_mode: 0,
    shader_on_demand: false,
    request_color_offscreen: true,
    request_depth_offscreen: true,
    clear: false,
};

/// Concrete `projView::AdvertiseLogoObject` SrPlayer state after its common
/// initialization path. This describes only the embedded player and its
/// scene-node placement; target selection and present dimensions remain
/// explicit because its empty `TargetScene` routes packets globally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChusanAdvertiseLogoPlayerProfile {
    pub draw_target_scene_only: bool,
    pub target_scene: &'static str,
    pub first_calc_matrix_enabled: bool,
    pub draw_mask: u32,
    pub layer_2d: u8,
    pub common_init_ends_enabled: bool,
}

pub const CHUSAN_ADVERTISE_LOGO_PLAYER: ChusanAdvertiseLogoPlayerProfile =
    ChusanAdvertiseLogoPlayerProfile {
        draw_target_scene_only: true,
        target_scene: "",
        first_calc_matrix_enabled: false,
        draw_mask: 0xFFFF,
        layer_2d: 100,
        common_init_ends_enabled: false,
    };

/// Concrete `CommonBackGroundObject` embedded SrPlayer state after resource
/// initialization. The later mode transition re-enables the player; this
/// profile keeps that lifecycle distinction explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChusanCommonBackgroundPlayerProfile {
    pub draw_target_scene_only: bool,
    pub target_scene: &'static str,
    pub first_calc_matrix_enabled: bool,
    pub draw_mask: u32,
    pub layer_2d: u8,
    pub common_init_ends_enabled: bool,
    pub mode_transition_enables: bool,
}

pub const CHUSAN_COMMON_BACKGROUND_PLAYER: ChusanCommonBackgroundPlayerProfile =
    ChusanCommonBackgroundPlayerProfile {
        draw_target_scene_only: true,
        target_scene: "",
        first_calc_matrix_enabled: false,
        draw_mask: 0xFFFF,
        layer_2d: 6,
        common_init_ends_enabled: false,
        mode_transition_enables: true,
    };

/// Concrete `PlayLinkedVerseGateObject::Impl` embedded SrPlayer state. The
/// resource id/path pairing comes from the game's load function and shipped
/// `SurfFileTableRecord.bin`, while the remaining values come from the common
/// SrPlayer initialization and the class-specific property write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChusanLinkedVerseGatePlayerProfile {
    pub surf_file_id: u32,
    pub surf_file_name: &'static str,
    pub surf_file_path: &'static str,
    pub draw_target_scene_only: bool,
    pub target_scene: &'static str,
    pub first_calc_matrix_enabled: bool,
    pub draw_mask: u32,
    pub layer_2d: u8,
}

pub const CHUSAN_LINKED_VERSE_GATE_PLAYER: ChusanLinkedVerseGatePlayerProfile =
    ChusanLinkedVerseGatePlayerProfile {
        surf_file_id: 84,
        surf_file_name: "LinkedVerseGate",
        surf_file_path: "play/linkedVerse/CHU_UI_LinkedVERSE_Gate_00.srd",
        draw_target_scene_only: true,
        target_scene: "",
        first_calc_matrix_enabled: false,
        draw_mask: 0xFFFF,
        layer_2d: 70,
    };

/// Initial SrRenderer layer key constructed by the host before any player
/// property overrides are applied.
pub const SRD_RENDERER_INITIAL_LAYER_KEY: u32 = 0x0000_8580;

/// Applies SrPlayer property 6 (`2DLayer`) to the host-owned layer key.
pub const fn srd_renderer_layer_key_for_2d_layer(layer_2d: u8) -> u32 {
    (SRD_RENDERER_INITIAL_LAYER_KEY & !0x7f00) | (((layer_2d as u32) << 8) & 0x7f00)
}

/// Camera target resolved from an SrPlayer's optional `TargetScene`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectTargetSnapshot {
    pub projection_view: Matrix4x4,
    pub render_size: [u32; 2],
}

impl ProjectTargetSnapshot {
    pub const fn new(projection_view: Matrix4x4, render_size: [u32; 2]) -> Self {
        Self {
            projection_view,
            render_size,
        }
    }
}

/// Immutable host and target state consumed while compiling one render plan.
///
/// There is deliberately no `Default`: an independent SRD does not identify a
/// unique game target, camera, or scene-node placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSnapshot {
    pub first_calc_matrix: Affine3x4,
    pub renderer_layer_key: u32,
    pub project_target: Option<ProjectTargetSnapshot>,
    pub target_projection_view: Matrix4x4,
    pub target_screen_size: [u32; 2],
}

impl WorldSnapshot {
    pub const fn new(
        first_calc_matrix: Affine3x4,
        renderer_layer_key: u32,
        project_target: Option<ProjectTargetSnapshot>,
        target_projection_view: Matrix4x4,
        target_screen_size: [u32; 2],
    ) -> Self {
        Self {
            first_calc_matrix,
            renderer_layer_key,
            project_target,
            target_projection_view,
            target_screen_size,
        }
    }
}

impl ChusanAirSceneTargetProfile {
    /// MainScene and BgScene both retain the five `PassBasic` objects installed
    /// by the common `air::Scene` constructor.
    pub const fn base_passes(self) -> &'static [BasePassProfile; 5] {
        &AIR_SCENE_BASE_PASSES
    }

    pub fn scene_pass_profile(self) -> Result<ScenePassProfile, ScenePassProfileError> {
        build_scene_pass_profile(self.base_passes())
    }

    /// Scene virtual `+0x44` dispatch state immediately after Chusan's
    /// concrete MainScene/BgScene setup.
    pub const fn initial_dispatch_mask(self) -> Option<u32> {
        scene_target_dispatch_mask(self.initial_enable, self.draw_index)
    }

    /// Rebuilds the target Camera `Projection * View` used by the game after
    /// `air::Camera` attaches to the scene. The attach callback overwrites the
    /// constructor's Aspect=1 with `scene_width / scene_height`.
    pub fn projection_view_for_present_size(
        self,
        width: u32,
        height: u32,
    ) -> Result<Matrix4x4, GameHostProfileError> {
        if width == 0 || height == 0 {
            return Err(GameHostProfileError(format!(
                "{} present size must be non-zero, got {width}x{height}",
                self.name
            )));
        }

        let aspect = (width as f32) / (height as f32);
        let view = build_look_at_rh_game([0.0, 0.0, 30.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let projection = build_perspective_fov_rh_game(45.0, aspect, 1.0, 30_000.0, 0.0, 0.0);
        Ok(mul_matrix4x4_game(&projection, &view))
    }
}

impl ChusanAdvertiseLogoPlayerProfile {
    /// Builds the exact type-1 filter inputs for Advertise's globally queued
    /// SRD commands at the end of the proven scene/player initialization.
    pub const fn initial_srd_target_filter(
        self,
        target: ChusanAirSceneTargetProfile,
    ) -> SrdType1TargetFilter {
        SrdType1TargetFilter {
            target_dispatch_mask: target.initial_dispatch_mask(),
            target_attribute: target.attribute,
            command_draw_mask: self.draw_mask,
        }
    }

    /// Combines the proven root-node matrix with an explicitly selected Chusan
    /// receiving target. `AdvertiseLogoObject` leaves the embedded SrPlayer
    /// parent null, so its composite matrix remains constructor identity. Its
    /// The empty `TargetScene` lookup is proven to return null in the shipped
    /// data and normal runtime registration path. `target` is the Scene that
    /// later receives the globally queued packet.
    pub fn host_context_for_target(
        self,
        target: ChusanAirSceneTargetProfile,
        present_width: u32,
        present_height: u32,
        target_screen_size: [u32; 2],
    ) -> Result<WorldSnapshot, GameHostProfileError> {
        if target_screen_size.contains(&0) {
            return Err(GameHostProfileError(format!(
                "{} target screen size must be non-zero, got {}x{}",
                target.name, target_screen_size[0], target_screen_size[1]
            )));
        }
        let target_projection_view =
            target.projection_view_for_present_size(present_width, present_height)?;
        Ok(WorldSnapshot::new(
            Affine3x4::IDENTITY,
            srd_renderer_layer_key_for_2d_layer(self.layer_2d),
            None,
            target_projection_view,
            target_screen_size,
        ))
    }
}

impl ChusanCommonBackgroundPlayerProfile {
    pub const fn initial_srd_target_filter(
        self,
        target: ChusanAirSceneTargetProfile,
    ) -> SrdType1TargetFilter {
        SrdType1TargetFilter {
            target_dispatch_mask: target.initial_dispatch_mask(),
            target_attribute: target.attribute,
            command_draw_mask: self.draw_mask,
        }
    }

    /// CommonBackGroundObject is globally registered and looked up by name;
    /// its audited class methods never attach the embedded SrPlayer to a
    /// GraphNode parent. The root FirstCalc matrix therefore remains identity.
    /// The empty `TargetScene` lookup is proven to return null; `target` is
    /// the Scene that later receives the globally queued packet.
    pub fn host_context_for_target(
        self,
        target: ChusanAirSceneTargetProfile,
        present_width: u32,
        present_height: u32,
        target_screen_size: [u32; 2],
    ) -> Result<WorldSnapshot, GameHostProfileError> {
        if target_screen_size.contains(&0) {
            return Err(GameHostProfileError(format!(
                "{} target screen size must be non-zero, got {}x{}",
                target.name, target_screen_size[0], target_screen_size[1]
            )));
        }
        let target_projection_view =
            target.projection_view_for_present_size(present_width, present_height)?;
        Ok(WorldSnapshot::new(
            Affine3x4::IDENTITY,
            srd_renderer_layer_key_for_2d_layer(self.layer_2d),
            None,
            target_projection_view,
            target_screen_size,
        ))
    }
}

impl ChusanLinkedVerseGatePlayerProfile {
    pub const fn initial_srd_target_filter(
        self,
        target: ChusanAirSceneTargetProfile,
    ) -> SrdType1TargetFilter {
        SrdType1TargetFilter {
            target_dispatch_mask: target.initial_dispatch_mask(),
            target_attribute: target.attribute,
            command_draw_mask: self.draw_mask,
        }
    }

    /// `PlayLinkedVerseGateObject::Impl` constructs the SrPlayer at `+0x68`.
    /// The complete class range has no GraphNode parent setter or direct parent
    /// write, so the constructor identity remains the FirstCalc input. Its
    /// empty TargetScene takes the already proven null/global-queue branch.
    pub fn host_context_for_target(
        self,
        target: ChusanAirSceneTargetProfile,
        present_width: u32,
        present_height: u32,
        target_screen_size: [u32; 2],
    ) -> Result<WorldSnapshot, GameHostProfileError> {
        if target_screen_size.contains(&0) {
            return Err(GameHostProfileError(format!(
                "{} target screen size must be non-zero, got {}x{}",
                target.name, target_screen_size[0], target_screen_size[1]
            )));
        }
        let target_projection_view =
            target.projection_view_for_present_size(present_width, present_height)?;
        Ok(WorldSnapshot::new(
            Affine3x4::IDENTITY,
            srd_renderer_layer_key_for_2d_layer(self.layer_2d),
            None,
            target_projection_view,
            target_screen_size,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_and_background_scene_settings_match_the_concrete_chusan_constructor() {
        assert_eq!(CHUSAN_MAIN_SCENE.name, "MainScene");
        assert_eq!(CHUSAN_MAIN_SCENE.registration_order, 10_000);
        assert_eq!(CHUSAN_MAIN_SCENE.draw_index, 0);
        assert_eq!(CHUSAN_MAIN_SCENE.present_mode, 1);
        assert_eq!(CHUSAN_MAIN_SCENE.rotation_mode, 0);

        assert_eq!(CHUSAN_BG_SCENE.name, "BgScene");
        assert_eq!(CHUSAN_BG_SCENE.registration_order, 9_900);
        assert_eq!(CHUSAN_BG_SCENE.draw_index, 16);
        assert_eq!(CHUSAN_BG_SCENE.present_mode, 0);
        assert_eq!(CHUSAN_BG_SCENE.rotation_mode, 0);

        assert!(CHUSAN_MAIN_SCENE.registration_sets_manager_current_target);
        assert!(!CHUSAN_BG_SCENE.registration_sets_manager_current_target);

        for profile in [CHUSAN_MAIN_SCENE, CHUSAN_BG_SCENE] {
            assert!(profile.initial_enable);
            assert_eq!(profile.present_index, 0);
            assert_eq!(profile.attribute, 0);
            assert!(!profile.shader_on_demand);
            assert!(profile.request_color_offscreen);
            assert!(profile.request_depth_offscreen);
            assert!(!profile.clear);
            let passes = profile.scene_pass_profile().unwrap();
            assert_eq!(passes.rules.len(), 5);
            assert_eq!(passes.target_entries[4].first, 0);
            assert_eq!(passes.target_entries[24].last, 4);
        }
        assert_eq!(CHUSAN_MAIN_SCENE.initial_dispatch_mask(), Some(1));
        assert_eq!(CHUSAN_BG_SCENE.initial_dispatch_mask(), Some(0x1_0000));
    }

    #[test]
    fn attached_air_camera_uses_the_present_aspect_ratio() {
        let square = CHUSAN_MAIN_SCENE
            .projection_view_for_present_size(1080, 1080)
            .unwrap();
        let portrait = CHUSAN_MAIN_SCENE
            .projection_view_for_present_size(1080, 1920)
            .unwrap();

        assert_eq!(square.rows[1], portrait.rows[1]);
        assert_eq!(portrait.rows[0][0], square.rows[0][0] * (1920.0 / 1080.0));
        assert_eq!(square.rows[2], portrait.rows[2]);
        assert_eq!(square.rows[3], portrait.rows[3]);
    }

    #[test]
    fn present_dimensions_are_required_instead_of_assuming_a_game_resolution() {
        assert!(
            CHUSAN_MAIN_SCENE
                .projection_view_for_present_size(0, 1080)
                .is_err()
        );
        assert!(
            CHUSAN_MAIN_SCENE
                .projection_view_for_present_size(1920, 0)
                .is_err()
        );
    }

    #[test]
    fn advertise_logo_common_init_matches_the_concrete_player_writes() {
        assert_eq!(
            CHUSAN_ADVERTISE_LOGO_PLAYER,
            ChusanAdvertiseLogoPlayerProfile {
                draw_target_scene_only: true,
                target_scene: "",
                first_calc_matrix_enabled: false,
                draw_mask: 0xFFFF,
                layer_2d: 100,
                common_init_ends_enabled: false,
            }
        );
    }

    #[test]
    fn advertise_default_draw_mask_initially_admits_main_and_rejects_background() {
        let state = crate::target_pass::SrdQueueState::surface(false);
        assert!(
            CHUSAN_ADVERTISE_LOGO_PLAYER
                .initial_srd_target_filter(CHUSAN_MAIN_SCENE)
                .accepts(state)
        );
        assert!(
            !CHUSAN_ADVERTISE_LOGO_PLAYER
                .initial_srd_target_filter(CHUSAN_BG_SCENE)
                .accepts(state)
        );
    }

    #[test]
    fn advertise_logo_host_context_uses_proven_null_lookup() {
        let context = CHUSAN_ADVERTISE_LOGO_PLAYER
            .host_context_for_target(CHUSAN_MAIN_SCENE, 1080, 1920, [1920, 1080])
            .unwrap();
        assert_eq!(context.first_calc_matrix, Affine3x4::IDENTITY);
        assert_eq!(context.renderer_layer_key, 0xe480);
        assert_eq!(
            context.target_projection_view,
            CHUSAN_MAIN_SCENE
                .projection_view_for_present_size(1080, 1920)
                .unwrap()
        );
        assert_eq!(context.project_target, None);
        assert_eq!(context.target_screen_size, [1920, 1080]);
    }

    #[test]
    fn common_background_profile_matches_the_concrete_player_writes() {
        assert_eq!(
            CHUSAN_COMMON_BACKGROUND_PLAYER,
            ChusanCommonBackgroundPlayerProfile {
                draw_target_scene_only: true,
                target_scene: "",
                first_calc_matrix_enabled: false,
                draw_mask: 0xFFFF,
                layer_2d: 6,
                common_init_ends_enabled: false,
                mode_transition_enables: true,
            }
        );
        let context = CHUSAN_COMMON_BACKGROUND_PLAYER
            .host_context_for_target(CHUSAN_MAIN_SCENE, 1080, 1920, [1920, 1080])
            .unwrap();
        assert_eq!(context.first_calc_matrix, Affine3x4::IDENTITY);
        assert_eq!(context.renderer_layer_key, 0x8680);
        let state = crate::target_pass::SrdQueueState::surface(false);
        assert!(
            CHUSAN_COMMON_BACKGROUND_PLAYER
                .initial_srd_target_filter(CHUSAN_MAIN_SCENE)
                .accepts(state)
        );
        assert!(
            !CHUSAN_COMMON_BACKGROUND_PLAYER
                .initial_srd_target_filter(CHUSAN_BG_SCENE)
                .accepts(state)
        );
    }

    #[test]
    fn linked_verse_gate_profile_matches_the_concrete_player_and_resource_table() {
        assert_eq!(
            CHUSAN_LINKED_VERSE_GATE_PLAYER,
            ChusanLinkedVerseGatePlayerProfile {
                surf_file_id: 84,
                surf_file_name: "LinkedVerseGate",
                surf_file_path: "play/linkedVerse/CHU_UI_LinkedVERSE_Gate_00.srd",
                draw_target_scene_only: true,
                target_scene: "",
                first_calc_matrix_enabled: false,
                draw_mask: 0xFFFF,
                layer_2d: 70,
            }
        );
        let context = CHUSAN_LINKED_VERSE_GATE_PLAYER
            .host_context_for_target(CHUSAN_MAIN_SCENE, 1080, 1920, [1920, 1080])
            .unwrap();
        assert_eq!(context.first_calc_matrix, Affine3x4::IDENTITY);
        assert_eq!(context.renderer_layer_key, 0xC680);
        assert_eq!(context.project_target, None);
        let state = crate::target_pass::SrdQueueState::surface(false);
        assert!(
            CHUSAN_LINKED_VERSE_GATE_PLAYER
                .initial_srd_target_filter(CHUSAN_MAIN_SCENE)
                .accepts(state)
        );
        assert!(
            !CHUSAN_LINKED_VERSE_GATE_PLAYER
                .initial_srd_target_filter(CHUSAN_BG_SCENE)
                .accepts(state)
        );
    }
}
