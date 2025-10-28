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

// Tile type constants
/// Empty/air tile
pub const TILE_EMPTY: u16 = 0;

/// Grass tile
pub const TILE_GRASS: u16 = 1;

/// Dirt tile
pub const TILE_DIRT: u16 = 2;

/// Maximum number of tile types (u16 can hold 0-65535)
pub const MAX_TILE_TYPES: usize = u16::MAX as usize + 1;

// Chunk loading/unloading radii
/// Radius of chunks to load around the camera (7x7 = 49 chunks)
pub const CHUNK_LOAD_RADIUS: i32 = 3; // 3 chunks in each direction = 7x7 grid

/// Radius of chunks to keep loaded (11x11 = 121 chunks, +2 above load radius to prevent thrashing)
pub const CHUNK_UNLOAD_RADIUS: i32 = 5; // 5 chunks in each direction = 11x11 grid
