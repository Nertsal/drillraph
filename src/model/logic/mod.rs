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
            }
        }

        self.update_camera(delta_time);
    }

    pub fn launch_drill(&mut self) {
        let Phase::Setup = self.phase else { return };
        log::debug!("Launch the drill!");
        self.phase = Phase::Drill;
        self.drill.target_speed = self.config.drill_speed;
    }

    fn end_drill_phase(&mut self) {
        let Phase::Drill = self.phase else { return };
        log::debug!("Ending drill phase");
        self.phase = Phase::Setup;
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
        self.drill.speed +=
            (self.drill.target_speed - self.drill.speed).clamp_abs(self.config.drill_acceleration);
        self.drill.collider.position +=
            self.drill.collider.rotation.unit_vec() * self.drill.speed * delta_time;
    }

    fn collide_drill(&mut self, _delta_time: FloatTime) {}

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
}
