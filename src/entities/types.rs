use bevy::prelude::*;

/// World position component - tracks entity position in world space (pixels)
#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Velocity component - movement speed in pixels per second
#[derive(Component, Debug, Clone, Copy)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

/// Direction the entity is facing (for animation purposes)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    NorthWest = 0, // Row 0 in sprite sheets
    NorthEast = 1, // Row 1 in sprite sheets
    SouthWest = 2, // Row 2 in sprite sheets
    SouthEast = 3, // Row 3 in sprite sheets
}

impl Direction {
    /// Get direction from velocity vector
    pub fn from_velocity(velocity: &Velocity) -> Self {
        if velocity.y >= 0.0 {
            // Moving up/north
            if velocity.x >= 0.0 {
                Direction::NorthEast
            } else {
                Direction::NorthWest
            }
        } else {
            // Moving down/south
            if velocity.x >= 0.0 {
                Direction::SouthEast
            } else {
                Direction::SouthWest
            }
        }
    }

    /// Get the row index in the sprite sheet for this direction
    pub fn sprite_row(&self) -> usize {
        *self as usize
    }
}

impl Default for Direction {
    fn default() -> Self {
        Direction::SouthEast
    }
}

/// Entity state machine
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    Idle,
    Moving,
    Attacking,
    Dead,
}

impl Default for EntityState {
    fn default() -> Self {
        EntityState::Idle
    }
}

/// Health component
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn percent(&self) -> f32 {
        self.current / self.max
    }
}

/// Marker component for the player character
#[derive(Component)]
pub struct Player;

/// Marker component for forest guardian creatures
#[derive(Component)]
pub struct ForestGuardian;

/// Marker component for snail creatures
#[derive(Component)]
pub struct Snail;

/// Winding path behavior - makes entities move in long, meandering paths
#[derive(Component, Debug, Clone)]
pub struct WindingPath {
    /// Current direction angle in radians
    pub current_angle: f32,
    /// Target direction angle in radians
    pub target_angle: f32,
    /// Base movement speed in pixels per second
    pub speed: f32,
    /// How far to travel before changing direction (in pixels)
    pub segment_length: f32,
    /// Distance traveled in current segment
    pub distance_traveled: f32,
    /// How fast to turn towards target angle (radians per second)
    pub turn_rate: f32,
    /// Minimum segment length
    pub min_segment_length: f32,
    /// Maximum segment length
    pub max_segment_length: f32,
    /// Maximum angle change between segments (in radians)
    pub max_angle_change: f32,
}

impl WindingPath {
    /// Create a new winding path behavior with default settings
    pub fn new(speed: f32) -> Self {
        use std::f32::consts::PI;
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        // Simple pseudo-random number generation using current time and hash
        let hasher_builder = RandomState::new();
        let mut hasher = hasher_builder.build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        let initial_angle = ((hash as f32) / (u64::MAX as f32)) * 2.0 * PI;

        Self {
            current_angle: initial_angle,
            target_angle: initial_angle,
            speed,
            segment_length: 200.0,
            distance_traveled: 0.0,
            turn_rate: 0.5, // Smooth turning
            min_segment_length: 150.0,
            max_segment_length: 400.0,
            max_angle_change: PI * 0.4, // Up to 72 degrees
        }
    }

    /// Create with custom parameters
    pub fn with_params(
        speed: f32,
        min_segment: f32,
        max_segment: f32,
        turn_rate: f32,
        max_angle_change: f32,
    ) -> Self {
        use std::f32::consts::PI;
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let hasher_builder = RandomState::new();
        let mut hasher = hasher_builder.build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        let initial_angle = ((hash as f32) / (u64::MAX as f32)) * 2.0 * PI;

        // Hash again for segment length
        let mut hasher2 = hasher_builder.build_hasher();
        (hash.wrapping_add(1)).hash(&mut hasher2);
        let hash2 = hasher2.finish();
        let rand_val = (hash2 as f32) / (u64::MAX as f32);

        Self {
            current_angle: initial_angle,
            target_angle: initial_angle,
            speed,
            segment_length: min_segment + rand_val * (max_segment - min_segment),
            distance_traveled: 0.0,
            turn_rate,
            min_segment_length: min_segment,
            max_segment_length: max_segment,
            max_angle_change,
        }
    }
}

/// Entity bundle containing common components all entities need
#[derive(Bundle)]
pub struct EntityBundle {
    pub position: Position,
    pub velocity: Velocity,
    pub direction: Direction,
    pub state: EntityState,
    pub health: Health,
}

impl EntityBundle {
    pub fn new(x: f32, y: f32, max_health: f32) -> Self {
        Self {
            position: Position::new(x, y),
            velocity: Velocity::zero(),
            direction: Direction::default(),
            state: EntityState::Idle,
            health: Health::new(max_health),
        }
    }
}
