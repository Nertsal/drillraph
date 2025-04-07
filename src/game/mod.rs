use crate::{model::*, prelude::*, render::util::UtilRender, ui::layout::*};

pub struct GameState {
    context: Context,
    util: UtilRender,

    model: Model,

    screen: Aabb2<f32>,
    ui_view: Aabb2<f32>,
    game_view: Aabb2<f32>,
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(),

            util: UtilRender::new(context.clone()),
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
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.layout(framebuffer.size());

        let pixel_scale = framebuffer.size().as_f32() / crate::TARGET_SCREEN_SIZE.as_f32();
        let pixel_scale = pixel_scale.x.min(pixel_scale.y).floor();

        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;
        ugli::clear(framebuffer, Some(palette.background), None, None);

        self.util.draw_nine_slice(
            self.ui_view,
            palette.ui_view,
            &sprites.border_thinner,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        self.util.draw_nine_slice(
            self.game_view,
            palette.game_view,
            &sprites.border_thinner,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
    }
}
