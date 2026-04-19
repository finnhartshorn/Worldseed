mod constants;
mod systems;
mod ui;

pub use constants::*;
pub use systems::*;
pub use ui::*;

use crate::AppState;
use bevy::prelude::*;

/// Plugin for the world map system
pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapConfig>()
            .init_resource::<MapState>()
            .add_systems(OnEnter(AppState::InGame), (reset_map_state, setup_map_ui))
            .add_systems(OnExit(AppState::InGame), cleanup_map_ui)
            .add_systems(
                Update,
                (
                    toggle_map_visibility,
                    cycle_map_resolution,
                    update_map_display,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Configuration for map display
#[derive(Resource)]
pub struct MapConfig {
    /// Supported sample kernel widths in world tiles for one minimap pixel.
    pub sample_sizes_in_tiles: Vec<u32>,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            sample_sizes_in_tiles: vec![1, 2, 3, 4],
        }
    }
}

/// Current state of the map modal.
#[derive(Resource)]
pub struct MapState {
    pub visible: bool,
    pub active_sample_size_index: usize,
}

impl Default for MapState {
    fn default() -> Self {
        Self {
            visible: false,
            active_sample_size_index: 1,
        }
    }
}

impl MapState {
    pub fn active_sample_size(&self, config: &MapConfig) -> u32 {
        config
            .sample_sizes_in_tiles
            .get(self.active_sample_size_index)
            .copied()
            .unwrap_or(1)
    }
}

/// Marker component for the map modal root
#[derive(Component)]
pub struct MapModal;

/// Marker component for the map content container
#[derive(Component)]
pub struct MapContent;

/// Marker component for the dynamic minimap contents that are rebuilt on refresh.
#[derive(Component)]
pub struct MapDynamicContent;
