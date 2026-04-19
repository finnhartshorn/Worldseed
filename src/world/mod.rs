pub mod entities_save;
pub mod generator;
pub mod loader;
pub mod manager;
pub mod savegame;
pub mod serialization;

// Re-export commonly used items
pub use generator::{generate_chunk, pregenerate_world_chunks};
pub use manager::{TileModification, WorldManager, WorldStats};
pub use savegame::{WorldElement, WorldGenerationConfig, WorldShape};
