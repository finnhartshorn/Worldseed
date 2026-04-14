use super::{
    Direction, EntityBundle, EntityState, ForestGuardian, GuardianAnimations, GrowingTree,
    Player, Position, RenderStratum, RoamingBehavior, RtsTree, RtsTreeVariant, Snail, TreeSpawner,
    TreeSpirit, TreeVariant, VariantTree, WindingPath, WorldRenderDepth,
};
use bevy::prelude::*;
use bevy::sprite::Anchor;

const PLAYER_DEPTH_BIAS: f32 = 0.0004;
const FOREST_GUARDIAN_DEPTH_BIAS: f32 = 0.0003;
const SNAIL_DEPTH_BIAS: f32 = 0.0002;
const TREE_SPIRIT_DEPTH_BIAS: f32 = -0.0001;
const RTS_TREE_DEPTH_BIAS: f32 = -0.0002;
const VARIANT_TREE_DEPTH_BIAS: f32 = -0.0003;

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
    add_uniform_crop_layout(
        texture_atlas_layouts,
        4,
        4,
        URect::new(12, 11, 20, 19),
    )
}

pub fn guardian_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(
        texture_atlas_layouts,
        8,
        4,
        URect::new(8, 0, 24, 21),
    )
}

pub fn guardian_walk_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(
        texture_atlas_layouts,
        6,
        4,
        URect::new(8, 0, 24, 21),
    )
}

pub fn snail_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(
        texture_atlas_layouts,
        4,
        4,
        URect::new(0, 8, 32, 19),
    )
}

pub fn tree_spirit_texture_atlas_layout(
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Handle<TextureAtlasLayout> {
    add_uniform_crop_layout(
        texture_atlas_layouts,
        8,
        4,
        URect::new(12, 10, 20, 19),
    )
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

/// Spawns a player character at the given position
pub fn spawn_player(
    commands: &mut Commands,
    position: Position,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
) -> Entity {
    let texture = assets.load("characters/human_walk.png");
    let texture_atlas_layout = human_texture_atlas_layout(texture_atlas_layouts);

    commands
        .spawn((
            Player,
            EntityBundle::new(position.x, position.y, 100.0),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, PLAYER_DEPTH_BIAS),
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
            EntityBundle::new(position.x, position.y, 150.0),
            RoamingBehavior::new(position, 100.0, 15.0), // Roam within 100px at 15px/s
            TreeSpawner::default_guardian(),             // Spawn trees periodically
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, FOREST_GUARDIAN_DEPTH_BIAS),
            Sprite::from_atlas_image(
                idle_texture.clone(),
                TextureAtlas {
                    layout: idle_layout.clone(),
                    index: 0,
                },
            ),
            world_anchor(),
            Transform::from_xyz(position.x, position.y, 0.0),
            AnimationIndices::new(0, 7),    // First row, 8 frames (idle has 8 frames per direction)
            AnimationTimer::from_fps(4.0),  // Slower idle animation
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
            EntityBundle::new(position.x, position.y, 500.0),
            WindingPath::new(2.5), // Very slow winding movement at 2.5 px/s (8x slower)
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, SNAIL_DEPTH_BIAS),
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
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, TREE_SPIRIT_DEPTH_BIAS),
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

/// Spawns a variant tree from the Crafting And Professions pack that grows over time.
///
/// Variant trees come in 5 types matching TreeVariant: Oak, Birch, Hickory, Pine, Willow.
/// Each type has a unique size and appearance from the spritesheet.
/// Uses `Sprite::rect` to crop the appropriate variant from the source image.
///
/// `base_scale` is set to 2.0 for standard 4× pixel-art display scale at maturity.
pub fn spawn_variant_tree(
    commands: &mut Commands,
    position: Position,
    variant: TreeVariant,
    growth_time: f32,
    assets: &Res<AssetServer>,
) -> Entity {
    let texture = assets.load("props/trees/Minifantasy_CraftingAndProfessionsLogging.png");

    let rect = match variant {
        TreeVariant::Oak => Rect::new(13.0, 38.0, 34.0, 63.0),
        TreeVariant::Birch => Rect::new(177.0, 9.0, 207.0, 63.0),
        TreeVariant::Hickory => Rect::new(49.0, 32.0, 79.0, 63.0),
        TreeVariant::Pine => Rect::new(93.0, 24.0, 115.0, 63.0),
        TreeVariant::Willow => Rect::new(128.0, 21.0, 168.0, 63.0),
    };

    let growing_tree = GrowingTree::with_base_scale(variant, growth_time, 2.0);
    let initial_scale = growing_tree.current_scale();

    commands
        .spawn((
            VariantTree,
            growing_tree,
            Position::new(position.x, position.y),
            WorldRenderDepth::with_bias(RenderStratum::WorldObject, VARIANT_TREE_DEPTH_BIAS),
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
