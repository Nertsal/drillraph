use crate::{model::*, prelude::*, render::util::UtilRender, ui::layout::*};

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

    cursor_screen_pos: vec2<f64>,
    cursor_ui_pos: vec2<Coord>,
    cursor_game_pos: vec2<Coord>,

    model: Model,
    hovering: Option<DragTarget>,
    drag: Option<Drag>,

    screen: Aabb2<f32>,
    ui_view: Aabb2<f32>,
    game_view: Aabb2<f32>,
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
}

impl GameState {
    pub fn new(context: Context) -> Self {
        Self {
            model: Model::new(context.clone()),
            hovering: None,
            drag: None,

            cursor_screen_pos: vec2::ZERO,
            cursor_ui_pos: vec2::ZERO,
            cursor_game_pos: vec2::ZERO,

            screen: Aabb2::ZERO,
            ui_view: Aabb2::ZERO,
            game_view: Aabb2::ZERO,

            util: UtilRender::new(context.clone()),
            ui_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            game_texture: geng_utils::texture::new_texture(context.geng.ugli(), vec2(1, 1)),
            context,
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
        self.util.draw_collider(
            &model.drill.collider,
            palette.drill,
            &model.camera,
            framebuffer,
        );

        // Drill vision
        self.context.geng.draw2d().circle_with_cut(
            framebuffer,
            &model.camera,
            model.drill.collider.position.as_f32(),
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

        ugli::clear(framebuffer, Some(palette.background), None, None);

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
            for (conn_i, connection) in node.connections.iter().enumerate() {
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

                if let Some(DragTarget::NodeConnection {
                    node: drag_node,
                    conn: drag_conn,
                }) = self.drag.as_ref().map(|drag| &drag.target)
                {
                    if *drag_node == node_i && *drag_conn == conn_i {
                        let from = node.position.center() + connection.offset;
                        let to = self.cursor_ui_pos;
                        self.context.geng.draw2d().draw2d(
                            framebuffer,
                            &nodes.camera,
                            &draw2d::Segment::new(Segment(from.as_f32(), to.as_f32()), 0.1, color),
                        );
                    }
                }
            }
        }
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
                if self
                    .model
                    .nodes
                    .nodes
                    .get(index)
                    .is_none_or(|node| matches!(node.kind, NodeKind::Power))
                {
                    // Cannot drag the power node
                    return;
                }
            }
            DragTarget::NodeConnection { node, conn } => {
                let Some(node) = self.model.nodes.nodes.get_mut(node) else {
                    return;
                };
                let Some(conn) = node.connections.get_mut(conn) else {
                    return;
                };
                conn.connected_to = None;
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
            DragTarget::NodeConnection { node, conn } => {
                if let Some(DragTarget::NodeConnection {
                    node: to_node,
                    conn: to_conn,
                }) = self.hovering.clone()
                {
                    let nodes = &mut self.model.nodes;
                    if let Some(node) = nodes.nodes.get_mut(node) {
                        if let Some(conn) = node.connections.get_mut(conn) {
                            conn.connected_to = Some(to_node);
                        }
                    }
                    if let Some(to_node) = nodes.nodes.get_mut(to_node) {
                        if let Some(to_conn) = to_node.connections.get_mut(to_conn) {
                            to_conn.connected_to = Some(node);
                        }
                    }
                }
            }
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
                    node.position = node.position.translate(
                        self.cursor_ui_pos + *from_position - drag.from_ui - node.position.center(),
                    );
                }
            }
            DragTarget::NodeConnection { .. } => {}
        }
    }

    fn update_hover(&mut self) {
        self.hovering = None;

        for (node_i, node) in self.model.nodes.nodes.iter().enumerate() {
            for (conn_i, connection) in node.connections.iter().enumerate() {
                let delta = node.position.center() + connection.offset - self.cursor_ui_pos;
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
