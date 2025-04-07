use crate::{model::*, prelude::*, render::util::UtilRender, ui::layout::*};

pub struct GameState {
    context: Context,
    util: UtilRender,
    ui_texture: ugli::Texture,
    game_texture: ugli::Texture,

    model: Model,

    screen: Aabb2<f32>,
    ui_view: Aabb2<f32>,
    game_view: Aabb2<f32>,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(context.clone()),

            util: UtilRender::new(context.clone()),
            ui_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            game_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            context,

            screen: Aabb2::ZERO,
            ui_view: Aabb2::ZERO,
            game_view: Aabb2::ZERO,
        }
    }

    fn layout(&mut self, framebuffer_size: vec2<usize>) {
        self.screen = Aabb2::ZERO.extend_positive(framebuffer_size.as_f32());
        let padding = 20.0;
        self.game_view = self.screen.extend_uniform(-padding);
        self.ui_view = self.game_view.split_left(0.66).extend_right(-padding / 2.0);
        self.game_view = self.game_view.extend_left(-padding / 2.0);
    }

    fn draw_game(&mut self, pixel_scale: f32) {
        geng_utils::texture::update_texture_size(
            &mut self.game_texture,
            (self.game_view.size() / pixel_scale).map(|x| x.floor() as usize),
            self.context.geng.ugli(),
        );

        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;
        let model = &self.model;

        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.game_texture,
            self.context.geng.ugli(),
        );
        ugli::clear(framebuffer, Some(palette.background), None, None);

        // Minerals
        for mineral in &model.minerals {
            let color = match mineral.kind {
                MineralKind::Resource(kind) => palette
                    .resources
                    .get(&kind)
                    .copied()
                    .unwrap_or(palette.default),
                MineralKind::Rock => palette.rock,
            };
            self.util
                .draw_collider(&mineral.collider, color, &model.camera, framebuffer);
        }

        // Drill
        self.util
            .draw_collider(&model.drill, palette.drill, &model.camera, framebuffer);

        // Drill vision
        self.context.geng.draw2d().circle_with_cut(
            framebuffer,
            &model.camera,
            model.drill.position.as_f32(),
            model.vision_radius.as_f32() * 0.97,
            model.vision_radius.as_f32(),
            palette.vision_circle,
        );
    }

    fn draw_nodes(&mut self, pixel_scale: f32) {
        geng_utils::texture::update_texture_size(
            &mut self.ui_texture,
            self.ui_view.size().map(|x| x.floor() as usize),
            self.context.geng.ugli(),
        );
        let ui_size = self.ui_texture.size();
        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.ui_texture,
            self.context.geng.ugli(),
        );

        let nodes = &self.model.nodes;
        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;

        let to_screen = |pos: vec2<Coord>| {
            crate::util::world_to_screen(&nodes.camera, ui_size.as_f32(), pos.as_f32())
        };

        for (node_i, node) in nodes.nodes.iter().enumerate() {
            // Body
            let color = match &node.kind {
                NodeKind::Power => palette.nodes.power,
                NodeKind::Fuel(..) => palette.nodes.fuel,
            };
            let position =
                Aabb2::from_corners(to_screen(node.position.min), to_screen(node.position.max));
            self.util.draw_nine_slice(
                position,
                color,
                &sprites.border_thinner,
                pixel_scale,
                &geng::PixelPerfectCamera,
                framebuffer,
            );

            // Connections
            for connection in &node.connections {
                let color = palette
                    .nodes
                    .connections
                    .get(&connection.color)
                    .copied()
                    .unwrap_or(palette.default);
                self.util.draw_circle_cut(
                    framebuffer,
                    &nodes.camera,
                    mat3::translate((node.position.center() + connection.offset).as_f32())
                        * mat3::scale_uniform(0.1),
                    color,
                    0.0,
                );

                let mut draw_connection = || -> Option<()> {
                    let node_j = connection.connected_to?;
                    if node_i > node_j {
                        return None;
                    }
                    let from = node.position.center() + connection.offset;
                    let to_node = nodes.nodes.get(node_j)?;
                    let to_conn = to_node
                        .connections
                        .iter()
                        .find(|conn| conn.connected_to == Some(node_i))?;
                    let to = to_node.position.center() + to_conn.offset;
                    self.context.geng.draw2d().draw2d(
                        framebuffer,
                        &nodes.camera,
                        &draw2d::Segment::new(Segment(from.as_f32(), to.as_f32()), 0.1, color),
                    );

                    Some(())
                };
                draw_connection();
            }
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.layout(framebuffer.size());

        let pixel_scale = framebuffer.size().as_f32() / crate::TARGET_SCREEN_SIZE.as_f32();
        let pixel_scale = pixel_scale.x.min(pixel_scale.y).floor();

        let context = self.context.clone();
        let palette = &context.assets.palette;
        let sprites = &context.assets.sprites;
        ugli::clear(framebuffer, Some(palette.background), None, None);

        // Ui
        self.draw_nodes(pixel_scale);
        let target = self.util.draw_texture_pp(
            &self.ui_texture,
            self.ui_view.center(),
            vec2(0.5, 0.5),
            Angle::ZERO,
            1.0,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        self.util.draw_nine_slice(
            target,
            palette.ui_view,
            &sprites.border_thinner,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Game
        self.draw_game(pixel_scale);
        let target = self.util.draw_texture_pp(
            &self.game_texture,
            self.game_view.center(),
            vec2(0.5, 0.5),
            Angle::ZERO,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        self.util.draw_nine_slice(
            target,
            palette.game_view,
            &sprites.border_thinner,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
    }
}
