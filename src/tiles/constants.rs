/// Size of each chunk in tiles (width and height)
pub const CHUNK_SIZE: usize = 32;

/// Total number of tiles in a chunk
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE; // 1,024 tiles

/// Size of each chunk as i32 for coordinate calculations
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;

/// Pixel size of each tile (Minifantasy uses 8x8 pixels)
pub const TILE_SIZE: f32 = 8.0;

/// Pixel size of each chunk
pub const CHUNK_PIXEL_SIZE: f32 = TILE_SIZE * CHUNK_SIZE as f32; // 256 pixels

/// Display size for tiles in the tilemap (we'll scale 8x8 to 32x32 for visibility)
pub const TILE_DISPLAY_SIZE: u32 = 32;

// Layer configuration
/// Number of tile layers per chunk
pub const NUM_LAYERS: usize = 3;

/// Layer indices
pub const LAYER_GROUND: usize = 0;      // Base terrain layer
pub const LAYER_DECORATION: usize = 1;  // Decorative tiles (flowers, rocks, etc.)
pub const LAYER_OVERLAY: usize = 2;     // Top layer (effects, particles, etc.)

/// Z-positions for each layer in world space
pub const LAYER_Z_GROUND: f32 = 0.0;
pub const LAYER_Z_DECORATION: f32 = 0.1;
pub const LAYER_Z_OVERLAY: f32 = 0.2;

/// Helper to get Z position for a layer index
pub const fn layer_z_position(layer: usize) -> f32 {
    match layer {
        LAYER_GROUND => LAYER_Z_GROUND,
        LAYER_DECORATION => LAYER_Z_DECORATION,
        LAYER_OVERLAY => LAYER_Z_OVERLAY,
        _ => LAYER_Z_GROUND,
    }
}

// Tile type constants
/// Empty/air tile
pub const TILE_EMPTY: u16 = 0;

/// Grass tile
pub const TILE_GRASS: u16 = 1;

/// Dirt tile
pub const TILE_DIRT: u16 = 2;

/// Fog of war - solid black
pub const TILE_FOG_BLACK: u16 = 3;

/// Shadow tiles for fog edges (from terrain.png grid positions)
pub const TILE_SHADOW_LIGHT_1: u16 = 4;  // Light gray shadow
pub const TILE_SHADOW_LIGHT_2: u16 = 5;  // Another light variant
pub const TILE_SHADOW_MEDIUM_1: u16 = 6; // Medium gray shadow
pub const TILE_SHADOW_MEDIUM_2: u16 = 7; // Another medium variant
pub const TILE_SHADOW_DARK_1: u16 = 8;   // Dark gray shadow
pub const TILE_SHADOW_DARK_2: u16 = 9;   // Another dark variant

/// Array of all shadow tiles for easy iteration
pub const SHADOW_TILES: [u16; 6] = [
    TILE_SHADOW_LIGHT_1,
    TILE_SHADOW_LIGHT_2,
    TILE_SHADOW_MEDIUM_1,
    TILE_SHADOW_MEDIUM_2,
    TILE_SHADOW_DARK_1,
    TILE_SHADOW_DARK_2,
];

/// Maximum number of tile types (u16 can hold 0-65535)
pub const MAX_TILE_TYPES: usize = u16::MAX as usize + 1;

// Chunk loading/unloading radii
/// Radius of chunks to load around the camera (7x7 = 49 chunks)
pub const CHUNK_LOAD_RADIUS: i32 = 3; // 3 chunks in each direction = 7x7 grid

/// Radius of chunks to keep loaded (11x11 = 121 chunks, +2 above load radius to prevent thrashing)
pub const CHUNK_UNLOAD_RADIUS: i32 = 5; // 5 chunks in each direction = 11x11 grid
