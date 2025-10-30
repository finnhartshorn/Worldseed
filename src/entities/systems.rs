use bevy::prelude::*;
use super::{Position, Velocity, Direction, EntityState, AnimationIndices, WindingPath, RoamingBehavior, Snail, GrowingTree, TreeSpirit};
use super::spawning::{AnimationTimer, update_animation_for_direction};
use crate::world::WorldManager;
use crate::tiles::TILE_DIRT;

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
pub fn apply_velocity(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
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
pub fn update_state_from_velocity(
    mut query: Query<(&Velocity, &mut EntityState)>,
) {
    for (velocity, mut state) in &mut query {
        match *state {
            EntityState::Dead => continue, // Dead entities don't change state
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
    use std::f32::consts::PI;
    use std::collections::hash_map::RandomState;
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
pub fn update_winding_path(
    time: Res<Time>,
    mut query: Query<(&mut Velocity, &mut WindingPath)>,
) {
    use std::f32::consts::PI;
    use std::collections::hash_map::RandomState;
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
        let turn_amount = (angle_diff.signum() * path.turn_rate * delta).clamp(-angle_diff.abs(), angle_diff.abs());
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
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    use crate::tiles::LAYER_GROUND;

    for position in snail_query.iter() {
        // Generate a random number using hash of position and time
        let hasher_builder = RandomState::new();
        let mut hasher = hasher_builder.build_hasher();
        position.x.to_bits().hash(&mut hasher);
        position.y.to_bits().hash(&mut hasher);
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        let rand_val = (hash as f32) / (u64::MAX as f32);

        // 20% chance to turn tile into dirt on the ground layer
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

/// Debug system to print entity information
#[allow(dead_code)]
pub fn debug_entities(
    query: Query<(Entity, &Position, &Velocity, &Direction, &EntityState)>,
) {
    for (entity, position, velocity, direction, state) in &query {
        info!(
            "Entity {:?}: pos=({:.1}, {:.1}), vel=({:.1}, {:.1}), dir={:?}, state={:?}",
            entity, position.x, position.y, velocity.x, velocity.y, direction, state
        );
    }
}
