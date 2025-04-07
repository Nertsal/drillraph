use super::*;

impl Model {
    pub fn generate_level(&mut self) {
        log::debug!("Generating next level..");

        // Reset drill
        self.drill.position = vec2::ZERO;
        self.drill.rotation = Angle::from_degrees(r32(-90.0));
        self.camera.center = self.drill.position.as_f32();

        // Spawn minerals
        self.minerals.clear();
        self.depth_generated = Coord::ZERO;
        self.spawn_depths();
    }

    pub fn spawn_depths(&mut self) {
        let max_depth = r32(self.camera.center.y - self.camera.fov.value() * 2.0);

        let strip_size = r32(1.0);
        while self.depth_generated > max_depth {
            self.generate_strip(self.depth_generated, self.depth_generated - strip_size);
            self.depth_generated -= strip_size;
        }
    }

    fn generate_strip(&mut self, y_max: Coord, y_min: Coord) {
        let mut rng = thread_rng();
    }
}
