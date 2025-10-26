---
applyTo: "**/*.rs"
---

# GGRS Rollback Determinism - Mandatory Rules

This project uses **GGRS rollback netcode** with **Bevy ECS**. ALL gameplay systems MUST be deterministic.

## 🚫 NEVER USE (Non-Deterministic):

### 1. Floating-Point Types in Gameplay Logic
- ❌ **NEVER use `f32` or `f64`** for positions, velocities, forces, damage, health, or any gameplay value
- ❌ **NEVER use `Vec2`, `Vec3`** for gameplay logic (these use f32 internally)
- ❌ **NEVER use `Transform::translation`** directly for gameplay (it's f32-based)
- ❌ **NEVER use `Time::delta()` or `Time::elapsed()`** for gameplay timing

### 2. Non-Deterministic Iteration
- ❌ **NEVER use `.iter()` or `.iter_mut()` directly** on queries with rollback components
- ❌ **NEVER iterate over `HashMap`, `HashSet`** without sorting first
- ❌ **NEVER use `Entity` IDs** for tracking game state or hit detection

### 3. Random Number Generation
- ❌ **NEVER use `rand::random()` or `rand::thread_rng()`**
- ❌ **NEVER use `fastrand`** or any non-rollback RNG

## ✅ ALWAYS USE (Deterministic):

### 1. Fixed-Point Math for ALL Gameplay
```rust
// ✅ CORRECT - Use bevy_fixed types:
use bevy_fixed::fixed_math;

// Positions and transforms
let pos = fixed_math::FixedVec2::new(fixed_math::new(10.0), fixed_math::new(20.0));
let transform = fixed_math::FixedTransform3D { ... };

// Velocities and forces
let velocity: fixed_math::FixedVec2 = ...;
let force: fixed_math::Fixed = fixed_math::new(50.0);

// Distance calculations
let distance = pos1.distance(&pos2); // Returns Fixed

// Direction vectors
let direction = (target - source).normalize_or_zero(); // Returns FixedVec2
```

### 2. Deterministic Iteration
```rust
// ✅ CORRECT - Use order_iter! or order_mut_iter!:
use utils::{net_id::GgrsNetId, order_iter, order_mut_iter};

// For read-only queries:
for (net_id, entity, component) in order_iter!(query) {
    // First component MUST be &GgrsNetId
}

// For mutable queries:
for (net_id, mut component) in order_mut_iter!(query) {
    // First component MUST be &GgrsNetId
}

// Query must include GgrsNetId and With<Rollback>:
Query<(&GgrsNetId, &Component), With<Rollback>>
Query<(&GgrsNetId, &mut Component), With<Rollback>>
```

### 3. Entity Tracking and State
```rust
// ✅ CORRECT - Use GgrsNetId for tracking:
pub struct GameState {
    entities_hit: Vec<GgrsNetId>,  // ✅ Use GgrsNetId
    target_id: GgrsNetId,           // ✅ Use GgrsNetId
}

// ❌ WRONG:
pub struct GameState {
    entities_hit: Vec<Entity>,  // ❌ Entity IDs are non-deterministic!
}

// ✅ CORRECT - Spawn entities with GgrsNetId:
let net_id = id_factory.next("entity_name".to_string());
commands.spawn((
    net_id,
    // other components
)).add_rollback();
```

### 4. Frame-Based Timing
```rust
// ✅ CORRECT - Use FrameCount resource:
use utils::frame::FrameCount;

pub fn system(frame: Res<FrameCount>) {
    let current_frame = frame.frame;
    let frames_elapsed = current_frame - start_frame;
    
    if frames_elapsed >= duration_frames {
        // Do something
    }
}

// ✅ CORRECT - Store frame numbers:
#[derive(Component)]
pub struct AttackState {
    pub started_frame: u32,
    pub last_attack_frame: u32,
    pub cooldown_frames: u32,
}
```

### 5. Rollback Component Registration
```rust
// ✅ ALWAYS register custom components for rollback:
app.rollback_component_with_clone::<MyComponent>()
   .rollback_component_with_reflect::<MyOtherComponent>();

// Components MUST derive:
#[derive(Component, Clone, Serialize, Deserialize)]
pub struct MyComponent { ... }
```

### 6. Deterministic RNG (When Needed)
```rust
// ✅ CORRECT - Use RollbackRng:
use bevy_fixed::rng::RollbackRng;

pub fn system(mut rng: ResMut<RollbackRng>) {
    let random_value = rng.next_fixed(); // Returns Fixed
    let random_angle = rng.next_fixed_symmetric(); // Returns Fixed in [-1, 1]
}
```

## 📋 Component Design Checklist

When creating a new component for gameplay:

- [ ] Uses `fixed_math::Fixed` or `fixed_math::FixedVec2/3` (NOT f32/Vec2/Vec3)
- [ ] Derives `Component, Clone, Serialize, Deserialize`
- [ ] Registered with `.rollback_component_with_clone::<T>()`
- [ ] Frame-based timing (NOT delta time)
- [ ] Tracks entities via `GgrsNetId` (NOT `Entity`)

## 📋 System Design Checklist

When creating a new gameplay system:

- [ ] Uses `Query<(&GgrsNetId, ...), With<Rollback>>`
- [ ] Iterates with `order_iter!()` or `order_mut_iter!()`
- [ ] Uses `Res<FrameCount>` for timing (NOT `Res<Time>`)
- [ ] All math uses `fixed_math::Fixed` types
- [ ] No f32/f64 conversions in hot path
- [ ] System added to rollback schedule

## 📋 Resource Design Checklist

When creating configuration resources:

- [ ] All numeric values are `fixed_math::Fixed` (NOT f32)
- [ ] Derives `Resource, Clone`
- [ ] Default implementation uses `fixed_math::new()` for values

```rust
// ✅ CORRECT:
#[derive(Resource, Clone)]
pub struct GameConfig {
    pub speed: fixed_math::Fixed,
    pub damage: fixed_math::Fixed,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            speed: fixed_math::new(100.0),
            damage: fixed_math::new(25.0),
        }
    }
}

// ❌ WRONG:
#[derive(Resource, Clone)]
pub struct GameConfig {
    pub speed: f32,  // ❌ Will cause desyncs!
    pub damage: f32, // ❌ Non-deterministic!
}
```

## 🎯 Common Patterns

### Pattern: Apply Force/Knockback
```rust
// ✅ CORRECT:
let direction = (target_pos - source_pos).normalize_or_zero();
let force_vec = direction * knockback_force; // All Fixed types
velocity.knockback = velocity.knockback + force_vec;
```

### Pattern: Distance Check
```rust
// ✅ CORRECT:
let distance_squared = pos1.distance_squared(&pos2); // Returns FixedWide
if distance_squared < (range * range) {
    // In range
}
```

### Pattern: Collision Detection in Order
```rust
// ✅ CORRECT:
for (hitbox_id, hitbox_transform, hitbox) in order_iter!(hitbox_query) {
    for (target_id, target_transform, target) in target_query.iter() {
        // Inner loop doesn't need ordering
        if is_colliding(&hitbox_transform, &target_transform) {
            // Process collision
        }
    }
}
```

## 🚨 Red Flags to Watch For

If you see ANY of these in gameplay code, it's likely WRONG:

- `f32`, `f64` in gameplay logic
- `Vec2`, `Vec3` for positions/velocities
- `.iter()` without `order_iter!`
- `Entity` stored in game state
- `Time::delta()` in rollback systems
- `rand::` functions
- Missing `With<Rollback>` filter
- Missing `GgrsNetId` in queries

## �� Key Imports

Always have these available:

```rust
use bevy_fixed::fixed_math;
use bevy_ggrs::{Rollback, AddRollbackCommandExtension};
use utils::{net_id::{GgrsNetId, GgrsNetIdFactory}, order_iter, order_mut_iter};
use utils::frame::FrameCount;
```

## 🔍 Code Review Questions

Before submitting/generating code, ask:

1. Does this system affect gameplay? → Must be deterministic
2. Does it use any f32/f64? → Replace with Fixed
3. Does it iterate over entities? → Must use order_iter!
4. Does it track entities? → Must use GgrsNetId
5. Does it use timing? → Must use FrameCount, not Time
6. Is the component registered for rollback? → Add to plugin
7. Does it generate random numbers? → Must use RollbackRng

---

**Remember: A single f32 or non-deterministic iteration can cause desyncs in multiplayer!**
