mod generate;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: FloatTime) {
        self.simulation_time += delta_time;
        self.validate_nodes();

        match self.phase {
            Phase::Setup => {}
            Phase::Drill => {
                self.move_drill(delta_time);
                self.collide_drill(delta_time);
                self.use_fuel(delta_time);
                self.spawn_depths();
            }
        }

        self.update_camera(delta_time);
        self.process_particles(delta_time);
    }

    pub fn launch_drill(&mut self) {
        let Phase::Setup = self.phase else { return };
        log::debug!("Launch the drill!");
        self.phase = Phase::Drill;
        self.drill.target_speed = self.config.drill_speed;
    }

    pub fn purchase_item(&mut self, index: usize) {
        let Phase::Setup = self.phase else { return };

        if self.shop.len() <= index {
            return;
        }
        let item = self.shop.remove(index);

        let kind = match item.node {
            ShopNode::FuelSmall => NodeKind::Fuel(Bounded::new_max(self.config.fuel_small_amount)),
            ShopNode::Fuel => NodeKind::Fuel(Bounded::new_max(self.config.fuel_normal_amount)),
        };

        let position = self.nodes.bounds.center();
        let position = Aabb2::point(position).extend_symmetric(vec2(1.0, 1.0).as_r32() / r32(2.0));
        self.nodes.nodes.push(Node {
            position,
            kind,
            connections: vec![],
        });
    }

    fn end_drill_phase(&mut self) {
        let Phase::Drill = self.phase else { return };
        log::debug!("Ending drill phase");
        self.phase = Phase::Setup;
        self.generate_level();
    }

    fn validate_nodes(&mut self) {
        let bounds = self.nodes.bounds;
        for node in &mut self.nodes.nodes {
            let offset = (bounds.min - node.position.min).map(|x| x.max(Coord::ZERO));
            node.position = node.position.translate(offset);

            let offset = (bounds.max - node.position.max).map(|x| x.min(Coord::ZERO));
            node.position = node.position.translate(offset);
        }
    }

    fn update_camera(&mut self, _delta_time: FloatTime) {
        self.camera.center = self.drill.collider.position.as_f32();
    }

    fn move_drill(&mut self, delta_time: FloatTime) {
        self.drill.speed += (self.drill.target_speed - self.drill.speed)
            .clamp_abs(self.config.drill_acceleration * delta_time);
        self.drill.collider.position +=
            self.drill.collider.rotation.unit_vec() * self.drill.speed * delta_time;
    }

    fn collide_drill(&mut self, _delta_time: FloatTime) {
        let mut collected = Vec::new();
        let mut collisions = HashSet::new();
        for (i, mineral) in self.minerals.iter().enumerate() {
            if !mineral.collider.check(&self.drill.collider) {
                continue;
            }
            collisions.insert(i);
            if self.drill.colliding_with.contains(&i) {
                continue;
            }

            match mineral.kind {
                MineralKind::Resource(kind) => {
                    if kind <= self.drill.drill_level {
                        // Collect
                        collected.push(i);
                    } else {
                        // Bounce
                        self.drill.speed = r32(0.5);
                    }
                }
                MineralKind::Rock => {
                    // Bounce
                    self.drill.speed = r32(0.5);
                }
            }
        }
        self.drill.colliding_with = collisions;

        let mut rng = thread_rng();
        let palette = &self.palette;
        for i in collected.into_iter().rev() {
            let mineral = self.minerals.swap_remove(i);
            if let Some(config) = self.config.minerals.get(&mineral.kind) {
                let value = mineral.amount * config.value;
                let position = rng.gen_circle(mineral.collider.position, r32(0.2));
                let speed = r32(0.5);
                let velocity =
                    Angle::from_degrees(r32(rng.gen_range(60.0..=120.0))).unit_vec() * speed;
                self.money += value;
                self.floating_texts.insert(FloatingText {
                    text: format!("+{}", value).into(),
                    position,
                    velocity,
                    size: r32(1.0),
                    color: palette.gold_text,
                    lifetime: Bounded::new_max(r32(1.0)),
                });
            }
        }
    }

    fn use_fuel(&mut self, delta_time: FloatTime) {
        let mut checked = HashSet::new();
        let mut to_check = VecDeque::new();
        to_check.push_front(0);
        while let Some(i) = to_check.pop_front() {
            if checked.contains(&i) {
                continue;
            }
            let Some(node) = self.nodes.nodes.get_mut(i) else {
                continue;
            };

            for conn in &node.connections {
                if let Some(i) = conn.connected_to {
                    to_check.push_back(i);
                }
            }

            if let NodeKind::Fuel(fuel) = &mut node.kind {
                if fuel.is_above_min() {
                    fuel.change(-delta_time);
                    return;
                }
            }

            checked.insert(i);
        }

        // Out of fuel
        self.end_drill_phase();
    }

    fn process_particles(&mut self, delta_time: FloatTime) {
        // Floating texts
        let mut dead_ids = Vec::new();
        for (id, position, velocity, lifetime) in query!(
            self.floating_texts,
            (id, &mut position, &velocity, &mut lifetime)
        ) {
            *position += *velocity * delta_time;
            lifetime.change(-delta_time);
            if lifetime.is_min() {
                dead_ids.push(id);
            }
        }
        for id in dead_ids {
            self.floating_texts.remove(id);
        }

        // Particles
        let mut dead_ids = Vec::new();
        for (id, position, velocity, lifetime) in query!(
            self.particles,
            (id, &mut position, &velocity, &mut lifetime)
        ) {
            *position += *velocity * delta_time;
            lifetime.change(-delta_time);
            if lifetime.is_min() {
                dead_ids.push(id);
            }
        }
        for id in dead_ids {
            self.particles.remove(id);
        }
        let spawn = self.particles_queue.drain(..).flat_map(spawn_particles);
        for particle in spawn {
            self.particles.insert(particle);
        }
    }
}
