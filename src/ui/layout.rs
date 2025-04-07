use geng::prelude::*;

pub use geng_utils::layout::*;

pub trait AreaOps {
    type Float: Float;

    fn get(&self) -> Aabb2<Self::Float>;

    fn set(&mut self, s: Aabb2<Self::Float>);

    fn square_longside(&self) -> Aabb2<Self::Float> {
        let area = self.get();
        let d = area.width() - area.height();
        let two = Self::Float::from_f32(2.0);
        if d > Self::Float::ZERO {
            Aabb2 {
                min: vec2(area.min.x, area.min.y - d / two),
                max: vec2(area.max.x, area.max.y + d / two),
            }
        } else {
            let d = -d;
            Aabb2 {
                min: vec2(area.min.x - d / two, area.min.y),
                max: vec2(area.max.x + d / two, area.max.y),
            }
        }
    }

    fn square_shortside(&self) -> Aabb2<Self::Float> {
        let area = self.get();
        let d = area.width() - area.height();
        let two = Self::Float::from_f32(2.0);
        if d > Self::Float::ZERO {
            Aabb2 {
                min: vec2(area.min.x + d / two, area.min.y),
                max: vec2(area.max.x - d / two, area.max.y),
            }
        } else {
            let d = -d;
            Aabb2 {
                min: vec2(area.min.x, area.min.y + d / two),
                max: vec2(area.max.x, area.max.y - d / two),
            }
        }
    }

    fn zero_size(&self, align: vec2<Self::Float>) -> Aabb2<Self::Float> {
        Aabb2::point(self.align_pos(align))
    }

    fn cut_left(&mut self, width: Self::Float) -> Aabb2<Self::Float> {
        let left = self.get().extend_right(width - self.get().width());
        self.set(self.get().extend_left(-width));
        left
    }

    fn split_left(&mut self, ratio: Self::Float) -> Aabb2<Self::Float> {
        self.cut_left(self.get().width() * ratio)
    }

    fn cut_right(&mut self, width: Self::Float) -> Aabb2<Self::Float> {
        let right = self.get().extend_left(width - self.get().width());
        self.set(self.get().extend_right(-width));
        right
    }

    fn split_right(&mut self, ratio: Self::Float) -> Aabb2<Self::Float> {
        self.cut_right(self.get().width() * ratio)
    }

    fn cut_top(&mut self, height: Self::Float) -> Aabb2<Self::Float> {
        let top = self.get().extend_down(height - self.get().height());
        self.set(self.get().extend_up(-height));
        top
    }

    fn split_top(&mut self, ratio: Self::Float) -> Aabb2<Self::Float> {
        self.cut_top(self.get().height() * ratio)
    }

    fn cut_bottom(&mut self, height: Self::Float) -> Aabb2<Self::Float> {
        let bottom = self.get().extend_up(height - self.get().height());
        self.set(self.get().extend_down(-height));
        bottom
    }

    fn split_bottom(&mut self, ratio: Self::Float) -> Aabb2<Self::Float> {
        self.cut_bottom(self.get().height() * ratio)
    }

    fn split_rows(&self, rows: usize) -> Vec<Aabb2<Self::Float>> {
        let row_height = self.get().height() / Self::Float::from_f32(rows as f32);
        (0..rows)
            .map(|i| {
                Aabb2::point(
                    self.get().top_left()
                        - vec2(0.0, (i + 1) as f32).map(Self::Float::from_f32) * row_height,
                )
                .extend_positive(vec2(self.get().width(), row_height))
            })
            .collect()
    }

    fn split_columns(&self, columns: usize) -> Vec<Aabb2<Self::Float>> {
        let column_width = self.get().width() / Self::Float::from_f32(columns as f32);
        (0..columns)
            .map(|i| {
                Aabb2::point(
                    self.get().bottom_left()
                        + vec2(i as f32, 0.0).map(Self::Float::from_f32) * column_width,
                )
                .extend_positive(vec2(column_width, self.get().height()))
            })
            .collect()
    }

    fn stack(&self, offset: vec2<Self::Float>, cells: usize) -> Vec<Aabb2<Self::Float>> {
        (0..cells)
            .map(|i| {
                self.get()
                    .translate(offset * Self::Float::from_f32(i as f32))
            })
            .collect()
    }

    fn stack_aligned(
        &self,
        offset: vec2<Self::Float>,
        cells: usize,
        align: vec2<Self::Float>,
    ) -> Vec<Aabb2<Self::Float>> {
        let mut cells = self.stack(offset, cells);
        let mut total = self.get();

        let min = |a, b| {
            if a < b {
                a
            } else {
                b
            }
        };
        let max = |a, b| {
            if a > b {
                a
            } else {
                b
            }
        };

        if let Some(last) = cells.last() {
            total.min.x = min(total.min.x, last.min.x);
            total.min.y = min(total.min.y, last.min.y);
            total.max.x = max(total.max.x, last.max.x);
            total.max.y = max(total.max.y, last.max.y);
        }
        for pos in &mut cells {
            *pos = pos.translate(self.get().size() * align - total.size() * align);
        }
        cells
    }

    fn with_width(&self, width: Self::Float, align: Self::Float) -> Aabb2<Self::Float> {
        align_aabb(
            vec2(width, self.get().height()),
            self.get(),
            vec2(align, Self::Float::from_f32(0.5)),
        )
    }

    fn with_height(&self, height: Self::Float, align: Self::Float) -> Aabb2<Self::Float> {
        align_aabb(
            vec2(self.get().width(), height),
            self.get(),
            vec2(Self::Float::from_f32(0.5), align),
        )
    }

    /// Get a point inside the aabb.
    /// (0.0, 0.0) corresponds to min.
    /// (1.0, 1.0) corresponds to max.
    fn align_pos(&self, align: vec2<Self::Float>) -> vec2<Self::Float> {
        self.get().min + self.get().size() * align
    }

    /// Align an aabb of the given size inside this one.
    fn align_aabb(&self, size: vec2<Self::Float>, align: vec2<Self::Float>) -> Aabb2<Self::Float> {
        let half = Self::Float::from_f32(0.5);
        let pos_aabb = self.get().extend_symmetric(-size * half);
        let pos = aabb_pos(pos_aabb, align);
        Aabb2::point(pos).extend_symmetric(size * half)
    }

    /// Fit an aabb of the given size into this one.
    fn fit_aabb(&self, size: vec2<Self::Float>, align: vec2<Self::Float>) -> Aabb2<Self::Float> {
        let ratio = self.get().size() / size;
        let ratio = if ratio.x < ratio.y { ratio.x } else { ratio.y };
        let fit_size = size * ratio;
        self.align_aabb(fit_size, align)
    }

    /// Fit an aabb of the given size by width into this one.
    fn fit_aabb_width(&self, size: vec2<Self::Float>, align: Self::Float) -> Aabb2<Self::Float> {
        let ratio = self.get().width() / size.x;
        let fit_size = size * ratio;
        self.align_aabb(fit_size, vec2(Self::Float::ZERO, align))
    }

    /// Fit an aabb of the given size by height into this one.
    fn fit_aabb_height(&self, size: vec2<Self::Float>, align: Self::Float) -> Aabb2<Self::Float> {
        let ratio = self.get().height() / size.y;
        let fit_size = size * ratio;
        self.align_aabb(fit_size, vec2(align, Self::Float::ZERO))
    }
}

impl AreaOps for Aabb2<f32> {
    type Float = f32;

    fn get(&self) -> Aabb2<Self::Float> {
        *self
    }

    fn set(&mut self, s: Aabb2<Self::Float>) {
        *self = s;
    }
}

impl AreaOps for Aabb2<R32> {
    type Float = R32;

    fn get(&self) -> Aabb2<Self::Float> {
        *self
    }

    fn set(&mut self, s: Aabb2<Self::Float>) {
        *self = s;
    }
}
