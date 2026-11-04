use std::sync::Arc;
use web_time::Instant;

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::audio::Audio;
use crate::ecs::prelude::*;
use crate::input::{Input, PadBackend, Pointer};
use crate::render::core::ClearColor;
use crate::render::d2::gfx::Gfx;
use crate::render::d2::{CursorWorld, View2D, gather_world, propagate_transforms, update_view};
use crate::sprites::Sprites;
use crate::ui::UI;

#[derive(Default, Resource)]
pub struct AppControl {
    pub should_exit: bool,
    pub fullscreen: Option<bool>,
}

#[derive(Resource)]
pub struct WindowConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "game".to_string(),
            width: 1280.0,
            height: 800.0,
            fullscreen: true,
        }
    }
}

pub type FrameUi = fn(&mut World, &mut UI);
pub type SetupFn = fn(&mut World);

pub struct App {
    world: World,
    frame_ui: FrameUi,
    ui: UI,
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
    #[cfg(target_arch = "wasm32")]
    pending: std::rc::Rc<std::cell::RefCell<Option<Gfx>>>,
    last: Instant,
    pads: Option<PadBackend>,
}

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run(mut self) -> Result<(), winit::error::EventLoopError> {
        let event_loop = winit::event_loop::EventLoop::new()?;
        event_loop.run_app(&mut self)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn run(self) -> Result<(), winit::error::EventLoopError> {
        use winit::platform::web::EventLoopExtWebSys;
        let event_loop = winit::event_loop::EventLoop::new()?;
        event_loop.spawn_app(self);
        Ok(())
    }

    pub fn new(mut world: World, frame_ui: FrameUi) -> Self {
        install_core(&mut world);
        if let Some(audio) = Audio::new() {
            world.insert_resource(audio);
        }

        App {
            world,
            frame_ui,
            ui: UI::new(),
            window: None,
            gfx: None,
            #[cfg(target_arch = "wasm32")]
            pending: Default::default(),
            last: Instant::now(),
            pads: PadBackend::new(),
        }
    }

    fn with_input(&mut self, f: impl FnOnce(&mut Input)) {
        if let Ok(input) = self.world.resource_mut::<Input>() {
            f(input);
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if self.gfx.is_none() {
            self.gfx = self.pending.borrow_mut().take();
            if let (Some(gfx), Some(window)) = (self.gfx.as_mut(), self.window.as_ref()) {
                let size = window.inner_size();
                gfx.resize(size.width, size.height);
            }
        }
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;

        let (w, h) = self.gfx.as_ref().map(Gfx::size).unwrap_or((0.0, 0.0));
        if let Some(pads) = self.pads.as_mut()
            && let Ok(input) = self.world.resource_mut::<Input>()
        {
            pads.poll(input);
        }
        tick(&mut self.world, &mut self.ui, Some(self.frame_ui), w, h, dt);

        if self
            .world
            .resource::<AppControl>()
            .map(|c| c.should_exit)
            .unwrap_or(false)
        {
            event_loop.exit();
        }
        let fullscreen = self
            .world
            .resource_mut::<AppControl>()
            .ok()
            .and_then(|c| c.fullscreen.take());
        if let (Some(on), Some(window)) = (fullscreen, self.window.as_ref()) {
            window.set_fullscreen(on.then_some(winit::window::Fullscreen::Borderless(None)));
        }

        let world_draws = gather_world(&self.world);
        let clear = self
            .world
            .resource::<ClearColor>()
            .copied()
            .unwrap_or_default()
            .0;
        if let Some(gfx) = self.gfx.as_mut() {
            if let Ok(sprites) = self.world.resource_mut::<Sprites>() {
                let dirty = sprites.take_dirty();
                if !dirty.is_empty() {
                    gfx.sync_sprites(sprites, &dirty);
                }
            }
            let pixel_world = self.world.resource::<View2D>().is_ok_and(|v| v.pixel_snap);
            gfx.render(clear, &world_draws, &self.ui.draws, pixel_world);
        }
    }
}

pub fn cursor_input(world: &World) -> Pointer {
    world
        .resource::<Input>()
        .map(Input::pointer)
        .unwrap_or_default()
}

pub fn install_core(world: &mut World) {
    world.insert_resource(AppControl::default());
    world.resource_core_mut::<Input>();
    world.insert_resource(View2D::default());
    world.insert_resource(CursorWorld::default());
    if world.resource::<ClearColor>().is_err() {
        world.insert_resource(ClearColor::default());
    }
    if world.resource::<Sprites>().is_err() {
        world.insert_resource(Sprites::default());
    }
}

pub fn tick(world: &mut World, ui: &mut UI, frame_ui: Option<FrameUi>, w: f32, h: f32, dt: f32) {
    if let Ok(input) = world.resource_mut::<Input>() {
        input.tick(dt);
    }
    if let Ok(time) = world.resource_mut::<Time>() {
        time.advance(dt);
    }
    propagate_transforms(world);
    update_view(world, Vec2::new(w, h));
    let cursor = world
        .resource::<Input>()
        .map(|i| Vec2::new(i.cursor.0, i.cursor.1))
        .unwrap_or_default();
    let cursor_world = world
        .resource::<View2D>()
        .map(|v| v.screen_to_world(cursor))
        .unwrap_or_default();
    world.insert_resource(CursorWorld(cursor_world));

    world.run();

    if let Some(frame_ui) = frame_ui {
        let cursor = cursor_input(world);
        ui.begin(w, h, dt, cursor);
        frame_ui(world, ui);
        ui.finish();
    }

    if let Ok(input) = world.resource_mut::<Input>() {
        input.end_frame();
    }
    if frame_ui.is_some() {
        propagate_transforms(world);
        update_view(world, Vec2::new(w, h));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (title, size, fullscreen) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| {
                (
                    c.title.clone(),
                    LogicalSize::new(c.width, c.height),
                    c.fullscreen,
                )
            })
            .unwrap_or_else(|_| ("game".to_string(), LogicalSize::new(1280.0, 800.0), true));

        let fullscreen = if fullscreen {
            Some(winit::window::Fullscreen::Borderless(None))
        } else {
            None
        };

        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(size)
            .with_fullscreen(fullscreen);
        #[cfg(target_arch = "wasm32")]
        let attributes = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            attributes.with_canvas(canvas).with_prevent_default(true)
        };
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let sprites = self
            .world
            .resource::<Sprites>()
            .expect("Sprites resource")
            .clone();
        #[cfg(target_arch = "wasm32")]
        {
            let slot = self.pending.clone();
            let target = window.clone();
            self.window = Some(window);
            wasm_bindgen_futures::spawn_local(async move {
                match Gfx::new(target, &sprites).await {
                    Ok(gfx) => *slot.borrow_mut() = Some(gfx),
                    Err(e) => log::error!("{e}"),
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        match pollster::block_on(Gfx::new(window.clone(), &sprites)) {
            Ok(gfx) => {
                self.window = Some(window);
                self.gfx = Some(gfx);
                self.last = Instant::now();
            }
            Err(e) => {
                log::error!("{e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(false) => self.with_input(Input::release_all),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.with_input(|input| input.cursor = (position.x as f32, position.y as f32));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.with_input(|input| input.set_mouse(button, state.is_pressed()));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
                };
                self.with_input(|input| input.add_scroll(d));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.with_input(|input| input.set_key(code, event.state.is_pressed()));
                }
                if event.state.is_pressed()
                    && let Some(text) = event.text.as_ref().map(|t| t.to_string())
                {
                    self.with_input(|input| input.push_text(&text));
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
