pub const DRAG_THRESHOLD: f32 = 6.0;

#[derive(Clone, Copy, Debug)]
pub struct Drag<T> {
    pub payload: T,
    pub start: (f32, f32),
    pub grab: (f32, f32),
    pub moved: f32,
    pub sticky: bool,
    fresh: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum DragEvent<T> {
    Idle,
    Dropped { payload: T, at: (f32, f32) },
    Cancelled(T),
}

#[derive(Clone, Copy, Debug)]
pub struct DragDrop<T> {
    pub held: Option<Drag<T>>,
    pub threshold: f32,
    pub sticky_click: bool,
}

impl<T: Copy> Default for DragDrop<T> {
    fn default() -> Self {
        DragDrop::new(DRAG_THRESHOLD, true)
    }
}

impl<T: Copy> DragDrop<T> {
    pub fn new(threshold: f32, sticky_click: bool) -> Self {
        DragDrop {
            held: None,
            threshold,
            sticky_click,
        }
    }

    pub fn pick(&mut self, payload: T, cursor: (f32, f32), anchor: (f32, f32)) {
        self.held = Some(Drag {
            payload,
            start: cursor,
            grab: (cursor.0 - anchor.0, cursor.1 - anchor.1),
            moved: 0.0,
            sticky: false,
            fresh: true,
        });
    }

    pub fn stick(&mut self, payload: T, cursor: (f32, f32)) {
        self.held = Some(Drag {
            payload,
            start: cursor,
            grab: (0.0, 0.0),
            moved: 0.0,
            sticky: true,
            fresh: false,
        });
    }

    pub fn sticky(&self) -> bool {
        self.held.is_some_and(|h| h.sticky)
    }

    pub fn update(
        &mut self,
        cursor: (f32, f32),
        pressed: bool,
        released: bool,
        cancel: bool,
    ) -> DragEvent<T> {
        let Some(h) = self.held.as_mut() else {
            return DragEvent::Idle;
        };
        let (dx, dy) = (cursor.0 - h.start.0, cursor.1 - h.start.1);
        h.moved = h.moved.max((dx * dx + dy * dy).sqrt());
        let fresh = std::mem::replace(&mut h.fresh, false);
        let payload = h.payload;
        if cancel {
            self.held = None;
            return DragEvent::Cancelled(payload);
        }
        if released && !h.sticky {
            if h.moved >= self.threshold {
                self.held = None;
                return DragEvent::Dropped {
                    payload,
                    at: cursor,
                };
            }
            if self.sticky_click {
                h.sticky = true;
                h.grab = (0.0, 0.0);
                return DragEvent::Idle;
            }
            self.held = None;
            return DragEvent::Cancelled(payload);
        }
        if pressed && h.sticky && !fresh {
            self.held = None;
            return DragEvent::Dropped {
                payload,
                at: cursor,
            };
        }
        DragEvent::Idle
    }

    pub fn payload(&self) -> Option<T> {
        self.held.map(|h| h.payload)
    }

    pub fn dragging(&self) -> bool {
        self.held
            .is_some_and(|h| h.sticky || h.moved >= self.threshold)
    }

    pub fn follow(&self, cursor: (f32, f32)) -> Option<(f32, f32)> {
        self.held
            .map(|h| (cursor.0 - h.grab.0, cursor.1 - h.grab.1))
    }

    pub fn clear(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_past_threshold_drops_on_release() {
        let mut d = DragDrop::new(6.0, true);
        d.pick(7, (0.0, 0.0), (0.0, 0.0));
        assert!(matches!(
            d.update((3.0, 0.0), false, false, false),
            DragEvent::Idle
        ));
        assert!(!d.dragging());
        assert!(matches!(
            d.update((20.0, 0.0), false, true, false),
            DragEvent::Dropped {
                payload: 7,
                at: (20.0, 0.0)
            }
        ));
        assert!(d.payload().is_none());
    }

    #[test]
    fn click_becomes_sticky_then_drops_on_next_press() {
        let mut d = DragDrop::new(6.0, true);
        d.pick(1, (5.0, 5.0), (0.0, 0.0));
        assert!(matches!(
            d.update((5.0, 5.0), true, false, false),
            DragEvent::Idle
        ));
        assert!(matches!(
            d.update((5.0, 5.0), false, true, false),
            DragEvent::Idle
        ));
        assert!(d.dragging());
        assert_eq!(d.follow((50.0, 50.0)), Some((50.0, 50.0)));
        assert!(matches!(
            d.update((50.0, 50.0), true, false, false),
            DragEvent::Dropped { payload: 1, .. }
        ));
    }

    #[test]
    fn click_without_sticky_cancels_and_cancel_returns_payload() {
        let mut d = DragDrop::new(6.0, false);
        d.pick(2, (0.0, 0.0), (0.0, 0.0));
        assert!(matches!(
            d.update((1.0, 0.0), false, true, false),
            DragEvent::Cancelled(2)
        ));
        d.pick(3, (0.0, 0.0), (0.0, 0.0));
        assert!(matches!(
            d.update((30.0, 0.0), false, false, true),
            DragEvent::Cancelled(3)
        ));
    }
}
