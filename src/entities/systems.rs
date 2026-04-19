use super::spawning::{
    spawn_tree_spirit, spawn_variant_tree, update_animation_for_direction,
    variant_tree_growth_texture_atlas_layout, variant_tree_shared_variation_texture_atlas_layout,
    AnimationTimer,
};
use super::{
    AnimationIndices, Direction, EntityState, ForestGuardian, GrowingTree, GrowthStage,
    GuardianAnimations, Position, RoamingBehavior, Snail, TreeSpawner, TreeVariant, VariantTree,
    Velocity, WindingPath, WorldRenderDepth,
};
use crate::tiles::TILE_DIRT;
use crate::world::WorldManager;
use bevy::prelude::*;

const GUARDIAN_MIN_TREE_SPACING: f32 = 24.0;
const GUARDIAN_LOCAL_DENSITY_RADIUS: f32 = 40.0;
const GUARDIAN_LOCAL_TREE_CAP: usize = 3;
const GUARDIAN_SPAWN_ATTEMPTS: u64 = 4;
const ROAMING_TARGET_ATTEMPTS: u64 = 6;
const WINDING_PATH_REDIRECTION_ATTEMPTS: u64 = 8;

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

fn is_land_position(world: &mut WorldManager, position: &Position) -> bool {
    world.has_land_at_world(Vec2::new(position.x, position.y))
}

fn is_land_vec2(world: &mut WorldManager, world_pos: Vec2) -> bool {
    world.has_land_at_world(world_pos)
}

fn random_angle_from_seed(seed: u64) -> f32 {
    use std::f32::consts::TAU;

    ((seed as f32) / (u64::MAX as f32)) * TAU
}

fn pick_roaming_target(
    world: &mut WorldManager,
    roaming: &RoamingBehavior,
    seed: u64,
) -> Option<Position> {
    for attempt in 0..ROAMING_TARGET_ATTEMPTS {
        let angle_seed = mix_seed(seed ^ attempt.wrapping_mul(0xA24B_AED4_963E_E407));
        let distance_seed =
            mix_seed(seed ^ attempt.wrapping_mul(0x9FB2_1C65_1E98_DF25) ^ 0xD1B5_4A32_D192_ED03);
        let rand_angle = random_angle_from_seed(angle_seed);
        let rand_distance = random_fraction(distance_seed) * roaming.roam_radius;
        let candidate = Position::new(
            roaming.home.x + rand_angle.cos() * rand_distance,
            roaming.home.y + rand_angle.sin() * rand_distance,
        );

        if is_land_position(world, &candidate) {
            return Some(candidate);
        }
    }

    None
}

fn pick_winding_path_redirection(
    world: &mut WorldManager,
    position: &Position,
    path: &WindingPath,
    delta: f32,
    seed: u64,
) -> Option<f32> {
    use std::f32::consts::PI;

    for attempt in 0..WINDING_PATH_REDIRECTION_ATTEMPTS {
        let attempt_seed = mix_seed(seed ^ attempt.wrapping_mul(0x94D0_49BB_1331_11EB));
        let rand1 = random_fraction(attempt_seed) - 0.5;
        let candidate_angle =
            (path.current_angle + rand1 * 2.0 * path.max_angle_change).rem_euclid(2.0 * PI);
        let next_position = Vec2::new(
            position.x + candidate_angle.cos() * path.speed * delta,
            position.y + candidate_angle.sin() * path.speed * delta,
        );

        if is_land_vec2(world, next_position) {
            return Some(candidate_angle);
        }
    }

    None
}

/// Updates entity position based on velocity
pub fn apply_velocity(
    time: Res<Time>,
    mut world: ResMut<WorldManager>,
    mut query: Query<(&mut Position, &mut Velocity)>,
) {
    let delta = time.delta_secs();
    for (mut position, mut velocity) in &mut query {
        let next_position = Vec2::new(
            position.x + velocity.x * delta,
            position.y + velocity.y * delta,
        );

        if (velocity.x != 0.0 || velocity.y != 0.0) && !is_land_vec2(&mut world, next_position) {
            velocity.x = 0.0;
            velocity.y = 0.0;
            continue;
        }

        position.x = next_position.x;
        position.y = next_position.y;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        candidate_spawn_position, find_spawn_location, is_spawn_location_valid, mix_seed,
        update_guardian_animation_from_state, update_tree_growth, validate_spawn_location,
        GUARDIAN_LOCAL_DENSITY_RADIUS, GUARDIAN_MIN_TREE_SPACING, GUARDIAN_SPAWN_ATTEMPTS,
    };
    use crate::entities::{
        spawning::{FOREST_GUARDIAN_DEPTH_BIAS, VARIANT_TREE_DEPTH_BIAS},
        AnimationIndices, Direction, EntityState, GrowingTree, GrowthStage, GuardianAnimations,
        Position, RenderStratum, RtsTree, TreeVariant, VariantTree, VariantTreeAppearance,
        WorldRenderDepth,
    };
    use bevy::prelude::*;
    use std::time::Duration;

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
    fn configured_world_object_biases_stay_below_one_y_sort_step() {
        let bias_gap = (FOREST_GUARDIAN_DEPTH_BIAS - VARIANT_TREE_DEPTH_BIAS).abs();

        assert!(bias_gap < WorldRenderDepth::Y_SORT_SCALE);
    }

    #[test]
    fn guardian_bias_only_wins_when_y_positions_are_nearly_identical() {
        let guardian_y = 0.0;
        let tree_y = -1.0;
        let guardian_depth =
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, FOREST_GUARDIAN_DEPTH_BIAS);
        let tree_depth =
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, VARIANT_TREE_DEPTH_BIAS);

        assert!(
            guardian_depth.z_for_position(&Position::new(0.0, guardian_y))
                > tree_depth.z_for_position(&Position::new(0.0, guardian_y))
        );
        assert!(
            tree_depth.z_for_position(&Position::new(0.0, tree_y))
                > guardian_depth.z_for_position(&Position::new(0.0, guardian_y))
        );
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

        let entity = app
            .world_mut()
            .spawn((
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
            ))
            .id();

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
            sprite
                .texture_atlas
                .as_ref()
                .map(|atlas| atlas.layout.clone()),
            Some(walk_layout)
        );
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            Some(18)
        );
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

        let entity = app
            .world_mut()
            .spawn((
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
            ))
            .id();

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
            sprite
                .texture_atlas
                .as_ref()
                .map(|atlas| atlas.layout.clone()),
            Some(idle_layout)
        );
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            Some(8)
        );
        assert_eq!(indices.first, 8);
        assert_eq!(indices.last, 15);
    }

    #[test]
    fn spawn_location_rejects_trees_inside_minimum_spacing() {
        let candidate = Position::new(0.0, 0.0);
        let tree_positions = [Position::new(GUARDIAN_MIN_TREE_SPACING - 1.0, 0.0)];

        let validation = validate_spawn_location(&candidate, &tree_positions);

        assert!(!is_spawn_location_valid(validation));
    }

    #[test]
    fn spawn_location_accepts_when_nearest_tree_is_outside_spacing_limit() {
        let candidate = Position::new(0.0, 0.0);
        let tree_positions = [Position::new(GUARDIAN_MIN_TREE_SPACING + 1.0, 0.0)];

        let validation = validate_spawn_location(&candidate, &tree_positions);

        assert!(is_spawn_location_valid(validation));
    }

    #[test]
    fn spawn_location_rejects_when_local_density_cap_is_reached() {
        let candidate = Position::new(0.0, 0.0);
        let edge = GUARDIAN_LOCAL_DENSITY_RADIUS - 1.0;
        let tree_positions = [
            Position::new(edge, 0.0),
            Position::new(-edge, 0.0),
            Position::new(0.0, edge),
        ];

        let validation = validate_spawn_location(&candidate, &tree_positions);

        assert_eq!(validation.nearby_tree_count, 3);
        assert!(!is_spawn_location_valid(validation));
    }

    #[test]
    fn spawn_location_ignores_trees_outside_density_radius() {
        let candidate = Position::new(0.0, 0.0);
        let tree_positions = [Position::new(GUARDIAN_LOCAL_DENSITY_RADIUS + 1.0, 0.0)];

        let validation = validate_spawn_location(&candidate, &tree_positions);

        assert_eq!(validation.nearby_tree_count, 0);
        assert!(is_spawn_location_valid(validation));
    }

    #[test]
    fn find_spawn_location_retries_until_it_finds_open_space() {
        let origin = Position::new(0.0, 0.0);
        let spawn_radius = 80.0;
        let seed = 0xDEAD_BEEF_CAFE_BABE;
        let candidates: Vec<_> = (0..GUARDIAN_SPAWN_ATTEMPTS)
            .map(|attempt| {
                candidate_spawn_position(
                    &origin,
                    spawn_radius,
                    mix_seed(seed ^ attempt.wrapping_mul(0xA24B_AED4_963E_E407)),
                    mix_seed(
                        seed ^ attempt.wrapping_mul(0x9FB2_1C65_1E98_DF25) ^ 0xD1B5_4A32_D192_ED03,
                    ),
                )
            })
            .collect();

        let (expected_index, blockers) = (1..candidates.len())
            .find_map(|index| {
                let blockers = candidates[..index].to_vec();
                let validation = validate_spawn_location(&candidates[index], &blockers);
                is_spawn_location_valid(validation).then_some((index, blockers))
            })
            .expect("expected at least one retry candidate to remain valid");

        let spawn_location = find_spawn_location(&origin, spawn_radius, &blockers, seed)
            .expect("retry should find space");

        assert!(spawn_location.distance_to(&candidates[0]) >= GUARDIAN_MIN_TREE_SPACING);
        assert!(spawn_location.distance_to(&candidates[expected_index]) < 0.01);
    }

    #[test]
    fn find_spawn_location_returns_none_when_all_attempts_are_crowded() {
        let origin = Position::new(0.0, 0.0);
        let spawn_radius = 80.0;
        let seed = 0x1234_5678_9ABC_DEF0;
        let tree_positions: Vec<_> = (0..GUARDIAN_SPAWN_ATTEMPTS)
            .map(|attempt| {
                candidate_spawn_position(
                    &origin,
                    spawn_radius,
                    mix_seed(seed ^ attempt.wrapping_mul(0xA24B_AED4_963E_E407)),
                    mix_seed(
                        seed ^ attempt.wrapping_mul(0x9FB2_1C65_1E98_DF25) ^ 0xD1B5_4A32_D192_ED03,
                    ),
                )
            })
            .collect();

        let spawn_location = find_spawn_location(&origin, spawn_radius, &tree_positions, seed);

        assert!(spawn_location.is_none());
    }

    #[test]
    fn variant_tree_growth_advances_atlas_frame_without_rescaling() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.add_systems(Update, update_tree_growth);

        let atlas_layout = Handle::<TextureAtlasLayout>::default();
        let entity = app
            .world_mut()
            .spawn((
                VariantTree,
                GrowingTree::with_variant_appearance(
                    TreeVariant::Oak,
                    1.0,
                    4.0,
                    VariantTreeAppearance::Original,
                ),
                Sprite::from_atlas_image(
                    Handle::<Image>::default(),
                    TextureAtlas {
                        layout: atlas_layout,
                        index: 0,
                    },
                ),
                Transform::from_scale(Vec3::splat(4.0)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.update();

        let mut query = app
            .world_mut()
            .query::<(&GrowingTree, &Sprite, &Transform)>();
        let (growing_tree, sprite, transform) = query
            .get(app.world(), entity)
            .expect("variant tree components should exist");

        assert_eq!(growing_tree.stage, GrowthStage::Sapling);
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            Some(1)
        );
        assert_eq!(transform.scale, Vec3::splat(4.0));
    }

    #[test]
    fn new_variant_tree_growth_uses_mature_sprite_at_three_quarter_scale() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.add_systems(Update, update_tree_growth);

        let entity = app
            .world_mut()
            .spawn((
                VariantTree,
                GrowingTree::with_variant_appearance(
                    TreeVariant::Willow,
                    1.0,
                    4.0,
                    VariantTreeAppearance::Variation2,
                ),
                Sprite::from_atlas_image(
                    Handle::<Image>::default(),
                    TextureAtlas {
                        layout: Handle::<TextureAtlasLayout>::default(),
                        index: 1,
                    },
                ),
                Transform::from_scale(Vec3::splat(4.0)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.update();

        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.update();

        let mut query = app
            .world_mut()
            .query::<(&GrowingTree, &Sprite, &Transform)>();
        let (growing_tree, sprite, transform) = query
            .get(app.world(), entity)
            .expect("variant tree components should exist");

        assert_eq!(growing_tree.stage, GrowthStage::YoungTree);
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            Some(10)
        );
        assert_eq!(transform.scale, Vec3::splat(3.0));
    }

    #[test]
    fn new_variant_tree_growth_restores_full_scale_when_mature() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.add_systems(Update, update_tree_growth);

        let entity = app
            .world_mut()
            .spawn((
                VariantTree,
                GrowingTree::with_variant_appearance(
                    TreeVariant::Birch,
                    1.0,
                    4.0,
                    VariantTreeAppearance::Variation3,
                ),
                Sprite::from_atlas_image(
                    Handle::<Image>::default(),
                    TextureAtlas {
                        layout: Handle::<TextureAtlasLayout>::default(),
                        index: 0,
                    },
                ),
                Transform::from_scale(Vec3::splat(4.0)),
            ))
            .id();

        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<Time<()>>()
                .advance_by(Duration::from_secs_f32(1.0));
            app.update();
        }

        let mut query = app
            .world_mut()
            .query::<(&GrowingTree, &Sprite, &Transform)>();
        let (growing_tree, sprite, transform) = query
            .get(app.world(), entity)
            .expect("variant tree components should exist");

        assert_eq!(growing_tree.stage, GrowthStage::MatureTree);
        assert_eq!(
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            Some(15)
        );
        assert_eq!(transform.scale, Vec3::splat(4.0));
    }

    #[test]
    fn pine_variant_never_uses_extra_appearance() {
        assert_eq!(
            TreeVariant::Pine.choose_appearance(0.99),
            VariantTreeAppearance::Original
        );
    }

    #[test]
    fn non_variant_tree_growth_keeps_scale_based_progression() {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.add_systems(Update, update_tree_growth);

        let entity = app
            .world_mut()
            .spawn((
                RtsTree,
                GrowingTree::with_base_scale(TreeVariant::Oak, 1.0, 2.0),
                Transform::from_scale(Vec3::splat(1.0)),
            ))
            .id();

        app.world_mut()
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.update();

        let mut query = app.world_mut().query::<(&GrowingTree, &Transform)>();
        let (growing_tree, transform) = query
            .get(app.world(), entity)
            .expect("scalable tree components should exist");

        assert_eq!(growing_tree.stage, GrowthStage::Sapling);
        assert_eq!(transform.scale, Vec3::splat(2.0));
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
                    update_animation_for_direction(
                        *direction,
                        &mut indices,
                        animations.idle_frames,
                    );
                }
                EntityState::Moving => {
                    // Switch to walk animation
                    if let Some(atlas) = &mut sprite.texture_atlas {
                        atlas.layout = animations.walk_layout.clone();
                    }
                    sprite.image = animations.walk_texture.clone();
                    // Keep the current facing row when changing animation state.
                    update_animation_for_direction(
                        *direction,
                        &mut indices,
                        animations.walk_frames,
                    );
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
    mut world: ResMut<WorldManager>,
    mut query: Query<(&Position, &mut Velocity, &mut RoamingBehavior)>,
) {
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
            roaming.target =
                pick_roaming_target(&mut world, &roaming, hash).unwrap_or(roaming.home);

            // Generate random pause duration
            let mut hasher3 = hasher_builder.build_hasher();
            (hash.wrapping_add(1)).hash(&mut hasher3);
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
            let next_position = Vec2::new(
                position.x + dir_x * roaming.speed * delta,
                position.y + dir_y * roaming.speed * delta,
            );
            if is_land_vec2(&mut world, next_position) {
                velocity.x = dir_x * roaming.speed;
                velocity.y = dir_y * roaming.speed;
            } else {
                roaming.target = roaming.home;
                velocity.x = 0.0;
                velocity.y = 0.0;
            }
        }
    }
}

/// Updates velocity for entities with winding path behavior
/// This creates smooth, meandering movement with long straight sections
pub fn update_winding_path(
    time: Res<Time>,
    mut world: ResMut<WorldManager>,
    mut query: Query<(&Position, &mut Velocity, &mut WindingPath)>,
) {
    use std::collections::hash_map::RandomState;
    use std::f32::consts::PI;
    use std::hash::{BuildHasher, Hash, Hasher};
    let delta = time.delta_secs();

    for (position, mut velocity, mut path) in &mut query {
        let hasher_builder = RandomState::new();

        // Calculate distance moved this frame
        let speed = path.speed;
        let distance_this_frame = speed * delta;
        path.distance_traveled += distance_this_frame;

        // Check if we've reached the end of current segment
        if path.distance_traveled >= path.segment_length {
            // Generate random numbers using hash
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

        let next_position = Vec2::new(
            position.x + velocity.x * delta,
            position.y + velocity.y * delta,
        );
        if !is_land_vec2(&mut world, next_position) {
            velocity.x = 0.0;
            velocity.y = 0.0;
            let mut hasher = hasher_builder.build_hasher();
            position.x.to_bits().hash(&mut hasher);
            position.y.to_bits().hash(&mut hasher);
            path.current_angle.to_bits().hash(&mut hasher);
            std::time::SystemTime::now().hash(&mut hasher);
            let redirect_seed = hasher.finish();

            if let Some(new_angle) =
                pick_winding_path_redirection(&mut world, position, &path, delta, redirect_seed)
            {
                path.current_angle = new_angle;
                path.target_angle = new_angle;
            }

            path.distance_traveled = path.segment_length;
        }
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
    assets: Option<Res<AssetServer>>,
    mut texture_atlas_layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    mut variant_tree_query: Query<
        (&mut GrowingTree, &mut Sprite, &mut Transform),
        With<VariantTree>,
    >,
    mut scalable_tree_query: Query<(&mut GrowingTree, &mut Transform), Without<VariantTree>>,
) {
    let delta = time.delta_secs();

    for (mut growing_tree, mut sprite, mut transform) in variant_tree_query.iter_mut() {
        if growing_tree.is_mature() {
            continue;
        }

        growing_tree.time_in_stage += delta;

        if growing_tree.time_in_stage >= growing_tree.time_to_next_stage {
            if let Some(next_stage) = growing_tree.stage.next() {
                growing_tree.stage = next_stage;
                growing_tree.time_in_stage = 0.0;

                apply_variant_tree_stage_visuals(
                    &growing_tree,
                    assets.as_ref(),
                    texture_atlas_layouts.as_mut(),
                    &mut sprite,
                    &mut transform,
                );

                info!("Variant tree advanced to stage {:?}", next_stage);
            }
        }
    }

    for (mut growing_tree, mut transform) in scalable_tree_query.iter_mut() {
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

fn apply_variant_tree_stage_visuals(
    growing_tree: &GrowingTree,
    assets: Option<&Res<AssetServer>>,
    texture_atlas_layouts: Option<&mut ResMut<Assets<TextureAtlasLayout>>>,
    sprite: &mut Sprite,
    transform: &mut Transform,
) {
    if growing_tree.appearance.uses_shared_mature_sheet()
        && matches!(
            growing_tree.stage,
            GrowthStage::YoungTree | GrowthStage::MatureTree
        )
    {
        let index = (growing_tree.appearance.mature_row() * 4
            + growing_tree
                .variant
                .shared_variation_column()
                .expect("non-pine variants should have shared variation column"))
            as usize;

        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = index;
        }

        if let (Some(assets), Some(texture_atlas_layouts)) = (assets, texture_atlas_layouts) {
            let texture = assets.load(
                growing_tree
                    .variant
                    .shared_variation_sheet_path()
                    .expect("non-pine variants should have shared variation sheet"),
            );
            let layout = variant_tree_shared_variation_texture_atlas_layout(texture_atlas_layouts);
            sprite.image = texture;
            sprite.texture_atlas = Some(TextureAtlas { layout, index });
        }
    } else {
        let index = growing_tree.stage.frame_index();

        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = index;
        }

        if let (Some(assets), Some(texture_atlas_layouts)) = (assets, texture_atlas_layouts) {
            let texture = assets.load(growing_tree.variant.growth_stage_asset_path());
            let layout = variant_tree_growth_texture_atlas_layout(
                texture_atlas_layouts,
                growing_tree.variant.growth_stage_frame_size(),
            );

            sprite.image = texture;
            sprite.texture_atlas = Some(TextureAtlas { layout, index });
        }
    }

    transform.scale = Vec3::splat(growing_tree.current_variant_tree_scale());
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SpawnLocationValidation {
    nearest_tree_distance: Option<f32>,
    nearby_tree_count: usize,
}

fn random_fraction(seed: u64) -> f32 {
    (seed as f32) / (u64::MAX as f32)
}

fn mix_seed(seed: u64) -> u64 {
    let mut value = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn candidate_spawn_position(
    origin: &Position,
    spawn_radius: f32,
    angle_seed: u64,
    distance_seed: u64,
) -> Position {
    use std::f32::consts::PI;

    let angle = random_fraction(angle_seed) * 2.0 * PI;
    let distance = random_fraction(distance_seed) * spawn_radius;

    Position::new(
        origin.x + angle.cos() * distance,
        origin.y + angle.sin() * distance,
    )
}

fn validate_spawn_location(
    candidate: &Position,
    tree_positions: &[Position],
) -> SpawnLocationValidation {
    let mut nearest_tree_distance: Option<f32> = None;
    let mut nearby_tree_count = 0;

    for tree_position in tree_positions {
        let distance = candidate.distance_to(tree_position);
        nearest_tree_distance = Some(match nearest_tree_distance {
            Some(current) => current.min(distance),
            None => distance,
        });

        if distance <= GUARDIAN_LOCAL_DENSITY_RADIUS {
            nearby_tree_count += 1;
        }
    }

    SpawnLocationValidation {
        nearest_tree_distance,
        nearby_tree_count,
    }
}

fn is_spawn_location_valid(validation: SpawnLocationValidation) -> bool {
    let spacing_ok = validation
        .nearest_tree_distance
        .is_none_or(|distance| distance >= GUARDIAN_MIN_TREE_SPACING);
    let density_ok = validation.nearby_tree_count < GUARDIAN_LOCAL_TREE_CAP;

    spacing_ok && density_ok
}

fn find_spawn_location(
    origin: &Position,
    spawn_radius: f32,
    tree_positions: &[Position],
    seed: u64,
) -> Option<Position> {
    for attempt in 0..GUARDIAN_SPAWN_ATTEMPTS {
        let angle_seed = mix_seed(seed ^ attempt.wrapping_mul(0xA24B_AED4_963E_E407));
        let distance_seed =
            mix_seed(seed ^ attempt.wrapping_mul(0x9FB2_1C65_1E98_DF25) ^ 0xD1B5_4A32_D192_ED03);
        let candidate = candidate_spawn_position(origin, spawn_radius, angle_seed, distance_seed);
        let validation = validate_spawn_location(&candidate, tree_positions);

        if is_spawn_location_valid(validation) {
            return Some(candidate);
        }
    }

    None
}

/// Spawns trees around entities with TreeSpawner component
pub fn update_tree_spawning(
    time: Res<Time>,
    mut world: ResMut<WorldManager>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut spawner_query: Query<(&Position, &mut TreeSpawner, Option<&ForestGuardian>)>,
    tree_query: Query<&Position, With<GrowingTree>>,
) {
    use std::collections::hash_map::RandomState;
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

            let existing_tree_positions: Vec<Position> = tree_query.iter().copied().collect();
            let Some(spawn_pos) = find_spawn_location(
                position,
                spawner.spawn_radius,
                &existing_tree_positions,
                hash,
            ) else {
                info!(
                    "Skipped tree spawn at ({:.1}, {:.1}); area is already dense",
                    position.x, position.y
                );
                reset_spawn_timer(&mut spawner, position, &hasher_builder);
                continue;
            };

            if !is_land_position(&mut world, &spawn_pos) {
                info!(
                    "Skipped tree spawn at ({:.1}, {:.1}); candidate landed in void",
                    spawn_pos.x, spawn_pos.y
                );
                reset_spawn_timer(&mut spawner, position, &hasher_builder);
                continue;
            }

            let mut hasher2 = hasher_builder.build_hasher();
            (hash.wrapping_add(100)).hash(&mut hasher2);
            let hash2 = hasher2.finish();

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

            if rand_type < 0.9 {
                // Spawn variant tree with matching tree type (90%)
                spawn_variant_tree(
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
                        "{:?} guardian spawned {:?} variant tree at ({:.1}, {:.1}) {}",
                        guardian.variant,
                        tree_variant,
                        spawn_pos.x,
                        spawn_pos.y,
                        if is_matching {
                            "(matching)"
                        } else {
                            "(different!)"
                        }
                    );
                } else {
                    info!(
                        "Entity spawned {:?} variant tree at ({:.1}, {:.1})",
                        tree_variant, spawn_pos.x, spawn_pos.y
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
                        spawn_pos.x,
                        spawn_pos.y,
                        if is_matching {
                            "(matching)"
                        } else {
                            "(different!)"
                        }
                    );
                } else {
                    info!(
                        "Entity spawned {:?} tree spirit at ({:.1}, {:.1})",
                        tree_variant, spawn_pos.x, spawn_pos.y
                    );
                }
            }

            reset_spawn_timer(&mut spawner, position, &hasher_builder);
        }
    }
}

fn reset_spawn_timer(
    spawner: &mut TreeSpawner,
    position: &Position,
    hasher_builder: &std::collections::hash_map::RandomState,
) {
    use std::hash::{BuildHasher, Hash, Hasher};

    let mut hasher_interval = hasher_builder.build_hasher();
    position.x.to_bits().hash(&mut hasher_interval);
    std::time::SystemTime::now().hash(&mut hasher_interval);
    let hash_interval = hasher_interval.finish();
    let rand_interval = (hash_interval as f32) / (u64::MAX as f32);
    spawner.spawn_timer = spawner.min_spawn_interval
        + rand_interval * (spawner.max_spawn_interval - spawner.min_spawn_interval);
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
