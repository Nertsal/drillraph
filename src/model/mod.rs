mod collider;
mod logic;
mod particles;

pub use self::{collider::*, particles::*};

use crate::prelude::*;

pub type Coord = R32;
pub type FloatTime = R32;
pub type ResourceCount = i64;
pub type Money = i64;
pub type Fuel = R32;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Config {
    pub map_width: Coord,
    pub coal_fuel_value: Fuel,
    /// How much to dim the node when it is not connected to power.
    pub unpowered_node_dim: f32,

    pub drill_size: Coord,
    pub drill_speed: Coord,
    pub drill_speed_0: Coord,
    pub drill_speed_1: Coord,
    pub drill_speed_2: Coord,
    pub drill_acceleration: Coord,
    pub drill_rotation_speed: Coord,

    pub vision: Coord,
    pub vision_0: Coord,
    pub vision_1: Coord,
    pub vision_2: Coord,

    pub sprint_boost: Coord,
    pub sprint_duration: Coord,
    pub sprint_cooldown: Coord,

    pub fuel_small_amount: Fuel,
    pub fuel_normal_amount: Fuel,

    pub minerals: HashMap<MineralKind, MineralConfig>,

    pub shop_0: ShopConfig,
    pub shop_1: ShopConfig,
    pub shop_2: ShopConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShopConfig {
    pub slots: usize,
    pub items: Vec<ShopItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShopItem {
    pub cost: Money,
    pub node: ShopNode,
    #[serde(default)]
    pub sold_out: bool,
}

#[derive(Debug, Clone)]
pub struct ShopItemTracked {
    pub item: ShopItem,
    pub tier: usize,
    pub index: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShopNode {
    FuelSmall,
    Fuel,
    TurnLeft,
    TurnRight,
    Battery,
    Upgrade,
    Speed,
    Light,
    Sprint,
    CoalFuel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MineralConfig {
    pub value: Money,
    pub generation: Vec<MineralGeneration>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MineralGeneration {
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
pub enum MineralKind {
    Resource(ResourceKind),
    Rock,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResourceKind {
    Coal,
    Iron,
    Bronze,
    Silver,
    Gold,
    Gem,
}

#[derive(Debug, Clone)]
pub struct Nodes {
    pub bounds: Aabb2<Coord>,
    pub camera: Camera2d,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Whether the node is connected to power.
    pub is_powered: bool,
    pub position: Aabb2<Coord>,
    pub kind: NodeKind,
    pub connections: Vec<NodeConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub offset: vec2<Coord>,
    pub kind: ConnectionKind,
    pub connected_to: Option<ConnectionId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    pub node: usize,
    pub connection: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConnectionKind {
    Normal,
    Fuel,
    Upgrade,
    Drill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Power,
    Fuel(Bounded<Fuel>),
    Shop {
        level: usize,
    },
    Drill {
        level: ResourceKind,
        power: Bounded<usize>,
    },
    TurnLeft,
    TurnRight,
    Sprint {
        cooldown: Bounded<FloatTime>,
    },
    Upgrade,
    Battery,
    Vision {
        level: usize,
    },
    Speed {
        level: usize,
    },
    CoalFuel(Bounded<Fuel>),
}

#[derive(Debug)]
pub enum Phase {
    Setup,
    Drill,
}

#[derive(Debug)]
pub struct DrillSprint {
    pub caused_by_node: usize,
    pub duration: Bounded<FloatTime>,
}

#[derive(Debug)]
pub struct Drill {
    pub collider: Collider,
    pub drill_level: ResourceKind,
    pub max_speed: Coord,
    pub speed: Coord,
    pub target_speed: Coord,
    pub colliding_with: HashSet<usize>,
    pub sprint: Option<DrillSprint>,
    pub vision_radius: Coord,
    pub can_turn_left: bool,
    pub can_turn_right: bool,
}

pub struct Model {
    pub context: Context,
    pub config: Config,
    pub palette: Palette,
    pub simulation_time: FloatTime,
    pub phase: Phase,

    pub camera: Camera2d,
    pub bounds: Aabb2<Coord>,
    pub ground_level: Coord,
    pub depth_generated: Coord,
    pub nodes: Nodes,

    pub money: Money,
    pub shop: Vec<ShopItemTracked>,
    pub drill: Drill,
    pub minerals: Vec<Mineral>,

    pub particles_queue: Vec<SpawnParticles>,
    pub particles: StructOf<Arena<Particle>>,
    pub floating_texts: StructOf<Arena<FloatingText>>,
}

impl Model {
    pub fn new(context: Context) -> Self {
        let config = &context.assets.config;
        let mut model = Self {
            context: context.clone(),
            config: config.clone(),
            palette: context.assets.palette.clone(),
            simulation_time: FloatTime::ZERO,
            phase: Phase::Setup,

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
                nodes: vec![
                    Node {
                        is_powered: false,
                        position: Aabb2::ZERO.extend_right(1.0).extend_down(1.0).as_r32(),
                        kind: NodeKind::Power,
                        connections: vec![
                            NodeConnection {
                                offset: vec2(1.0, 0.5).as_r32(),
                                kind: ConnectionKind::Fuel,
                                connected_to: None,
                            },
                            NodeConnection {
                                offset: vec2(0.5, 0.0).as_r32(),
                                kind: ConnectionKind::Drill,
                                connected_to: None,
                            },
                        ],
                    },
                    Node {
                        is_powered: false,
                        position: Aabb2::point(vec2(3.0, -2.0))
                            .extend_right(2.0)
                            .extend_down(1.0)
                            .as_r32(),
                        kind: NodeKind::Fuel(Bounded::new_max(config.fuel_small_amount)),
                        connections: vec![NodeConnection {
                            offset: vec2(0.0, 0.5).as_r32(),
                            kind: ConnectionKind::Fuel,
                            connected_to: None,
                        }],
                    },
                    Node {
                        is_powered: false,
                        position: Aabb2::point(vec2(2.0, -6.0))
                            .extend_right(2.0)
                            .extend_down(1.0)
                            .as_r32(),
                        kind: NodeKind::Drill {
                            level: ResourceKind::Iron,
                            power: Bounded::new_zero(0),
                        },
                        connections: vec![
                            NodeConnection {
                                offset: vec2(0.0, 0.5).as_r32(),
                                kind: ConnectionKind::Drill,
                                connected_to: None,
                            },
                            NodeConnection {
                                offset: vec2(0.5, 1.0).as_r32(),
                                kind: ConnectionKind::Upgrade,
                                connected_to: None,
                            },
                            NodeConnection {
                                offset: vec2(1.0, 0.5).as_r32(),
                                kind: ConnectionKind::Normal,
                                connected_to: None,
                            },
                        ],
                    },
                    Node {
                        is_powered: false,
                        position: Aabb2::point(vec2(0.0, -10.0))
                            .extend_right(3.0)
                            .extend_up(1.0)
                            .as_r32(),
                        kind: NodeKind::Shop { level: 0 },
                        connections: vec![NodeConnection {
                            offset: vec2(0.5, 1.0).as_r32(),
                            kind: ConnectionKind::Upgrade,
                            connected_to: None,
                        }],
                    },
                ],
            },

            money: 0,
            shop: Vec::new(),
            drill: Drill {
                collider: Collider::circle(vec2::ZERO, config.drill_size),
                drill_level: ResourceKind::Iron,
                speed: Coord::ZERO,
                max_speed: config.drill_speed,
                target_speed: Coord::ZERO,
                colliding_with: HashSet::new(),
                sprint: None,
                vision_radius: config.vision,
                can_turn_left: false,
                can_turn_right: false,
            },
            minerals: vec![],

            particles_queue: Vec::new(),
            particles: default(),
            floating_texts: default(),
        };
        model.generate_level();
        model
    }
}
