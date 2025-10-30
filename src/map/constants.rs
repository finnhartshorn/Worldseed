/// Map tile size in pixels (Minifantasy standard)
pub const MAP_TILE_SIZE: f32 = 8.0;

/// Map tile indices from Minifantasy_MapsLandAndSea.png (216×88 pixels = 27×11 tiles)
/// Organized in a grid layout

// Basic terrain tiles (row 0-1)
pub const MAP_TILE_GRASS_PLAIN: usize = 0;      // Bright grass
pub const MAP_TILE_GRASS_SPARSE: usize = 1;     // Light grass
pub const MAP_TILE_DIRT: usize = 2;             // Brown dirt
pub const MAP_TILE_SAND: usize = 3;             // Sandy terrain

// Water tiles (row 2-5)
pub const MAP_TILE_WATER_DEEP: usize = 54;      // Row 2, col 0 - Dark water
pub const MAP_TILE_WATER_SHALLOW: usize = 55;   // Lighter water

// Default tile for unknown/unloaded terrain
pub const MAP_TILE_UNKNOWN: usize = 54;         // Use deep water for unexplored

/// Tileset grid dimensions
pub const MAP_TILESET_COLS: usize = 27;
pub const MAP_TILESET_ROWS: usize = 11;
