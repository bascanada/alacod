use bevy::{prelude::*, reflect::TypePath, sprite::Anchor, utils::HashMap};
use bevy_ggrs::{prelude::*, GgrsSchedule};
use bevy_common_assets::ron::RonAssetPlugin;
use serde::Deserialize;

// CONFIG

// 1a. Define your custom enum that CAN be deserialized
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "PascalCase")] // Allows "BottomCenter" in RON file
pub enum ConfigurableAnchor {
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
    CenterLeft,
    CenterRight,
    TopLeft,
    TopCenter,
    TopRight,
    // Add Custom(Vec2) if you need it, requires slightly more complex mapping
}

impl ConfigurableAnchor {

    pub fn to_anchor(&self) -> Anchor {
         return match self {
             ConfigurableAnchor::Center => Anchor::Center,
             ConfigurableAnchor::BottomLeft => Anchor::BottomLeft,
             ConfigurableAnchor::BottomCenter => Anchor::BottomCenter,
             ConfigurableAnchor::BottomRight => Anchor::BottomRight,
             ConfigurableAnchor::CenterLeft => Anchor::CenterLeft,
             ConfigurableAnchor::CenterRight => Anchor::CenterRight,
             ConfigurableAnchor::TopLeft => Anchor::TopLeft,
             ConfigurableAnchor::TopCenter => Anchor::TopCenter,
             ConfigurableAnchor::TopRight => Anchor::TopRight,
             // Add Custom case here if you defined it
         };
    }
}

// -- Sprite Sheet Layout Configuration --
#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
pub struct SpriteSheetConfig {
    pub path: String,
    pub tile_size: (u32, u32),
    pub columns: u32,
    pub rows: u32,
    pub name: String,
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub offset_z: f32,
    pub animated: bool,
    pub anchor: ConfigurableAnchor,
}


// -- Animation Definition Configuration --
#[derive(Deserialize, Debug, Clone)]
pub struct AnimationIndices {
    pub start: usize,
    pub end: usize, // Inclusive end index
}

#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
pub struct AnimationMapConfig {
    pub frame_duration: u64,
    pub animations: HashMap<String, AnimationIndices>,
}

// COMPONENT
#[derive(Component, Default, Clone, Debug)]
pub struct LayerName {
    pub name: String
}

#[derive(Component)]
pub struct AnimatedLayer {}

#[derive(Component)]
pub struct ColoredLayer {}

#[derive(Component, Clone)]
pub struct ActiveLayers {
    pub layers: HashMap<String, String>,
}

#[derive(Component, Reflect, Default, Clone, Debug, PartialEq, Eq)]
#[reflect(Component, PartialEq)] // Reflect needed for GGRS state hashing
pub struct AnimationState(pub String);

// Handles are loaded once, assume they don't change and don't need rollback/reflection
#[derive(Component)]
pub struct CharacterAnimationHandles {
    pub spritesheets: HashMap<String, Handle<SpriteSheetConfig>>,
    pub animations: Handle<AnimationMapConfig>,
    pub starting_index: usize
}

#[derive(Component)]
struct AnimationTimer {
    frame_timer: Timer,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacingDirection {
    Left,
    Right,
}

impl Default for FacingDirection {
    fn default() -> Self {
        FacingDirection::Right
    }
}

impl FacingDirection {
    pub fn to_int(&self) -> i32 {
        match self {
            FacingDirection::Left => -1,
            FacingDirection::Right => 1,
        }
    }
    
}

// Bundle

#[derive(Bundle)]
pub struct AnimationBundle {
    state: AnimationState,
    handles: CharacterAnimationHandles,
    timer: AnimationTimer,
    active_layers: ActiveLayers,
    facing_direction: FacingDirection,
}

impl AnimationBundle {
    pub fn new(
        spritesheets: HashMap<String, Handle<SpriteSheetConfig>>,
        animations: Handle<AnimationMapConfig>,

        starting_index: usize,

        starting_layers: HashMap<String, String>,
    ) -> Self {
        Self {
            state: AnimationState("Idle".into()),
            timer: AnimationTimer {
                frame_timer: Timer::from_seconds(1., TimerMode::Repeating),
            },
            handles: CharacterAnimationHandles {
                spritesheets,
                animations,
                starting_index,
            },
            active_layers: ActiveLayers {
                layers: starting_layers,
            },
            facing_direction: FacingDirection::default()
        }
    }
}






// Animates sprite based on AnimationState
fn animate_sprite_system(
    time: Res<Time>,
    animation_configs: Res<Assets<AnimationMapConfig>>,
    mut query: Query<(
        &Children,
        &CharacterAnimationHandles,
        &mut AnimationTimer,
        &AnimationState,
    )>,
    mut query_sprites: Query<(&mut Sprite, &LayerName), With<AnimatedLayer>>,
    
) {
    for (childs, config_handles, mut timer, state) in query.iter_mut() {
        if let Some(anim_config) = animation_configs.get(&config_handles.animations) {
            timer.frame_timer.tick(time.delta());
            if timer.frame_timer.just_finished() {
                for child in childs.iter() {
                    if let Ok((mut sprite, _)) = query_sprites.get_mut(*child) {
                        if let Some(atlas) = &mut sprite.texture_atlas {
                            if let Some(indices) = anim_config.animations.get(&state.0) {
                                let start_index = indices.start;
                                let end_index = indices.end;
                                if atlas.index < start_index || atlas.index > end_index {
                                    atlas.index = start_index;
                                } else {
                                    atlas.index = (atlas.index + 1 - start_index)
                                        % (end_index - start_index + 1)
                                        + start_index;
                                }
                            } else {
                                atlas.index = anim_config
                                    .animations
                                    .get("Idle")
                                    .map_or(0, |idx| idx.start);
                            }
                        }
                    }
                }
            }
        }
    }
}

// Updates animation timer duration if AnimationMapConfig reloads
fn check_animation_config_reload_system(
    mut ev_asset: EventReader<AssetEvent<AnimationMapConfig>>,
    animation_configs: Res<Assets<AnimationMapConfig>>,
    mut query: Query<(&CharacterAnimationHandles, &mut AnimationTimer)>,
    asset_server: Res<AssetServer>,
) {
    let mut updates_needed = HashMap::new(); // Handle ID -> new duration

    // Collect updates needed from asset events
    for event in ev_asset.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if let Some(config) = animation_configs.get(*id) {
                    updates_needed.insert(*id, config.frame_duration);
                }
            }
            _ => {}
        }
    }

    // Apply updates to relevant entities
    for (config_handles, mut anim_timer) in query.iter_mut() {
        if let Some(new_duration) = updates_needed.get(&config_handles.animations.id()) {
            anim_timer
                .frame_timer
                .set_duration(bevy::utils::Duration::from_millis(*new_duration));
            anim_timer.frame_timer.reset();
        }
        // Apply initial duration after startup load (if needed)
        else if anim_timer.frame_timer.duration().as_secs_f32() == 1.0 {
            // Check default
            if asset_server
                .load_state(&config_handles.animations)
                .is_loaded()
            {
                if let Some(config) = animation_configs.get(&config_handles.animations) {
                    anim_timer
                        .frame_timer
                        .set_duration(bevy::utils::Duration::from_millis(config.frame_duration));
                    anim_timer.frame_timer.reset();
                }
            }
        }
    }
}

fn character_visuals_update_system(
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    spritesheet_configs: Res<Assets<SpriteSheetConfig>>,
    mut ev_asset: EventReader<AssetEvent<SpriteSheetConfig>>,
    query: Query<(&Children, Entity, &CharacterAnimationHandles)>,
    mut query_sprite: Query<(&mut Sprite,&mut Transform, &LayerName)>,
) {
    for event in ev_asset.read() {
        if let AssetEvent::Modified { id } | AssetEvent::Added { id } = event {
            // Find entities using the modified spritesheet config
            for (childs, entity, config_handle) in query.iter() {
                for handle in config_handle.spritesheets.values() {
                    if handle.id() == *id {
                        if let Some(new_config) = spritesheet_configs.get(handle) {
                            info!(
                                "Spritesheet config modified {}",
                                new_config.name,
                            );
                            let new_layout = TextureAtlasLayout::from_grid(
                                UVec2::new(new_config.tile_size.0, new_config.tile_size.1),
                                new_config.columns,
                                new_config.rows,
                                None,
                                None,
                            );

                            for child in childs.iter() {
                                if let Ok((mut sprite, mut transform, layer_name)) = query_sprite.get_mut(*child) {
                                    if layer_name.name == new_config.name {
                                        sprite.texture_atlas = Some(TextureAtlas {
                                            layout: texture_atlas_layouts.add(new_layout.clone()),
                                            index: config_handle.starting_index,
                                        });
                                        transform.translation.x = new_config.offset_x;
                                        transform.translation.z = new_config.offset_z;
                                        transform.translation.y = new_config.offset_y;
                                        transform.scale = Vec3::splat(new_config.scale);
                                        sprite.image = asset_server.load(&new_config.path);
                                        sprite.anchor = new_config.anchor.to_anchor();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}



// SYSTEM THAT RUN ON THE BEVY SCHEDULE FOR SYNCH

pub fn set_sprite_flip(
    query: Query<(&Children, &FacingDirection)>,
    mut sprite_query: Query<&mut Sprite>,
) {
    for (childrens, direction) in query.iter() {
        for child in childrens.iter() {
            if let Ok(mut sprite) = sprite_query.get_mut(*child) {
                match direction {
                    FacingDirection::Left => {
                        sprite.flip_x = true;
                    }
                    FacingDirection::Right => {
                        sprite.flip_x = false;
                    }
                }
            }
        }
    }
}

pub fn create_child_sprite(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,

    parent_entity: Entity,
    spritesheet_config: &SpriteSheetConfig,
    current_frame_index: usize
) -> Entity {
    let texture_handle: Handle<Image> = asset_server.load(&spritesheet_config.path);
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(
            spritesheet_config.tile_size.0,
            spritesheet_config.tile_size.1,
        ),
        spritesheet_config.columns,
        spritesheet_config.rows,
        None,
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);


    let mut entity_commands = commands.spawn((
        Sprite {
            image: texture_handle.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle.clone(),
                index: current_frame_index,
            }),
            anchor: spritesheet_config.anchor.to_anchor(),
            ..default()
        },
        Transform::from_scale(Vec3::splat(spritesheet_config.scale))
            .with_translation(Vec3::new(spritesheet_config.offset_x, spritesheet_config.offset_y, spritesheet_config.offset_z)),
            //.with_rotation(Quat::IDENTITY),
        LayerName { name: spritesheet_config.name.clone() },
    ));

    if spritesheet_config.animated {
        entity_commands.insert(AnimatedLayer{});
    }

    let sprite = entity_commands.add_rollback().id();

    commands.entity(parent_entity).add_child(sprite);

    sprite
}

// PLUGIN

pub struct D2AnimationPlugin;

impl Plugin for D2AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<SpriteSheetConfig>::new(&["ron"]));
        app.add_plugins(RonAssetPlugin::<AnimationMapConfig>::new(&["ron"]));
        
        app
            .rollback_component_with_reflect::<AnimationState>()
            .rollback_component_with_clone::<LayerName>()
            .rollback_component_with_clone::<ActiveLayers>();

        app.add_systems(
            Update,
            (
                character_visuals_update_system,
                animate_sprite_system.after(character_visuals_update_system),
                check_animation_config_reload_system.after(animate_sprite_system),
            )
        );
    }
}