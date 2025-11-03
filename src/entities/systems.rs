use super::spawning::{spawn_tree_spirit, update_animation_for_direction, AnimationTimer};
use super::{
    AnimationIndices, Direction, EntityState, ForestGuardian, GrowingTree, Position,
    RoamingBehavior, Snail, TreeSpawner, TreeSpirit, TreeVariant, Velocity, WindingPath,
};
use crate::tiles::TILE_DIRT;
use crate::world::WorldManager;
use bevy::prelude::*;

/// Syncs entity Position component with Transform for rendering
pub fn sync_position_with_transform(
    mut query: Query<(&Position, &mut Transform), Changed<Position>>,
) {
    for (position, mut transform) in &mut query {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
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

/// Advances tree growth through stages over time
pub fn update_tree_growth(
    time: Res<Time>,
    mut tree_query: Query<(&mut GrowingTree, &mut Transform), With<TreeSpirit>>,
) {
    let delta = time.delta_secs();

    for (mut growing_tree, mut transform) in tree_query.iter_mut() {
        // Skip if already mature
        if growing_tree.is_mature() {
            continue;
        }

        // Accumulate time in current stage
        growing_tree.time_in_stage += delta;

        // Check if ready to advance to next stage
        if growing_tree.time_in_stage >= growing_tree.time_to_next_stage {
            if let Some(next_stage) = growing_tree.stage.next() {
                // Advance to next stage
                growing_tree.stage = next_stage;
                growing_tree.time_in_stage = 0.0;

                // Update scale based on new stage
                let new_scale = next_stage.scale();
                transform.scale = Vec3::splat(new_scale);

                info!(
                    "Tree advanced to stage {:?} with scale {:.1}",
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

            // Spawn the tree
            spawn_tree_spirit(
                &mut commands,
                Position::new(spawn_x, spawn_y),
                tree_variant,
                spawner.tree_growth_time,
                &assets,
                &mut texture_atlas_layouts,
            );

            if let Some(guardian) = guardian {
                let is_matching = tree_variant == guardian.variant;
                info!(
                    "{:?} guardian spawned {:?} tree at ({:.1}, {:.1}) {}",
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
                    "Entity spawned {:?} tree at ({:.1}, {:.1})",
                    tree_variant, spawn_x, spawn_y
                );
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
