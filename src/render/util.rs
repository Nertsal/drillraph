use crate::model::{Collider, Shape};

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct TextRenderOptions {
    pub size: f32,
    pub align: vec2<f32>,
    pub color: Color,
    pub rotation: Angle,
}

impl TextRenderOptions {
    pub fn new(size: f32) -> Self {
        Self { size, ..default() }
    }

    pub fn size(self, size: f32) -> Self {
        Self { size, ..self }
    }

    pub fn align(self, align: vec2<f32>) -> Self {
        Self { align, ..self }
    }

    pub fn color(self, color: Color) -> Self {
        Self { color, ..self }
    }
}

impl Default for TextRenderOptions {
    fn default() -> Self {
        Self {
            size: 1.0,
            align: vec2::splat(0.5),
            color: Color::WHITE,
            rotation: Angle::ZERO,
        }
    }
}

pub struct UtilRender {
    context: Context,
    pub unit_quad: ugli::VertexBuffer<draw2d::TexturedVertex>,
}

impl UtilRender {
    pub fn new(context: Context) -> Self {
        Self {
            unit_quad: geng_utils::geometry::unit_quad_geometry(context.geng.ugli()),
            context,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_texture_pp(
        &self,
        texture: &ugli::Texture,
        position: vec2<f32>,
        align: vec2<f32>,
        rotation: Angle<f32>,
        pixel_scale: f32,
        color: Color,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) -> Aabb2<f32> {
        let draw = geng_utils::texture::DrawTexture::new(texture).pixel_perfect(
            position,
            align,
            pixel_scale,
            camera,
            framebuffer,
        );
        self.context.geng.draw2d().draw2d(
            framebuffer,
            camera,
            &draw2d::TexturedQuad::unit_colored(draw.texture, color).transform(
                mat3::translate(draw.target.center())
                    * mat3::rotate(rotation)
                    * mat3::scale(draw.target.size() / 2.0),
            ),
        );
        draw.target
    }

    pub fn draw_nine_slice(
        &self,
        pos: Aabb2<f32>,
        color: Color,
        texture: &ugli::Texture,
        scale: f32,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let whole = Aabb2::ZERO.extend_positive(vec2::splat(1.0));

        // TODO: configurable
        let mid = Aabb2 {
            min: vec2(0.3, 0.3),
            max: vec2(0.7, 0.7),
        };

        let size = mid.min * texture.size().as_f32() * scale;
        let size = vec2(size.x.min(pos.width()), size.y.min(pos.height()));

        let tl = Aabb2::from_corners(mid.top_left(), whole.top_left());
        let tm = Aabb2::from_corners(mid.top_left(), vec2(mid.max.x, whole.max.y));
        let tr = Aabb2::from_corners(mid.top_right(), whole.top_right());
        let rm = Aabb2::from_corners(mid.top_right(), vec2(whole.max.x, mid.min.y));
        let br = Aabb2::from_corners(mid.bottom_right(), whole.bottom_right());
        let bm = Aabb2::from_corners(mid.bottom_right(), vec2(mid.min.x, whole.min.y));
        let bl = Aabb2::from_corners(mid.bottom_left(), whole.bottom_left());
        let lm = Aabb2::from_corners(mid.bottom_left(), vec2(whole.min.x, mid.max.y));

        let slices: Vec<draw2d::TexturedVertex> = [tl, tm, tr, rm, br, bm, bl, lm, mid]
            .into_iter()
            .flat_map(|slice| {
                let [a, b, c, d] = slice.corners().map(|a_vt| {
                    let a_pos = vec2(
                        if a_vt.x == mid.min.x {
                            pos.min.x + size.x
                        } else if a_vt.x == mid.max.x {
                            pos.max.x - size.x
                        } else {
                            pos.min.x + pos.width() * a_vt.x
                        },
                        if a_vt.y == mid.min.y {
                            pos.min.y + size.y
                        } else if a_vt.y == mid.max.y {
                            pos.max.y - size.y
                        } else {
                            pos.min.y + pos.height() * a_vt.y
                        },
                    );
                    draw2d::TexturedVertex {
                        a_pos,
                        a_color: Color::WHITE,
                        a_vt,
                    }
                });
                [a, b, c, a, c, d]
            })
            .collect();
        let slices = ugli::VertexBuffer::new_dynamic(self.context.geng.ugli(), slices);

        ugli::draw(
            framebuffer,
            &self.context.assets.shaders.texture,
            ugli::DrawMode::Triangles,
            &slices,
            (
                ugli::uniforms! {
                    u_model_matrix: mat3::identity(),
                    u_color: color,
                    u_texture: texture,
                },
                camera.uniforms(framebuffer.size().as_f32()),
            ),
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()),
                ..default()
            },
        );
    }

    pub fn draw_collider(
        &self,
        collider: &Collider,
        color: Rgba<f32>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        match collider.shape {
            Shape::Circle { radius } => {
                self.draw_circle_cut(
                    framebuffer,
                    camera,
                    mat3::translate(collider.position.as_f32())
                        * mat3::scale_uniform(radius.as_f32()),
                    color,
                    0.0,
                );
            }
            Shape::Rectangle { width, height } => {
                self.context.geng.draw2d().draw2d_transformed(
                    framebuffer,
                    camera,
                    &draw2d::Quad::new(
                        Aabb2::ZERO.extend_symmetric(vec2(width, height).as_f32() / 2.0),
                        color,
                    ),
                    (mat3::translate(collider.position) * mat3::rotate(collider.rotation)).as_f32(),
                );
            }
        }
    }

    pub fn draw_circle_cut(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        transform: mat3<f32>,
        color: Color,
        cut: f32,
    ) {
        self.draw_circle_arc(
            framebuffer,
            camera,
            transform,
            color,
            cut,
            Angle::from_radians(-std::f32::consts::PI)..=Angle::from_radians(std::f32::consts::PI),
        );
    }

    pub fn draw_circle_arc(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        transform: mat3<f32>,
        color: Color,
        cut: f32,
        range: RangeInclusive<Angle>,
    ) {
        let arc_min = range.start().as_radians();
        let arc_max = range.end().as_radians();
        let framebuffer_size = framebuffer.size();
        ugli::draw(
            framebuffer,
            &self.context.assets.shaders.ellipse,
            ugli::DrawMode::TriangleFan,
            &self.unit_quad,
            (
                ugli::uniforms! {
                    u_model_matrix: transform,
                    u_color: color,
                    u_framebuffer_size: framebuffer_size,
                    u_inner_cut: cut,
                    u_arc_min: arc_min,
                    u_arc_max: arc_max,
                },
                camera.uniforms(framebuffer_size.map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: None,
                ..Default::default()
            },
        );
    }

    pub fn draw_text(
        &self,
        text: impl AsRef<str>,
        position: vec2<impl Float>,
        font: &Font,
        options: TextRenderOptions,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_text_with(
            text,
            position,
            0.0,
            font,
            options,
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()),
                ..default()
            },
            camera,
            framebuffer,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_with(
        &self,
        text: impl AsRef<str>,
        position: vec2<impl Float>,
        z_index: f32,
        font: &Font,
        mut options: TextRenderOptions,
        params: ugli::DrawParameters,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let text = text.as_ref();
        let framebuffer_size = framebuffer.size().as_f32();

        let position = position.map(Float::as_f32);
        let position = crate::util::world_to_screen(camera, framebuffer_size, position);

        let scale = crate::util::world_to_screen(
            camera,
            framebuffer_size,
            vec2::splat(std::f32::consts::FRAC_1_SQRT_2),
        ) - crate::util::world_to_screen(camera, framebuffer_size, vec2::ZERO);
        options.size *= scale.len();
        let font_size = options.size * 0.6; // TODO: could rescale all dependent code but whatever

        let mut position = position;
        for line in text.lines() {
            let measure = font.measure(line, font_size);
            let size = measure.size();
            let align = size * (options.align - vec2::splat(0.5)); // Centered by default
            let descent = -font.descent() * font_size;
            let align = vec2(
                measure.center().x + align.x,
                descent + (measure.max.y - descent) * options.align.y,
            );

            let transform = mat3::translate(position)
                * mat3::rotate(options.rotation)
                * mat3::translate(-align);

            font.draw_with(
                framebuffer,
                line,
                z_index,
                font_size,
                options.color,
                transform,
                params.clone(),
            );
            position.y -= options.size; // NOTE: larger than text size to space out better
        }
    }

    pub fn draw_chain(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        camera: &impl geng::AbstractCamera2d,
        chain: &draw2d::Chain,
    ) {
        let framebuffer_size = framebuffer.size();
        ugli::draw(
            framebuffer,
            &self.context.assets.shaders.solid,
            ugli::DrawMode::Triangles,
            &ugli::VertexBuffer::new_dynamic(self.context.geng.ugli(), chain.vertices.clone()),
            (
                ugli::uniforms! {
                    u_color: Rgba::WHITE,
                    u_framebuffer_size: framebuffer_size,
                    u_model_matrix: chain.transform,
                },
                camera.uniforms(framebuffer_size.map(|x| x as f32)),
            ),
            ugli::DrawParameters {
                blend_mode: None,
                ..Default::default()
            },
        );
    }

    pub fn draw_quad_outline(
        &self,
        quad: Aabb2<impl Float>,
        outline_width: f32,
        color: Color,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let quad = quad.map(|x| x.as_f32());
        let vec2(width, height) = quad.size();
        let [a, b, c, d] = Aabb2::ZERO
            .extend_symmetric(vec2(width, height) / 2.0)
            .extend_uniform(-outline_width / 2.0)
            .corners();
        let m = (a + b) / 2.0;
        self.draw_chain(
            framebuffer,
            camera,
            &draw2d::Chain::new(Chain::new(vec![m, b, c, d, a, m]), outline_width, color, 1)
                .translate(quad.center()),
        );
    }
}
