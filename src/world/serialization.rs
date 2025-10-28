use crate::tiles::{ChunkData, ChunkPos, CHUNK_AREA};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

/// Magic number for chunk files ("TILE" in ASCII)
const MAGIC_NUMBER: [u8; 4] = [b'T', b'I', b'L', b'E'];

/// Current chunk file format version
const VERSION: u16 = 1;

/// Error type for serialization operations
#[derive(Debug)]
pub enum SerializationError {
    Io(io::Error),
    InvalidMagicNumber,
    InvalidVersion(u16),
    InvalidChunkSize(usize),
    InvalidChecksum,
}

impl From<io::Error> for SerializationError {
    fn from(err: io::Error) -> Self {
        SerializationError::Io(err)
    }
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationError::Io(e) => write!(f, "IO error: {}", e),
            SerializationError::InvalidMagicNumber => write!(f, "Invalid magic number"),
            SerializationError::InvalidVersion(v) => write!(f, "Invalid version: {}", v),
            SerializationError::InvalidChunkSize(s) => write!(f, "Invalid chunk size: {}", s),
            SerializationError::InvalidChecksum => write!(f, "Checksum mismatch"),
        }
    }
}

impl std::error::Error for SerializationError {}

/// Save a chunk to disk in binary format
pub fn save_chunk<P: AsRef<Path>>(
    chunk: &ChunkData,
    path: P,
) -> Result<(), SerializationError> {
    // Ensure directory exists
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;

    // Write header
    file.write_all(&MAGIC_NUMBER)?;
    file.write_all(&VERSION.to_le_bytes())?;

    // Write chunk position
    file.write_all(&chunk.position.x.to_le_bytes())?;
    file.write_all(&chunk.position.y.to_le_bytes())?;

    // Write tile data (u16 array, little-endian)
    let mut tile_bytes = Vec::with_capacity(CHUNK_AREA * 2);
    for &tile in chunk.tiles.iter() {
        tile_bytes.extend_from_slice(&tile.to_le_bytes());
    }
    file.write_all(&tile_bytes)?;

    // Calculate and write checksum (CRC32)
    let checksum = crc32fast::hash(&tile_bytes);
    file.write_all(&checksum.to_le_bytes())?;

    file.sync_all()?;
    Ok(())
}

/// Load a chunk from disk
pub fn load_chunk<P: AsRef<Path>>(path: P) -> Result<ChunkData, SerializationError> {
    let mut file = File::open(path)?;

    // Read and verify magic number
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    if magic != MAGIC_NUMBER {
        return Err(SerializationError::InvalidMagicNumber);
    }

    // Read and verify version
    let mut version_bytes = [0u8; 2];
    file.read_exact(&mut version_bytes)?;
    let version = u16::from_le_bytes(version_bytes);
    if version != VERSION {
        return Err(SerializationError::InvalidVersion(version));
    }

    // Read chunk position
    let mut x_bytes = [0u8; 4];
    let mut y_bytes = [0u8; 4];
    file.read_exact(&mut x_bytes)?;
    file.read_exact(&mut y_bytes)?;
    let position = ChunkPos::new(i32::from_le_bytes(x_bytes), i32::from_le_bytes(y_bytes));

    // Read tile data
    let mut tile_bytes = vec![0u8; CHUNK_AREA * 2];
    file.read_exact(&mut tile_bytes)?;

    // Read and verify checksum
    let mut checksum_bytes = [0u8; 4];
    file.read_exact(&mut checksum_bytes)?;
    let expected_checksum = u32::from_le_bytes(checksum_bytes);
    let actual_checksum = crc32fast::hash(&tile_bytes);
    if actual_checksum != expected_checksum {
        return Err(SerializationError::InvalidChecksum);
    }

    // Convert bytes to tile array
    let mut tiles = Box::new([0u16; CHUNK_AREA]);
    for (i, chunk) in tile_bytes.chunks_exact(2).enumerate() {
        tiles[i] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }

    Ok(ChunkData { position, tiles })
}

/// Check if a chunk file exists
pub fn chunk_exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Delete a chunk file
pub fn delete_chunk<P: AsRef<Path>>(path: P) -> Result<(), io::Error> {
    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::TILE_GRASS;
    use std::env;

    #[test]
    fn test_save_and_load_chunk() {
        let temp_dir = env::temp_dir();
        let chunk_path = temp_dir.join("test_chunk.bin");

        // Create test chunk
        let original = ChunkData::filled(ChunkPos::new(5, -3), TILE_GRASS);

        // Save chunk
        save_chunk(&original, &chunk_path).expect("Failed to save chunk");

        // Load chunk
        let loaded = load_chunk(&chunk_path).expect("Failed to load chunk");

        // Verify
        assert_eq!(loaded.position, original.position);
        assert_eq!(loaded.tiles[0], TILE_GRASS);
        assert_eq!(loaded.tiles[CHUNK_AREA - 1], TILE_GRASS);

        // Cleanup
        let _ = fs::remove_file(chunk_path);
    }

    #[test]
    fn test_chunk_exists() {
        let temp_dir = env::temp_dir();
        let chunk_path = temp_dir.join("test_exists_chunk.bin");

        assert!(!chunk_exists(&chunk_path));

        let chunk = ChunkData::filled(ChunkPos::new(0, 0), TILE_GRASS);
        save_chunk(&chunk, &chunk_path).unwrap();

        assert!(chunk_exists(&chunk_path));

        // Cleanup
        let _ = fs::remove_file(chunk_path);
    }
}
