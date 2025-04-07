use super::*;

impl Model {
    pub fn generate_level(&mut self) {
        log::debug!("Generating next level..");

        // Reset drill
        self.drill.collider.position = vec2(Coord::ZERO, self.ground_level);
        self.drill.collider.rotation = Angle::from_degrees(r32(-90.0));

        // Reset nodes
        for node in &mut self.nodes.nodes {
            match &mut node.kind {
                NodeKind::Power => {}
                NodeKind::Shop { .. } => {}
                NodeKind::Fuel(bounded) => {
                    bounded.set_ratio(r32(1.0));
                }
            }
        }

        self.camera.center = self.drill.collider.position.as_f32();
        self.bounds = Aabb2::from_corners(
            vec2(-self.config.map_width / r32(2.0), r32(-10000.0)),
            vec2(self.config.map_width / r32(2.0), r32(10000.0)),
        );

        // Spawn minerals
        self.minerals.clear();
        self.depth_generated = self.ground_level;
        self.spawn_depths();
    }

    pub fn spawn_depths(&mut self) {
        let max_depth = r32(self.camera.center.y - self.camera.fov.value() * 2.0);

        let strip_size = r32(0.5);
        while self.depth_generated > max_depth {
            self.generate_strip(self.depth_generated, self.depth_generated - strip_size);
            self.depth_generated -= strip_size;
        }
    }

    fn generate_strip(&mut self, y_max: Coord, y_min: Coord) {
        let mut rng = thread_rng();

        for (&mineral_kind, config) in &self.config.minerals {
            for config in &config.generation {
                let [mut mineral_min, mut mineral_max] = config.range;
                if mineral_min > mineral_max {
                    std::mem::swap(&mut mineral_min, &mut mineral_max);
                }

                if !(mineral_min..=mineral_max).contains(&y_max) {
                    continue;
                }

                let density = config.density;
                let n_spawns = (y_max - y_min) * self.config.map_width * density;
                let n_spawns = n_spawns.floor().as_f32() as usize
                    + rng.gen_bool(n_spawns.fract().as_f32() as f64) as usize;
                for _ in 0..n_spawns {
                    // Spawn a mineral
                    let position = vec2(
                        rng.gen_range(self.bounds.min.x..=self.bounds.max.x),
                        rng.gen_range(y_min..=y_max),
                    );
                    self.minerals.push(Mineral {
                        collider: Collider::circle(position, r32(0.15)),
                        kind: mineral_kind,
                        amount: 1,
                    });
                }
            }
        }
    }
}
