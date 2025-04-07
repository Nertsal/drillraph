mod generate;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: FloatTime) {
        self.simulation_time += delta_time;
        self.validate_nodes();
        self.update_camera(delta_time);

        match self.phase {
            Phase::Setup => {}
            Phase::Drill => {
                self.move_drill(delta_time);
                self.collide_drill(delta_time);
                self.use_fuel(delta_time);
            }
        }
    }

    pub fn launch_drill(&mut self) {
        let Phase::Setup = self.phase else { return };
        log::debug!("Launch the drill!");
        self.phase = Phase::Drill;
        self.drill.target_speed = self.config.drill_speed;
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

    fn collide_drill(&mut self, delta_time: FloatTime) {}

    fn use_fuel(&mut self, delta_time: FloatTime) {}
}
