mod collider;
mod logic;

pub use self::collider::*;

use crate::prelude::*;

pub type Coord = R32;
pub type FloatTime = R32;
pub type ResourceCount = i64;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Config {
    pub drill_size: Coord,
    pub minerals: HashMap<MineralKind, Vec<MineralConfig>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MineralConfig {
    pub range: [Coord; 2],
    pub density: R32,
}

#[derive(Debug, Clone)]
pub struct Mineral {
    pub collider: Collider,
    pub kind: MineralKind,
    pub amount: ResourceCount,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum MineralKind {
    Resource(ResourceKind),
    Rock,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Coal,
    Iron,
    Bronze,
    Silver,
    Gold,
    Gem,
}

pub struct Model {
    pub simulation_time: FloatTime,

    pub camera: Camera2d,
    pub ground_level: Coord,
    pub depth_generated: Coord,

    pub drill: Collider,
    pub vision_radius: Coord,
    pub minerals: Vec<Mineral>,
}

impl Model {
    pub fn new(context: Context) -> Self {
        let config = &context.assets.config;
        let mut model = Self {
            simulation_time: FloatTime::ZERO,

            camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(15.0),
            },
            ground_level: Coord::ZERO,
            depth_generated: Coord::ZERO,

            drill: Collider::circle(vec2::ZERO, config.drill_size),
            vision_radius: r32(2.0),
            minerals: vec![],
        };
        model.generate_level();
        model
    }
}
