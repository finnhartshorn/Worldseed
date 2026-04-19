use crate::entities::{
    Direction, EntityState, ForestGuardian, GrowingTree, Health, Position, RoamingBehavior,
    TreeSpawner, Velocity, WindingPath,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const ENTITIES_FILE: &str = "entities.bin";
const ENTITIES_VERSION: u16 = 1;

#[derive(Debug)]
pub enum EntitySaveError {
    Io(io::Error),
    Codec(Box<bincode::ErrorKind>),
}

impl From<io::Error> for EntitySaveError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<Box<bincode::ErrorKind>> for EntitySaveError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        Self::Codec(err)
    }
}

impl std::fmt::Display for EntitySaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntitySaveError::Io(err) => write!(f, "IO error: {err}"),
            EntitySaveError::Codec(err) => write!(f, "Serialization error: {err}"),
        }
    }
}

impl std::error::Error for EntitySaveError {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedWorldEntities {
    pub version: u16,
    pub entities: Vec<SavedEntity>,
}

impl SavedWorldEntities {
    pub fn new(entities: Vec<SavedEntity>) -> Self {
        Self {
            version: ENTITIES_VERSION,
            entities,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SavedEntity {
    Human(SavedActor),
    ForestGuardian(SavedForestGuardian),
    Snail(SavedSnail),
    TreeSpirit(SavedGrowingTreeEntity),
    VariantTree(SavedGrowingTreeEntity),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedActor {
    pub position: Position,
    pub velocity: Velocity,
    pub direction: Direction,
    pub state: EntityState,
    pub health: Health,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedForestGuardian {
    pub actor: SavedActor,
    pub guardian: ForestGuardian,
    pub roaming: RoamingBehavior,
    pub tree_spawner: TreeSpawner,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedSnail {
    pub actor: SavedActor,
    pub winding_path: WindingPath,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedGrowingTreeEntity {
    pub position: Position,
    pub growing_tree: GrowingTree,
}

pub fn entities_save_path<P: AsRef<Path>>(world_dir: P) -> PathBuf {
    world_dir.as_ref().join(ENTITIES_FILE)
}

pub fn entities_save_exists<P: AsRef<Path>>(world_dir: P) -> bool {
    entities_save_path(world_dir).exists()
}

pub fn save_world_entities<P: AsRef<Path>>(
    world_dir: P,
    entities: &SavedWorldEntities,
) -> Result<(), EntitySaveError> {
    let path = entities_save_path(world_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let encoded = bincode::serialize(entities)?;
    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}

pub fn load_world_entities<P: AsRef<Path>>(
    world_dir: P,
) -> Result<Option<SavedWorldEntities>, EntitySaveError> {
    let path = entities_save_path(world_dir);
    if !path.exists() {
        return Ok(None);
    }

    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let decoded: SavedWorldEntities = bincode::deserialize(&bytes)?;
    Ok(Some(decoded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{GrowingTree, TreeVariant};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("worldseed-entities-{unique}"));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn entity_save_round_trip() {
        let root = temp_root();
        let entities = SavedWorldEntities::new(vec![
            SavedEntity::Human(SavedActor {
                position: Position::new(1.0, 2.0),
                velocity: Velocity::new(3.0, 4.0),
                direction: Direction::NorthWest,
                state: EntityState::Moving,
                health: Health {
                    current: 80.0,
                    max: 100.0,
                },
            }),
            SavedEntity::ForestGuardian(SavedForestGuardian {
                actor: SavedActor {
                    position: Position::new(5.0, 6.0),
                    velocity: Velocity::new(0.0, 1.0),
                    direction: Direction::SouthEast,
                    state: EntityState::Idle,
                    health: Health {
                        current: 120.0,
                        max: 150.0,
                    },
                },
                guardian: ForestGuardian::new(TreeVariant::Birch),
                roaming: RoamingBehavior::new(Position::new(5.0, 6.0), 100.0, 15.0),
                tree_spawner: TreeSpawner::default_guardian(),
            }),
            SavedEntity::Snail(SavedSnail {
                actor: SavedActor {
                    position: Position::new(7.0, 8.0),
                    velocity: Velocity::new(0.5, -0.25),
                    direction: Direction::SouthWest,
                    state: EntityState::Moving,
                    health: Health {
                        current: 400.0,
                        max: 500.0,
                    },
                },
                winding_path: WindingPath::new(2.5),
            }),
            SavedEntity::TreeSpirit(SavedGrowingTreeEntity {
                position: Position::new(9.0, 10.0),
                growing_tree: GrowingTree::with_growth_time(TreeVariant::Oak, 3.0),
            }),
            SavedEntity::VariantTree(SavedGrowingTreeEntity {
                position: Position::new(11.0, 12.0),
                growing_tree: GrowingTree::with_variant_appearance(
                    TreeVariant::Willow,
                    5.0,
                    4.0,
                    crate::entities::VariantTreeAppearance::Variation2,
                ),
            }),
        ]);

        save_world_entities(&root, &entities).expect("entity save should succeed");
        let loaded = load_world_entities(&root)
            .expect("entity load should succeed")
            .expect("entity file should exist");

        assert_eq!(loaded, entities);

        let _ = fs::remove_dir_all(root);
    }
}
