use bevy::prelude::*;
use super::{EntityBundle, Position, Player, ForestGuardian, Snail, Direction, WindingPath};

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
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands
        .spawn((
            Player,
            EntityBundle::new(position.x, position.y, 100.0),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            Transform::from_xyz(position.x, position.y, 1.0),
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
    let texture = assets.load(format!("creatures/forest_guardians/{}_guardian_idle.png", variant));
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands
        .spawn((
            ForestGuardian,
            EntityBundle::new(position.x, position.y, 150.0),
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            Transform::from_xyz(position.x, position.y, 1.0),
            AnimationIndices::new(0, 7), // First row, 8 frames
            AnimationTimer::from_fps(6.67), // ~0.15s per frame
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
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands
        .spawn((
            Snail,
            EntityBundle::new(position.x, position.y, 50.0),
            WindingPath::new(20.0), // Slow winding movement at 20 px/s
            Sprite::from_atlas_image(
                texture,
                TextureAtlas {
                    layout: texture_atlas_layout,
                    index: 0,
                },
            ),
            Transform::from_xyz(position.x, position.y, 1.0),
            AnimationIndices::new(0, 3), // First row, 4 frames
            AnimationTimer::from_fps(6.67), // ~0.15s per frame
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
