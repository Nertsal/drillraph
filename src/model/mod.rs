mod collider;
mod logic;

pub use self::collider::*;

use crate::prelude::*;

pub type Coord = R32;
pub type FloatTime = R32;
pub type ResourceCount = i64;
pub type Fuel = R32;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Config {
    pub drill_size: Coord,
    pub map_width: Coord,
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

pub struct Nodes {
    pub bounds: Aabb2<Coord>,
    pub camera: Camera2d,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub position: Aabb2<Coord>,
    pub kind: NodeKind,
    pub connections: Vec<NodeConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub offset: vec2<Coord>,
    pub color: ConnectionColor,
    pub connected_to: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConnectionColor {
    Blue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Power,
    Fuel(Bounded<Fuel>),
}

pub struct Model {
    pub config: Config,
    pub simulation_time: FloatTime,

    pub camera: Camera2d,
    pub bounds: Aabb2<Coord>,
    pub ground_level: Coord,
    pub depth_generated: Coord,
    pub nodes: Nodes,

    pub drill: Collider,
    pub vision_radius: Coord,
    pub minerals: Vec<Mineral>,
}

impl Model {
    pub fn new(context: Context) -> Self {
        let config = &context.assets.config;
        let mut model = Self {
            config: config.clone(),
            simulation_time: FloatTime::ZERO,

            camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(15.0),
            },
            bounds: Aabb2::ZERO,
            ground_level: Coord::ZERO,
            depth_generated: Coord::ZERO,
            nodes: Nodes {
                bounds: Aabb2::ZERO.extend_right(r32(10.0)).extend_down(r32(10.0)),
                camera: Camera2d {
                    center: vec2(5.0, -5.0),
                    rotation: Angle::ZERO,
                    fov: Camera2dFov::Vertical(11.0),
                },
                nodes: vec![Node {
                    position: Aabb2::ZERO.extend_right(r32(1.0)).extend_down(r32(1.0)),
                    kind: NodeKind::Power,
                    connections: vec![NodeConnection {
                        offset: vec2(0.5, 0.0).as_r32(),
                        color: ConnectionColor::Blue,
                        connected_to: None,
                    }],
                }],
            },

            drill: Collider::circle(vec2::ZERO, config.drill_size),
            vision_radius: r32(2.0),
            minerals: vec![],
        };
        model.generate_level();
        model
    }
}
