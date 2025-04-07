mod collider;
mod logic;

pub use self::collider::*;

use crate::prelude::*;

pub type Coord = R32;
pub type FloatTime = R32;

pub struct Model {
    pub simulation_time: FloatTime,

    pub ground_level: Coord,

    pub drill: Collider,
}

impl Model {
    pub fn new(context: Context) -> Self {
        let config = &context.assets.config;
        Self {
            simulation_time: FloatTime::ZERO,

            ground_level: Coord::ZERO,

            drill: Collider::circle(vec2::ZERO, config.drill_size),
        }
    }
}
