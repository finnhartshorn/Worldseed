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
    NorthEast = 0, // Row 0 in sprite sheets (up-right)
    NorthWest = 1, // Row 1 in sprite sheets (up-left)
    SouthEast = 2, // Row 2 in sprite sheets (down-right)
    SouthWest = 3, // Row 3 in sprite sheets (down-left)
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

/// Marker component for growing tree spirits
#[derive(Component)]
pub struct TreeSpirit;

/// Growth stages for trees
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthStage {
    Seed,           // Initial planted seed (small sprite)
    Sapling,        // Young sapling (medium sprite)
    YoungTree,      // Growing tree (scaled up from sapling)
    MatureTree,     // Fully grown tree (full size)
}

impl GrowthStage {
    /// Get the scale factor for this growth stage
    pub fn scale(&self) -> f32 {
        match self {
            GrowthStage::Seed => 0.5,
            GrowthStage::Sapling => 1.0,
            GrowthStage::YoungTree => 1.5,
            GrowthStage::MatureTree => 2.0,
        }
    }

    /// Get the next growth stage
    pub fn next(&self) -> Option<GrowthStage> {
        match self {
            GrowthStage::Seed => Some(GrowthStage::Sapling),
            GrowthStage::Sapling => Some(GrowthStage::YoungTree),
            GrowthStage::YoungTree => Some(GrowthStage::MatureTree),
            GrowthStage::MatureTree => None,
        }
    }
}

/// Component for trees that grow over time
#[derive(Component, Debug, Clone, Copy)]
pub struct GrowingTree {
    /// Current growth stage
    pub stage: GrowthStage,
    /// Time spent in current stage (seconds)
    pub time_in_stage: f32,
    /// Time required to advance to next stage (seconds)
    pub time_to_next_stage: f32,
    /// Tree variant (oak, birch, hickory, pine, willow)
    pub variant: TreeVariant,
}

impl GrowingTree {
    pub fn new(variant: TreeVariant) -> Self {
        Self {
            stage: GrowthStage::Seed,
            time_in_stage: 0.0,
            time_to_next_stage: 5.0, // 5 seconds per stage by default
            variant,
        }
    }

    pub fn with_growth_time(variant: TreeVariant, growth_time: f32) -> Self {
        Self {
            stage: GrowthStage::Seed,
            time_in_stage: 0.0,
            time_to_next_stage: growth_time,
            variant,
        }
    }

    pub fn is_mature(&self) -> bool {
        self.stage == GrowthStage::MatureTree
    }
}

/// Tree variants available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeVariant {
    Oak,
    Birch,
    Hickory,
    Pine,
    Willow,
}

impl TreeVariant {
    pub fn as_str(&self) -> &str {
        match self {
            TreeVariant::Oak => "oak",
            TreeVariant::Birch => "birch",
            TreeVariant::Hickory => "hickory",
            TreeVariant::Pine => "pine",
            TreeVariant::Willow => "willow",
        }
    }
}

/// Roaming behavior - makes entities roam within a fixed radius of their spawn point
#[derive(Component, Debug, Clone, Copy)]
pub struct RoamingBehavior {
    /// The center point to roam around (usually spawn position)
    pub home: Position,
    /// Maximum distance from home position
    pub roam_radius: f32,
    /// Movement speed in pixels per second
    pub speed: f32,
    /// Current target position to move towards
    pub target: Position,
    /// How long to wait at each target position (in seconds)
    pub pause_duration: f32,
    /// Time spent paused at current position
    pub pause_timer: f32,
    /// Minimum pause duration
    pub min_pause_duration: f32,
    /// Maximum pause duration
    pub max_pause_duration: f32,
}

impl RoamingBehavior {
    /// Create a new roaming behavior with default settings
    pub fn new(home: Position, roam_radius: f32, speed: f32) -> Self {
        Self {
            home,
            roam_radius,
            speed,
            target: home,
            pause_duration: 2.0,
            pause_timer: 0.0,
            min_pause_duration: 1.0,
            max_pause_duration: 4.0,
        }
    }

    /// Create with custom pause duration range
    pub fn with_pause_range(
        home: Position,
        roam_radius: f32,
        speed: f32,
        min_pause: f32,
        max_pause: f32,
    ) -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let hasher_builder = RandomState::new();
        let mut hasher = hasher_builder.build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let hash = hasher.finish();
        let rand_val = (hash as f32) / (u64::MAX as f32);

        Self {
            home,
            roam_radius,
            speed,
            target: home,
            pause_duration: min_pause + rand_val * (max_pause - min_pause),
            pause_timer: 0.0,
            min_pause_duration: min_pause,
            max_pause_duration: max_pause,
        }
    }

    /// Check if entity is within roaming radius of home
    pub fn is_within_bounds(&self, position: &Position) -> bool {
        position.distance_to(&self.home) <= self.roam_radius
    }

    /// Check if entity is close to current target
    pub fn is_at_target(&self, position: &Position, threshold: f32) -> bool {
        position.distance_to(&self.target) < threshold
    }
}

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
