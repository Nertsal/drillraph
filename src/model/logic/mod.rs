mod generate;

use super::*;

impl Model {
    pub fn update(&mut self, delta_time: FloatTime) {
        self.simulation_time += delta_time;
        self.validate_nodes();
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
}
