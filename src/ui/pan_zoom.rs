use glam::Vec2;

use super::rect::Rect;

#[derive(Clone, Copy)]
pub struct PanZoom {
    pub offset: Vec2,
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub moved: f32,
    drag: Option<(Vec2, Vec2)>,
}

impl PanZoom {
    pub fn new(min_zoom: f32, max_zoom: f32) -> Self {
        PanZoom {
            offset: Vec2::ZERO,
            zoom: 1.0,
            min_zoom,
            max_zoom,
            moved: 0.0,
            drag: None,
        }
    }

    pub fn update(
        &mut self,
        canvas: Rect,
        cursor: Vec2,
        pressed: bool,
        held: bool,
        released: bool,
        scroll: f32,
    ) -> bool {
        let mid = Vec2::new(canvas.x + canvas.w / 2.0, canvas.y + canvas.h / 2.0);
        let over = canvas.contains((cursor.x, cursor.y));
        if over && scroll != 0.0 {
            let old = self.zoom;
            self.zoom = (self.zoom * (1.0 + scroll * 0.1)).clamp(self.min_zoom, self.max_zoom);
            let anchor = cursor - mid;
            self.offset = anchor + (self.offset - anchor) * (self.zoom / old);
        }
        if pressed && over {
            self.drag = Some((cursor, self.offset));
            self.moved = 0.0;
        }
        if let Some((start, origin)) = self.drag {
            self.offset = origin + (cursor - start);
            self.moved = self.moved.max(cursor.distance(start));
        }
        let clicked = released && self.moved < 6.0;
        if !held {
            self.drag = None;
            self.moved = 0.0;
        }
        clicked
    }

    pub fn clamp(&mut self, min: Vec2, max: Vec2) {
        self.offset = self.offset.clamp(-max * self.zoom, -min * self.zoom);
    }

    pub fn origin(&self, canvas: Rect) -> Vec2 {
        Vec2::new(canvas.x + canvas.w / 2.0, canvas.y + canvas.h / 2.0) + self.offset
    }
}

impl Default for PanZoom {
    fn default() -> Self {
        PanZoom::new(0.4, 1.8)
    }
}
