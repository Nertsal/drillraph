mod generate;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: FloatTime) {
        self.simulation_time += delta_time;
        self.update_nodes(delta_time);

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

        // Check prerequisites

        // 2. Drill has power
        if !self.nodes.nodes.iter().all(|node| {
            if let NodeKind::Drill { power, .. } = &node.kind {
                power.is_max()
            } else {
                true
            }
        }) {
            log::debug!("Launch impossible: drill does not have enough power");
            return;
        }

        // 1. At least one fuel
        {
            let mut to_check = VecDeque::new();
            to_check.push_front(0);
            let mut checked = HashSet::new();
            let mut has_fuel = false;
            while let Some(i) = to_check.pop_front() {
                if !checked.insert(i) {
                    continue;
                }
                let Some(node) = self.nodes.nodes.get(i) else {
                    return;
                };
                if let NodeKind::Fuel(..) = node.kind {
                    has_fuel = true;
                    break;
                }
                for conn in &node.connections {
                    if let Some(to) = conn.connected_to {
                        to_check.push_back(to);
                    }
                }
            }
            if !has_fuel {
                log::debug!("Launch impossible: no fuel connected");
                return;
            }
        }

        log::debug!("Launch the drill!");
        self.phase = Phase::Drill;
        self.drill.target_speed = self.config.drill_speed;
    }

    pub fn start_sprint(&mut self, node_i: usize) {
        let Phase::Drill = self.phase else { return };

        if self.drill.sprint.is_some() {
            return;
        };

        let Some(node) = self.nodes.nodes.get_mut(node_i) else {
            return;
        };

        let NodeKind::Sprint { cooldown } = &mut node.kind else {
            return;
        };

        if cooldown.is_above_min() {
            return;
        }

        cooldown.set_ratio(r32(1.0));
        self.drill.sprint = Some(DrillSprint {
            caused_by_node: node_i,
            duration: Bounded::new_max(self.config.sprint_duration),
        });
        self.drill.speed += self.config.sprint_boost;
    }

    pub fn purchase_item(&mut self, index: usize) {
        let Phase::Setup = self.phase else { return };

        let Some(item) = self.shop.get(index) else {
            return;
        };

        if item.item.cost > self.money {
            return; // Cannot afford
        }

        let item = self.shop.remove(index);
        self.money -= item.item.cost;
        let shop = match item.tier {
            0 => &mut self.config.shop_0,
            1 => &mut self.config.shop_1,
            _ => &mut self.config.shop_2,
        };
        if let Some(item) = shop.items.get_mut(item.index) {
            item.sold_out = true;
        }

        let kind = match item.item.node {
            ShopNode::FuelSmall => NodeKind::Fuel(Bounded::new_max(self.config.fuel_small_amount)),
            ShopNode::Fuel => NodeKind::Fuel(Bounded::new_max(self.config.fuel_normal_amount)),
            ShopNode::TurnLeft => NodeKind::TurnLeft,
            ShopNode::TurnRight => NodeKind::TurnRight,
            ShopNode::Battery => NodeKind::Battery,
            ShopNode::Upgrade => NodeKind::Upgrade,
            ShopNode::Speed => NodeKind::Speed { level: 0 },
            ShopNode::Light => NodeKind::Vision { level: 0 },
            ShopNode::Sprint => NodeKind::Sprint {
                cooldown: Bounded::new_zero(r32(1.0)),
            },
        };

        let position = self.nodes.bounds.center();
        let position = Aabb2::point(position).extend_symmetric(vec2(1.0, 1.0).as_r32() / r32(2.0));
        let mk_cons = |cons: &[((f32, f32), ConnectionKind)]| {
            cons.iter()
                .map(|&((x, y), kind)| NodeConnection {
                    offset: vec2(x, y).as_r32(),
                    kind,
                    connected_to: None,
                })
                .collect::<Vec<_>>()
        };
        self.nodes.nodes.push(Node {
            position,
            kind,
            connections: match item.item.node {
                ShopNode::FuelSmall | ShopNode::Fuel => {
                    mk_cons(&[((0.0, 0.5), ConnectionKind::Fuel)])
                }
                ShopNode::TurnLeft | ShopNode::TurnRight | ShopNode::Battery | ShopNode::Sprint => {
                    mk_cons(&[
                        ((0.0, 0.5), ConnectionKind::Normal),
                        ((1.0, 0.5), ConnectionKind::Normal),
                    ])
                }
                ShopNode::Upgrade => mk_cons(&[
                    ((0.5, 0.0), ConnectionKind::Upgrade),
                    ((0.5, 1.0), ConnectionKind::Upgrade),
                ]),
                ShopNode::Speed | ShopNode::Light => mk_cons(&[
                    ((0.0, 0.5), ConnectionKind::Modifier),
                    ((0.5, 1.0), ConnectionKind::Upgrade),
                    ((1.0, 0.5), ConnectionKind::Fuel),
                ]),
            },
        });
    }

    fn end_drill_phase(&mut self) {
        let Phase::Drill = self.phase else { return };
        log::debug!("Ending drill phase");
        self.phase = Phase::Setup;
        self.generate_level();
    }

    fn update_nodes(&mut self, delta_time: FloatTime) {
        let bounds = self.nodes.bounds;
        let mut shop_i = 0;
        let mut drill_i = 0;
        let mut vision_i = None;
        let mut speed_i = None;
        for (node_i, node) in self.nodes.nodes.iter_mut().enumerate() {
            let offset = (bounds.min - node.position.min).map(|x| x.max(Coord::ZERO));
            node.position = node.position.translate(offset);
            let offset = (bounds.max - node.position.max).map(|x| x.min(Coord::ZERO));
            node.position = node.position.translate(offset);

            match node.kind {
                NodeKind::Shop { .. } => shop_i = node_i,
                NodeKind::Drill { .. } => drill_i = node_i,
                NodeKind::Vision { .. } => vision_i = Some(node_i),
                NodeKind::Speed { .. } => speed_i = Some(node_i),
                _ => {}
            }

            if let Phase::Drill = self.phase {
                if let NodeKind::Sprint { cooldown } = &mut node.kind {
                    if self.drill.sprint.is_none() {
                        cooldown.change(-delta_time);
                    }
                }
            }
        }

        // Count upgrades
        enum CountNode {
            Upgrade,
            Battery,
        }
        let count_nodes = |nodes: &Nodes, index: usize, kind: CountNode| -> usize {
            let mut to_check = VecDeque::new();
            to_check.push_front(index);
            let mut checked = HashSet::new();
            let mut upgrades = 0;
            while let Some(i) = to_check.pop_front() {
                if !checked.insert(i) {
                    continue;
                }
                let Some(node) = nodes.nodes.get(i) else {
                    continue;
                };
                match (&kind, &node.kind) {
                    (CountNode::Upgrade, NodeKind::Upgrade)
                    | (CountNode::Battery, NodeKind::Battery) => upgrades += 1,
                    _ => {}
                }
                for conn in &node.connections {
                    if let Some(to) = conn.connected_to {
                        to_check.push_back(to);
                    }
                }
            }
            upgrades
        };

        // Update shop level
        let shop_upgrades = count_nodes(&self.nodes, shop_i, CountNode::Upgrade);
        if let Some(node) = self.nodes.nodes.get_mut(shop_i) {
            if let NodeKind::Shop { level } = &mut node.kind {
                *level = shop_upgrades;
            }
        }

        // Update drill level
        let drill_upgrades = count_nodes(&self.nodes, drill_i, CountNode::Upgrade);
        let drill_batteries = count_nodes(&self.nodes, drill_i, CountNode::Battery);
        if let Some(node) = self.nodes.nodes.get_mut(drill_i) {
            if let NodeKind::Drill { level, power } = &mut node.kind {
                *level = match drill_upgrades {
                    0 => ResourceKind::Iron,
                    1 => ResourceKind::Bronze,
                    2 => ResourceKind::Silver,
                    _ => ResourceKind::Gold,
                };
                *power = Bounded::new(drill_batteries, 0..=drill_upgrades);
                self.drill.drill_level = *level;
            }
        }

        // Update vision level
        if let Some(vision_i) = vision_i {
            let vision_upgrades = count_nodes(&self.nodes, vision_i, CountNode::Upgrade);
            if let Some(node) = self.nodes.nodes.get_mut(vision_i) {
                if let NodeKind::Vision { level } = &mut node.kind {
                    *level = vision_upgrades;
                    self.drill.vision_radius = match *level {
                        0 => self.config.vision,
                        1 => self.config.vision_0,
                        2 => self.config.vision_1,
                        _ => self.config.vision_2,
                    };
                }
            }
        }

        // Update speed level
        if let Some(speed_i) = speed_i {
            let speed_upgrades = count_nodes(&self.nodes, speed_i, CountNode::Upgrade);
            if let Some(node) = self.nodes.nodes.get_mut(speed_i) {
                if let NodeKind::Speed { level } = &mut node.kind {
                    *level = speed_upgrades;
                    self.drill.max_speed = match *level {
                        0 => self.config.drill_speed,
                        1 => self.config.drill_speed_0,
                        2 => self.config.drill_speed_1,
                        _ => self.config.drill_speed_2,
                    };
                }
            }
        }
    }

    fn update_camera(&mut self, _delta_time: FloatTime) {
        self.camera.center = self.drill.collider.position.as_f32();
    }

    fn move_drill(&mut self, delta_time: FloatTime) {
        // Move and accelerate
        self.drill.target_speed = if self.drill.sprint.is_some() {
            self.drill.max_speed + self.config.sprint_boost
        } else {
            self.drill.max_speed
        };
        self.drill.speed += (self.drill.target_speed - self.drill.speed)
            .clamp_abs(self.config.drill_acceleration * delta_time);
        self.drill.collider.position +=
            self.drill.collider.rotation.unit_vec() * self.drill.speed * delta_time;

        // Update sprint
        if let Some(sprint) = &mut self.drill.sprint {
            if let Some(node) = self.nodes.nodes.get_mut(sprint.caused_by_node) {
                if let NodeKind::Sprint { cooldown } = &mut node.kind {
                    cooldown.set_ratio(r32(1.0));
                }
            }

            sprint.duration.change(-delta_time);
            if sprint.duration.is_min() {
                self.drill.sprint = None;
            }
        }
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
