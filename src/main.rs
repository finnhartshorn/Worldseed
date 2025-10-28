use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    sprite_render::{TileData, TilemapChunk, TilemapChunkTileData},
};

mod tiles;
mod world;

use world::{loader, WorldManager};

// UI sprite vertical offsets for proper centering
const HUMAN_SPRITE_OFFSET: f32 = 1.0;
const FOREST_GUARDIAN_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET_X: f32 = 10.0;

// Camera zoom configuration
const ZOOM_MIN: f32 = 0.5;  // Max zoom in (smaller = more zoomed in)
const ZOOM_MAX: f32 = 3.0;  // Max zoom out (larger = more zoomed out)
const ZOOM_SPEED: f32 = 0.1; // Zoom change per input

// Animation components
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

// Marker components for creatures
#[derive(Component)]
struct ForestGuardian;

#[derive(Component)]
struct Snail;

// UI marker components
#[derive(Component)]
struct GuardianSubmenu;

#[derive(Component)]
struct GuardianButton;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_resource::<WorldManager>()
        .add_systems(Startup, (setup_world, setup_ui))
        .add_systems(
            Update,
            (
                update_tileset_image,
                animate_sprite,
                move_camera,
                zoom_camera,
                loader::update_camera_chunk,
                loader::load_chunks_around_camera.after(loader::update_camera_chunk),
                loader::unload_distant_chunks.after(loader::load_chunks_around_camera),
            ),
        )
        .run();
}

fn setup_world(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Spawn camera at origin
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 999.0)));

    // Load character sprite sheet
    let texture = assets.load("characters/human_walk.png");

    // Create texture atlas layout: 4 columns x 4 rows, each sprite is 32x32 pixels
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    // Animation indices for the first row (frames 0-3)
    let animation_indices = AnimationIndices { first: 0, last: 3 };

    // Spawn animated character sprite
    commands.spawn((
        Sprite::from_atlas_image(
            texture,
            TextureAtlas {
                layout: texture_atlas_layout,
                index: animation_indices.first,
            },
        ),
        Transform::from_xyz(0.0, 0.0, 1.0), // Position at center, above tilemap
        animation_indices,
        // AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        AnimationTimer(Timer::from_seconds(0.2, TimerMode::Repeating)),
    ));

    // Load Forest Guardian texture
    let guardian_texture = assets.load("creatures/forest_guardians/oak_guardian_idle.png");
    let guardian_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
    let guardian_atlas_layout = texture_atlas_layouts.add(guardian_layout);
    let guardian_animation = AnimationIndices { first: 0, last: 7 }; // First row (direction)

    // Spawn Forest Guardian
    commands.spawn((
        Sprite::from_atlas_image(
            guardian_texture,
            TextureAtlas {
                layout: guardian_atlas_layout,
                index: guardian_animation.first,
            },
        ),
        Transform::from_xyz(-100.0, 0.0, 1.0),
        guardian_animation,
        AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
        ForestGuardian,
    ));

    // Load Snail texture
    let snail_texture = assets.load("creatures/snail/snail_crawl.png");
    let snail_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
    let snail_atlas_layout = texture_atlas_layouts.add(snail_layout);
    let snail_animation = AnimationIndices { first: 0, last: 3 }; // First row (direction)

    // Spawn Snail
    commands.spawn((
        Sprite::from_atlas_image(
            snail_texture,
            TextureAtlas {
                layout: snail_atlas_layout,
                index: snail_animation.first,
            },
        ),
        Transform::from_xyz(100.0, 0.0, 1.0),
        snail_animation,
        AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
        Snail,
    ));

    info!("Tilemap, characters, and creatures setup complete");
}

fn update_tileset_image(
    chunk_query: Query<&TilemapChunk>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in events.read() {
        // Check if any chunk uses this texture
        for chunk in chunk_query.iter() {
            if event.is_loaded_with_dependencies(chunk.tileset.id()) {
                if let Some(image) = images.get_mut(&chunk.tileset) {
                    // Reinterpret the vertically-stacked texture as an array texture with 2 layers
                    // terrain_array.png is 8x16 (two 8x8 tiles stacked)
                    image.reinterpret_stacked_2d_as_array(2);
                    info!("Tileset reinterpreted as 2-layer array texture");
                }
                break; // Only need to reinterpret once per texture
            }
        }
    }
}

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = if atlas.index == indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}

/// Camera movement system for testing chunk loading
fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    if let Ok(mut transform) = camera_query.single_mut() {
        let speed = 200.0; // pixels per second
        let delta = time.delta_secs();

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            transform.translation.y += speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            transform.translation.y -= speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            transform.translation.x += speed * delta;
        }
    }
}

/// Camera zoom system - supports scroll wheel and keyboard (- and = keys)
fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if let Ok(mut projection) = camera_query.single_mut() {
        let mut zoom_delta = 0.0;

        // Handle scroll wheel input
        for event in scroll_events.read() {
            zoom_delta -= event.y * ZOOM_SPEED;
        }

        // Handle keyboard input (- to zoom out, = to zoom in)
        if keyboard.just_pressed(KeyCode::Minus) {
            zoom_delta += ZOOM_SPEED;
        }
        if keyboard.just_pressed(KeyCode::Equal) {
            zoom_delta -= ZOOM_SPEED;
        }

        // Apply zoom delta and clamp to bounds
        if zoom_delta != 0.0 {
            if let Projection::Orthographic(ref mut ortho) = projection.as_mut() {
                ortho.scale = (ortho.scale + zoom_delta).clamp(ZOOM_MIN, ZOOM_MAX);
            }
        }
    }
}

fn setup_ui(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Root UI container on the left side
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Start,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|parent| {
            // Load textures for UI buttons
            let guardian_texture = assets.load("creatures/forest_guardians/oak_guardian_idle.png");
            let guardian_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
            let guardian_atlas_layout = texture_atlas_layouts.add(guardian_layout);

            let human_texture = assets.load("characters/human_walk.png");
            let human_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
            let human_atlas_layout = texture_atlas_layouts.add(human_layout);

            let snail_texture = assets.load("creatures/snail/snail_crawl.png");
            let snail_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
            let snail_atlas_layout = texture_atlas_layouts.add(snail_layout);

            // Button 1 - With Human sprite
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.6)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .observe(button_interaction)
                .with_children(|button| {
                    button.spawn((
                        ImageNode {
                            image: human_texture.clone(),
                            image_mode: NodeImageMode::Stretch,
                            texture_atlas: Some(TextureAtlas {
                                layout: human_atlas_layout.clone(),
                                index: 0,
                            }),
                            ..default()
                        },
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            margin: UiRect {
                                top: Val::Px(HUMAN_SPRITE_OFFSET),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });

            // Button 2 - With Forest Guardian sprite (with submenu row)
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    // Main guardian button
                    row.spawn((
                        Button,
                        GuardianButton,
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(0.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                        BorderRadius::all(Val::Px(4.0)),
                    ))
                    .observe(button_interaction)
                    .observe(guardian_button_right_click)
                    .with_children(|button| {
                        // Add guardian sprite directly
                        button.spawn((
                            ImageNode {
                                image: guardian_texture.clone(),
                                image_mode: NodeImageMode::Stretch,
                                texture_atlas: Some(TextureAtlas {
                                    layout: guardian_atlas_layout.clone(),
                                    index: 0, // First frame
                                }),
                                ..default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect {
                                    top: Val::Px(FOREST_GUARDIAN_SPRITE_OFFSET),
                                    ..default()
                                },
                                ..default()
                            },
                        ));
                    });

                    // Submenu container (initially hidden)
                    let guardian_layout_submenu =
                        TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
                    let guardians = [
                        ("Oak", "oak_guardian_idle.png"),
                        ("Birch", "birch_guardian_idle.png"),
                        ("Hickory", "hickory_guardian_idle.png"),
                        ("Pine", "pine_guardian_idle.png"),
                        ("Willow", "willow_guardian_idle.png"),
                    ];

                    row.spawn((
                        GuardianSubmenu,
                        Node {
                            display: Display::None, // Hidden by default
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                    ))
                    .with_children(|submenu| {
                        for (_name, filename) in guardians.iter() {
                            let texture =
                                assets.load(format!("creatures/forest_guardians/{}", filename));
                            let layout = texture_atlas_layouts.add(guardian_layout_submenu.clone());

                            submenu
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        display: Display::Flex,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::all(Val::Px(0.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                                    BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                                    BorderRadius::all(Val::Px(4.0)),
                                ))
                                .observe(button_interaction)
                                .with_children(|button| {
                                    button.spawn((
                                        ImageNode {
                                            image: texture.clone(),
                                            image_mode: NodeImageMode::Stretch,
                                            texture_atlas: Some(TextureAtlas {
                                                layout: layout.clone(),
                                                index: 0,
                                            }),
                                            ..default()
                                        },
                                        Node {
                                            width: Val::Px(64.0),
                                            height: Val::Px(64.0),
                                            margin: UiRect {
                                                top: Val::Px(FOREST_GUARDIAN_SPRITE_OFFSET),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                        }
                    });
                });

            // Button 3 - With Snail sprite
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.2, 0.25)),
                    BorderColor::all(Color::srgb(0.5, 0.4, 0.5)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .observe(button_interaction)
                .with_children(|button| {
                    button.spawn((
                        ImageNode {
                            image: snail_texture.clone(),
                            image_mode: NodeImageMode::Stretch,
                            texture_atlas: Some(TextureAtlas {
                                layout: snail_atlas_layout.clone(),
                                index: 0,
                            }),
                            ..default()
                        },
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            margin: UiRect {
                                top: Val::Px(SNAIL_SPRITE_OFFSET),
                                left: Val::Px(SNAIL_SPRITE_OFFSET_X),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });

            // Button 4 - Empty
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .observe(button_interaction);
        });
}

fn button_interaction(
    trigger: On<Pointer<Click>>,
    mut buttons: Query<&mut BackgroundColor, With<Button>>,
) {
    if let Ok(mut bg_color) = buttons.get_mut(trigger.entity) {
        // Toggle button color on click
        if bg_color.0 == Color::srgb(0.2, 0.2, 0.2) {
            *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
        } else if bg_color.0 == Color::srgb(0.2, 0.2, 0.3) {
            *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.4));
        } else if bg_color.0 == Color::srgb(0.15, 0.3, 0.15) {
            *bg_color = BackgroundColor(Color::srgb(0.2, 0.4, 0.2));
        } else if bg_color.0 == Color::srgb(0.25, 0.2, 0.25) {
            *bg_color = BackgroundColor(Color::srgb(0.35, 0.3, 0.35));
        } else {
            *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
        }
    }
}

fn guardian_button_right_click(
    _trigger: On<Pointer<Click>>,
    mut submenu_query: Query<&mut Node, With<GuardianSubmenu>>,
) {
    // Toggle submenu visibility
    if let Ok(mut node) = submenu_query.single_mut() {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
}
