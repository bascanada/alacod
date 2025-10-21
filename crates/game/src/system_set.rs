use bevy::prelude::SystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum RollbackSystemSet {
    Input,
    Interaction,
    Movement,
    Weapon,
    CollisionDamage,
    DeathManagement,
    AnimationUpdates,
    EnemySpawning,
    EnemyAI,
    FrameCounter,
}
