use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SLOT_META_FILE: &str = "meta.bin";
const WORLD_META_FILE: &str = "meta.bin";

#[derive(Debug)]
pub enum SaveGameError {
    Io(io::Error),
    Codec(Box<bincode::ErrorKind>),
}

impl From<io::Error> for SaveGameError {
    fn from(err: io::Error) -> Self {
        SaveGameError::Io(err)
    }
}

impl From<Box<bincode::ErrorKind>> for SaveGameError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        SaveGameError::Codec(err)
    }
}

impl std::fmt::Display for SaveGameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveGameError::Io(err) => write!(f, "IO error: {err}"),
            SaveGameError::Codec(err) => write!(f, "Serialization error: {err}"),
        }
    }
}

impl std::error::Error for SaveGameError {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    pub id: String,
    pub display_name: String,
    pub created_at: u64,
    pub last_played_at: u64,
    #[serde(default = "default_available_columns")]
    pub available_columns: u8,
}

#[derive(Deserialize)]
struct LegacySaveSlotMetadata {
    id: String,
    display_name: String,
    created_at: u64,
    last_played_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorldMetadata {
    pub id: String,
    pub display_name: String,
    pub created_at: u64,
    pub last_played_at: u64,
}

pub fn list_slots<P: AsRef<Path>>(root: P) -> Result<Vec<SaveSlotMetadata>, SaveGameError> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut slots: Vec<SaveSlotMetadata> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let meta_path = entry.path().join(SLOT_META_FILE);
        if !meta_path.exists() {
            continue;
        }

        slots.push(read_slot_metadata(&meta_path)?);
    }

    slots.sort_by(|a, b| {
        b.last_played_at
            .cmp(&a.last_played_at)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
    Ok(slots)
}

pub fn list_worlds<P: AsRef<Path>>(
    root: P,
    slot_id: &str,
) -> Result<Vec<WorldMetadata>, SaveGameError> {
    let worlds_root = slot_worlds_path(root.as_ref(), slot_id);
    if !worlds_root.exists() {
        return Ok(Vec::new());
    }

    let mut worlds: Vec<WorldMetadata> = Vec::new();
    for entry in fs::read_dir(worlds_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let meta_path = entry.path().join(WORLD_META_FILE);
        if !meta_path.exists() {
            continue;
        }

        worlds.push(read_metadata(&meta_path)?);
    }

    worlds.sort_by(|a, b| {
        b.last_played_at
            .cmp(&a.last_played_at)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });
    Ok(worlds)
}

pub fn create_slot<P: AsRef<Path>>(root: P) -> Result<SaveSlotMetadata, SaveGameError> {
    let root = root.as_ref();
    fs::create_dir_all(root)?;

    let existing_slots = list_slots(root)?;
    let slot_id = unique_id("slot", root)?;
    let now = unix_timestamp_secs();
    let metadata = SaveSlotMetadata {
        id: slot_id.clone(),
        display_name: format!("Slot {}", existing_slots.len() + 1),
        created_at: now,
        last_played_at: now,
        available_columns: default_available_columns(),
    };

    let slot_path = slot_path(root, &slot_id);
    fs::create_dir_all(slot_path.join("worlds"))?;
    write_metadata(slot_path.join(SLOT_META_FILE), &metadata)?;
    Ok(metadata)
}

pub fn create_world<P: AsRef<Path>>(
    root: P,
    slot_id: &str,
) -> Result<WorldMetadata, SaveGameError> {
    let root = root.as_ref();
    let worlds = list_worlds(root, slot_id)?;
    let worlds_root = slot_worlds_path(root, slot_id);
    fs::create_dir_all(&worlds_root)?;

    let world_id = unique_id("world", &worlds_root)?;
    let now = unix_timestamp_secs();
    let metadata = WorldMetadata {
        id: world_id.clone(),
        display_name: format!("World {}", worlds.len() + 1),
        created_at: now,
        last_played_at: now,
    };

    let world_path = worlds_root.join(&world_id);
    fs::create_dir_all(world_path.join("world/chunks"))?;
    write_metadata(world_path.join(WORLD_META_FILE), &metadata)?;
    touch_slot(root, slot_id)?;
    Ok(metadata)
}

pub fn most_recent_slot<P: AsRef<Path>>(
    root: P,
) -> Result<Option<SaveSlotMetadata>, SaveGameError> {
    Ok(list_slots(root)?.into_iter().next())
}

pub fn most_recent_world<P: AsRef<Path>>(
    root: P,
    slot_id: &str,
) -> Result<Option<WorldMetadata>, SaveGameError> {
    Ok(list_worlds(root, slot_id)?.into_iter().next())
}

pub fn touch_slot<P: AsRef<Path>>(root: P, slot_id: &str) -> Result<(), SaveGameError> {
    let meta_path = slot_path(root.as_ref(), slot_id).join(SLOT_META_FILE);
    let mut metadata = read_slot_metadata(&meta_path)?;
    metadata.last_played_at = unix_timestamp_secs();
    write_metadata(meta_path, &metadata)
}

pub fn set_slot_available_columns<P: AsRef<Path>>(
    root: P,
    slot_id: &str,
    available_columns: u8,
) -> Result<(), SaveGameError> {
    let meta_path = slot_path(root.as_ref(), slot_id).join(SLOT_META_FILE);
    let mut metadata = read_slot_metadata(&meta_path)?;
    metadata.available_columns = available_columns.max(1);
    write_metadata(meta_path, &metadata)
}

pub fn touch_world<P: AsRef<Path>>(
    root: P,
    slot_id: &str,
    world_id: &str,
) -> Result<(), SaveGameError> {
    let meta_path = world_path(root.as_ref(), slot_id, world_id).join(WORLD_META_FILE);
    let mut metadata: WorldMetadata = read_metadata(&meta_path)?;
    metadata.last_played_at = unix_timestamp_secs();
    write_metadata(meta_path, &metadata)?;
    touch_slot(root, slot_id)
}

pub fn world_save_path<P: AsRef<Path>>(root: P, slot_id: &str, world_id: &str) -> PathBuf {
    world_path(root.as_ref(), slot_id, world_id).join("world")
}

pub fn slot_path<P: AsRef<Path>>(root: P, slot_id: &str) -> PathBuf {
    root.as_ref().join(slot_id)
}

pub fn world_path<P: AsRef<Path>>(root: P, slot_id: &str, world_id: &str) -> PathBuf {
    slot_worlds_path(root.as_ref(), slot_id).join(world_id)
}

fn slot_worlds_path(root: &Path, slot_id: &str) -> PathBuf {
    slot_path(root, slot_id).join("worlds")
}

fn unique_id(prefix: &str, parent: &Path) -> Result<String, SaveGameError> {
    let base = format!("{prefix}-{}", unix_timestamp_secs());
    let mut candidate = base.clone();
    let mut suffix = 1usize;

    while parent.join(&candidate).exists() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }

    Ok(candidate)
}

fn default_available_columns() -> u8 {
    1
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn write_metadata<T: Serialize, P: AsRef<Path>>(path: P, value: &T) -> Result<(), SaveGameError> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }

    let encoded = bincode::serialize(value)?;
    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}

fn read_metadata<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(
    path: P,
) -> Result<T, SaveGameError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bincode::deserialize(&bytes)?)
}

fn read_slot_metadata<P: AsRef<Path>>(path: P) -> Result<SaveSlotMetadata, SaveGameError> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    match bincode::deserialize(&bytes) {
        Ok(metadata) => Ok(metadata),
        Err(_) => {
            let legacy: LegacySaveSlotMetadata = bincode::deserialize(&bytes)?;
            Ok(SaveSlotMetadata {
                id: legacy.id,
                display_name: legacy.display_name,
                created_at: legacy.created_at,
                last_played_at: legacy.last_played_at,
                available_columns: default_available_columns(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("worldseed-savegame-{unique}"));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn create_slot_and_worlds_round_trip() {
        let root = temp_root();
        let slot = create_slot(&root).expect("slot should be created");
        let world = create_world(&root, &slot.id).expect("world should be created");

        let slots = list_slots(&root).expect("slots should list");
        let worlds = list_worlds(&root, &slot.id).expect("worlds should list");

        assert_eq!(slots.len(), 1);
        assert_eq!(worlds.len(), 1);
        assert_eq!(slots[0].id, slot.id);
        assert_eq!(slots[0].available_columns, 1);
        assert_eq!(worlds[0].id, world.id);
        assert!(world_save_path(&root, &slot.id, &world.id).exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn set_slot_available_columns_persists() {
        let root = temp_root();
        let slot = create_slot(&root).expect("slot should be created");

        set_slot_available_columns(&root, &slot.id, 4).expect("slot columns should update");

        let slots = list_slots(&root).expect("slots should list");
        assert_eq!(slots[0].available_columns, 4);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_slot_metadata_defaults_available_columns_to_one() {
        #[derive(Serialize)]
        struct LegacySaveSlotMetadata {
            id: String,
            display_name: String,
            created_at: u64,
            last_played_at: u64,
        }

        let root = temp_root();
        let slot_id = "slot-legacy";
        let metadata = LegacySaveSlotMetadata {
            id: slot_id.to_string(),
            display_name: "Legacy Slot".to_string(),
            created_at: 1,
            last_played_at: 2,
        };

        let slot_dir = slot_path(&root, slot_id);
        fs::create_dir_all(slot_dir.join("worlds")).expect("slot dir should be created");
        write_metadata(slot_dir.join(SLOT_META_FILE), &metadata)
            .expect("legacy metadata should write");

        let slots = list_slots(&root).expect("slots should list");
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].available_columns, 1);

        let _ = fs::remove_dir_all(root);
    }
}
