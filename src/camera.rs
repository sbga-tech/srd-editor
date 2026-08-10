use std::fmt;

use crate::projection::{Matrix4x4, mul_matrix4x4_game};
use crate::vtbf::{Block, Property, SrdFile};

const ANGLE_UNIT_TO_DEGREES: f32 = f32::from_bits(0x3bb4_0000);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const CAMERA_EPSILON: f32 = f32::from_bits(0x38d1_b717);
const VECTOR_EPSILON: f32 = f32::from_bits(0x3400_0000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraError(pub String);

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CameraError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraDefinition {
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub angle_units: i32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraDefinition {
    fn default() -> Self {
        Self::ZERO_FALLBACK
    }
}

impl CameraDefinition {
    pub const ZERO_FALLBACK: Self = Self {
        position: [0.0; 3],
        target: [0.0; 3],
        angle_units: 0,
        near: 0.0,
        far: 0.0,
    };
    pub fn from_project_block(file: &SrdFile, project: &Block) -> Result<Self, CameraError> {
        if !project.is_tag(b"PROJ") {
            return Err(CameraError("camera parent block is not PROJ".into()));
        }
        let mut camera = Self::default();
        for block in project
            .children
            .iter()
            .filter(|block| block.is_tag(b"CAM "))
        {
            camera.apply_block(file, block)?;
        }
        Ok(camera)
    }

    pub fn angle_degrees(self) -> f32 {
        (self.angle_units as f32) * ANGLE_UNIT_TO_DEGREES
    }

    pub fn runtime_matrices(self, projection_width: f32) -> CameraMatrices {
        let mut position = self.position;
        let delta = [
            self.target[0] - position[0],
            self.target[1] - position[1],
            self.target[2] - position[2],
        ];
        let distance = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt();
        if distance < VECTOR_EPSILON {
            position[2] += CAMERA_EPSILON;
        }

        let view = build_look_at_rh_game(position, self.target, [0.0, 1.0, 0.0]);
        let projection = build_perspective_fov_rh_game(
            self.angle_degrees(),
            projection_width,
            self.near,
            self.far,
            0.0,
            0.0,
        );
        let projection_view = mul_matrix4x4_game(&projection, &view);
        CameraMatrices {
            view,
            projection,
            projection_view,
        }
    }

    fn apply_block(&mut self, file: &SrdFile, block: &Block) -> Result<(), CameraError> {
        for property in &block.properties {
            match property.code {
                0x12 => self.position = vector3(file, property, "CAM 0x12")?,
                0x13 => self.target = vector3(file, property, "CAM 0x13")?,
                0x14 => {
                    self.angle_units = property
                        .read_signed_scalar(file)
                        .ok_or_else(|| CameraError("invalid CAM 0x14 angle".into()))?;
                }
                0x15 => self.near = scalar_f32(file, property, "CAM 0x15")?,
                0x16 => self.far = scalar_f32(file, property, "CAM 0x16")?,
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraMatrices {
    pub view: Matrix4x4,
    pub projection: Matrix4x4,
    pub projection_view: Matrix4x4,
}

pub fn build_perspective_fov_rh_game(
    fov_y_degrees: f32,
    aspect: f32,
    near: f32,
    far: f32,
    offset_x: f32,
    offset_y: f32,
) -> Matrix4x4 {
    let radians = fov_y_degrees * DEGREES_TO_RADIANS;
    let half_radians = radians * 0.5;
    let cotangent = (1.0f64 / tan_game(half_radians)) as f32;
    let far = if far <= near + CAMERA_EPSILON {
        near + CAMERA_EPSILON
    } else {
        far
    };
    let depth_reciprocal = 1.0 / (near - far);

    Matrix4x4 {
        rows: [
            [cotangent / aspect, 0.0, offset_x, 0.0],
            [0.0, cotangent, offset_y, 0.0],
            [
                0.0,
                0.0,
                depth_reciprocal * far,
                (near * far) * depth_reciprocal,
            ],
            [0.0, 0.0, -1.0, 0.0],
        ],
    }
}

pub fn build_orthographic_rh_game(
    width: f32,
    height: f32,
    near: f32,
    far: f32,
    offset_x: f32,
    offset_y: f32,
) -> Matrix4x4 {
    let far = if far <= near + CAMERA_EPSILON {
        near + CAMERA_EPSILON
    } else {
        far
    };
    let depth_reciprocal = 1.0 / (far - near);
    Matrix4x4 {
        rows: [
            [2.0 / width, 0.0, offset_x, 0.0],
            [0.0, 2.0 / height, offset_y, 0.0],
            [0.0, 0.0, -depth_reciprocal, -(depth_reciprocal * near)],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

pub fn build_look_at_rh_game(position: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Matrix4x4 {
    let up = if up.iter().all(|component| component.abs() < VECTOR_EPSILON) {
        [0.0, 1.0, 0.0]
    } else {
        up
    };

    let direction = [
        position[0] - target[0],
        position[1] - target[1],
        position[2] - target[2],
    ];
    let direction_length = ((direction[0] * direction[0] + direction[1] * direction[1])
        + direction[2] * direction[2])
        .sqrt();
    let direction_reciprocal = 1.0 / direction_length;
    let forward = [
        direction_reciprocal * direction[0],
        direction_reciprocal * direction[1],
        direction_reciprocal * direction[2],
    ];

    let right_unnormalized = [
        forward[2] * up[1] - forward[1] * up[2],
        forward[0] * up[2] - forward[2] * up[0],
        forward[1] * up[0] - forward[0] * up[1],
    ];
    let right_length = ((right_unnormalized[0] * right_unnormalized[0]
        + right_unnormalized[1] * right_unnormalized[1])
        + right_unnormalized[2] * right_unnormalized[2])
        .sqrt();
    let right_reciprocal = 1.0 / right_length;
    let right = [
        right_reciprocal * right_unnormalized[0],
        right_reciprocal * right_unnormalized[1],
        right_reciprocal * right_unnormalized[2],
    ];
    let corrected_up = [
        forward[1] * right[2] - forward[2] * right[1],
        forward[2] * right[0] - forward[0] * right[2],
        forward[0] * right[1] - forward[1] * right[0],
    ];

    Matrix4x4 {
        rows: [
            [
                right[0],
                right[1],
                right[2],
                negative_dot_game(right, position),
            ],
            [
                corrected_up[0],
                corrected_up[1],
                corrected_up[2],
                negative_dot_game(corrected_up, position),
            ],
            [
                forward[0],
                forward[1],
                forward[2],
                negative_dot_game(forward, position),
            ],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

fn negative_dot_game(axis: [f32; 3], position: [f32; 3]) -> f32 {
    ((-axis[1] * position[1]) + (-axis[0] * position[0])) + (-axis[2] * position[2])
}

fn tan_game(angle: f32) -> f64 {
    const INV_PI: f64 = f64::from_bits(0x3fd4_5f30_6dc9_c883);
    let turns = f64::from(angle) * INV_PI;
    let fraction = turns - f64::from(turns.trunc() as i32);

    if fraction < 0.0 {
        if fraction < -0.5 {
            if fraction >= -0.75 {
                -1.0 / tan_polynomial(fraction + 0.5)
            } else {
                tan_polynomial(fraction + 1.0)
            }
        } else if fraction < -0.25 {
            1.0 / tan_polynomial(-0.5 - fraction)
        } else {
            tan_polynomial(fraction)
        }
    } else if fraction <= 0.5 {
        if fraction > 0.25 {
            1.0 / tan_polynomial(0.5 - fraction)
        } else {
            tan_polynomial(fraction)
        }
    } else if fraction > 0.75 {
        tan_polynomial(fraction - 1.0)
    } else {
        -1.0 / tan_polynomial(fraction - 0.5)
    }
}

#[allow(clippy::assign_op_pattern)] // Preserve the binary's multiply/store/add sequence.
fn tan_polynomial(turn_fraction: f64) -> f64 {
    const PI: f64 = f64::from_bits(0x4009_21fb_5444_2d18);
    const COEFFICIENTS: [f64; 11] = [
        f64::from_bits(0x3f04_97d8_eea2_5259),
        f64::from_bits(0x3f19_67e1_8afc_afad),
        f64::from_bits(0x3f2f_57d7_734d_1664),
        f64::from_bits(0x3f43_5582_4803_6744),
        f64::from_bits(0x3f57_da36_452b_75e3),
        f64::from_bits(0x3f6d_6d3d_0e15_7de0),
        f64::from_bits(0x3f82_26e3_55e6_c23d),
        f64::from_bits(0x3f96_64f4_882c_10fa),
        f64::from_bits(0x3fab_a1ba_1ba1_ba1c),
        f64::from_bits(0x3fc1_1111_1111_1111),
        f64::from_bits(0x3fd5_5555_5555_5555),
    ];
    let x = turn_fraction * PI;
    let squared = x * x;
    let mut polynomial = COEFFICIENTS[0];
    for coefficient in &COEFFICIENTS[1..] {
        polynomial = polynomial * squared;
        polynomial += coefficient;
    }
    polynomial = polynomial * squared;
    polynomial += 1.0;
    polynomial * x
}

fn vector3(file: &SrdFile, property: &Property, label: &str) -> Result<[f32; 3], CameraError> {
    Ok([
        property
            .read_scalar_as_f32_at(file, 0)
            .ok_or_else(|| CameraError(format!("invalid {label}[0]")))?,
        property
            .read_scalar_as_f32_at(file, 1)
            .ok_or_else(|| CameraError(format!("invalid {label}[1]")))?,
        property
            .read_scalar_as_f32_at(file, 2)
            .ok_or_else(|| CameraError(format!("invalid {label}[2]")))?,
    ])
}

fn scalar_f32(file: &SrdFile, property: &Property, label: &str) -> Result<f32, CameraError> {
    property
        .read_scalar_as_f32(file)
        .ok_or_else(|| CameraError(format!("invalid {label}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_angle_unit_matches_the_game_constant() {
        let camera = CameraDefinition {
            angle_units: 8191,
            ..CameraDefinition::default()
        };
        assert_eq!(camera.angle_degrees().to_bits(), 0x4233_fa60);
    }

    #[test]
    fn project_without_camera_block_uses_exact_zero_fallback() {
        let file = SrdFile::parse(b"VTBF\0\0\0\0SRFF\0\0\0\0".to_vec()).unwrap();
        let project = Block {
            offset: 16,
            sub_sig: [0; 4],
            size_field: 0,
            tag: *b"PROJ",
            properties: Vec::new(),
            property_tail: 0..0,
            children: Vec::new(),
        };

        assert_eq!(
            CameraDefinition::from_project_block(&file, &project).unwrap(),
            CameraDefinition::ZERO_FALLBACK,
        );
    }

    #[test]
    fn look_at_camera_on_positive_z_matches_the_game_basis() {
        assert_eq!(
            build_look_at_rh_game([0.0, 0.0, 1000.0], [0.0; 3], [0.0, 1.0, 0.0]),
            Matrix4x4 {
                rows: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, -1000.0],
                    [0.0, 0.0, 0.0, 1.0],
                ]
            }
        );
    }

    #[test]
    fn runtime_combined_matrix_is_projection_times_view() {
        let camera = CameraDefinition {
            position: [0.0, 0.0, 1000.0],
            target: [0.0; 3],
            angle_units: 8191,
            near: 10.0,
            far: 100_000.0,
        };
        let matrices = camera.runtime_matrices(1920.0);
        assert_eq!(
            matrices.projection_view,
            mul_matrix4x4_game(&matrices.projection, &matrices.view)
        );
        assert_eq!(matrices.projection.rows[3], [0.0, 0.0, -1.0, 0.0]);
    }

    #[test]
    fn coincident_camera_points_nudge_position_z_before_look_at() {
        let camera = CameraDefinition {
            angle_units: 8191,
            near: 1.0,
            far: 10.0,
            ..CameraDefinition::default()
        };
        let matrices = camera.runtime_matrices(640.0);
        assert_eq!(matrices.view.rows[2], [0.0, 0.0, 1.0, -CAMERA_EPSILON]);
    }

    #[test]
    fn orthographic_builder_uses_centered_width_and_height() {
        assert_eq!(
            build_orthographic_rh_game(640.0, 480.0, 10.0, 110.0, 0.0, 0.0),
            Matrix4x4 {
                rows: [
                    [2.0 / 640.0, 0.0, 0.0, 0.0],
                    [0.0, 2.0 / 480.0, 0.0, 0.0],
                    [0.0, 0.0, -0.01, -0.099999994],
                    [0.0, 0.0, 0.0, 1.0],
                ]
            }
        );
    }
}
