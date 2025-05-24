use fixed::{types::extra::{U16, U32}, FixedI32, FixedI64};
use fixed_trigonometry::*;
use fixed_trigonometry::atan::atan2;
use fixed_sqrt::FixedSqrt;
use bevy::{math::Affine3A, prelude::*};
use serde::{Serialize, Deserialize};

// Define our fixed-point types
// Using 16.16 fixed point for general use (good balance of range and precision)
pub type Fixed = FixedI32<U16>;
// Using 32.32 for intermediate calculations that need more precision
pub type FixedWide = FixedI64<U32>;


pub fn new(f: f32) -> Fixed {
    Fixed::from_num(f)
}

pub fn to_f32(f: Fixed) -> f32 {
    f.to_num::<f32>()
}


// Fixed-point 2D vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct FixedVec2 {
    pub x: Fixed,
    pub y: Fixed,
}

// Fixed-point 3D vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct FixedVec3 {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
}


// Conversion constants
pub const FIXED_ZERO: Fixed= Fixed::from_bits(0);
pub const FIXED_ONE: Fixed = Fixed::from_bits(1 << 16);
pub const FIXED_HALF: Fixed = Fixed::from_bits(1 << 15);
pub const FIXED_PI: Fixed = Fixed::from_bits(205887); // π in 16.16 fixed point
pub const FIXED_TAU: Fixed = Fixed::from_bits(411775); // 2π in 16.16 fixed point

impl FixedVec2 {
    pub const ZERO: Self = Self {
        x: Fixed::from_bits(0),
        y: Fixed::from_bits(0),
    };
    
    pub fn new(x: Fixed, y: Fixed) -> Self {
        Self { x, y }
    }
    
    pub fn from_f32(x: f32, y: f32) -> Self {
        Self {
            x: Fixed::from_num(x),
            y: Fixed::from_num(y),
        }
    }
    
    pub fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.x.to_num::<f32>(), self.y.to_num::<f32>())
    }
    
    pub fn dot(&self, other: &Self) -> Fixed {
        self.x.saturating_mul(other.x).saturating_add(self.y.saturating_mul(other.y))
    }
    
    pub fn length_squared(&self) -> Fixed {
        self.dot(self)
    }
    
    pub fn length(&self) -> Fixed {
        // Use fixed-sqrt crate for deterministic square root
        self.length_squared().sqrt()
    }
    
    pub fn distance(&self, other: &Self) -> Fixed {
        (*self - *other).length()
    }
    
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > Fixed::from_bits(0) {
            Self {
                x: self.x.saturating_div(len),
                y: self.y.saturating_div(len),
            }
        } else {
            Self::ZERO
        }
    }
    
    pub fn normalize_or_zero(&self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > Fixed::from_bits(256) { // Small epsilon to avoid division by very small numbers
            self.normalize()
        } else {
            Self::ZERO
        }
    }
    
    pub fn clamp_length_max(&self, max: Fixed) -> Self {
        let len_sq = self.length_squared();
        let max_sq = max.saturating_mul(max);
        if len_sq > max_sq {
            self.normalize() * max
        } else {
            *self
        }
    }
}

impl std::ops::Add for FixedVec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_add(other.x),
            y: self.y.saturating_add(other.y),
        }
    }
}

impl std::ops::Sub for FixedVec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_sub(other.x),
            y: self.y.saturating_sub(other.y),
        }
    }
}

impl std::ops::Mul<Fixed> for FixedVec2 {
    type Output = Self;
    fn mul(self, scalar: Fixed) -> Self {
        Self {
            x: self.x.saturating_mul(scalar),
            y: self.y.saturating_mul(scalar),
        }
    }
}

impl std::ops::Div<Fixed> for FixedVec2 {
    type Output = Self;
    fn div(self, scalar: Fixed) -> Self {
        Self {
            x: self.x.saturating_div(scalar),
            y: self.y.saturating_div(scalar),
        }
    }
}

impl std::ops::AddAssign for FixedVec2 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl std::ops::Neg for FixedVec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl FixedVec3 {
    pub const ZERO: Self = Self {
        x: Fixed::from_bits(0),
        y: Fixed::from_bits(0),
        z: Fixed::from_bits(0),
    };

    pub const ONE: Self = Self {
        x: FIXED_ONE,
        y: FIXED_ONE,
        z: FIXED_ONE,
    };
    
    pub fn new(x: Fixed, y: Fixed, z: Fixed) -> Self {
        Self { x, y, z }
    }
    
    pub fn from_f32(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: Fixed::from_num(x),
            y: Fixed::from_num(y),
            z: Fixed::from_num(z),
        }
    }
    
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(
            self.x.to_num::<f32>(),
            self.y.to_num::<f32>(),
            self.z.to_num::<f32>(),
        )
    }

    pub fn truncate(&self) -> FixedVec2 {
        FixedVec2 { x: self.x, y: self.y }
    }
    
    pub fn dot(&self, other: &Self) -> Fixed {
        self.x.saturating_mul(other.x)
            .saturating_add(self.y.saturating_mul(other.y))
            .saturating_add(self.z.saturating_mul(other.z))
    }
    
    pub fn length_squared(&self) -> Fixed {
        self.dot(self)
    }
    
    pub fn length(&self) -> Fixed {
        // Use fixed-sqrt crate for deterministic square root
        self.length_squared().sqrt()
    }
    
    pub fn distance(&self, other: &Self) -> Fixed {
        (*self - *other).length() // Relies on Sub impl
    }

    pub fn distance_squared(&self, other: &Self) -> Fixed {
        (*self - *other).length_squared() // Relies on Sub impl
    }

    pub fn mul_element_wise(&self, other: Self) -> Self {
        Self {
            x: self.x.saturating_mul(other.x),
            y: self.y.saturating_mul(other.y),
            z: self.z.saturating_mul(other.z),
        }
    }
    
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > Fixed::from_bits(0) { // Exactly zero check is fine for fixed point
            Self {
                x: self.x.saturating_div(len),
                y: self.y.saturating_div(len),
                z: self.z.saturating_div(len),
            }
        } else {
            // Behavior for zero-length vector normalization:
            // Option 1: Return ZERO (common)
            Self::ZERO
            // Option 2: Return a default direction e.g., (1,0,0) if that makes sense for your game
            // Self::new(super::FIXED_ONE, Fixed::ZERO, Fixed::ZERO) 
            // Option 3: Panic, if normalizing a zero vector is considered an unrecoverable error
            // panic!("Attempted to normalize a zero-length FixedVec3");
        }
    }
    
    pub fn normalize_or_zero(&self) -> Self {
        let len_sq = self.length_squared();
        // Using the same epsilon as FixedVec2 (256 in bits for 16.16 is 256/65536 = 0.00390625)
        // This epsilon applies to length_squared.
        if len_sq > Fixed::from_bits(256) { 
            self.normalize()
        } else {
            Self::ZERO
        }
    }
    
    pub fn clamp_length_max(&self, max: Fixed) -> Self {
        let len_sq = self.length_squared();
        let max_sq = max.saturating_mul(max);
        if len_sq > max_sq {
            // self.normalize() * max // Relies on Mul<Fixed> impl
            let normalized = self.normalize(); // Avoid normalizing if length is zero by using normalize logic
            if normalized == Self::ZERO && max > Fixed::ZERO { // If original was zero, but max is positive, result is still zero
                 Self::ZERO
            } else {
                 normalized.saturating_mul_scalar(max) // Using a helper for clarity or direct if Mul implemented
            }
        } else {
            *self
        }
    }

    // Helper for scalar multiplication if you prefer named methods too
    pub fn saturating_mul_scalar(&self, scalar: Fixed) -> Self {
        Self {
            x: self.x.saturating_mul(scalar),
            y: self.y.saturating_mul(scalar),
            z: self.z.saturating_mul(scalar),
        }
    }

    // Helper for scalar division if you prefer named methods too
    pub fn saturating_div_scalar(&self, scalar: Fixed) -> Self {
        // Consider division by zero: saturating_div will return Fixed::MAX/MIN.
        // Assert or handle as per your game's requirements if scalar can be zero.
        Self {
            x: self.x.saturating_div(scalar),
            y: self.y.saturating_div(scalar),
            z: self.z.saturating_div(scalar),
        }
    }

    // Cross product for FixedVec3
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y.saturating_mul(other.z).saturating_sub(self.z.saturating_mul(other.y)),
            y: self.z.saturating_mul(other.x).saturating_sub(self.x.saturating_mul(other.z)),
            z: self.x.saturating_mul(other.y).saturating_sub(self.y.saturating_mul(other.x)),
        }
    }

    pub fn splat(value: Fixed) -> Self {
        Self {
            x: value,
            y: value,
            z: value,
        }
    }
}

// --- Operator Overloads ---

impl std::ops::Add for FixedVec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_add(other.x),
            y: self.y.saturating_add(other.y),
            z: self.z.saturating_add(other.z),
        }
    }
}

impl std::ops::AddAssign for FixedVec3 {
    fn add_assign(&mut self, other: Self) {
        self.x = self.x.saturating_add(other.x);
        self.y = self.y.saturating_add(other.y);
        self.z = self.z.saturating_add(other.z);
    }
}

impl std::ops::Sub for FixedVec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_sub(other.x),
            y: self.y.saturating_sub(other.y),
            z: self.z.saturating_sub(other.z),
        }
    }
}

impl std::ops::SubAssign for FixedVec3 {
    fn sub_assign(&mut self, other: Self) {
        self.x = self.x.saturating_sub(other.x);
        self.y = self.y.saturating_sub(other.y);
        self.z = self.z.saturating_sub(other.z);
    }
}

impl std::ops::Mul<Fixed> for FixedVec3 { // Scalar multiplication
    type Output = Self;
    fn mul(self, scalar: Fixed) -> Self {
        self.saturating_mul_scalar(scalar)
    }
}

// Optional: if you want `Fixed * FixedVec3`
// impl std::ops::Mul<FixedVec3> for Fixed {
//     type Output = FixedVec3;
//     fn mul(self, vec: FixedVec3) -> FixedVec3 {
//         vec.saturating_mul_scalar(self)
//     }
// }

impl std::ops::Div<Fixed> for FixedVec3 { // Scalar division
    type Output = Self;
    fn div(self, scalar: Fixed) -> Self {
        // Add handling for division by zero if `saturating_div`'s behavior (returning MAX/MIN)
        // is not desired for your game logic.
        // e.g., if scalar == Fixed::from_bits(0) { panic!("Division by zero"); }
        self.saturating_div_scalar(scalar)
    }
}

impl std::ops::Neg for FixedVec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x, // Assumes Fixed implements Neg
            y: -self.y,
            z: -self.z,
        }
    }
}



// Fixed-point angle functions
pub fn atan2_fixed(y: Fixed, x: Fixed) -> Fixed {
    // Convert to the format expected by fixed_trigonometry
    let angle_rad = atan2(y, x); // This returns a fixed-point angle in radians
    angle_rad
}

pub fn sin_fixed(angle: Fixed) -> Fixed {
    sin(angle)
}

pub fn cos_fixed(angle: Fixed) -> Fixed {
    cos(angle)
}

// Fixed-point matrix for rotations
#[derive(Debug, Clone, Copy)]
pub struct FixedMat2 {
    pub m00: Fixed, pub m01: Fixed,
    pub m10: Fixed, pub m11: Fixed,
}

impl FixedMat2 {
    pub fn from_angle(angle: Fixed) -> Self {
        let c = cos_fixed(angle);
        let s = sin_fixed(angle);
        Self {
            m00: c, m01: -s,
            m10: s, m11: c,
        }
    }
    
    pub fn mul_vec2(&self, v: FixedVec2) -> FixedVec2 {
        FixedVec2 {
            x: self.m00.saturating_mul(v.x).saturating_add(self.m01.saturating_mul(v.y)),
            y: self.m10.saturating_mul(v.x).saturating_add(self.m11.saturating_mul(v.y)),
        }
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedMat3 {
    // Columns of the matrix
    pub x_axis: FixedVec3, // First column
    pub y_axis: FixedVec3, // Second column
    pub z_axis: FixedVec3, // Third column
}

impl FixedMat3 {
    pub const IDENTITY: Self = Self {
        x_axis: FixedVec3 { x: FIXED_ONE, y: Fixed::ZERO, z: Fixed::ZERO },
        y_axis: FixedVec3 { x: Fixed::ZERO, y: FIXED_ONE, z: Fixed::ZERO },
        z_axis: FixedVec3 { x: Fixed::ZERO, y: Fixed::ZERO, z: FIXED_ONE },
    };

    // Helper to create from a Bevy Quat (via Bevy Mat3)
    // This is useful for initializing your fixed-point rotation from Bevy's system
    pub fn from_rotation_bevy_quat(q: Quat) -> Self {
        let mat3 = Mat3::from_quat(q);
        Self {
            x_axis: vec3_to_fixed(mat3.x_axis),
            y_axis: vec3_to_fixed(mat3.y_axis),
            z_axis: vec3_to_fixed(mat3.z_axis),
        }
    }

    // Multiply by a FixedVec3 (M * v)
    pub fn mul_vec3(&self, v: FixedVec3) -> FixedVec3 {
        FixedVec3 {
            x: (self.x_axis.x.saturating_mul(v.x))
                .saturating_add(self.y_axis.x.saturating_mul(v.y))
                .saturating_add(self.z_axis.x.saturating_mul(v.z)),
            y: (self.x_axis.y.saturating_mul(v.x))
                .saturating_add(self.y_axis.y.saturating_mul(v.y))
                .saturating_add(self.z_axis.y.saturating_mul(v.z)),
            z: (self.x_axis.z.saturating_mul(v.x))
                .saturating_add(self.y_axis.z.saturating_mul(v.y))
                .saturating_add(self.z_axis.z.saturating_mul(v.z)),
        }
    }

    // If you need to create rotation matrices from Euler angles (e.g., YXZ order):
    pub fn from_euler_angles_yxz(angles_rad_fixed: FixedVec3) -> Self {
        let sx = sin_fixed(angles_rad_fixed.x);
        let cx = cos_fixed(angles_rad_fixed.x);
        let sy = sin_fixed(angles_rad_fixed.y);
        let cy = cos_fixed(angles_rad_fixed.y);
        let sz = sin_fixed(angles_rad_fixed.z);
        let cz = cos_fixed(angles_rad_fixed.z);

        let x_axis = FixedVec3 {
            x: cy.saturating_mul(cz).saturating_add(sx.saturating_mul(sy).saturating_mul(sz)),
            y: cx.saturating_mul(sz),
            z: (cy.saturating_mul(sx).saturating_mul(sz)).saturating_sub(sy.saturating_mul(cz)),
        };
        let y_axis = FixedVec3 {
            x: (cy.saturating_mul(sz)).saturating_sub(sx.saturating_mul(sy).saturating_mul(cz)),
            y: cx.saturating_mul(cz),
            z: sy.saturating_mul(sz).saturating_add(cy.saturating_mul(sx).saturating_mul(cz)),
        };
        let z_axis = FixedVec3 {
            x: cx.saturating_mul(sy),
            y: sx.saturating_mul(-FIXED_ONE), // -sx
            z: cy.saturating_mul(cx),
        };
        Self { x_axis, y_axis, z_axis }
    }

    pub fn from_rotation_z(angle: Fixed) -> Self {
        let c = cos_fixed(angle); // Assumes cos_fixed(Fixed) -> Fixed exists
        let s = sin_fixed(angle); // Assumes sin_fixed(Fixed) -> Fixed exists

        // Standard Z-axis rotation matrix:
        // [ c, -s,  0 ]
        // [ s,  c,  0 ]
        // [ 0,  0,  1 ]
        // Stored column-major in FixedMat3 (x_axis, y_axis, z_axis)

        Self {
            x_axis: FixedVec3::new(c, s, Fixed::ZERO),
            y_axis: FixedVec3::new(-s, c, Fixed::ZERO), // -s uses std::ops::Neg for Fixed
            z_axis: FixedVec3::new(Fixed::ZERO, Fixed::ZERO, FIXED_ONE), // Assumes FIXED_ONE is your 1.0 Fixed value
        }
    }
}

// Ensure FixedVec3 has arithmetic operations like Add, Sub.
// Example for Add, if not already present:
/*
impl std::ops::Add for FixedVec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_add(other.x),
            y: self.y.saturating_add(other.y),
            z: self.z.saturating_add(other.z),
        }
    }
}
impl std::ops::Sub for FixedVec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x.saturating_sub(other.x),
            y: self.y.saturating_sub(other.y),
            z: self.z.saturating_sub(other.z),
        }
    }
}
// ... and other ops like AddAssign, SubAssign, Neg, Mul<Fixed>, Div<Fixed>
*/

// Conversion helpers
pub fn vec2_to_fixed(v: Vec2) -> FixedVec2 {
    FixedVec2::from_f32(v.x, v.y)
}

pub fn vec3_to_fixed(v: Vec3) -> FixedVec3 {
    FixedVec3 {
        x: Fixed::from_num(v.x),
        y: Fixed::from_num(v.y),
        z: Fixed::from_num(v.z),
    }
}

pub fn fixed_to_vec3(v: FixedVec3) -> Vec3 {
    Vec3::new(
        v.x.to_num::<f32>(),
        v.y.to_num::<f32>(),
        v.z.to_num::<f32>()
    )
}

pub fn fixed_to_vec2(v: FixedVec2) -> Vec2 {
    Vec2::new(
        v.x.to_num::<f32>(),
        v.y.to_num::<f32>(),
    )
}



#[derive(Component, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FixedPosition {
    pub value: FixedVec3,
}


impl Into<FixedPosition> for &FixedVec3 {
   fn into(self) -> FixedPosition {
       FixedPosition { value: self.clone() }
   } 
}


#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct FixedTransform3D {
    pub translation: FixedVec3,
    pub rotation: FixedMat3,
    pub scale: FixedVec3,
}


impl FixedTransform3D {
    pub const IDENTITY: Self = Self {
        translation: FixedVec3::ZERO,
        rotation: FixedMat3::IDENTITY,
        scale: FixedVec3::ONE, // Default scale is 1,1,1
    };

    pub fn new(translation: FixedVec3, rotation: FixedMat3, scale: FixedVec3) -> Self {
        Self { translation, rotation, scale }
    }

    // Equivalent to Bevy's Transform::transform_point, now including scale
    // Order: Scale -> Rotate -> Translate
    pub fn transform_point(&self, local_point: FixedVec3) -> FixedVec3 {
        // 1. Apply scale
        let scaled_point = local_point.mul_element_wise(self.scale);
        
        // 2. Apply rotation
        let rotated_point = self.rotation.mul_vec3(scaled_point);
        
        // 3. Apply translation
        // Assumes FixedVec3 implements std::ops::Add
        rotated_point + self.translation
    }

    // Helper to create from Bevy's Transform for initialization
    pub fn from_bevy_transform(transform: &Transform) -> Self {
        // Compute the Affine3A matrix from the Transform
        let affine: Affine3A = transform.compute_affine();
        
        // Now decompose the Affine3A matrix
        let (scale_f32, rot_quat_f32, translation_f32) = affine.to_scale_rotation_translation();
        
        Self {
            translation: vec3_to_fixed(translation_f32), // Assuming vec3_to_fixed exists
            rotation: FixedMat3::from_rotation_bevy_quat(rot_quat_f32), // Assuming this exists
            scale: vec3_to_fixed(scale_f32), // Convert f32 scale to FixedVec3
        }
    } 

    // If you need to convert back to a Bevy Transform (e.g., for rendering sync)
    // This can be lossy, especially the rotation part (Mat3 -> Quat).
    pub fn to_bevy_transform(&self) -> Transform {
        let translation = fixed_to_vec3(self.translation); // Assuming fixed_to_vec3 exists
        let scale = fixed_to_vec3(self.scale);

        // Rotation conversion is the trickiest part to do accurately and robustly
        // from a FixedMat3 back to a Bevy Quat.
        // Placeholder: ideally, you'd have a robust FixedMat3 -> Bevy Quat conversion.
        // One way is FixedMat3 -> bevy::Mat3 -> bevy::Quat
        let bevy_rot_mat3 = bevy::math::Mat3::from_cols(
            fixed_to_vec3(self.rotation.x_axis),
            fixed_to_vec3(self.rotation.y_axis),
            fixed_to_vec3(self.rotation.z_axis)
        );
        let rotation_quat = Quat::from_mat3(&bevy_rot_mat3); // Note: Mat3 to Quat can have issues for some matrices (e.g. non-orthogonal)

        Transform {
            translation,
            rotation: rotation_quat,
            scale,
        }
    }
}

pub fn sync_bevy_transforms_from_fixed(
    mut query: Query<(&FixedTransform3D, &mut Transform)>
) {
    for (fixed_transform, mut bevy_transform) in query.iter_mut() {
        // Sync translation
        bevy_transform.translation = fixed_to_vec3(fixed_transform.translation); // Or fixed_transform.translation.to_vec3()

        // Sync scale - THIS IS THE NEWLY ADDED/UPDATED PART
        bevy_transform.scale = fixed_to_vec3(fixed_transform.scale); // Or fixed_transform.scale.to_vec3()

        // Sync rotation (remains the same as before, handling Mat3 to Quat conversion)
        // This part requires a robust way to convert your FixedMat3 to Bevy's Quat.
        // For example, converting FixedMat3 -> bevy::math::Mat3 -> bevy::prelude::Quat
        let bevy_rot_mat3 = bevy::math::Mat3::from_cols(
            fixed_to_vec3(fixed_transform.rotation.x_axis), // Or .x_axis.to_vec3()
            fixed_to_vec3(fixed_transform.rotation.y_axis), // Or .y_axis.to_vec3()
            fixed_to_vec3(fixed_transform.rotation.z_axis)  // Or .z_axis.to_vec3()
        );
        bevy_transform.rotation = Quat::from_mat3(&bevy_rot_mat3);
        // Note: Mat3 to Quat conversion can be problematic for non-orthogonal matrices
        // or matrices with shear. Ensure your FixedMat3 correctly represents a pure rotation.
        // If your primary fixed-point rotation representation is Euler angles or a fixed-point quaternion,
        // converting from that to Bevy's Quat might be more direct or stable.
    }
}