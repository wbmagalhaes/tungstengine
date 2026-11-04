use crate::ui::rect::Rect;

#[derive(Default)]
pub struct Response {
    pub hovered: bool,
    pub clicked: bool,
    pub pressed: bool,
    pub hover: f32,
    pub entered: bool,
    pub exited: bool,
    pub changed: bool,
    pub rect: Rect,
}

impl Response {
    pub fn clicked(&self) -> bool {
        self.clicked
    }
    pub fn changed(&self) -> bool {
        self.changed
    }
    pub fn rect(&self) -> Rect {
        self.rect
    }
    pub fn pressed(&self) -> bool {
        self.pressed
    }
    pub fn mark_changed(mut self) -> Self {
        self.changed = true;
        self
    }
    pub fn hovered(&self) -> bool {
        self.hovered
    }
    pub fn entered(&self) -> bool {
        self.entered
    }
    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn on_click(self, f: impl FnOnce()) -> Self {
        if self.clicked {
            f();
        }
        self
    }
    pub fn on_enter(self, f: impl FnOnce()) -> Self {
        if self.entered {
            f();
        }
        self
    }
    pub fn on_exit(self, f: impl FnOnce()) -> Self {
        if self.exited {
            f();
        }
        self
    }
    pub fn on_hover(self, f: impl FnOnce(f32)) -> Self {
        if self.hovered {
            f(self.hover);
        }
        self
    }

    pub fn hover(&self) -> f32 {
        self.hover
    }
}
