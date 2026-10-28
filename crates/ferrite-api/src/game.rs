use std::fmt;
use std::ops::{Add, Mul, Sub};

use crate::__private::sdk::game as host;

pub const MAX_RAYCAST_DISTANCE: f32 = 1024.0;
pub const TICKS_PER_DAY: u64 = 24_000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    pub fn horizontal_distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.z - other.z)
    }

    pub fn block_pos(self) -> BlockPos {
        BlockPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, scale: f64) -> Self {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn center(self) -> Vec3 {
        Vec3::new(
            f64::from(self.x) + 0.5,
            f64::from(self.y) + 0.5,
            f64::from(self.z) + 0.5,
        )
    }

    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
}

impl fmt::Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.x, self.y, self.z)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "String", into = "String"))]
pub enum Dimension {
    Overworld,
    Nether,
    End,
    Other(String),
}

impl Dimension {
    pub fn id(&self) -> &str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:the_nether",
            Self::End => "minecraft:the_end",
            Self::Other(id) => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Overworld => "Overworld",
            Self::Nether => "Nether",
            Self::End => "End",
            Self::Other(id) => id,
        }
    }
}

impl From<String> for Dimension {
    fn from(id: String) -> Self {
        match id.as_str() {
            "minecraft:overworld" => Self::Overworld,
            "minecraft:the_nether" => Self::Nether,
            "minecraft:the_end" => Self::End,
            _ => Self::Other(id),
        }
    }
}

impl From<Dimension> for String {
    fn from(dimension: Dimension) -> Self {
        match dimension {
            Dimension::Other(id) => id,
            known => known.id().to_owned(),
        }
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    South,
    West,
    North,
    East,
}

impl Direction {
    pub fn from_yaw(yaw: f32) -> Self {
        match ((yaw / 90.0).round() as i32).rem_euclid(4) {
            0 => Self::South,
            1 => Self::West,
            2 => Self::North,
            _ => Self::East,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::South => "South",
            Self::West => "West",
            Self::North => "North",
            Self::East => "East",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub food: u32,
    pub dimension: Dimension,
}

impl Player {
    pub const MAX_HEALTH: f32 = 20.0;
    pub const MAX_FOOD: u32 = 20;

    pub fn block_pos(&self) -> BlockPos {
        self.position.block_pos()
    }

    pub fn facing(&self) -> Direction {
        Direction::from_yaw(self.yaw)
    }

    pub fn look_direction(&self) -> Vec3 {
        let (yaw, pitch) = (
            f64::from(self.yaw).to_radians(),
            f64::from(self.pitch).to_radians(),
        );
        Vec3::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            yaw.cos() * pitch.cos(),
        )
    }

    pub fn is_hungry(&self) -> bool {
        self.food < Self::MAX_FOOD
    }
}

impl From<host::Player> for Player {
    fn from(p: host::Player) -> Self {
        Self {
            position: Vec3::new(p.x, p.y, p.z),
            yaw: p.yaw,
            pitch: p.pitch,
            health: p.health,
            food: p.food,
            dimension: p.dimension.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Clock {
    pub game_time: u64,
    pub day_time: u64,
}

impl Clock {
    pub fn day(self) -> u64 {
        self.day_time / TICKS_PER_DAY
    }

    pub fn time_of_day(self) -> u32 {
        (self.day_time % TICKS_PER_DAY) as u32
    }

    pub fn hours_minutes(self) -> (u8, u8) {
        let tod = self.time_of_day();
        let hours = (tod / 1000 + 6) % 24;
        let minutes = (tod % 1000) * 60 / 1000;
        (hours as u8, minutes as u8)
    }

    pub fn is_night(self) -> bool {
        (12_542..23_460).contains(&self.time_of_day())
    }

    pub fn is_day(self) -> bool {
        !self.is_night()
    }
}

impl From<host::Clock> for Clock {
    fn from(c: host::Clock) -> Self {
        Self {
            game_time: c.game_time,
            day_time: c.day_time,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub block_pos: BlockPos,
    pub point: Vec3,
    pub block: String,
}

impl From<host::Hit> for Hit {
    fn from(h: host::Hit) -> Self {
        Self {
            block_pos: BlockPos::new(h.block_x, h.block_y, h.block_z),
            point: Vec3::new(h.x, h.y, h.z),
            block: h.block,
        }
    }
}

pub fn player() -> Option<Player> {
    host::get_player().map(Player::from)
}

pub fn clock() -> Option<Clock> {
    host::get_clock().map(Clock::from)
}

pub fn raycast(distance: f32) -> Option<Hit> {
    host::raycast(distance.clamp(0.0, MAX_RAYCAST_DISTANCE)).map(Hit::from)
}

pub fn in_world() -> bool {
    host::get_clock().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(yaw: f32, pitch: f32) -> Player {
        Player {
            position: Vec3::new(-0.5, 64.0, 10.9),
            yaw,
            pitch,
            health: 20.0,
            food: 20,
            dimension: Dimension::Overworld,
        }
    }

    #[test]
    fn block_pos_floors_negative_coordinates() {
        assert_eq!(player(0.0, 0.0).block_pos(), BlockPos::new(-1, 64, 10));
    }

    #[test]
    fn facing_matches_vanilla_yaw() {
        assert_eq!(player(0.0, 0.0).facing(), Direction::South);
        assert_eq!(player(90.0, 0.0).facing(), Direction::West);
        assert_eq!(player(180.0, 0.0).facing(), Direction::North);
        assert_eq!(player(-90.0, 0.0).facing(), Direction::East);
        assert_eq!(player(-180.0, 0.0).facing(), Direction::North);
        assert_eq!(player(539.0, 0.0).facing(), Direction::North);
        assert_eq!(player(719.0, 0.0).facing(), Direction::South);
    }

    #[test]
    fn look_direction_is_a_unit_vector() {
        let south = player(0.0, 0.0).look_direction();
        assert!((south.z - 1.0).abs() < 1e-9 && south.x.abs() < 1e-9);
        let up = player(0.0, -90.0).look_direction();
        assert!((up.y - 1.0).abs() < 1e-9);
        assert!((player(37.0, 21.0).look_direction().length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dimensions_round_trip_through_ids() {
        for id in [
            "minecraft:overworld",
            "minecraft:the_nether",
            "minecraft:the_end",
            "mymod:void",
        ] {
            let dimension = Dimension::from(id.to_owned());
            assert_eq!(dimension.id(), id);
            assert_eq!(String::from(dimension), id);
        }
        assert_eq!(
            Dimension::from("minecraft:the_nether".to_owned()),
            Dimension::Nether
        );
    }

    #[test]
    fn clock_reads_like_the_f3_screen() {
        let dawn = Clock {
            game_time: 0,
            day_time: 0,
        };
        assert_eq!(dawn.hours_minutes(), (6, 0));
        assert!(dawn.is_day());

        let night = Clock {
            game_time: 0,
            day_time: 3 * TICKS_PER_DAY + 18_000,
        };
        assert_eq!(night.day(), 3);
        assert_eq!(night.hours_minutes(), (0, 0));
        assert!(night.is_night());

        assert_eq!(
            Clock {
                game_time: 0,
                day_time: 6_500
            }
            .hours_minutes(),
            (12, 30)
        );
    }

    #[test]
    fn vector_math() {
        let a = Vec3::new(1.0, 2.0, 2.0);
        assert_eq!(a.length(), 3.0);
        assert_eq!(a.distance(Vec3::ZERO), 3.0);
        assert_eq!(a.horizontal_distance(Vec3::new(4.0, 100.0, 6.0)), 5.0);
        assert_eq!(a * 2.0 - a + Vec3::ZERO, a);
        assert_eq!(BlockPos::new(1, 2, 3).center(), Vec3::new(1.5, 2.5, 3.5));
        assert_eq!(BlockPos::new(1, 2, 3).offset(-1, 0, 1).to_string(), "0 2 4");
    }
}
