mod collider;
mod logic;

pub use self::collider::*;

use crate::prelude::*;

pub type Coord = R32;
pub type FloatTime = R32;

pub struct Model {
    pub simulation_time: FloatTime,

    pub camera: Camera2d,
    pub ground_level: Coord,

    pub drill: Collider,
    pub vision_radius: Coord,
}

impl Model {
    pub fn new(context: Context) -> Self {
        let config = &context.assets.config;
        Self {
            simulation_time: FloatTime::ZERO,

            camera: Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: Camera2dFov::Vertical(15.0),
            },
            ground_level: Coord::ZERO,

            drill: Collider::circle(vec2::ZERO, config.drill_size),
            vision_radius: r32(2.0),
        }
    }
}
