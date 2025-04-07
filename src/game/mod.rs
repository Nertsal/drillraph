use crate::{model::*, prelude::*, render::util::UtilRender, ui::layout::*};

pub struct GameState {
    context: Context,
    util: UtilRender,
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

        self.util.draw_nine_slice(
            self.ui_view,
            palette.ui_view,
            &sprites.border_thinner,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

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
