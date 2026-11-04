use std::marker::PhantomData;

use crate::ui::animation::Anim;
use crate::ui::immediate::UI;
use crate::ui::widget::Widget;

pub struct Group<F, O> {
    body: F,
    anim: Option<Anim>,
    key: Option<String>,
    phantom: PhantomData<O>,
}

impl<F: FnOnce(&mut UI) -> O, O> Group<F, O> {
    pub fn new(body: F) -> Self {
        Group {
            body,
            anim: None,
            key: None,
            phantom: PhantomData,
        }
    }

    pub fn anim(mut self, anim: Anim) -> Self {
        self.anim = Some(anim);
        self
    }

    pub fn fade_in(self, dur: f32) -> Self {
        self.anim(Anim::new(dur).fade())
    }

    pub fn slide_in(self, dur: f32, x: f32, y: f32) -> Self {
        self.anim(Anim::new(dur).fade().slide(x, y))
    }

    pub fn pop_in(self, dur: f32, scale: f32) -> Self {
        self.anim(Anim::new(dur).fade().pop(scale))
    }

    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }
}

impl<F: FnOnce(&mut UI) -> O, O> Widget for Group<F, O> {
    type Out = O;

    fn render(self, ui: &mut UI) -> O {
        (self.body)(ui)
    }

    fn presence(&self) -> Option<Anim> {
        self.anim
    }

    fn presence_key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}
