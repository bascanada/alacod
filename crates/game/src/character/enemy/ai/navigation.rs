//! Flow Field Navigation System
//!
//! This module provides a shared pathfinding solution for hordes of enemies.
//! Uses a lightweight BFS (Breadth-First Search) with limited radius for performance.
//! O(1) direction lookups per enemy after computation.
//!
//! IMPORTANT: Uses BTreeMap/BTreeSet for deterministic iteration order (GGRS rollback).

use bevy::prelude::*;
use bevy_fixed::fixed_math;
use bevy_ggrs::Rollback;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use utils::{frame::FrameCount, net_id::GgrsNetId};

use crate::character::player::Player;
use crate::collider::{Collider, ColliderShape, Wall};

use super::obstacle::{Obstacle, ObstacleType};

/// Grid cell size in fixed-point units (matches PathfindingConfig.node_size)
pub const GRID_CELL_SIZE: i32 = 20;

/// Grid position for flow field calculations
/// Implements Ord for deterministic BTreeMap/BTreeSet ordering
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Convert from FixedVec2 world position to grid position
    pub fn from_fixed(pos: fixed_math::FixedVec2) -> Self {
        Self {
            x: pos.x.to_num::<i32>() / GRID_CELL_SIZE,
            y: pos.y.to_num::<i32>() / GRID_CELL_SIZE,
        }
    }

    /// Convert to world position (center of cell)
    pub fn to_fixed(self) -> fixed_math::FixedVec2 {
        let half_cell = GRID_CELL_SIZE / 2;
        fixed_math::FixedVec2::new(
            fixed_math::Fixed::from_num(self.x * GRID_CELL_SIZE + half_cell),
            fixed_math::Fixed::from_num(self.y * GRID_CELL_SIZE + half_cell),
        )
    }

    /// Get 4-directional neighbors
    pub fn neighbors_4(&self) -> [GridPos; 4] {
        [
            GridPos::new(self.x + 1, self.y),
            GridPos::new(self.x - 1, self.y),
            GridPos::new(self.x, self.y + 1),
            GridPos::new(self.x, self.y - 1),
        ]
    }

    /// Get 8-directional neighbors (including diagonals)
    pub fn neighbors_8(&self) -> [GridPos; 8] {
        [
            GridPos::new(self.x + 1, self.y),
            GridPos::new(self.x - 1, self.y),
            GridPos::new(self.x, self.y + 1),
            GridPos::new(self.x, self.y - 1),
            GridPos::new(self.x + 1, self.y + 1),
            GridPos::new(self.x - 1, self.y + 1),
            GridPos::new(self.x + 1, self.y - 1),
            GridPos::new(self.x - 1, self.y - 1),
        ]
    }

    /// Manhattan distance to another grid position
    pub fn manhattan_distance(&self, other: &GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

/// Navigation profile determines which obstacles block an enemy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize, PartialOrd, Ord)]
pub enum NavProfile {
    /// Respects all obstacles (walls, windows, barricades, water, pits)
    #[default]
    Ground,
    /// Ignores breakable obstacles for pathfinding (still attacks them)
    GroundBreaker,
    /// Ignores water and pits (flying enemies)
    Flying,
    /// Ignores everything except solid walls (ghosts)
    Phasing,
}

impl NavProfile {
    /// Returns true if this profile can pass through the given obstacle type
    pub fn can_pass(&self, obstacle_type: ObstacleType) -> bool {
        match self {
            NavProfile::Ground => false,
            NavProfile::GroundBreaker => obstacle_type.is_breakable(),
            NavProfile::Flying => matches!(obstacle_type, ObstacleType::Water | ObstacleType::Pit),
            NavProfile::Phasing => obstacle_type != ObstacleType::Wall,
        }
    }
}

/// A single flow field for a specific navigation profile
/// Uses BTreeMap for deterministic iteration (GGRS rollback compatibility)
#[derive(Clone, Debug, Default)]
pub struct FlowField {
    /// For each cell, the next cell to move to (toward target)
    pub directions: BTreeMap<GridPos, GridPos>,
    /// Cost to reach target from each cell
    pub costs: BTreeMap<GridPos, u32>,
}

impl FlowField {
    /// Get the direction to move from a given position
    pub fn get_direction(&self, pos: GridPos) -> Option<GridPos> {
        self.directions.get(&pos).copied()
    }

    /// Get the world-space direction vector from a given position
    pub fn get_direction_vector(&self, pos: fixed_math::FixedVec2) -> Option<fixed_math::FixedVec2> {
        let grid_pos = GridPos::from_fixed(pos);
        let next_pos = self.get_direction(grid_pos)?;

        let current_world = grid_pos.to_fixed();
        let next_world = next_pos.to_fixed();

        let direction = next_world - current_world;
        if direction.length_squared() > fixed_math::FixedWide::ZERO {
            Some(direction.normalize_or_zero())
        } else {
            None
        }
    }
}

/// Cache of flow fields for different navigation profiles
/// Uses BTreeMap/BTreeSet for deterministic iteration (GGRS rollback compatibility)
#[derive(Resource, Default, Clone)]
pub struct FlowFieldCache {
    /// Target position (player) at last calculation
    pub target_pos: GridPos,
    /// Frame when last updated
    pub last_update_frame: u32,
    /// Update interval in frames
    pub update_interval: u32,
    /// Cached flow fields per navigation profile
    pub layers: BTreeMap<NavProfile, FlowField>,
    /// Blocked cells per obstacle type (for building flow fields)
    pub blocked_cells: BTreeMap<ObstacleType, BTreeSet<GridPos>>,
    /// All permanently blocked cells (walls)
    pub wall_cells: BTreeSet<GridPos>,
}

impl FlowFieldCache {
    pub fn new() -> Self {
        Self {
            target_pos: GridPos::default(),
            last_update_frame: 0,
            update_interval: 15, // Update every 15 frames (~4 times per second at 60 FPS)
            layers: BTreeMap::new(),
            blocked_cells: BTreeMap::new(),
            wall_cells: BTreeSet::new(),
        }
    }

    /// Get the flow field for a specific navigation profile
    pub fn get_flow_field(&self, profile: NavProfile) -> Option<&FlowField> {
        self.layers.get(&profile)
    }

    /// Check if a cell is blocked for a given navigation profile
    pub fn is_blocked(&self, pos: &GridPos, profile: NavProfile) -> bool {
        // Walls always block (except for Phasing which ignores non-Wall obstacles)
        if self.wall_cells.contains(pos) {
            return true;
        }

        // Check other obstacle types based on profile
        for (obstacle_type, cells) in &self.blocked_cells {
            if cells.contains(pos) && !profile.can_pass(*obstacle_type) {
                return true;
            }
        }

        false
    }
}

/// Configuration for flow field updates
#[derive(Resource, Clone)]
pub struct FlowFieldConfig {
    /// How often to recalculate flow fields (in frames)
    pub update_interval: u32,
    /// Maximum search radius from target
    pub max_search_radius: i32,
    /// Use 8-directional movement (vs 4-directional)
    pub use_8_directions: bool,
    /// Diagonal movement cost multiplier (for 8-directional)
    pub diagonal_cost: u32,
}

impl Default for FlowFieldConfig {
    fn default() -> Self {
        Self {
            update_interval: 10,  // Update every ~166ms at 60fps (was 5, reduced for web perf)
            max_search_radius: 40, // 40 cells * 20 units = 800 units radius (was 60, reduced for web perf)
            use_8_directions: true, // 8 directions for smoother diagonal movement
            diagonal_cost: 14,
        }
    }
}

/// System to update the global flow field cache
pub fn update_flow_field_system(
    frame: Res<FrameCount>,
    config: Res<FlowFieldConfig>,
    player_query: Query<(&GgrsNetId, &fixed_math::FixedTransform3D), With<Player>>,
    wall_query: Query<
        (&fixed_math::FixedTransform3D, &Collider),
        (With<Wall>, Without<Obstacle>),
    >,
    obstacle_query: Query<
        (&fixed_math::FixedTransform3D, &Collider, &Obstacle),
        With<Rollback>,
    >,
    mut cache: ResMut<FlowFieldCache>,
) {
    // Check if we need to update
    if frame.frame < cache.last_update_frame + cache.update_interval {
        return;
    }

    // GGRS CRITICAL: Must select player deterministically by sorting by net_id
    let target_pos = {
        let mut players: Vec<_> = player_query.iter().collect();
        if players.is_empty() {
            return; // No players, nothing to do
        }
        // Sort by net_id for deterministic selection
        players.sort_unstable_by_key(|(net_id, _)| net_id.0);
        let (_, transform) = players[0];
        GridPos::from_fixed(transform.translation.truncate())
    };

    // Check if target moved significantly
    if target_pos == cache.target_pos && !cache.layers.is_empty() {
        cache.last_update_frame = frame.frame;
        return;
    }

    // Update target and frame
    cache.target_pos = target_pos;
    cache.last_update_frame = frame.frame;

    // Rebuild blocked cell cache
    rebuild_blocked_cells(&mut cache, &wall_query, &obstacle_query);

    // Only compute Ground profile for now (most common case)
    // Other profiles can be added on-demand later
    let flow_field = build_flow_field(target_pos, NavProfile::Ground, &cache, &config);

    // GGRS debug trace - log flow field update for desync debugging
    trace!(
        "[Frame {}] FlowField updated: target=({}, {}), cells={}, walls={}",
        frame.frame,
        target_pos.x,
        target_pos.y,
        flow_field.directions.len(),
        cache.wall_cells.len()
    );

    cache.layers.insert(NavProfile::Ground, flow_field);
}

/// Rebuild the blocked cell cache from current wall and obstacle positions
fn rebuild_blocked_cells(
    cache: &mut FlowFieldCache,
    wall_query: &Query<
        (&fixed_math::FixedTransform3D, &Collider),
        (With<Wall>, Without<Obstacle>),
    >,
    obstacle_query: &Query<
        (&fixed_math::FixedTransform3D, &Collider, &Obstacle),
        With<Rollback>,
    >,
) {
    cache.wall_cells.clear();
    cache.blocked_cells.clear();

    // Add wall cells
    for (transform, collider) in wall_query.iter() {
        let cells = get_collider_cells(transform.translation.truncate(), collider);
        cache.wall_cells.extend(cells);
    }

    // Add obstacle cells by type
    for (transform, collider, obstacle) in obstacle_query.iter() {
        if !obstacle.blocks_movement {
            continue;
        }
        let cells = get_collider_cells(transform.translation.truncate(), collider);
        cache
            .blocked_cells
            .entry(obstacle.obstacle_type)
            .or_default()
            .extend(cells);
    }
}

/// Get all grid cells occupied by a collider
fn get_collider_cells(
    pos: fixed_math::FixedVec2,
    collider: &Collider,
) -> Vec<GridPos> {
    let mut cells = Vec::new();
    let offset = fixed_math::FixedVec2::new(collider.offset.x, collider.offset.y);
    let center = pos + offset;
    let center_grid = GridPos::from_fixed(center);

    match &collider.shape {
        ColliderShape::Circle { radius } => {
            let radius_cells = (radius.to_num::<i32>() / GRID_CELL_SIZE) + 1;
            for dx in -radius_cells..=radius_cells {
                for dy in -radius_cells..=radius_cells {
                    cells.push(GridPos::new(center_grid.x + dx, center_grid.y + dy));
                }
            }
        }
        ColliderShape::Rectangle { width, height } => {
            let half_w = width.to_num::<i32>() / 2 / GRID_CELL_SIZE + 1;
            let half_h = height.to_num::<i32>() / 2 / GRID_CELL_SIZE + 1;
            for dx in -half_w..=half_w {
                for dy in -half_h..=half_h {
                    cells.push(GridPos::new(center_grid.x + dx, center_grid.y + dy));
                }
            }
        }
    }

    cells
}

/// Build a flow field using simple BFS (much faster than Dijkstra for unweighted graphs)
fn build_flow_field(
    target: GridPos,
    profile: NavProfile,
    cache: &FlowFieldCache,
    config: &FlowFieldConfig,
) -> FlowField {
    let mut flow_field = FlowField::default();
    let mut visited: BTreeSet<GridPos> = BTreeSet::new();
    let mut queue: VecDeque<GridPos> = VecDeque::new();

    // Start BFS from target
    queue.push_back(target);
    visited.insert(target);
    flow_field.directions.insert(target, target);
    flow_field.costs.insert(target, 0);

    let max_cells = (config.max_search_radius * config.max_search_radius * 4) as usize;
    let mut cells_processed = 0;

    while let Some(current) = queue.pop_front() {
        // Safety limit to prevent infinite loops
        cells_processed += 1;
        if cells_processed > max_cells {
            break;
        }

        let current_cost = *flow_field.costs.get(&current).unwrap_or(&0);

        // Get neighbors (4 or 8 directions)
        let neighbors = if config.use_8_directions {
            current.neighbors_8().to_vec()
        } else {
            current.neighbors_4().to_vec()
        };

        for neighbor in neighbors {
            // Skip if already visited
            if visited.contains(&neighbor) {
                continue;
            }

            // Check bounds (manhattan distance from target)
            if neighbor.manhattan_distance(&target) > config.max_search_radius {
                continue;
            }

            // Check if blocked for this profile
            if cache.is_blocked(&neighbor, profile) {
                continue;
            }

            // Mark as visited and add to queue
            visited.insert(neighbor);
            queue.push_back(neighbor);

            // Direction points TOWARD target (so we store 'current' as the next step)
            flow_field.directions.insert(neighbor, current);
            flow_field.costs.insert(neighbor, current_cost + 1);
        }
    }

    flow_field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_pos_conversion() {
        let world_pos = fixed_math::FixedVec2::new(
            fixed_math::Fixed::from_num(45),
            fixed_math::Fixed::from_num(65),
        );
        let grid_pos = GridPos::from_fixed(world_pos);
        assert_eq!(grid_pos.x, 2); // 45 / 20 = 2
        assert_eq!(grid_pos.y, 3); // 65 / 20 = 3
    }

    #[test]
    fn test_nav_profile_can_pass() {
        assert!(!NavProfile::Ground.can_pass(ObstacleType::Window));
        assert!(NavProfile::GroundBreaker.can_pass(ObstacleType::Window));
        assert!(NavProfile::Flying.can_pass(ObstacleType::Water));
        assert!(!NavProfile::Flying.can_pass(ObstacleType::Wall));
        assert!(NavProfile::Phasing.can_pass(ObstacleType::Window));
        assert!(!NavProfile::Phasing.can_pass(ObstacleType::Wall));
    }
}
