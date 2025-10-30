use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    sprite_render::TilemapChunk,
};

mod entities;
mod tiles;
mod world;

use entities::{
    animate_sprite, apply_velocity, snail_dirt_trail, spawn_forest_guardian, spawn_player,
    spawn_snail, spawn_tree_spirit, sync_position_with_transform, update_animation_from_direction,
    update_direction_from_velocity, update_roaming_behavior, update_state_from_velocity,
    update_tree_growth, update_winding_path, Position, TreeVariant,
};
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
                // Asset and rendering updates
                update_tileset_image,
                // AI behaviors (before velocity application)
                update_roaming_behavior,
                update_winding_path,
                // Entity state updates
                apply_velocity,
                update_state_from_velocity,
                update_direction_from_velocity,
                update_animation_from_direction,
                sync_position_with_transform.after(apply_velocity),
                // Entity interactions with world
                snail_dirt_trail.after(sync_position_with_transform),
                // Tree growth
                update_tree_growth,
                // Animation
                animate_sprite,
                // Camera controls
                move_camera,
                zoom_camera,
                // World management
                loader::update_camera_chunk,
                loader::load_chunks_around_camera.after(loader::update_camera_chunk),
                loader::unload_distant_chunks.after(loader::load_chunks_around_camera),
                loader::apply_tile_modifications.after(snail_dirt_trail),
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

    // Spawn player character at world origin
    spawn_player(
        &mut commands,
        Position::new(0.0, 0.0),
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn forest guardian to the left
    spawn_forest_guardian(
        &mut commands,
        Position::new(-100.0, 0.0),
        "oak",
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn snail to the right
    spawn_snail(
        &mut commands,
        Position::new(100.0, 0.0),
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn a test tree spirit above the player - grows every 3 seconds per stage
    spawn_tree_spirit(
        &mut commands,
        Position::new(0.0, 100.0),
        TreeVariant::Oak,
        3.0, // 3 seconds per growth stage
        &assets,
        &mut texture_atlas_layouts,
    );

    info!("World setup complete with entities using position and state components");
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
