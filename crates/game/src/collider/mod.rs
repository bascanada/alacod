use bevy::prelude::*;
use bevy_ggrs::AddRollbackCommandExtension;
use serde::{Deserialize, Serialize};
use utils::fixed_math;



#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColliderShape {
    Circle { radius: fixed_math::Fixed },
    Rectangle { width: fixed_math::Fixed, height: fixed_math::Fixed },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColliderConfig {
    pub shape: ColliderShape,
    pub offset: fixed_math::FixedVec3
}

impl Into<Collider> for &ColliderConfig {
    fn into(self) -> Collider {
        Collider { shape: self.shape.clone(), offset: self.offset.clone()}
    } 
}



#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Collider {
    pub shape: ColliderShape,
    pub offset: fixed_math::FixedVec3, // Offset from entity transform
}


#[derive(Component, Clone)]
pub struct Wall;


#[derive(Component, Clone, Serialize, Deserialize)]
pub struct CollisionLayer(pub usize);


#[derive(Resource)]
pub struct CollisionSettings {
    pub enemy_layer: usize,
    pub environment_layer: usize,
    pub player_layer: usize,
    pub wall_layer: usize,
    pub layer_matrix: [[bool; 8]; 8], // Collision matrix for which layers collide
}

impl Default for CollisionSettings {
    fn default() -> Self {
        // Initialize empty collision matrix
        let mut layer_matrix = [[false; 8]; 8];
        
        // Define collision layers
        let enemy_layer = 1;
        let environment_layer = 2;
        let player_layer = 3;
        let wall_layer = 4;
        
        // Set up collision relationships
        layer_matrix[enemy_layer][wall_layer] = true; // Symmetric for simplicity
        layer_matrix[enemy_layer][player_layer] = true; // Symmetric for simplicity
        layer_matrix[player_layer][enemy_layer] = true;

        layer_matrix[wall_layer][enemy_layer] = true;
        layer_matrix[wall_layer][player_layer] = true;
        layer_matrix[player_layer][wall_layer] = true;
        
        // Player bullets shouldn't hit players
        
        Self {
            enemy_layer,
            environment_layer,
            player_layer,
            wall_layer,
            layer_matrix,
        }
    }
}


pub fn is_colliding(
    pos_a: &fixed_math::FixedVec3,
    collider_a: &Collider,
    pos_b: &fixed_math::FixedVec3,
    collider_b: &Collider,
) -> bool {
    let final_pos_a = *pos_a + collider_a.offset;
    let final_pos_b = *pos_b + collider_b.offset;

   match (&collider_a.shape, &collider_b.shape) {
        // Circle to Circle
        (ColliderShape::Circle { radius: radius_a }, ColliderShape::Circle { radius: radius_b }) => {
            let distance_sq = (final_pos_a - final_pos_b).length_squared();
            let combined_radius = *radius_a + *radius_b;
            distance_sq < combined_radius.saturating_mul(combined_radius)
        },
        
        // Rectangle to Rectangle
        (ColliderShape::Rectangle { width: width_a, height: height_a }, 
         ColliderShape::Rectangle { width: width_b, height: height_b }) => {
            let half_width_a = width_a.saturating_div(fixed_math::Fixed::from_num(2));
            let half_height_a = height_a.saturating_div(fixed_math::Fixed::from_num(2));
            let half_width_b = width_b.saturating_div(fixed_math::Fixed::from_num(2));
            let half_height_b = height_b.saturating_div(fixed_math::Fixed::from_num(2));
            
            let min_a_x = final_pos_a.x - half_width_a;
            let max_a_x = final_pos_a.x + half_width_a;
            let min_a_y = final_pos_a.y - half_height_a;
            let max_a_y = final_pos_a.y + half_height_a;
            
            let min_b_x = final_pos_b.x - half_width_b;
            let max_b_x = final_pos_b.x + half_width_b;
            let min_b_y = final_pos_b.y - half_height_b;
            let max_b_y = final_pos_b.y + half_height_b;
            
            // Check for overlap
            min_a_x <= max_b_x && 
            max_a_x >= min_b_x && 
            min_a_y <= max_b_y && 
            max_a_y >= min_b_y
        },
        
        // Circle to Rectangle
        (ColliderShape::Circle { radius }, 
         ColliderShape::Rectangle { width, height }) => {
            circle_rect_collision_fixed(final_pos_a, *radius, final_pos_b, *width, *height)
        },
        
        // Rectangle to Circle (swap arguments)
        (ColliderShape::Rectangle { width, height }, 
         ColliderShape::Circle { radius }) => {
            circle_rect_collision_fixed(final_pos_b, *radius, final_pos_a, *width, *height)
        },
    }
} 

// Helper function for circle-to-rectangle collision
fn circle_rect_collision_fixed(
    circle_pos: fixed_math::FixedVec3,
    circle_radius: fixed_math::Fixed,
    rect_pos: fixed_math::FixedVec3,
    rect_width: fixed_math::Fixed,
    rect_height: fixed_math::Fixed,
) -> bool {
    let half_width = rect_width.saturating_div(fixed_math::Fixed::from_num(2));
    let half_height = rect_height.saturating_div(fixed_math::Fixed::from_num(2));
    
    // Find the closest point on the rectangle to the circle center
    let closest_x = circle_pos.x.max(rect_pos.x - half_width).min(rect_pos.x + half_width);
    let closest_y = circle_pos.y.max(rect_pos.y - half_height).min(rect_pos.y + half_height);
    
    // Calculate distance from circle center to closest point
    let diff = fixed_math::FixedVec2::new(circle_pos.x - closest_x, circle_pos.y - closest_y);
    let distance_sq = diff.length_squared();
    
    // Circle and rectangle collide if this distance is less than the circle radius
    distance_sq < circle_radius.saturating_mul(circle_radius)
}


// test function for wall

pub fn spawn_test_wall(
    commands: &mut Commands,
    position: Vec3,
    size: Vec2,
    collision_settings: &Res<CollisionSettings>,
    color: Color,
) {
    let translation = fixed_math::FixedVec3::new(fixed_math::new(position.x), fixed_math::new(position.y), fixed_math::new(position.z));
    let transform = fixed_math::FixedTransform3D::new(translation, fixed_math::FixedMat3::IDENTITY, fixed_math::FixedVec3::ONE);

    commands.spawn((
        Wall,
        transform.to_bevy_transform(),
        transform,
        Sprite {
            color,
            custom_size: Some(size),
            ..Default::default()
        },
        Collider {
            shape: ColliderShape::Rectangle { 
                width: fixed_math::Fixed::from_num(size.x), 
                height: fixed_math::Fixed::from_num(size.y)
            },
            offset: fixed_math::FixedVec3::ZERO,
        },
        CollisionLayer(collision_settings.wall_layer),
    )).add_rollback();
}
