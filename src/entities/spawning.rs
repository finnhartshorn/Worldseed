use super::{
    Direction, EntityBundle, EntityState, ForestGuardian, GrowingTree, GuardianAnimations, Health,
    Human, Position, RenderStratum, RoamingBehavior, RtsTree, RtsTreeVariant, Snail, TreeSpawner,
    TreeSpirit, TreeVariant, VariantTree, VariantTreeAppearance, Velocity, WindingPath,
    WorldClickableEntity, WorldRenderDepth,
};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};

// Keep world-object biases below one y-sort step so vertical position, not bias,
// determines draw order except at effectively identical foot positions.
pub(crate) const PLAYER_DEPTH_BIAS: f32 = 0.000004;
pub(crate) const FOREST_GUARDIAN_DEPTH_BIAS: f32 = 0.000003;
pub(crate) const SNAIL_DEPTH_BIAS: f32 = 0.000002;
pub(crate) const TREE_SPIRIT_DEPTH_BIAS: f32 = -0.000001;
pub(crate) const RTS_TREE_DEPTH_BIAS: f32 = -0.000002;
pub(crate) const VARIANT_TREE_DEPTH_BIAS: f32 = -0.000003;

fn world_anchor() -> Anchor {
    Anchor::BOTTOM_CENTER
}

fn add_uniform_crop_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    columns: u32,
    rows: u32,
    crop: URect,
) -> Handle<TextureAtlasLayout> {
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(columns * 32, rows * 32));

    for row in 0..rows {
        for col in 0..columns {
            let origin = UVec2::new(col * 32, row * 32);
            layout.add_texture(URect {
                min: origin + crop.min,
                max: origin + crop.max,
            });
        }
    }

    texture_atlas_layouts.add(layout)
}

pub fn human_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(texture_atlas_layouts, 4, 4, URect::new(12, 11, 20, 19))
}

pub fn guardian_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(texture_atlas_layouts, 8, 4, URect::new(8, 0, 24, 21))
}

pub fn guardian_walk_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(texture_atlas_layouts, 6, 4, URect::new(8, 0, 24, 21))
}

pub fn snail_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(texture_atlas_layouts, 4, 4, URect::new(0, 8, 32, 19))
}

pub fn tree_spirit_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(texture_atlas_layouts, 8, 4, URect::new(12, 10, 20, 19))
}

pub fn variant_tree_growth_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    frame_size: UVec2,
) -> Handle<TextureAtlasLayout> {
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(frame_size.x * 4, frame_size.y));

    for col in 0..4 {
        let origin = UVec2::new(col * frame_size.x, 0);
        layout.add_texture(URect {
            min: origin,
            max: origin + frame_size,
        });
    }

    texture_atlas_layouts.add(layout)
}

pub fn variant_tree_shared_variation_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(192, 240));
    let rects = [
        URect::new(13, 38, 34, 63),
        URect::new(49, 32, 79, 63),
        URect::new(88, 21, 128, 63),
        URect::new(137, 9, 167, 63),
        URect::new(14, 78, 34, 103),
        URect::new(50, 64, 78, 103),
        URect::new(96, 74, 121, 103),
        URect::new(137, 68, 167, 103),
        URect::new(11, 145, 35, 167),
        URect::new(48, 129, 78, 167),
        URect::new(93, 122, 124, 167),
        URect::new(137, 106, 183, 167),
        URect::new(10, 202, 37, 231),
        URect::new(49, 184, 79, 231),
        URect::new(91, 186, 125, 231),
        URect::new(137, 170, 183, 231),
    ];

    for rect in rects {
        layout.add_texture(rect);
    }

    texture_atlas_layouts.add(layout)
}

pub fn choose_variant_tree_appearance(
    variant: TreeVariant,
    position: Position,
) -> VariantTreeAppearance {
    let hasher_builder = RandomState::new();
    let mut hasher = hasher_builder.build_hasher();
    variant.as_str().hash(&mut hasher);
    position.x.to_bits().hash(&mut hasher);
    position.y.to_bits().hash(&mut hasher);
    std::time::SystemTime::now().hash(&mut hasher);

    let rand_val = (hasher.finish() as f32) / (u64::MAX as f32);
    variant.choose_appearance(rand_val)
}

/// Animation components
#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

impl AnimationIndices {
    pub fn new(first: usize, last: usize) -> Self {
        Self { first, last }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

impl AnimationTimer {
    pub fn from_fps(fps: f32) -> Self {
        Self(Timer::from_seconds(1.0 / fps, TimerMode::Repeating))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SavedActorState {
    pub position: Position,
    pub velocity: Velocity,
    pub direction: Direction,
    pub state: EntityState,
    pub health: Health,
}

fn entity_bundle_from_state(state: SavedActorState) -> EntityBundle {
    EntityBundle {
        position: state.position,
        velocity: state.velocity,
        direction: state.direction,
        state: state.state,
        health: state.health,
    }
}

fn atlas_index_for_direction(direction: Direction, frames_per_direction: usize) -> usize {
    direction.sprite_row() * frames_per_direction
}

/// Spawns a human unit at the given position
pub fn spawn_human(
    commands: &mut Commands,
    position: Position,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load("characters/human_walk.png");
    let texture_atlas_layout = human_texture_atlas_layout(texture_atlas_layouts);

    commands
        .spawn((
            Human,
            WorldClickableEntity,
            EntityBundle::new(position.x, position.y, 100.0),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, PLAYER_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0),
            AnimationIndices::new(0, 3), // First row, 4 frames
            AnimationTimer::from_fps(5.0),
        ))
        .id()
}

pub fn spawn_saved_human(
    commands: &mut Commands,
    actor: SavedActorState,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load("characters/human_walk.png");
    let texture_atlas_layout = human_texture_atlas_layout(texture_atlas_layouts);
    let atlas_index = atlas_index_for_direction(actor.direction, 4);

    commands
        .spawn((
            Human,
            WorldClickableEntity,
            entity_bundle_from_state(actor),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, PLAYER_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: atlas_index,
                },
            ),
            world_anchor(),
            Transform::from_xyz(actor.position.x, actor.position.y, 0.0),
            AnimationIndices::new(atlas_index, atlas_index + 3),
            AnimationTimer::from_fps(5.0),
        ))
        .id()
}

/// Spawns a forest guardian at the given position
pub fn spawn_forest_guardian(
    commands: &mut Commands,
    position: Position,
    variant: &str, // "oak", "birch", "hickory", "pine", "willow"
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    // Load both idle and walk textures
    let idle_texture = assets.load(format!(
        "creatures/forest_guardians/{}_guardian_idle.png",
        variant
    ));
    let walk_texture = assets.load(format!(
        "creatures/forest_guardians/{}_guardian_walk.png",
        variant
    ));

    // Create texture atlas layouts for both animations
    let idle_layout = guardian_texture_atlas_layout(texture_atlas_layouts);
    let walk_layout = guardian_walk_texture_atlas_layout(texture_atlas_layouts);

    let tree_variant = TreeVariant::from_str(variant);

    commands
        .spawn((
            ForestGuardian::new(tree_variant),
            WorldClickableEntity,
            EntityBundle::new(position.x, position.y, 150.0),
            RoamingBehavior::new(position, 100.0, 15.0), // Roam within 100px at 15px/s
            TreeSpawner::default_guardian(),             // Spawn trees periodically
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, FOREST_GUARDIAN_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                idle_texture.clone(),
                TextureAtlas {
                    layout: idle_layout.clone(),
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0),
            AnimationIndices::new(0, 7), // First row, 8 frames (idle has 8 frames per direction)
            AnimationTimer::from_fps(4.0), // Slower idle animation
            GuardianAnimations {
                idle_texture,
                idle_layout,
                walk_texture,
                walk_layout,
                idle_frames: 8,
                walk_frames: 6,
                current_state: EntityState::Idle,
            },
        ))
        .id()
}

pub fn spawn_saved_forest_guardian(
    commands: &mut Commands,
    actor: SavedActorState,
    guardian: ForestGuardian,
    roaming: RoamingBehavior,
    tree_spawner: TreeSpawner,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let variant = guardian.variant.as_str();
    let idle_texture = assets.load(format!(
        "creatures/forest_guardians/{}_guardian_idle.png",
        variant
    ));
    let walk_texture = assets.load(format!(
        "creatures/forest_guardians/{}_guardian_walk.png",
        variant
    ));
    let idle_layout = guardian_texture_atlas_layout(texture_atlas_layouts);
    let walk_layout = guardian_walk_texture_atlas_layout(texture_atlas_layouts);
    let (texture, layout, frames_per_direction) = match actor.state {
        EntityState::Moving => (walk_texture.clone(), walk_layout.clone(), 6),
        _ => (idle_texture.clone(), idle_layout.clone(), 8),
    };
    let atlas_index = atlas_index_for_direction(actor.direction, frames_per_direction);

    commands
        .spawn((
            guardian,
            WorldClickableEntity,
            entity_bundle_from_state(actor),
            roaming,
            tree_spawner,
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, FOREST_GUARDIAN_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout,
                    index: atlas_index,
                },
            ),
            world_anchor(),
            Transform::from_xyz(actor.position.x, actor.position.y, 0.0),
            AnimationIndices::new(atlas_index, atlas_index + frames_per_direction - 1),
            AnimationTimer::from_fps(4.0),
            GuardianAnimations {
                idle_texture,
                idle_layout,
                walk_texture,
                walk_layout,
                idle_frames: 8,
                walk_frames: 6,
                current_state: actor.state,
            },
        ))
        .id()
}

/// Spawns a snail at the given position
pub fn spawn_snail(
    commands: &mut Commands,
    position: Position,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load("creatures/snail/snail_crawl.png");
    let texture_atlas_layout = snail_texture_atlas_layout(texture_atlas_layouts);

    commands
        .spawn((
            Snail,
            WorldClickableEntity,
            EntityBundle::new(position.x, position.y, 500.0),
            WindingPath::new(2.5), // Very slow winding movement at 2.5 px/s (8x slower)
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, SNAIL_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0).with_scale(Vec3::splat(4.0)), // 4x bigger
            AnimationIndices::new(0, 3),   // First row, 4 frames
            AnimationTimer::from_fps(2.0), // Slower animation at 2 FPS (~0.5s per frame)
        ))
        .id()
}

pub fn spawn_saved_snail(
    commands: &mut Commands,
    actor: SavedActorState,
    winding_path: WindingPath,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load("creatures/snail/snail_crawl.png");
    let texture_atlas_layout = snail_texture_atlas_layout(texture_atlas_layouts);
    let atlas_index = atlas_index_for_direction(actor.direction, 4);

    commands
        .spawn((
            Snail,
            WorldClickableEntity,
            entity_bundle_from_state(actor),
            winding_path,
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, SNAIL_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: atlas_index,
                },
            ),
            world_anchor(),
            Transform::from_xyz(actor.position.x, actor.position.y, 0.0)
                .with_scale(Vec3::splat(4.0)),
            AnimationIndices::new(atlas_index, atlas_index + 3),
            AnimationTimer::from_fps(2.0),
        ))
        .id()
}

/// Spawns a tree spirit that grows over time
pub fn spawn_tree_spirit(
    commands: &mut Commands,
    position: Position,
    variant: TreeVariant,
    growth_time: f32, // Time in seconds for each growth stage
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    // Start with the idle animation sprite (we'll use this for all growth stages with scaling)
    let texture = assets.load(format!(
        "creatures/tree_spirits/{}_spirit_idle.png",
        variant.as_str()
    ));

    // Tree spirit sprite sheets are 4 rows (directions) with multiple frames per row
    // We'll use idle animation (assume similar to guardians: multiple frames per direction)
    let texture_atlas_layout = tree_spirit_texture_atlas_layout(texture_atlas_layouts);

    let growing_tree = GrowingTree::with_growth_time(variant, growth_time);
    let initial_scale = growing_tree.stage.scale();

    commands
        .spawn((
            TreeSpirit,
            WorldClickableEntity,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, TREE_SPIRIT_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0).with_scale(Vec3::splat(initial_scale)),
            AnimationIndices::new(0, 7), // First row, 8 frames (assuming same as guardians)
            AnimationTimer::from_fps(4.0), // Slow idle animation
        ))
        .id()
}

pub fn spawn_saved_tree_spirit(
    commands: &mut Commands,
    position: Position,
    growing_tree: GrowingTree,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load(format!(
        "creatures/tree_spirits/{}_spirit_idle.png",
        growing_tree.variant.as_str()
    ));
    let texture_atlas_layout = tree_spirit_texture_atlas_layout(texture_atlas_layouts);

    commands
        .spawn((
            TreeSpirit,
            WorldClickableEntity,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, TREE_SPIRIT_DEPTH_BIAS),
            Pickable::default(),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0)
                .with_scale(Vec3::splat(growing_tree.current_scale())),
            AnimationIndices::new(0, 7),
            AnimationTimer::from_fps(4.0),
        ))
        .id()
}

/// Spawns an RTS Humans tree prop that grows from a seed to full size over time.
///
/// The two tree variants come from `assets/props/rts_humans/Tileset_And_Props.png`
/// and are cropped via `Sprite::rect` because they have different sizes:
/// - Large: 22×26 px source region (x 21–42, y 18–43)
/// - Small: 15×21 px source region (x 53–67, y 20–40)
///
/// `base_scale` is set to 2.0 so that a MatureTree (stage scale 2.0 × base 2.0 = 4.0)
/// renders at the standard 4× pixel-art display scale, matching other game entities.
pub fn spawn_rts_tree(
    commands: &mut Commands,
    position: Position,
    variant: RtsTreeVariant,
    growth_time: f32,
    assets: &Res<AssetServer>,
) -> Entity {
    let texture = assets.load("props/rts_humans/Tileset_And_Props.png");

    let rect = match variant {
        RtsTreeVariant::Large => Rect::new(21.0, 18.0, 42.0, 43.0),
        RtsTreeVariant::Small => Rect::new(53.0, 20.0, 67.0, 40.0),
    };

    // base_scale 2.0: MatureTree stage (2.0) × base (2.0) = 4.0 final scale
    let growing_tree = GrowingTree::with_base_scale(TreeVariant::Oak, growth_time, 2.0);
    let initial_scale = growing_tree.current_scale(); // 2.0 × 0.5 = 1.0

    commands
        .spawn((
            RtsTree,
            variant,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, RTS_TREE_DEPTH_BIAS),
            Sprite {
                image: texture,
                rect: Some(rect),
                ..default()
            },
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0).with_scale(Vec3::splat(initial_scale)),
        ))
        .id()
}

/// Spawns a variant tree from a per-variant four-stage growth sheet.
///
/// Variant trees come in 5 types matching TreeVariant: Oak, Birch, Hickory, Pine, Willow.
/// Each type starts on frame 0 (Seed) and advances to frame 3 (MatureTree).
pub fn spawn_variant_tree(
    commands: &mut Commands,
    position: Position,
    variant: TreeVariant,
    growth_time: f32,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let early_texture = assets.load(variant.growth_stage_asset_path());
    let early_texture_atlas_layout = variant_tree_growth_texture_atlas_layout(
        texture_atlas_layouts,
        variant.growth_stage_frame_size(),
    );
    let appearance = choose_variant_tree_appearance(variant, position);
    let initial_scale = variant.variant_tree_display_scale();
    let growing_tree =
        GrowingTree::with_variant_appearance(variant, growth_time, initial_scale, appearance);

    let sprite = if appearance.uses_shared_mature_sheet() {
        Sprite::from_atlas_image(
            early_texture,
            TextureAtlas {
                layout: early_texture_atlas_layout,
                index: 0,
            },
        )
    } else {
        Sprite::from_atlas_image(
            early_texture,
            TextureAtlas {
                layout: early_texture_atlas_layout,
                index: 0,
            },
        )
    };

    commands
        .spawn((
            VariantTree,
            WorldClickableEntity,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, VARIANT_TREE_DEPTH_BIAS),
            Pickable::default(),
            sprite,
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0).with_scale(Vec3::splat(initial_scale)),
        ))
        .id()
}

pub fn spawn_saved_variant_tree(
    commands: &mut Commands,
    position: Position,
    growing_tree: GrowingTree,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let sprite = sprite_for_saved_variant_tree(&growing_tree, assets, texture_atlas_layouts);
    let scale = growing_tree.current_variant_tree_scale();

    commands
        .spawn((
            VariantTree,
            WorldClickableEntity,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, VARIANT_TREE_DEPTH_BIAS),
            Pickable::default(),
            sprite,
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0).with_scale(Vec3::splat(scale)),
        ))
        .id()
}

fn sprite_for_saved_variant_tree(
    growing_tree: &GrowingTree,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Sprite {
    if growing_tree.appearance.uses_shared_mature_sheet()
        && matches!(
            growing_tree.stage,
            super::GrowthStage::YoungTree | super::GrowthStage::MatureTree
        )
    {
        let texture = assets.load(
            growing_tree
                .variant
                .shared_variation_sheet_path()
                .expect("saved shared appearance should have a backing sheet"),
        );
        let layout = variant_tree_shared_variation_texture_atlas_layout(texture_atlas_layouts);
        let index = (growing_tree.appearance.mature_row() * 4
            + growing_tree
                .variant
                .shared_variation_column()
                .expect("saved shared appearance should have a valid column"))
            as usize;

        Sprite::from_atlas_image(texture, TextureAtlas { layout, index })
    } else {
        let texture = assets.load(growing_tree.variant.growth_stage_asset_path());
        let layout = variant_tree_growth_texture_atlas_layout(
            texture_atlas_layouts,
            growing_tree.variant.growth_stage_frame_size(),
        );
        let index = growing_tree.stage.frame_index();

        Sprite::from_atlas_image(texture, TextureAtlas { layout, index })
    }
}

/// Updates animation indices based on entity direction and state
pub fn update_animation_for_direction(
    direction: Direction,
    indices: &mut AnimationIndices,
    frames_per_direction: usize,
) {
    let row = direction.sprite_row();
    indices.first = row * frames_per_direction;
    indices.last = indices.first + frames_per_direction - 1;
}
