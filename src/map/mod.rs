mod constants;
mod ui;
mod systems;

pub use constants::*;
pub use ui::*;
pub use systems::*;

use bevy::prelude::*;

/// Plugin for the world map system
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapConfig>()
            .init_resource::<MapState>()
            .add_systems(Startup, setup_map_ui)
            .add_systems(Update, (
                toggle_map_visibility,
                update_map_display,
            ));
    }
}

/// Configuration for map display
#[derive(Resource)]
pub struct MapConfig {
    /// How many game world chunks are represented by one map tile
    pub chunks_per_map_tile: u32,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            chunks_per_map_tile: 4, // Default: 4 chunks = 1 map tile
        }
    }
}

/// Current state of the map modal
#[derive(Resource, Default)]
pub struct MapState {
    pub visible: bool,
}

/// Marker component for the map modal root
#[derive(Component)]
pub struct MapModal;

/// Marker component for the map content container
#[derive(Component)]
pub struct MapContent;
