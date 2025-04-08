use crate::{
    model::*,
    prelude::*,
    render::{
        mask::MaskedRender,
        util::{TextRenderOptions, UtilRender},
    },
    ui::layout::*,
};

use geng_utils::key::EventKey;

#[derive(geng::asset::Load, Serialize, Deserialize, Debug, Clone)]
#[load(serde = "ron")]
pub struct Controls {
    pub launch: Vec<EventKey>,
    pub left: Vec<EventKey>,
    pub right: Vec<EventKey>,
}

pub struct GameState {
    context: Context,
    util: UtilRender,
    ui_texture: ugli::Texture,
    game_texture: ugli::Texture,
    mask: MaskedRender,
    screen_texture: ugli::Texture,

    real_time: FloatTime,
    cursor_screen_pos: vec2<f64>,
    cursor_ui_pos: vec2<Coord>,
    cursor_game_pos: vec2<Coord>,

    model: Model,
    hovering: Option<DragTarget>,
    drag: Option<Drag>,
    turn_input: R32,

    screen: Aabb2<f32>,
    ui_view: Aabb2<f32>,
    game_view: Aabb2<f32>,

    show_shop: bool,
    shop_view: Aabb2<f32>,
    shop_items: Vec<Aabb2<f32>>,
}

#[derive(Debug)]
pub struct Drag {
    pub from_screen: vec2<f64>,
    pub from_ui: vec2<Coord>,
    pub target: DragTarget,
}

#[derive(Debug, Clone)]
pub enum DragTarget {
    Node {
        index: usize,
        from_position: vec2<Coord>,
    },
    NodeConnection {
        node: usize,
        conn: usize,
    },
    Shop {
        item: usize,
    },
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(context.clone()),
            hovering: None,
            drag: None,
            turn_input: R32::ZERO,

            real_time: FloatTime::ZERO,
            cursor_screen_pos: vec2::ZERO,
            cursor_ui_pos: vec2::ZERO,
            cursor_game_pos: vec2::ZERO,

            screen: Aabb2::ZERO,
            ui_view: Aabb2::ZERO,
            game_view: Aabb2::ZERO,

            show_shop: false,
            shop_view: Aabb2::ZERO,
            shop_items: Vec::new(),

            util: UtilRender::new(context.clone()),
            ui_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            game_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            mask: MaskedRender::new(&context.geng, &context.assets, vec2(1, 1)),
            screen_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            context,
        }
    }

    fn layout(&mut self, pixel_scale: f32, framebuffer_size: vec2<usize>) {
        self.screen = Aabb2::ZERO.extend_positive(framebuffer_size.as_f32());
        let padding = 20.0;
        self.game_view = self.screen.extend_uniform(-padding);
        self.ui_view = self.game_view.split_left(0.66).extend_right(-padding / 2.0);
        self.game_view = self.game_view.extend_left(-padding / 2.0);

        let shop_size = 4.0 * 50.0 * pixel_scale;
        self.shop_view = self
            .ui_view
            .with_width(shop_size, 0.5)
            .with_height(shop_size, 0.5);
    }

    fn draw_game(&mut self, pixel_scale: f32) {
        let size = (self.game_view.size() / pixel_scale).map(|x| x.floor() as usize);
        geng_utils::texture::update_texture_size(
            &mut self.game_texture,
            size,
            self.context.geng.ugli(),
        );
        self.mask.update_size(size);

        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;
        let model = &self.model;

        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.game_texture,
            self.context.geng.ugli(),
        );
        ugli::clear(framebuffer, Some(palette.background), None, None);

        // Background
        ugli::draw(
            framebuffer,
            &self.context.assets.shaders.tiled_texture,
            ugli::DrawMode::TriangleFan,
            &self.util.unit_quad,
            (
                ugli::uniforms! {
                    u_texture: &*sprites.drill_background_largedot_purple,
                    u_offset: vec2(0.0, 0.0),
                    u_scale: vec2(framebuffer.size().as_f32().aspect(), 1.0) * 3.0,
                },
                model.camera.uniforms(framebuffer.size().as_f32()),
            ),
            ugli::DrawParameters::default(),
        );

        // Vision mask
        let mut mask = self.mask.start();
        self.context.geng.draw2d().circle(
            &mut mask.mask,
            &model.camera,
            model.drill.collider.position.as_f32(),
            model.drill.vision_radius.as_f32(),
            Color::WHITE,
        );

        // Minerals
        for mineral in &model.minerals {
            let color = match mineral.kind {
                MineralKind::Resource(kind) => {
                    let texture = match kind {
                        ResourceKind::Coal => &sprites.coal_ore,
                        ResourceKind::Iron => &sprites.iron_ore,
                        ResourceKind::Bronze => &sprites.bronze_ore,
                        ResourceKind::Silver => &sprites.silver_ore,
                        ResourceKind::Gold => &sprites.gold_ore,
                        ResourceKind::Gem => continue, // &sprites.gem,
                    };
                    self.util.draw_texture_pp(
                        texture,
                        mineral.collider.position.as_f32(),
                        vec2(0.5, 0.5),
                        mineral.collider.rotation.as_f32(),
                        1.0,
                        Color::WHITE,
                        &model.camera,
                        &mut mask.color,
                    );
                    continue;
                }
                MineralKind::Rock => palette.rock,
            };
            self.util
                .draw_collider(&mineral.collider, color, &model.camera, &mut mask.color);
        }

        self.mask.draw(ugli::DrawParameters::default(), framebuffer);

        // Level bounds
        self.context.geng.draw2d().draw2d(
            framebuffer,
            &model.camera,
            &draw2d::Segment::new(
                Segment(
                    vec2(model.bounds.min.x.as_f32(), model.camera.center.y + 20.0),
                    vec2(model.bounds.min.x.as_f32(), model.camera.center.y - 20.0),
                ),
                0.2,
                palette.wall,
            ),
        );
        self.context.geng.draw2d().draw2d(
            framebuffer,
            &model.camera,
            &draw2d::Segment::new(
                Segment(
                    vec2(model.bounds.max.x.as_f32(), model.camera.center.y + 20.0),
                    vec2(model.bounds.max.x.as_f32(), model.camera.center.y - 20.0),
                ),
                0.2,
                palette.wall,
            ),
        );

        // Drill
        self.util.draw_texture_pp(
            &sprites.drill,
            model.drill.collider.position.as_f32(),
            vec2(0.5, 0.5),
            model.drill.collider.rotation.as_f32() + Angle::from_degrees(90.0),
            1.0,
            Color::WHITE,
            &model.camera,
            framebuffer,
        );

        // Drill vision
        self.context.geng.draw2d().circle_with_cut(
            framebuffer,
            &model.camera,
            model.drill.collider.position.as_f32(),
            model.drill.vision_radius.as_f32() * 0.97,
            model.drill.vision_radius.as_f32(),
            palette.vision_circle,
        );

        // Floating Text
        for (text, position, size, color, lifetime) in query!(
            self.model.floating_texts,
            (&text, &position, &size, &color, &lifetime)
        ) {
            let t = lifetime.get_ratio().as_f32().sqrt();
            self.util.draw_text(
                text,
                position.as_f32(),
                &self.context.assets.fonts.revolver_game,
                TextRenderOptions::new(size.as_f32() * t).color(*color),
                &model.camera,
                framebuffer,
            );
        }
    }

    fn draw_game_ui(&mut self, pixel_scale: f32) {
        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.screen_texture,
            self.context.geng.ugli(),
        );

        let font_size = 25.0 * pixel_scale;
        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;

        // Depth meter
        let depth = -self.model.drill.collider.position.y.as_f32().ceil() as i64;
        let pos = self.game_view.top_right() - vec2(0.5, 0.5) * font_size;
        self.util.draw_text(
            "DEPTH",
            pos,
            &self.context.assets.fonts.revolver_display,
            TextRenderOptions::new(font_size)
                .align(vec2(1.0, 1.0))
                .color(palette.game_view),
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        self.util.draw_text(
            format!("{}", depth),
            pos - vec2(0.0, 0.9) * font_size,
            &self.context.assets.fonts.revolver_game,
            TextRenderOptions::new(font_size)
                .align(vec2(1.0, 1.0))
                .color(palette.depth_text),
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        // Coins
        let pos = self.ui_view.top_right() - vec2(1.5, 0.5) * font_size;
        self.util.draw_texture_pp(
            &sprites.coin,
            pos - vec2(0.5, 0.0) * font_size,
            vec2(1.0, 0.5),
            Angle::ZERO,
            pixel_scale,
            Color::WHITE,
            &geng::PixelPerfectCamera,
            framebuffer,
        );
        self.util.draw_text(
            format!("{}", self.model.money),
            pos,
            &self.context.assets.fonts.revolver_game,
            TextRenderOptions::new(font_size)
                .align(vec2(0.0, 0.5))
                .color(palette.gold_text),
            &geng::PixelPerfectCamera,
            framebuffer,
        );
    }

    fn draw_nodes(&mut self, pixel_scale: f32) {
        let size = self.ui_view.size().map(|x| x.floor() as usize);
        geng_utils::texture::update_texture_size(
            &mut self.ui_texture,
            size,
            self.context.geng.ugli(),
        );
        let ui_size = self.ui_texture.size();
        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.ui_texture,
            self.context.geng.ugli(),
        );

        let nodes = &mut self.model.nodes;
        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;

        ugli::clear(framebuffer, Some(palette.background), None, None);

        // Background
        let offset = vec2(10.0, 10.0) * self.real_time.as_f32();
        ugli::draw(
            framebuffer,
            &self.context.assets.shaders.tiled_texture,
            ugli::DrawMode::TriangleFan,
            &self.util.unit_quad,
            (
                ugli::uniforms! {
                    u_texture: &*sprites.drill_background_green,
                    u_offset: offset,
                    u_scale: vec2(framebuffer.size().as_f32().aspect(), 1.0) * 3.0,
                },
                geng::PixelPerfectCamera.uniforms(framebuffer.size().as_f32()),
            ),
            ugli::DrawParameters::default(),
        );

        let to_screen = |pos: vec2<Coord>| {
            crate::util::world_to_screen(&nodes.camera, ui_size.as_f32(), pos.as_f32())
        };
        let to_world =
            |pos: vec2<f32>| nodes.camera.screen_to_world(ui_size.as_f32(), pos).as_r32();

        for node in &mut nodes.nodes {
            // Body
            let texture = match &node.kind {
                NodeKind::Power => &sprites.power_node,
                NodeKind::Shop { .. } => &sprites.shop_0_node,
                NodeKind::Fuel(fuel) => {
                    if fuel.max() == self.context.assets.config.fuel_small_amount {
                        &sprites.fuel_small_node
                    } else {
                        &sprites.fuel_normal_node
                    }
                }
                NodeKind::TurnLeft | NodeKind::TurnRight => &sprites.turn_node,
                NodeKind::Sprint { .. } => &sprites.sprint_node,
                NodeKind::Upgrade => &sprites.upgrade_node,
                NodeKind::Drill { level, .. } => match level {
                    ResourceKind::Coal | ResourceKind::Iron => &sprites.drill_iron,
                    ResourceKind::Bronze => &sprites.drill_bronze,
                    ResourceKind::Silver => &sprites.drill_silver,
                    ResourceKind::Gold => &sprites.drill_gold,
                    ResourceKind::Gem => &sprites.drill_gold,
                },
                NodeKind::Battery => &sprites.battery_node,
                NodeKind::Vision { level } => match level {
                    0 => &sprites.drill_0_light,
                    1 => &sprites.drill_1_light,
                    _ => &sprites.drill_2_light,
                },
                NodeKind::Speed { level } => match level {
                    0 => &sprites.drill_0_speed,
                    1 => &sprites.drill_1_speed,
                    _ => &sprites.drill_2_speed,
                },
                NodeKind::CoalFuel(..) => &sprites.coal_fuel_node,
            };
            let position = node.position.map_bounds(to_screen);
            let position = self.util.draw_texture_pp(
                texture,
                position.center(),
                vec2(0.5, 0.5),
                Angle::ZERO,
                pixel_scale,
                Color::WHITE,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            node.position = Aabb2::point(node.position.center())
                .extend_symmetric(position.map_bounds(to_world).size() / r32(2.0));
        }

        for (node_i, node) in nodes.nodes.iter().enumerate() {
            let is_hovered = matches!(
                self.hovering,
                Some(DragTarget::Node { index, .. }) if index == node_i
            );
            let is_pressed = is_hovered
                && self
                    .context
                    .geng
                    .window()
                    .is_button_pressed(geng::MouseButton::Left)
                || matches!(
                    self.drag.as_ref().map(|drag|&drag.target),
                    Some(DragTarget::Node { index, .. }) if *index == node_i
                );

            let position = node.position.map_bounds(to_screen);

            // Connections
            for (conn_i, connection) in node.connections.iter().enumerate() {
                let color = palette
                    .nodes
                    .connections
                    .get(&connection.kind)
                    .copied()
                    .unwrap_or(palette.default);
                self.util.draw_texture_pp(
                    &sprites.connect_dot,
                    position.align_pos(connection.offset.as_f32()),
                    vec2(0.5, 0.5),
                    Angle::ZERO,
                    pixel_scale,
                    color,
                    &geng::PixelPerfectCamera,
                    framebuffer,
                );

                let mut draw_connection = |nodes: &Nodes| -> Option<()> {
                    let node_j = connection.connected_to?;
                    if node_i > node_j {
                        return None;
                    }
                    let from = node.position.align_pos(connection.offset);
                    let to_node = nodes.nodes.get(node_j)?;
                    let to_conn = to_node
                        .connections
                        .iter()
                        .find(|conn| conn.connected_to == Some(node_i))?;
                    let to = to_node.position.align_pos(to_conn.offset);
                    self.context.geng.draw2d().draw2d(
                        framebuffer,
                        &nodes.camera,
                        &draw2d::Segment::new(Segment(from.as_f32(), to.as_f32()), 0.1, color),
                    );

                    Some(())
                };
                draw_connection(nodes);

                if let Some(DragTarget::NodeConnection {
                    node: drag_node,
                    conn: drag_conn,
                }) = self.drag.as_ref().map(|drag| &drag.target)
                {
                    if *drag_node == node_i && *drag_conn == conn_i {
                        let from = node.position.align_pos(connection.offset);
                        let to = self.cursor_ui_pos;
                        self.context.geng.draw2d().draw2d(
                            framebuffer,
                            &nodes.camera,
                            &draw2d::Segment::new(Segment(from.as_f32(), to.as_f32()), 0.1, color),
                        );
                    }
                }
            }

            let node_button = |normal, pressed, framebuffer: &mut ugli::Framebuffer| {
                let texture = if is_pressed { pressed } else { normal };
                let mut pixel_scale = pixel_scale;
                if !is_pressed && is_hovered {
                    pixel_scale *= 1.25;
                }
                self.util.draw_texture_pp(
                    texture,
                    position.center(),
                    vec2(0.5, 0.5),
                    Angle::ZERO,
                    pixel_scale,
                    Color::WHITE,
                    &geng::PixelPerfectCamera,
                    framebuffer,
                );
            };
            match &node.kind {
                NodeKind::Power => {
                    node_button(
                        &sprites.power_button_normal,
                        &sprites.power_button_pressed,
                        framebuffer,
                    );
                }
                &NodeKind::Shop { level } => {
                    let (normal, pressed) = match level {
                        0 => (
                            &sprites.shop_0_button_normal,
                            &sprites.shop_0_button_pressed,
                        ),
                        1 => (
                            &sprites.shop_1_button_normal,
                            &sprites.shop_1_button_pressed,
                        ),
                        _ => (
                            &sprites.shop_2_button_normal,
                            &sprites.shop_2_button_pressed,
                        ),
                    };
                    node_button(normal, pressed, framebuffer);
                }
                NodeKind::Fuel(fuel) => {
                    let pos = node
                        .position
                        .as_f32()
                        .extend_uniform(-0.1)
                        .extend_up(-0.075)
                        .as_r32();
                    let mut pos = Aabb2::from_corners(to_screen(pos.min), to_screen(pos.max))
                        .with_height(pixel_scale * 4.0, 1.0);
                    self.util.draw_quad_outline(
                        pos,
                        pixel_scale,
                        palette.fuel_back,
                        &geng::PixelPerfectCamera,
                        framebuffer,
                    );
                    self.context.geng.draw2d().quad(
                        framebuffer,
                        &geng::PixelPerfectCamera,
                        pos.split_left(fuel.get_ratio().as_f32()),
                        palette.fuel_front,
                    );
                }
                NodeKind::CoalFuel(fuel) => {
                    let pos = node
                        .position
                        .as_f32()
                        .extend_uniform(-0.1)
                        .extend_up(-0.075)
                        .as_r32();
                    let mut pos = Aabb2::from_corners(to_screen(pos.min), to_screen(pos.max))
                        .with_height(pixel_scale * 4.0, 1.0);
                    self.util.draw_quad_outline(
                        pos,
                        pixel_scale,
                        palette.fuel_back,
                        &geng::PixelPerfectCamera,
                        framebuffer,
                    );
                    self.context.geng.draw2d().quad(
                        framebuffer,
                        &geng::PixelPerfectCamera,
                        pos.split_left(fuel.get_ratio().as_f32()),
                        palette.fuel_front,
                    );
                }
                NodeKind::TurnLeft => {
                    node_button(
                        &sprites.turn_left_button_normal,
                        &sprites.turn_left_button_pressed,
                        framebuffer,
                    );
                }
                NodeKind::TurnRight => {
                    node_button(
                        &sprites.turn_right_button_normal,
                        &sprites.turn_right_button_pressed,
                        framebuffer,
                    );
                }
                NodeKind::Sprint { cooldown } => {
                    let normal = &sprites.sprint_button_normal;
                    let pressed = &sprites.sprint_button_pressed;
                    let disabled = &sprites.sprint_button_disabled;

                    let is_pressed = is_pressed || self.model.drill.sprint.is_some();
                    let texture = if is_pressed {
                        pressed
                    } else if cooldown.is_above_min() {
                        disabled
                    } else {
                        normal
                    };
                    {
                        let mut pixel_scale = pixel_scale;
                        if !is_pressed && is_hovered {
                            pixel_scale *= 1.25;
                        }
                        self.util.draw_texture_pp(
                            texture,
                            position.center(),
                            vec2(0.5, 0.5),
                            Angle::ZERO,
                            pixel_scale,
                            Color::WHITE,
                            &geng::PixelPerfectCamera,
                            framebuffer,
                        );
                    }

                    // Cooldown
                    let pos = node
                        .position
                        .as_f32()
                        .extend_uniform(-0.1)
                        .extend_down(0.025)
                        .as_r32();
                    let mut pos = Aabb2::from_corners(to_screen(pos.min), to_screen(pos.max))
                        .with_height(pixel_scale * 4.0, 0.0);
                    self.util.draw_quad_outline(
                        pos,
                        pixel_scale,
                        palette.sprint_back,
                        &geng::PixelPerfectCamera,
                        framebuffer,
                    );
                    self.context.geng.draw2d().quad(
                        framebuffer,
                        &geng::PixelPerfectCamera,
                        pos.split_left(cooldown.get_ratio().as_f32()),
                        palette.sprint_front,
                    );
                }
                NodeKind::Upgrade => {}
                NodeKind::Drill { power, .. } => {
                    let cell_size = pixel_scale * 4.0;
                    let position = position.top_right() - vec2(2.0, 4.0) * pixel_scale;
                    let mut position = Aabb2::point(position)
                        .extend_down(cell_size)
                        .extend_left(cell_size);
                    for _ in power.min()..power.value() {
                        self.context.geng.draw2d().quad(
                            framebuffer,
                            &geng::PixelPerfectCamera,
                            position,
                            palette.battery_front,
                        );
                        position = position.translate(vec2(-5.0, 0.0) * pixel_scale);
                    }
                    for _ in power.value()..power.max() {
                        self.util.draw_quad_outline(
                            position,
                            pixel_scale,
                            palette.battery_back,
                            &geng::PixelPerfectCamera,
                            framebuffer,
                        );
                        position = position.translate(vec2(-5.0, 0.0) * pixel_scale);
                    }
                }
                NodeKind::Battery => {}
                NodeKind::Vision { .. } => {}
                NodeKind::Speed { .. } => {}
            }
        }
    }

    fn draw_shop(&mut self, pixel_scale: f32) {
        if !self.show_shop {
            return;
        }

        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.screen_texture,
            self.context.geng.ugli(),
        );

        let palette = &self.context.assets.palette;
        let sprites = &self.context.assets.sprites;

        self.util.draw_nine_slice(
            self.shop_view,
            Color::WHITE,
            &sprites.border_shop,
            pixel_scale,
            &geng::PixelPerfectCamera,
            framebuffer,
        );

        let coin = &sprites.coin;
        let cost_height = (coin.size().y as f32 + 4.0) * pixel_scale;

        let padding = pixel_scale * 5.0;
        let bounds = self.shop_view.extend_uniform(-padding);
        let mut next_pos = bounds.top_left();
        let mut row_height: f32 = 0.0;

        self.shop_items.clear();
        for (index, item) in self.model.shop.iter().enumerate() {
            let is_hovered = matches!(
                self.hovering,
                Some(DragTarget::Shop { item }) if item == index
            );

            let texture = match item.item.node {
                ShopNode::FuelSmall => &sprites.fuel_small_node,
                ShopNode::Fuel => &sprites.fuel_normal_node,
                ShopNode::TurnLeft => &sprites.turn_left_button_normal,
                ShopNode::TurnRight => &sprites.turn_right_button_normal,
                ShopNode::Battery => &sprites.battery_node,
                ShopNode::Upgrade => &sprites.upgrade_node,
                ShopNode::Speed => &sprites.drill_0_speed,
                ShopNode::Light => &sprites.drill_0_light,
                ShopNode::Sprint => &sprites.sprint_node,
                ShopNode::CoalFuel => &sprites.coal_fuel_node,
            };
            let size = texture.size().as_f32() * pixel_scale;
            row_height = row_height.max(size.y + cost_height);
            if bounds.max.x - next_pos.x < size.x {
                // Next row
                next_pos = vec2(bounds.min.x, next_pos.y - row_height);
            }

            let mut pixel_scale = pixel_scale;
            if is_hovered {
                pixel_scale *= 1.1;
            }
            let position = Aabb2::point(next_pos)
                .extend_right(size.x)
                .extend_down(size.y);
            let position = self.util.draw_texture_pp(
                texture,
                position.center(),
                vec2(0.5, 0.5),
                Angle::ZERO,
                pixel_scale,
                Color::WHITE,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.shop_items.push(position);
            next_pos.x += size.x + padding;

            let mut position = Aabb2::point(position.bottom_left())
                .extend_right(position.width())
                .extend_down(cost_height)
                .with_width(position.width() - padding, 0.5);
            let coin_pos = position.cut_left(coin.size().x as f32 * pixel_scale);
            self.util.draw_texture_pp(
                coin,
                coin_pos.center(),
                vec2(0.5, 0.5),
                Angle::ZERO,
                pixel_scale,
                Color::WHITE,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.util.draw_text(
                format!("{}", item.item.cost),
                position.align_pos(vec2(0.0, 0.5)),
                &self.context.assets.fonts.revolver_game,
                TextRenderOptions::new(coin.size().y as f32 * pixel_scale)
                    .color(palette.gold_text)
                    .align(vec2(0.0, 0.5)),
                &geng::PixelPerfectCamera,
                framebuffer,
            );
        }
    }

    fn toggle_shop(&mut self) {
        self.show_shop = !self.show_shop;
    }

    fn mouse_down(&mut self) {
        self.end_drag();
        if let Some(target) = self.hovering.clone() {
            self.start_drag(target);
        }
    }

    fn start_drag(&mut self, target: DragTarget) {
        match target {
            DragTarget::Node { index, .. } => {
                let Some(node) = self.model.nodes.nodes.get(index) else {
                    return;
                };
                match node.kind {
                    NodeKind::Power => {
                        // Cannot drag the power node - launch the drill instead
                        self.model.launch_drill();
                        return;
                    }
                    NodeKind::Shop { .. } => {
                        // Cannot drag the shop node - open the shop
                        self.toggle_shop();
                        return;
                    }
                    NodeKind::Sprint { .. } => {
                        // We can still drag the node - start sprinting
                        self.model.start_sprint(index);
                    }
                    _ => (),
                }
            }
            DragTarget::NodeConnection { node: node_i, conn } => {
                if !matches!(self.model.phase, Phase::Setup) {
                    return;
                }
                let Some(node) = self.model.nodes.nodes.get_mut(node_i) else {
                    return;
                };
                let Some(conn) = node.connections.get_mut(conn) else {
                    return;
                };
                // Remove connection
                if let Some(i) = conn.connected_to.take() {
                    if let Some(node) = self.model.nodes.nodes.get_mut(i) {
                        for conn in &mut node.connections {
                            if conn.connected_to == Some(node_i) {
                                conn.connected_to = None;
                            }
                        }
                    }
                }
            }
            DragTarget::Shop { item } => {
                // Cannot drag shop items - buy them
                self.model.purchase_item(item);
                return;
            }
        }

        let drag = Drag {
            from_screen: self.cursor_screen_pos,
            from_ui: self.cursor_ui_pos,
            target,
        };
        log::debug!("Started drag: {:?}", drag);
        self.drag = Some(drag);
    }

    fn end_drag(&mut self) {
        let Some(drag) = self.drag.take() else { return };

        match drag.target {
            DragTarget::Node { .. } => {}
            DragTarget::NodeConnection {
                node: node_i,
                conn: conn_i,
            } => {
                if !matches!(self.model.phase, Phase::Setup) {
                    return;
                }
                if let Some(DragTarget::NodeConnection {
                    node: to_node_i,
                    conn: to_conn_i,
                }) = self.hovering.clone()
                {
                    if node_i != to_node_i {
                        let nodes = &mut self.model.nodes;
                        if let Some(node) = nodes.nodes.get_mut(node_i) {
                            if let Some(conn) = node.connections.get_mut(conn_i) {
                                let color = conn.kind;
                                if let Some(to_node) = nodes.nodes.get_mut(to_node_i) {
                                    if let Some(to_conn) = to_node.connections.get_mut(to_conn_i) {
                                        if to_conn.kind == color {
                                            to_conn.connected_to = Some(node_i);
                                            if let Some(node) = nodes.nodes.get_mut(node_i) {
                                                if let Some(conn) = node.connections.get_mut(conn_i)
                                                {
                                                    conn.connected_to = Some(to_node_i);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            DragTarget::Shop { .. } => {}
        }
    }

    fn update_drag(&mut self) {
        let Some(drag) = &mut self.drag else { return };

        match &mut drag.target {
            DragTarget::Node {
                index,
                from_position,
            } => {
                let nodes = &mut self.model.nodes;
                if let Some(node) = nodes.nodes.get_mut(*index) {
                    match node.kind {
                        NodeKind::TurnLeft if self.model.drill.can_turn_left => {
                            self.turn_input += r32(1.0)
                        }
                        NodeKind::TurnRight if self.model.drill.can_turn_right => {
                            self.turn_input -= r32(1.0)
                        }
                        _ => {}
                    }
                    node.position = node.position.translate(
                        self.cursor_ui_pos + *from_position - drag.from_ui - node.position.center(),
                    );
                }
            }
            DragTarget::NodeConnection { .. } => {}
            DragTarget::Shop { .. } => {}
        }
    }

    fn update_hover(&mut self) {
        self.hovering = None;

        if self.show_shop && self.shop_view.contains(self.cursor_screen_pos.as_f32()) {
            for (i, pos) in self.shop_items.iter().enumerate() {
                if pos.contains(self.cursor_screen_pos.as_f32()) {
                    self.hovering = Some(DragTarget::Shop { item: i });
                    return;
                }
            }
            return;
        }

        for (node_i, node) in self.model.nodes.nodes.iter().enumerate() {
            for (conn_i, connection) in node.connections.iter().enumerate() {
                let delta = node.position.align_pos(connection.offset) - self.cursor_ui_pos;
                if delta.len() < r32(0.2) {
                    self.hovering = Some(DragTarget::NodeConnection {
                        node: node_i,
                        conn: conn_i,
                    });
                    return;
                }
            }

            if node.position.contains(self.cursor_ui_pos) {
                self.hovering = Some(DragTarget::Node {
                    index: node_i,
                    from_position: node.position.center(),
                });
            }
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        let delta_time = r32(delta_time as f32);
        self.real_time += delta_time;
        self.turn_input = R32::ZERO;
        self.update_hover();
        self.update_drag();
        self.model.drill.collider.rotation += Angle::from_radians(
            self.turn_input * self.model.config.drill_rotation_speed * delta_time,
        );
        self.model.update(delta_time);
    }

    fn handle_event(&mut self, event: geng::Event) {
        let controls = &self.context.assets.controls;
        if geng_utils::key::is_event_press(&event, &controls.launch) {
            self.model.launch_drill();
        }

        match event {
            geng::Event::MousePress { .. } => {
                self.mouse_down();
            }
            geng::Event::MouseRelease { .. } => {
                self.end_drag();
            }
            geng::Event::CursorMove { position } => {
                self.cursor_screen_pos = position;
                self.cursor_ui_pos = self
                    .model
                    .nodes
                    .camera
                    .screen_to_world(
                        self.ui_texture.size().as_f32(),
                        self.cursor_screen_pos.as_f32() - self.ui_view.bottom_left(),
                    )
                    .as_r32();
                self.cursor_game_pos = self
                    .model
                    .camera
                    .screen_to_world(
                        self.game_texture.size().as_f32(),
                        self.cursor_screen_pos.as_f32() - self.game_view.bottom_left(),
                    )
                    .as_r32();
                self.update_hover();
                self.update_drag();
            }
            _ => {}
        }
    }

    fn draw(&mut self, final_framebuffer: &mut ugli::Framebuffer) {
        let pixel_scale = final_framebuffer.size().as_f32() / crate::TARGET_SCREEN_SIZE.as_f32();
        let pixel_scale = pixel_scale.x.min(pixel_scale.y).floor().max(0.25);
        self.layout(pixel_scale, final_framebuffer.size());

        self.draw_nodes(pixel_scale);
        self.draw_game(pixel_scale);

        geng_utils::texture::update_texture_size(
            &mut self.screen_texture,
            final_framebuffer.size(),
            self.context.geng.ugli(),
        );

        let framebuffer = &mut geng_utils::texture::attach_texture(
            &mut self.screen_texture,
            self.context.geng.ugli(),
        );

        let context = self.context.clone();
        let palette = &context.assets.palette;
        let sprites = &context.assets.sprites;
        ugli::clear(framebuffer, Some(palette.background), None, None);

        // Ui
        {
            let draw = geng_utils::texture::DrawTexture::new(&self.ui_texture).pixel_perfect(
                self.ui_view.center(),
                vec2(0.5, 0.5),
                1.0,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.util.draw_nine_slice(
                draw.target.extend_uniform(2.0 * pixel_scale),
                Color::WHITE,
                &sprites.border_ui,
                pixel_scale,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.context.geng.draw2d().draw2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::TexturedQuad::unit(draw.texture).transform(
                    mat3::translate(draw.target.center())
                        * mat3::rotate(Angle::ZERO)
                        * mat3::scale(draw.target.size() / 2.0),
                ),
            );
        }

        // Game
        {
            let draw = geng_utils::texture::DrawTexture::new(&self.game_texture).pixel_perfect(
                self.game_view.center(),
                vec2(0.5, 0.5),
                pixel_scale,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.util.draw_nine_slice(
                draw.target.extend_uniform(2.0 * pixel_scale),
                Color::WHITE,
                &sprites.border_game,
                pixel_scale,
                &geng::PixelPerfectCamera,
                framebuffer,
            );
            self.context.geng.draw2d().draw2d(
                framebuffer,
                &geng::PixelPerfectCamera,
                &draw2d::TexturedQuad::unit(draw.texture).transform(
                    mat3::translate(draw.target.center())
                        * mat3::rotate(Angle::ZERO)
                        * mat3::scale(draw.target.size() / 2.0),
                ),
            );
        }

        self.draw_game_ui(pixel_scale);
        self.draw_shop(pixel_scale);

        // Postprocessing
        ugli::draw(
            final_framebuffer,
            &self.context.assets.shaders.crt,
            ugli::DrawMode::TriangleFan,
            &self.util.unit_quad,
            ugli::uniforms! {
                u_texture: &self.screen_texture,
            },
            ugli::DrawParameters::default(),
        );
    }
}
