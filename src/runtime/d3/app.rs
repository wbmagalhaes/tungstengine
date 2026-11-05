use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use glam::{Mat4, Vec3};

use crate::animation::Animator;
use crate::ecs::prelude::*;
use crate::ui::UI;

use crate::input::{Input, PadBackend};
use crate::render::d3::gfx::Gfx;
use crate::render::d3::{
    Camera, CameraView, CursorRay, Faces, Hovered, Lighting, MainCam, MeshRegistry, Outline,
    OverlayHook, PostChain, PostRegistry, RenderItem, RenderSlot, Renderer, Scene, Shader,
    ShaderRegistry, Transition, TransitionMask, propagate_transforms,
};

#[derive(Default, Resource)]
pub struct AppControl {
    pub should_exit: bool,
    pub fullscreen: Option<bool>,
}

#[derive(Resource)]
pub struct WindowConfig {
    pub title: String,
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            title: "game".to_string(),
            fullscreen: false,
        }
    }
}

pub type FrameUi = fn(&mut World, &mut UI);

pub struct App {
    world: World,
    frame_ui: FrameUi,
    ui: UI,
    window: Option<Arc<Window>>,
    gfx: Option<Gfx>,
    last: Instant,
    pads: Option<PadBackend>,
}

fn main_camera(world: &World) -> Option<(Mat4, f32, f32, f32)> {
    use std::sync::Once;
    static NO_CAM: Once = Once::new();
    static MULTI_CAM: Once = Once::new();

    let mut iter = world.query::<(&GlobalTransform, &Camera, &MainCam)>();
    let Some((_, g, c, _)) = iter.next() else {
        NO_CAM.call_once(|| log::warn!("no MainCam tagged; falling back to identity camera"));
        return None;
    };
    if iter.next().is_some() {
        MULTI_CAM.call_once(|| log::warn!("multiple MainCam entities; using the first"));
    }
    Some((g.matrix(), c.fov, c.near, c.far))
}

fn for_each_slot(
    world: &World,
    e: Entity,
    g: &GlobalTransform,
    r: &Renderer,
    mut visit: impl FnMut(Mat4, &RenderSlot),
) {
    let model = Mat4::from_translation(crate::ecs::render_offset(world, e)) * g.matrix();
    let animator = world.get::<Animator>(e);
    for slot in &r.slots {
        let model = match animator {
            Some(a) => model * a.delta(slot.bone),
            None => model,
        };
        visit(model, slot);
    }
}

fn gather_scene(world: &World) -> Scene {
    let camera = main_camera(world)
        .map(|(world, fov, near, far)| CameraView {
            world,
            fov,
            near,
            far,
        })
        .unwrap_or_default();

    let mut items = Vec::new();
    for (e, g, r) in world.query::<(&GlobalTransform, &Renderer)>() {
        for_each_slot(world, e, g, r, |model, s| {
            items.push(RenderItem {
                mesh: s.mesh,
                model,
                color: s.material.color,
                shader: s.material.shader,
                faces: s.material.faces,
                metallic: s.material.metallic,
                roughness: s.material.roughness,
                uv_offset: s.material.uv_offset,
                uv_scale: s.material.uv_scale,
                params: s.material.params,
                cast_shadows: r.cast_shadows,
                outline_occlude: r.outline_occlude,
            });
        });
    }

    let mut outlines = Vec::new();
    let mut outline_color = [0.0; 4];
    let mut outline_width = 0.0;
    for (root, o, _) in world.query::<(&Outline, &Hovered)>() {
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            if let Some(c) = world.get::<Children>(n) {
                stack.extend(c.0.iter().copied());
            }
            let (Some(g), Some(r)) = (world.get::<GlobalTransform>(n), world.get::<Renderer>(n))
            else {
                continue;
            };
            for_each_slot(world, n, g, r, |model, s| {
                outlines.push(RenderItem {
                    mesh: s.mesh,
                    model,
                    color: o.color,
                    shader: Shader::UNLIT,
                    faces: Faces::Both,
                    metallic: 0.0,
                    roughness: 1.0,
                    uv_offset: [0.0, 0.0],
                    uv_scale: [1.0, 1.0],
                    params: [0.0; 16],
                    cast_shadows: false,
                    outline_occlude: true,
                });
            });
        }
        outline_color = o.color.to_array();
        outline_width = o.width;
    }

    let lighting = world.resource::<Lighting>().copied().unwrap_or_default();
    let time = world.resource::<Time>().map(|t| t.elapsed).unwrap_or(0.0);
    let post = world
        .resource::<PostChain>()
        .map(|c| c.passes.clone())
        .unwrap_or_default();

    Scene {
        camera,
        items,
        outlines,
        outline_color,
        outline_width,
        lighting,
        time,
        post,
    }
}

pub fn install_core(world: &mut World) {
    world.insert_resource(AppControl::default());
    world.resource_core_mut::<Input>();
    world.insert_resource(CursorRay::default());
    world.insert_resource(Transition::default());
    if world.resource::<MeshRegistry>().is_err() {
        world.insert_resource(MeshRegistry::new());
    }
    if world.resource::<ShaderRegistry>().is_err() {
        world.insert_resource(ShaderRegistry::new());
    }
    if world.resource::<PostRegistry>().is_err() {
        world.insert_resource(PostRegistry::default());
    }
    if world.resource::<PostChain>().is_err() {
        world.insert_resource(PostChain::default());
    }
    if world.resource::<crate::sprites::Sprites>().is_err() {
        world.insert_resource(crate::sprites::Sprites::default());
    }
}

pub fn tick(world: &mut World, ui: &mut UI, frame_ui: FrameUi, w: f32, h: f32, dt: f32) {
    if let Ok(input) = world.resource_mut::<Input>() {
        input.tick(dt);
    }
    if let Ok(time) = world.resource_mut::<Time>() {
        time.advance(dt);
    }
    if let Ok(transition) = world.resource_mut::<Transition>() {
        transition.advance(dt);
    }
    let cursor = world
        .resource::<Input>()
        .map(|i| i.cursor)
        .unwrap_or((0.0, 0.0));
    update_cursor_ray(world, cursor, w, h);
    world.run();
    crate::runtime::ui_pass(world, ui, frame_ui, (w, h, dt));
    if let Ok(input) = world.resource_mut::<Input>() {
        input.end_frame();
    }
    propagate_transforms(world);
}

fn update_cursor_ray(world: &mut World, cursor: (f32, f32), w: f32, h: f32) {
    let Some((cam_world, fov, near, far)) = main_camera(world) else {
        return;
    };

    let aspect = w / h.max(1.0);
    let inv = (Mat4::perspective_rh(fov, aspect, near, far) * cam_world.inverse()).inverse();
    let ndc_x = cursor.0 / w * 2.0 - 1.0;
    let ndc_y = 1.0 - cursor.1 / h * 2.0;
    let near_p = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far_p = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));

    if let Ok(ray) = world.resource_mut::<CursorRay>() {
        ray.origin = near_p;
        ray.dir = (far_p - near_p).normalize_or_zero();
    }
}

impl App {
    pub fn run(mut self) -> Result<(), winit::error::EventLoopError> {
        let event_loop = winit::event_loop::EventLoop::new()?;
        event_loop.run_app(&mut self)
    }

    pub fn new(mut world: World, frame_ui: FrameUi) -> Self {
        install_core(&mut world);

        App {
            world,
            frame_ui,
            ui: UI::new(),
            window: None,
            gfx: None,
            last: Instant::now(),
            pads: PadBackend::new(),
        }
    }

    fn with_input(&mut self, f: impl FnOnce(&mut Input)) {
        if let Ok(input) = self.world.resource_mut::<Input>() {
            f(input);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let title = self
            .world
            .resource::<WindowConfig>()
            .map(|c| c.title.clone())
            .unwrap_or_else(|_| "game".to_string());
        let fullscreen = self
            .world
            .resource::<WindowConfig>()
            .is_ok_and(|c| c.fullscreen)
            .then_some(winit::window::Fullscreen::Borderless(None));
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_fullscreen(fullscreen);
        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let registry = self
            .world
            .resource::<MeshRegistry>()
            .expect("MeshRegistry resource");
        let shaders = self
            .world
            .resource::<ShaderRegistry>()
            .expect("ShaderRegistry resource");
        let (shadow_size, samples) = self
            .world
            .resource::<crate::render::d3::GfxConfig>()
            .map(|c| (c.shadow_size, c.msaa))
            .unwrap_or((2048, 1));
        let sprites = self
            .world
            .resource::<crate::sprites::Sprites>()
            .expect("Sprites resource");
        let mask = self
            .world
            .resource::<TransitionMask>()
            .cloned()
            .unwrap_or_default();
        let gfx = pollster::block_on(Gfx::new(
            window.clone(),
            registry,
            shaders,
            sprites,
            &mask.0,
            shadow_size,
            samples,
        ));
        match gfx {
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
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.1);
                self.last = now;

                if let Some(pads) = self.pads.as_mut()
                    && let Ok(input) = self.world.resource_mut::<Input>()
                {
                    pads.poll(input);
                }
                let fullscreen = self
                    .world
                    .resource_mut::<AppControl>()
                    .ok()
                    .and_then(|c| c.fullscreen.take());
                if let (Some(on), Some(window)) = (fullscreen, self.window.as_ref()) {
                    window
                        .set_fullscreen(on.then_some(winit::window::Fullscreen::Borderless(None)));
                }

                let (w, h) = self.gfx.as_ref().map(Gfx::size).unwrap_or((0.0, 0.0));
                tick(&mut self.world, &mut self.ui, self.frame_ui, w, h, dt);

                let exit = self
                    .world
                    .resource::<AppControl>()
                    .map(|c| c.should_exit)
                    .unwrap_or(false);

                if exit {
                    event_loop.exit();
                }

                let mut scene = gather_scene(&self.world);
                if let Ok(clear) = self.world.resource::<crate::render::ClearColor>() {
                    scene.lighting.background = clear.0;
                }
                let overlay = self.world.resource::<OverlayHook>().ok().copied();
                let examine = overlay.and_then(|o| (o.scene)(&self.world));
                let examine_intro = overlay.map_or(0.0, |o| (o.intro)(&self.world));
                let transition = self
                    .world
                    .resource::<Transition>()
                    .map(|t| t.view())
                    .unwrap_or_default();

                if cfg!(debug_assertions)
                    && let Some(changed) = self
                        .world
                        .resource_mut::<ShaderRegistry>()
                        .ok()
                        .map(|s| s.reload_changed())
                    && !changed.is_empty()
                    && let (Some(gfx), Ok(shaders)) =
                        (self.gfx.as_mut(), self.world.resource::<ShaderRegistry>())
                {
                    for handle in changed {
                        gfx.reload_program(handle, shaders);
                    }
                }

                if cfg!(debug_assertions)
                    && let Some(changed) = self
                        .world
                        .resource_mut::<PostRegistry>()
                        .ok()
                        .map(|p| p.reload_changed())
                    && !changed.is_empty()
                    && let (Some(gfx), Ok(post)) =
                        (self.gfx.as_mut(), self.world.resource::<PostRegistry>())
                {
                    for handle in changed {
                        gfx.reload_post_program(handle, post);
                    }
                }

                if let Some(gfx) = self.gfx.as_mut() {
                    if let Ok(shaders) = self.world.resource::<ShaderRegistry>() {
                        gfx.ensure_programs(shaders);
                    }
                    if let Ok(post) = self.world.resource::<PostRegistry>() {
                        gfx.ensure_post_programs(post);
                    }
                    gfx.render(
                        &scene,
                        examine.as_ref(),
                        examine_intro,
                        &self.ui.draws,
                        &transition,
                    );
                    self.world.insert_resource(gfx.stats());
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
