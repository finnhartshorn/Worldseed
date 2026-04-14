use super::spawning::{
    spawn_tree_spirit, spawn_variant_tree, update_animation_for_direction, AnimationTimer,
};
use super::{
    AnimationIndices, Direction, EntityState, ForestGuardian, GuardianAnimations, GrowingTree,
    Position, RoamingBehavior, Snail, TreeSpawner, TreeVariant, Velocity, WindingPath,
    WorldRenderDepth,
};
use crate::tiles::TILE_DIRT;
use crate::world::WorldManager;
use bevy::prelude::*;

/// Syncs world positions into sprite transforms and assigns deterministic depth.
pub fn sync_world_render_transform(
    mut query: Query<
        (&Position, &WorldRenderDepth, &mut Transform),
        Or<(Changed<Position>, Changed<WorldRenderDepth>)>,
    >,
) {
    for (position, render_depth, mut transform) in &mut query {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
        transform.translation.z = render_depth.z_for_position(position);
    }
}

/// Updates entity position based on velocity
pub fn apply_velocity(time: Res<Time>, mut query: Query<(&mut Position, &Velocity)>) {
    let delta = time.delta_secs();
    for (mut position, velocity) in &mut query {
        position.x += velocity.x * delta;
        position.y += velocity.y * delta;
    }
}

#[cfg(test)]
mod tests {
    use crate::entities::{
        update_guardian_animation_from_state, AnimationIndices, Direction, EntityState,
        GuardianAnimations, Position, RenderStratum, WorldRenderDepth,
    };
    use bevy::prelude::*;

    #[test]
    fn southern_positions_render_in_front() {
        let north = Position::new(0.0, 64.0);
        let south = Position::new(0.0, -64.0);
        let depth = WorldRenderDepth::new(RenderStratum::WorldObject);

        assert!(depth.z_for_position(&south) > depth.z_for_position(&north));
    }

    #[test]
    fn strata_are_non_overlapping() {
        let position = Position::new(0.0, 0.0);
        let ground = WorldRenderDepth::new(RenderStratum::Ground);
        let world = WorldRenderDepth::new(RenderStratum::WorldObject);
        let overlay = WorldRenderDepth::new(RenderStratum::Overlay);

        assert!(ground.z_for_position(&position) < world.z_for_position(&position));
        assert!(world.z_for_position(&position) < overlay.z_for_position(&position));
    }

    #[test]
    fn depth_bias_breaks_same_position_ties_deterministically() {
        let position = Position::new(10.0, 10.0);
        let base = WorldRenderDepth::new(RenderStratum::WorldObject);
        let biased = WorldRenderDepth::with_bias(RenderStratum::WorldObject, 0.0004);

        assert!(biased.z_for_position(&position) > base.z_for_position(&position));
    }

    #[test]
    fn large_world_positions_stay_within_world_object_stratum() {
        let far_north = Position::new(0.0, 100_000.0);
        let far_south = Position::new(0.0, -100_000.0);
        let decoration = WorldRenderDepth::new(RenderStratum::Decoration);
        let world = WorldRenderDepth::new(RenderStratum::WorldObject);
        let overlay = WorldRenderDepth::new(RenderStratum::Overlay);

        assert!(decoration.z_for_position(&far_north) < world.z_for_position(&far_north));
        assert!(world.z_for_position(&far_north) < overlay.z_for_position(&far_north));
        assert!(decoration.z_for_position(&far_south) < world.z_for_position(&far_south));
        assert!(world.z_for_position(&far_south) < overlay.z_for_position(&far_south));
    }

    #[test]
    fn guardian_state_switch_preserves_direction_row_for_walk_animation() {
        let mut app = App::new();
        app.add_systems(Update, update_guardian_animation_from_state);

        let idle_layout = Handle::<TextureAtlasLayout>::default();
        let walk_layout = Handle::<TextureAtlasLayout>::default();
        let idle_texture = Handle::<Image>::default();
        let walk_texture = Handle::<Image>::default();

        let entity = app.world_mut().spawn((
            EntityState::Moving,
            Direction::NorthWest,
            GuardianAnimations {
                idle_texture: idle_texture.clone(),
                idle_layout: idle_layout.clone(),
                walk_texture: walk_texture.clone(),
                walk_layout: walk_layout.clone(),
                idle_frames: 8,
                walk_frames: 6,
                current_state: EntityState::Idle,
            },
            Sprite::from_atlas_image(
                idle_texture,
                TextureAtlas {
                    layout: idle_layout,
                    index: 27,
                },
            ),
            AnimationIndices::new(24, 31),
        )).id();

        app.update();

        let mut query = app
            .world_mut()
            .query::<(&Sprite, &AnimationIndices, &GuardianAnimations)>();
        let (sprite, indices, animations) = query
            .get(app.world(), entity)
            .expect("guardian components should exist");

        assert_eq!(animations.current_state, EntityState::Moving);
        assert_eq!(sprite.image, walk_texture);
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.layout.clone()),
            Some(walk_layout)
        );
        assert_eq!(sprite.texture_atlas.as_ref().map(|atlas| atlas.index), Some(18));
        assert_eq!(indices.first, 18);
        assert_eq!(indices.last, 23);
    }

    #[test]
    fn guardian_state_switch_preserves_direction_row_for_idle_animation() {
        let mut app = App::new();
        app.add_systems(Update, update_guardian_animation_from_state);

        let idle_layout = Handle::<TextureAtlasLayout>::default();
        let walk_layout = Handle::<TextureAtlasLayout>::default();
        let idle_texture = Handle::<Image>::default();
        let walk_texture = Handle::<Image>::default();

        let entity = app.world_mut().spawn((
            EntityState::Idle,
            Direction::SouthWest,
            GuardianAnimations {
                idle_texture: idle_texture.clone(),
                idle_layout: idle_layout.clone(),
                walk_texture: walk_texture.clone(),
                walk_layout: walk_layout.clone(),
                idle_frames: 8,
                walk_frames: 6,
                current_state: EntityState::Moving,
            },
            Sprite::from_atlas_image(
                walk_texture,
                TextureAtlas {
                    layout: walk_layout,
                    index: 11,
                },
            ),
            AnimationIndices::new(6, 11),
        )).id();

        app.update();

        let mut query = app
            .world_mut()
            .query::<(&Sprite, &AnimationIndices, &GuardianAnimations)>();
        let (sprite, indices, animations) = query
            .get(app.world(), entity)
            .expect("guardian components should exist");

        assert_eq!(animations.current_state, EntityState::Idle);
        assert_eq!(sprite.image, idle_texture);
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.layout.clone()),
            Some(idle_layout)
        );
        assert_eq!(sprite.texture_atlas.as_ref().map(|atlas| atlas.index), Some(8));
        assert_eq!(indices.first, 8);
        assert_eq!(indices.last, 15);
    }
}

/// Updates entity direction based on velocity
pub fn update_direction_from_velocity(
    mut query: Query<(&Velocity, &mut Direction), Changed<Velocity>>,
) {
    for (velocity, mut direction) in &mut query {
        if velocity.magnitude() > 0.1 {
            // Only update direction if actually moving
            *direction = Direction::from_velocity(velocity);
        }
    }
}

/// Updates entity state based on velocity
pub fn update_state_from_velocity(mut query: Query<(&Velocity, &mut EntityState)>) {
    for (velocity, mut state) in &mut query {
        match *state {
            EntityState::Dead => continue,      // Dead entities don't change state
            EntityState::Attacking => continue, // Don't interrupt attacking
            _ => {
                if velocity.magnitude() > 0.1 {
                    *state = EntityState::Moving;
                } else {
                    *state = EntityState::Idle;
                }
            }
        }
    }
}

/// Updates forest guardian animations based on entity state
/// Switches between idle and walk animations when EntityState changes
pub fn update_guardian_animation_from_state(
    mut query: Query<(
        &EntityState,
        &Direction,
        &mut GuardianAnimations,
        &mut Sprite,
        &mut AnimationIndices,
    )>,
) {
    for (state, direction, mut animations, mut sprite, mut indices) in &mut query {
        // Only switch if state has changed
        if animations.current_state != *state {
            animations.current_state = *state;

            match *state {
                EntityState::Idle => {
                    // Switch to idle animation
                    if let Some(atlas) = &mut sprite.texture_atlas {
                        atlas.layout = animations.idle_layout.clone();
                    }
                    sprite.image = animations.idle_texture.clone();
                    // Keep the current facing row when changing animation state.
                    update_animation_for_direction(*direction, &mut indices, animations.idle_frames);
                }
                EntityState::Moving => {
                    // Switch to walk animation
                    if let Some(atlas) = &mut sprite.texture_atlas {
                        atlas.layout = animations.walk_layout.clone();
                    }
                    sprite.image = animations.walk_texture.clone();
                    // Keep the current facing row when changing animation state.
                    update_animation_for_direction(*direction, &mut indices, animations.walk_frames);
                }
                EntityState::Attacking | EntityState::Dead => {
                    // For now, keep current animation for attacking/dead states
                    // Could be extended with attack/death animations later
                }
            }

            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = indices.first;
            }
        }
    }
}

/// Updates animation indices when direction changes
/// This system ensures the correct row of the sprite sheet is used based on direction
pub fn update_animation_from_direction(
    mut query: Query<(&Direction, &mut AnimationIndices, &Sprite), Changed<Direction>>,
) {
    for (direction, mut indices, sprite) in &mut query {
        if sprite.texture_atlas.is_some() {
            // Calculate frames per direction from current row span
            // When entities spawn, indices cover one row, so the count equals frames per direction
            let frames_per_direction = indices.last - indices.first + 1;
            if frames_per_direction > 0 {
                update_animation_for_direction(*direction, &mut indices, frames_per_direction);
            }
        }
    }
}

/// Animates sprites by cycling through animation frames
pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = if atlas.index >= indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}

/// Updates velocity for entities with roaming behavior
/// This makes entities roam randomly within a fixed radius of their home position
pub fn update_roaming_behavior(
    time: Res<Time>,
    mut query: Query<(&Position, &mut Velocity, &mut RoamingBehavior)>,
) {
    use std::collections::hash_map::RandomState;
    use std::f32::consts::PI;
    use std::hash::{BuildHasher, Hash, Hasher};
    let delta = time.delta_secs();

    for (position, mut velocity, mut roaming) in &mut query {
        // If we're paused, count down the pause timer
        if roaming.pause_timer > 0.0 {
            roaming.pause_timer -= delta;
            velocity.x = 0.0;
            velocity.y = 0.0;
            continue;
        }

        // Check if we've reached the target (within 5 pixels)
        if roaming.is_at_target(position, 5.0) {
            // Generate random numbers for next target
            let hasher_builder = RandomState::new();
            let mut hasher = hasher_builder.build_hasher();
            position.x.to_bits().hash(&mut hasher);
            position.y.to_bits().hash(&mut hasher);
            std::time::SystemTime::now().hash(&mut hasher);
            let hash = hasher.finish();

            // Random angle
            let rand_angle = ((hash as f32) / (u64::MAX as f32)) * 2.0 * PI;

            // Random distance within roam radius
            let mut hasher2 = hasher_builder.build_hasher();
            (hash.wrapping_add(1)).hash(&mut hasher2);
            let hash2 = hasher2.finish();
            let rand_distance = ((hash2 as f32) / (u64::MAX as f32)) * roaming.roam_radius;

            // Calculate new target position within bounds
            let offset_x = rand_angle.cos() * rand_distance;
            let offset_y = rand_angle.sin() * rand_distance;
            roaming.target.x = roaming.home.x + offset_x;
            roaming.target.y = roaming.home.y + offset_y;

            // Generate random pause duration
            let mut hasher3 = hasher_builder.build_hasher();
            (hash2.wrapping_add(1)).hash(&mut hasher3);
            let hash3 = hasher3.finish();
            let rand_pause = (hash3 as f32) / (u64::MAX as f32);
            roaming.pause_duration = roaming.min_pause_duration
                + rand_pause * (roaming.max_pause_duration - roaming.min_pause_duration);
            roaming.pause_timer = roaming.pause_duration;

            // Stop moving while paused
            velocity.x = 0.0;
            velocity.y = 0.0;
            continue;
        }

        // Calculate direction to target
        let dx = roaming.target.x - position.x;
        let dy = roaming.target.y - position.y;
        let distance = (dx * dx + dy * dy).sqrt();

        // If we're very close, just stop (handled above on next frame)
        if distance < 0.1 {
            velocity.x = 0.0;
            velocity.y = 0.0;
        } else {
            // Move towards target at roaming speed
            let dir_x = dx / distance;
            let dir_y = dy / distance;
            velocity.x = dir_x * roaming.speed;
            velocity.y = dir_y * roaming.speed;
        }
    }
}

/// Updates velocity for entities with winding path behavior
/// This creates smooth, meandering movement with long straight sections
pub fn update_winding_path(time: Res<Time>, mut query: Query<(&mut Velocity, &mut WindingPath)>) {
    use std::collections::hash_map::RandomState;
    use std::f32::consts::PI;
    use std::hash::{BuildHasher, Hash, Hasher};
    let delta = time.delta_secs();

    for (mut velocity, mut path) in &mut query {
        // Calculate distance moved this frame
        let speed = path.speed;
        let distance_this_frame = speed * delta;
        path.distance_traveled += distance_this_frame;

        // Check if we've reached the end of current segment
        if path.distance_traveled >= path.segment_length {
            // Generate random numbers using hash
            let hasher_builder = RandomState::new();
            let mut hasher = hasher_builder.build_hasher();
            (path.current_angle.to_bits() as u64).hash(&mut hasher);
            path.distance_traveled.to_bits().hash(&mut hasher);
            let hash = hasher.finish();
            let rand1 = ((hash as f32) / (u64::MAX as f32)) - 0.5;

            // Pick a new target direction with constrained angle change
            let angle_change = rand1 * 2.0 * path.max_angle_change;
            path.target_angle = path.current_angle + angle_change;

            // Normalize target angle to [0, 2π]
            path.target_angle = path.target_angle.rem_euclid(2.0 * PI);

            // Generate another random number for segment length
            let mut hasher2 = hasher_builder.build_hasher();
            (hash.wrapping_add(1)).hash(&mut hasher2);
            let hash2 = hasher2.finish();
            let rand2 = (hash2 as f32) / (u64::MAX as f32);

            // Pick a new segment length
            path.segment_length = path.min_segment_length
                + rand2 * (path.max_segment_length - path.min_segment_length);

            // Reset distance counter
            path.distance_traveled = 0.0;
        }

        // Smoothly interpolate current angle towards target angle
        let angle_diff = path.target_angle - path.current_angle;

        // Handle wrapping around 0/2π boundary (choose shortest rotation)
        let angle_diff = if angle_diff > PI {
            angle_diff - 2.0 * PI
        } else if angle_diff < -PI {
            angle_diff + 2.0 * PI
        } else {
            angle_diff
        };

        // Apply turn rate
        let turn_amount = (angle_diff.signum() * path.turn_rate * delta)
            .clamp(-angle_diff.abs(), angle_diff.abs());
        path.current_angle += turn_amount;

        // Normalize current angle to [0, 2π]
        path.current_angle = path.current_angle.rem_euclid(2.0 * PI);

        // Update velocity based on current angle
        velocity.x = path.current_angle.cos() * speed;
        velocity.y = path.current_angle.sin() * speed;
    }
}

/// Makes snails turn tiles they walk over into dirt with a 20% chance
pub fn snail_dirt_trail(
    mut world: ResMut<WorldManager>,
    snail_query: Query<&Position, (With<Snail>, Changed<Position>)>,
) {
    use crate::tiles::LAYER_GROUND;
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    for position in snail_query.iter() {
        // Generate a random number using hash of position and time
        let hasher_builder = RandomState::new();
        let mut hasher = hasher_builder.build_hasher();
        position.x.to_bits().hash(&mut hasher);
        position.y.to_bits().hash(&mut hasher);
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        let rand_val = (hash as f32) / (u64::MAX as f32);

        if rand_val < 0.2 {
            world.queue_tile_modification(position.x, position.y, TILE_DIRT, LAYER_GROUND);
        }
    }
}

/// Advances tree growth through stages over time.
/// Works for any entity with a GrowingTree component (tree spirits, RTS trees, etc.)
pub fn update_tree_growth(
    time: Res<Time>,
    mut tree_query: Query<(&mut GrowingTree, &mut Transform)>,
) {
    let delta = time.delta_secs();

    for (mut growing_tree, mut transform) in tree_query.iter_mut() {
        if growing_tree.is_mature() {
            continue;
        }

        growing_tree.time_in_stage += delta;

        if growing_tree.time_in_stage >= growing_tree.time_to_next_stage {
            if let Some(next_stage) = growing_tree.stage.next() {
                growing_tree.stage = next_stage;
                growing_tree.time_in_stage = 0.0;

                let new_scale = growing_tree.base_scale * next_stage.scale();
                transform.scale = Vec3::splat(new_scale);

                info!(
                    "Tree advanced to stage {:?} (scale {:.2})",
                    next_stage, new_scale
                );
            }
        }
    }
}

/// Spawns trees around entities with TreeSpawner component
pub fn update_tree_spawning(
    time: Res<Time>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut spawner_query: Query<(&Position, &mut TreeSpawner, Option<&ForestGuardian>)>,
) {
    use std::collections::hash_map::RandomState;
    use std::f32::consts::PI;
    use std::hash::{BuildHasher, Hash, Hasher};

    let delta = time.delta_secs();

    for (position, mut spawner, guardian) in spawner_query.iter_mut() {
        // Count down spawn timer
        spawner.spawn_timer -= delta;

        // Check if it's time to spawn a tree
        if spawner.spawn_timer <= 0.0 {
            // Generate random values using hash
            let hasher_builder = RandomState::new();
            let mut hasher = hasher_builder.build_hasher();
            position.x.to_bits().hash(&mut hasher);
            position.y.to_bits().hash(&mut hasher);
            std::time::SystemTime::now().hash(&mut hasher);
            let hash = hasher.finish();

            // Random angle for tree placement
            let rand_angle = ((hash as f32) / (u64::MAX as f32)) * 2.0 * PI;

            // Random distance within spawn radius
            let mut hasher2 = hasher_builder.build_hasher();
            (hash.wrapping_add(1)).hash(&mut hasher2);
            let hash2 = hasher2.finish();
            let rand_distance = ((hash2 as f32) / (u64::MAX as f32)) * spawner.spawn_radius;

            // Calculate spawn position
            let spawn_x = position.x + rand_angle.cos() * rand_distance;
            let spawn_y = position.y + rand_angle.sin() * rand_distance;

            // Determine tree variant based on guardian variant (if present)
            let tree_variant = if let Some(guardian) = guardian {
                // Generate random value for variant selection
                let mut hasher3 = hasher_builder.build_hasher();
                (hash2.wrapping_add(1)).hash(&mut hasher3);
                let hash3 = hasher3.finish();
                let rand_variant = (hash3 as f32) / (u64::MAX as f32);

                if rand_variant < 0.95 {
                    // 95% chance: spawn matching variant
                    guardian.variant
                } else {
                    // 5% chance: spawn different variant
                    let mut hasher4 = hasher_builder.build_hasher();
                    (hash3.wrapping_add(1)).hash(&mut hasher4);
                    let hash4 = hasher4.finish();
                    let rand_other = (hash4 as f32) / (u64::MAX as f32);
                    guardian.variant.random_other(rand_other)
                }
            } else {
                // No guardian component, pick fully random variant
                let mut hasher3 = hasher_builder.build_hasher();
                (hash2.wrapping_add(1)).hash(&mut hasher3);
                let hash3 = hasher3.finish();
                let variant_index = (hash3 % 5) as usize;
                match variant_index {
                    0 => TreeVariant::Oak,
                    1 => TreeVariant::Birch,
                    2 => TreeVariant::Hickory,
                    3 => TreeVariant::Pine,
                    _ => TreeVariant::Willow,
                }
            };

            // Randomly choose between variant tree (90%) or tree spirit (10%)
            let mut hasher_type = hasher_builder.build_hasher();
            (hash2.wrapping_add(42)).hash(&mut hasher_type);
            let hash_type = hasher_type.finish();
            let rand_type = (hash_type as f32) / (u64::MAX as f32);

            let spawn_pos = Position::new(spawn_x, spawn_y);
            if rand_type < 0.9 {
                // Spawn variant tree with matching tree type (90%)
                spawn_variant_tree(
                    &mut commands,
                    spawn_pos,
                    tree_variant,
                    spawner.tree_growth_time,
                    &assets,
                );

                if let Some(guardian) = guardian {
                    let is_matching = tree_variant == guardian.variant;
                    info!(
                        "{:?} guardian spawned {:?} variant tree at ({:.1}, {:.1}) {}",
                        guardian.variant,
                        tree_variant,
                        spawn_x,
                        spawn_y,
                        if is_matching {
                            "(matching)"
                        } else {
                            "(different!)"
                        }
                    );
                } else {
                    info!(
                        "Entity spawned {:?} variant tree at ({:.1}, {:.1})",
                        tree_variant, spawn_x, spawn_y
                    );
                }
            } else {
                // Spawn animated tree spirit (10%)
                spawn_tree_spirit(
                    &mut commands,
                    spawn_pos,
                    tree_variant,
                    spawner.tree_growth_time,
                    &assets,
                    &mut texture_atlas_layouts,
                );

                if let Some(guardian) = guardian {
                    let is_matching = tree_variant == guardian.variant;
                    info!(
                        "{:?} guardian spawned {:?} tree spirit at ({:.1}, {:.1}) {}",
                        guardian.variant,
                        tree_variant,
                        spawn_x,
                        spawn_y,
                        if is_matching {
                            "(matching)"
                        } else {
                            "(different!)"
                        }
                    );
                } else {
                    info!(
                        "Entity spawned {:?} tree spirit at ({:.1}, {:.1})",
                        tree_variant, spawn_x, spawn_y
                    );
                }
            }

            // Reset spawn timer with random interval
            let mut hasher_interval = hasher_builder.build_hasher();
            position.x.to_bits().hash(&mut hasher_interval);
            std::time::SystemTime::now().hash(&mut hasher_interval);
            let hash_interval = hasher_interval.finish();
            let rand_interval = (hash_interval as f32) / (u64::MAX as f32);
            spawner.spawn_timer = spawner.min_spawn_interval
                + rand_interval * (spawner.max_spawn_interval - spawner.min_spawn_interval);
        }
    }
}

/// Debug system to print entity information
#[allow(dead_code)]
pub fn debug_entities(query: Query<(Entity, &Position, &Velocity, &Direction, &EntityState)>) {
    for (entity, position, velocity, direction, state) in &query {
        info!(
            "Entity {:?}: pos=({:.1}, {:.1}), vel=({:.1}, {:.1}), dir={:?}, state={:?}",
            entity, position.x, position.y, velocity.x, velocity.y, direction, state
        );
    }
}
