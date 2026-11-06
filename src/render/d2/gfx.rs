use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::color::Color;
use crate::sprites::Sprites;
use crate::ui::{DrawRect, RoundShape};

use super::components::LayerShader;

const QUAD_WGSL: &str = r#"
struct Globals { screen: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) silhouette: f32,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) silhouette: f32,
};

fn shade(texel: vec4<f32>, color: vec4<f32>, silhouette: f32) -> vec4<f32> {
    return mix(texel * color, vec4<f32>(color.rgb, texel.a * color.a), silhouette);
}

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2<f32>(
        in.pos.x / globals.screen.x * 2.0 - 1.0,
        1.0 - in.pos.y / globals.screen.y * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    out.silhouette = in.silhouette;
    return out;
}

@fragment
fn fs_mask(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(tex, samp, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * a);
}

@fragment
fn fs_pixel(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(tex));
    let texel = (floor(in.uv * size) + vec2<f32>(0.5)) / size;
    return shade(textureSample(tex, samp, texel), in.color, in.silhouette);
}

@fragment
fn fs_mask_add(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(tex, samp, in.uv).r * in.color.a;
    return vec4<f32>(in.color.rgb * a, a);
}

@fragment
fn fs_smooth_add(in: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(tex, samp, in.uv) * in.color;
    return vec4<f32>(texel.rgb * texel.a, texel.a);
}

@fragment
fn fs_smooth(in: VsOut) -> @location(0) vec4<f32> {
    return shade(textureSample(tex, samp, in.uv), in.color, in.silhouette);
}

@fragment
fn fs_rgba(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(tex));
    let px = in.uv * size;
    let seam = floor(px + 0.5);
    let width = max(fwidth(px), vec2<f32>(1e-4));
    let snapped = seam + clamp((px - seam) / width, vec2<f32>(-0.5), vec2<f32>(0.5));
    return shade(textureSample(tex, samp, snapped / size), in.color, in.silhouette);
}
"#;

const RICH_WGSL: &str = r#"
const GLASS_TAPS: i32 = 16;
struct Globals { screen: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) silhouette: f32,
    @location(4) local: vec2<f32>,
    @location(5) half_size: vec2<f32>,
    @location(6) radii: vec4<f32>,
    @location(7) border: f32,
    @location(8) border_color: vec4<f32>,
    @location(9) clip_shape: vec4<f32>,
    @location(10) clip_radii: vec4<f32>,
    @location(11) gradient_to: vec4<f32>,
    @location(12) gradient: vec2<f32>,
    @location(13) gradient_via: vec4<f32>,
    @location(14) sweep: vec2<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) silhouette: f32,
    @location(3) local: vec2<f32>,
    @location(4) half_size: vec2<f32>,
    @location(5) radii: vec4<f32>,
    @location(6) border: f32,
    @location(7) border_color: vec4<f32>,
    @location(8) clip_shape: vec4<f32>,
    @location(9) clip_radii: vec4<f32>,
    @location(10) screen_pos: vec2<f32>,
    @location(11) gradient_to: vec4<f32>,
    @location(12) gradient: vec2<f32>,
    @location(13) gradient_via: vec4<f32>,
    @location(14) sweep: vec2<f32>,
};

fn shade(texel: vec4<f32>, color: vec4<f32>, silhouette: f32) -> vec4<f32> {
    return mix(texel * color, vec4<f32>(color.rgb, texel.a * color.a), silhouette);
}

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    let ndc = vec2<f32>(
        in.pos.x / globals.screen.x * 2.0 - 1.0,
        1.0 - in.pos.y / globals.screen.y * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    out.silhouette = in.silhouette;
    out.local = in.local;
    out.half_size = in.half_size;
    out.radii = in.radii;
    out.border = in.border;
    out.border_color = in.border_color;
    out.clip_shape = in.clip_shape;
    out.clip_radii = in.clip_radii;
    out.screen_pos = in.pos;
    out.gradient_to = in.gradient_to;
    out.gradient = in.gradient;
    out.gradient_via = in.gradient_via;
    out.sweep = in.sweep;
    return out;
}

fn round_distance(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    let side = select(radii.wz, radii.xy, p.y < 0.0);
    let r = select(side.y, side.x, p.x < 0.0);
    let q = abs(p) - half_size + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

fn clip_cover(in: VsOut) -> f32 {
    let d = round_distance(in.screen_pos - in.clip_shape.xy, in.clip_shape.zw, in.clip_radii);
    let aa = max(fwidth(d), 1e-4);
    return select(1.0, clamp(0.5 - d / aa, 0.0, 1.0), in.clip_shape.z > 0.0);
}

fn clipped(in: VsOut, paint: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(paint.rgb, paint.a * clip_cover(in));
}

fn sweep_paint(in: VsOut, d: f32) -> vec4<f32> {
    let turn = atan2(in.local.y, in.local.x) / 6.2831853 + 0.5;
    let wave = 0.5 + 0.5 * cos(6.2831853 * (turn - in.sweep.x));
    let shine = wave * wave * wave;
    let aa = max(fwidth(d), 1e-4);
    let outer = clamp(0.5 - d / aa, 0.0, 1.0);
    let inner = clamp(0.5 - (d + in.border) / aa, 0.0, 1.0);
    let color = mix(in.border_color, in.gradient_to, shine);
    return vec4<f32>(color.rgb, color.a * outer * (1.0 - inner));
}

fn round_paint(in: VsOut) -> vec4<f32> {
    let d = round_distance(in.local, in.half_size, in.radii);
    if (in.sweep.y > 0.5) {
        return sweep_paint(in, d);
    }
    let aa = max(fwidth(d), 1e-4);
    let outer = clamp(0.5 - d / aa, 0.0, 1.0);
    let inner = clamp(0.5 - (d + in.border) / aa, 0.0, 1.0);
    let along = vec2<f32>(sin(in.gradient.x), -cos(in.gradient.x));
    let span = 2.0 * (abs(in.half_size.x * along.x) + abs(in.half_size.y * along.y));
    let ramp = clamp(dot(in.local, along) / max(span, 1e-4) + 0.5, 0.0, 1.0);
    let first = vec4<f32>(in.color.rgb * in.color.a, in.color.a);
    let last = vec4<f32>(in.gradient_to.rgb * in.gradient_to.a, in.gradient_to.a);
    let middle = vec4<f32>(in.gradient_via.rgb * in.gradient_via.a, in.gradient_via.a);
    let lower = mix(first, middle, ramp * 2.0);
    let upper = mix(middle, last, ramp * 2.0 - 1.0);
    let fill = select(first, select(upper, lower, ramp < 0.5), in.gradient.y > 0.5);
    let edge = vec4<f32>(in.border_color.rgb * in.border_color.a, in.border_color.a);
    let painted = mix(edge, fill, select(inner, 1.0, in.border <= 0.0)) * outer;
    return vec4<f32>(painted.rgb / max(painted.a, 1e-4), painted.a);
}

@fragment
fn fs_glass(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.screen_pos / globals.screen;
    var behind = vec3<f32>(0.0);
    for (var i = 0; i < GLASS_TAPS; i++) {
        let tap = f32(i);
        let reach = sqrt((tap + 0.5) / f32(GLASS_TAPS)) * in.sweep.x;
        let turn = tap * 2.3999632;
        let offset = vec2<f32>(cos(turn), sin(turn)) * reach / globals.screen;
        behind += textureSampleLevel(tex, samp, uv + offset, 0.0).rgb;
    }
    behind /= f32(GLASS_TAPS);
    let d = round_distance(in.local, in.half_size, in.radii);
    let aa = max(fwidth(d), 1e-4);
    let cover = clamp(0.5 - d / aa, 0.0, 1.0);
    let paint = round_paint(in);
    let mixed = behind * (1.0 - paint.a) + paint.rgb * paint.a;
    return clipped(in, vec4<f32>(mixed, cover));
}

@fragment
fn fs_mask(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(tex, samp, in.uv).r;
    let plain = vec4<f32>(in.color.rgb, in.color.a * a);
    return clipped(in, select(plain, round_paint(in), in.half_size.x > 0.0));
}

@fragment
fn fs_pixel(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(tex));
    let texel = (floor(in.uv * size) + vec2<f32>(0.5)) / size;
    return clipped(in, shade(textureSample(tex, samp, texel), in.color, in.silhouette));
}

@fragment
fn fs_mask_add(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(tex, samp, in.uv).r * in.color.a;
    return vec4<f32>(in.color.rgb * a, a) * clip_cover(in);
}

@fragment
fn fs_smooth_add(in: VsOut) -> @location(0) vec4<f32> {
    let texel = textureSample(tex, samp, in.uv) * in.color;
    return vec4<f32>(texel.rgb * texel.a, texel.a) * clip_cover(in);
}

@fragment
fn fs_smooth(in: VsOut) -> @location(0) vec4<f32> {
    return clipped(in, shade(textureSample(tex, samp, in.uv), in.color, in.silhouette));
}

@fragment
fn fs_rgba(in: VsOut) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(tex));
    let px = in.uv * size;
    let seam = floor(px + 0.5);
    let width = max(fwidth(px), vec2<f32>(1e-4));
    let snapped = seam + clamp((px - seam) / width, vec2<f32>(-0.5), vec2<f32>(0.5));
    return clipped(in, shade(textureSample(tex, samp, snapped / size), in.color, in.silhouette));
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    silhouette: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RichVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    silhouette: f32,
    local: [f32; 2],
    half_size: [f32; 2],
    radii: [f32; 4],
    border: f32,
    border_color: [f32; 4],
    clip_shape: [f32; 4],
    clip_radii: [f32; 4],
    gradient_to: [f32; 4],
    gradient: [f32; 2],
    gradient_via: [f32; 4],
    sweep: [f32; 2],
}

pub(super) const LAYER_PRELUDE: &str = r#"
struct LayerGlobals {
    screen: vec2<f32>,
    time: f32,
    pad: f32,
    params: array<vec4<f32>, 4>,
};
@group(0) @binding(0) var<uniform> layer: LayerGlobals;
@group(0) @binding(1) var layer_tex: texture_2d<f32>;
@group(0) @binding(2) var layer_samp: sampler;

fn fullscreen_position(i: u32) -> vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

fn fullscreen_uv(i: u32) -> vec2<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec2<f32>(p.x, 1.0 - p.y);
}
"#;

const BLUR_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var out: VsOut;
    out.clip = fullscreen_position(i);
    out.uv = fullscreen_uv(i);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let step = layer.params[0].xy;
    var sum = textureSampleLevel(layer_tex, layer_samp, in.uv, 0.0) * 0.227;
    sum += textureSampleLevel(layer_tex, layer_samp, in.uv + step * 1.38, 0.0) * 0.316 * 0.5;
    sum += textureSampleLevel(layer_tex, layer_samp, in.uv - step * 1.38, 0.0) * 0.316 * 0.5;
    sum += textureSampleLevel(layer_tex, layer_samp, in.uv + step * 3.23, 0.0) * 0.07 * 0.5;
    sum += textureSampleLevel(layer_tex, layer_samp, in.uv - step * 3.23, 0.0) * 0.07 * 0.5;
    let total = 0.227 + 0.316 + 0.07;
    let blurred = sum / total;
    let copy = textureSampleLevel(layer_tex, layer_samp, in.uv, 0.0);
    return select(blurred, copy, step.x == 0.0 && step.y == 0.0);
}
"#;

const BLUR_DIVISOR: u32 = 4;

struct BlurTargets {
    size: (u32, u32),
    scene: wgpu::TextureView,
    passes: [(wgpu::TextureView, wgpu::BindGroup); 2],
    blit: wgpu::BindGroup,
    glass: wgpu::BindGroup,
}

const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct LayerFrame<'a> {
    pub shaders: &'a [LayerShader],
    pub draws: &'a [(i32, Vec<DrawRect>)],
    pub time: f32,
}

struct LayerProgram {
    layer: i32,
    source: &'static str,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    target: wgpu::TextureView,
    group: wgpu::BindGroup,
    size: (u32, u32),
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LayerUniform {
    screen: [f32; 2],
    time: f32,
    pad: f32,
    params: [f32; 16],
}

struct Range {
    tex: u32,
    rich: bool,
    glass: bool,
    start: u32,
    count: u32,
    scissor: [u32; 4],
}

pub struct Gfx {
    gpu: GpuContext,
    atlas: wgpu::Texture,
    uniform: wgpu::Buffer,
    mask_pipeline: wgpu::RenderPipeline,
    rgba_pipeline: wgpu::RenderPipeline,
    pixel_pipeline: wgpu::RenderPipeline,
    smooth_pipeline: wgpu::RenderPipeline,
    add_mask_pipeline: wgpu::RenderPipeline,
    add_smooth_pipeline: wgpu::RenderPipeline,
    rich_mask_pipeline: wgpu::RenderPipeline,
    rich_rgba_pipeline: wgpu::RenderPipeline,
    rich_smooth_pipeline: wgpu::RenderPipeline,
    rich_glass_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    blur_targets: Option<BlurTargets>,
    quad_layout_group: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    layer_layout: wgpu::BindGroupLayout,
    layer_pipeline_layout: wgpu::PipelineLayout,
    layer_sampler: wgpu::Sampler,
    layer_programs: Vec<LayerProgram>,
    atlas_bind_group: wgpu::BindGroup,
    sprite_bind_groups: Vec<wgpu::BindGroup>,
    sprite_textures: Vec<wgpu::Texture>,
    vertex_buf: wgpu::Buffer,
    vertex_cap: u64,
    index_buf: wgpu::Buffer,
    index_cap: u64,
    verts: Vec<QuadVertex>,
    rich_verts: Vec<RichVertex>,
    rich_buf: wgpu::Buffer,
    rich_cap: u64,
    indices: Vec<u32>,
}

pub use crate::render::core::GfxError;
use crate::render::core::{GpuContext, GpuLimits};

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("quad-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

const QUAD_ATTRS: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32];
const RICH_ATTRS: [wgpu::VertexAttribute; 15] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32,
    4 => Float32x2,
    5 => Float32x2,
    6 => Float32x4,
    7 => Float32,
    8 => Float32x4,
    9 => Float32x4,
    10 => Float32x4,
    11 => Float32x4,
    12 => Float32x2,
    13 => Float32x4,
    14 => Float32x2
];

fn quad_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<QuadVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &QUAD_ATTRS,
    }
}

fn rich_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<RichVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &RICH_ATTRS,
    }
}

fn rich_vertex(q: &DrawRect, pos: [f32; 2], uv: [f32; 2], silhouette: f32) -> RichVertex {
    let round = q.round.unwrap_or(RoundShape {
        center: pos,
        half: [0.0; 2],
        radii: [0.0; 4],
        rotation: 0.0,
        sweep: None,
        blur: 0.0,
        gradient: None,
        border: 0.0,
        border_color: [0.0; 4],
    });
    let (sin, cos) = round.rotation.sin_cos();
    let (dx, dy) = (pos[0] - round.center[0], pos[1] - round.center[1]);
    RichVertex {
        pos,
        uv,
        color: q.color,
        silhouette,
        local: [dx * cos + dy * sin, dy * cos - dx * sin],
        half_size: round.half,
        radii: round.radii,
        border: round.border,
        border_color: round.border_color,
        clip_shape: q.clip_round.map_or([0.0; 4], |c| {
            [c.center[0], c.center[1], c.half[0], c.half[1]]
        }),
        clip_radii: q.clip_round.map_or([0.0; 4], |c| c.radii),
        gradient_to: round
            .sweep
            .map(|s| s.1)
            .or(round.gradient.map(|g| g.to))
            .unwrap_or([0.0; 4]),
        gradient: round.gradient.map_or([0.0; 2], |g| [g.angle, 1.0]),
        gradient_via: round.gradient.map_or([0.0; 4], |g| g.via),
        sweep: round.sweep.map_or([round.blur, 0.0], |s| [s.0, 1.0]),
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    fragment: &str,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
    vertex: wgpu::VertexBufferLayout,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[vertex],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

impl Gfx {
    pub async fn new(window: Arc<Window>, sprites: &Sprites) -> Result<Gfx, GfxError> {
        let gpu = GpuContext::new(window, GpuLimits::WebGl2).await?;
        let (device, queue, config) = (&gpu.device, &gpu.queue, &gpu.config);

        let atlas = gpu.create_ui_atlas();
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let atlas_bind_group = bind_group(device, &bgl, &uniform, &atlas_view, &linear);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad-shader"),
            source: wgpu::ShaderSource::Wgsl(QUAD_WGSL.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let mask_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_mask",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            quad_layout(),
        );
        let pixel_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_pixel",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            quad_layout(),
        );
        let rgba_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_rgba",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            quad_layout(),
        );
        let smooth_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_smooth",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
            quad_layout(),
        );
        let add_mask_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_mask_add",
            LAYER_FORMAT,
            ADDITIVE,
            quad_layout(),
        );
        let add_smooth_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_smooth_add",
            LAYER_FORMAT,
            ADDITIVE,
            quad_layout(),
        );
        let rich_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rich-shader"),
            source: wgpu::ShaderSource::Wgsl(RICH_WGSL.into()),
        });
        let rich = |fragment: &str| {
            pipeline(
                device,
                &layout,
                &rich_shader,
                fragment,
                config.format,
                wgpu::BlendState::ALPHA_BLENDING,
                rich_layout(),
            )
        };
        let rich_mask_pipeline = rich("fs_mask");
        let rich_rgba_pipeline = rich("fs_rgba");
        let rich_smooth_pipeline = rich("fs_smooth");
        let rich_glass_pipeline = rich("fs_glass");
        let layer_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layer-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("layer-layout"),
                bind_group_layouts: &[Some(&layer_layout)],
                immediate_size: 0,
            });
        let layer_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("layer-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mut sprite_textures = Vec::new();
        let mut sprite_bind_groups = Vec::new();
        for img in sprites.images() {
            let extent = wgpu::Extent3d {
                width: img.width,
                height: img.height,
                depth_or_array_layers: 1,
            };
            let tex = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("sprite-texture"),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &img.rgba,
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            sprite_bind_groups.push(bind_group(device, &bgl, &uniform, &view, &linear));
            sprite_textures.push(tex);
        }

        let blur_source = format!("{LAYER_PRELUDE}\n{BLUR_WGSL}");
        let blur_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur-shader"),
            source: wgpu::ShaderSource::Wgsl(blur_source.into()),
        });
        let blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blur-pipeline"),
            layout: Some(&layer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blur_module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &blur_module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let vertex_cap = 4096 * std::mem::size_of::<QuadVertex>() as u64;
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-verts"),
            size: vertex_cap,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rich_cap = 256 * std::mem::size_of::<RichVertex>() as u64;
        let rich_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rich-verts"),
            size: rich_cap,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_cap = 6144 * std::mem::size_of::<u32>() as u64;
        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-indices"),
            size: index_cap,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Gfx {
            gpu,
            atlas,
            uniform,
            mask_pipeline,
            rgba_pipeline,
            pixel_pipeline,
            smooth_pipeline,
            add_mask_pipeline,
            add_smooth_pipeline,
            rich_mask_pipeline,
            rich_rgba_pipeline,
            rich_smooth_pipeline,
            rich_glass_pipeline,
            blur_pipeline,
            blur_targets: None,
            quad_layout_group: bgl.clone(),
            linear_sampler: linear.clone(),
            layer_layout,
            layer_pipeline_layout,
            layer_sampler,
            layer_programs: Vec::new(),
            atlas_bind_group,
            sprite_bind_groups,
            sprite_textures,
            vertex_buf,
            vertex_cap,
            index_buf,
            index_cap,
            verts: Vec::new(),
            rich_verts: Vec::new(),
            rich_buf,
            rich_cap,
            indices: Vec::new(),
        })
    }

    pub fn size(&self) -> (f32, f32) {
        self.gpu.size()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
    }

    pub fn sync_sprites(&self, sprites: &Sprites, dirty: &[usize]) {
        for &i in dirty {
            let (Some(tex), Some(img)) = (self.sprite_textures.get(i), sprites.images().get(i))
            else {
                continue;
            };
            self.gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &img.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(img.width * 4),
                    rows_per_image: Some(img.height),
                },
                wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn sync_atlas(&self) {
        self.gpu.sync_ui_atlas(&self.atlas);
    }

    fn scissor(&self, clip: [f32; 4]) -> [u32; 4] {
        let sw = self.gpu.config.width as f32;
        let sh = self.gpu.config.height as f32;
        let x0 = clip[0].clamp(0.0, sw);
        let y0 = clip[1].clamp(0.0, sh);
        let x1 = (clip[0] + clip[2]).clamp(0.0, sw);
        let y1 = (clip[1] + clip[3]).clamp(0.0, sh);
        [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]
    }

    fn batch(
        &mut self,
        quads: &[DrawRect],
        order: impl Iterator<Item = usize>,
        ranges: &mut Vec<Range>,
    ) {
        let mut cur: Option<(u32, bool, bool, [u32; 4])> = None;
        let mut start = self.indices.len() as u32;
        for qi in order {
            let q = &quads[qi];
            let sc = self.scissor(q.clip);
            let rich = q.round.is_some() || q.clip_round.is_some();
            let glass = q.round.is_some_and(|r| r.blur > 0.0);
            if cur != Some((q.tex, rich, glass, sc)) {
                if let Some((tex, rich, glass, scissor)) = cur {
                    ranges.push(Range {
                        tex,
                        rich,
                        glass,
                        start,
                        count: self.indices.len() as u32 - start,
                        scissor,
                    });
                }
                cur = Some((q.tex, rich, glass, sc));
                start = self.indices.len() as u32;
            }
            let [x0, y0] = q.min;
            let [x1, y1] = q.max;
            let pts = q.quad.unwrap_or([[x0, y0], [x1, y0], [x0, y1], [x1, y1]]);
            let [u0, v0] = q.uv_min;
            let [u1, v1] = q.uv_max;
            let uvs = q.uvs.unwrap_or([[u0, v0], [u1, v0], [u0, v1], [u1, v1]]);
            let silhouette = f32::from(u8::from(q.silhouette));
            let base = if rich {
                self.rich_verts.len()
            } else {
                self.verts.len()
            } as u32;
            for (pos, uv) in pts.into_iter().zip(uvs) {
                if rich {
                    self.rich_verts.push(rich_vertex(q, pos, uv, silhouette));
                } else {
                    self.verts.push(QuadVertex {
                        pos,
                        uv,
                        color: q.color,
                        silhouette,
                    });
                }
            }
            self.indices
                .extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
        }
        if let Some((tex, rich, glass, scissor)) = cur {
            ranges.push(Range {
                tex,
                rich,
                glass,
                start,
                count: self.indices.len() as u32 - start,
                scissor,
            });
        }
    }

    fn upload(&mut self) {
        let vbytes: &[u8] = bytemuck::cast_slice(&self.verts);
        if vbytes.len() as u64 > self.vertex_cap {
            self.vertex_cap = (vbytes.len() as u64).next_power_of_two();
            self.vertex_buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad-verts"),
                size: self.vertex_cap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !vbytes.is_empty() {
            self.gpu.queue.write_buffer(&self.vertex_buf, 0, vbytes);
        }
        let rbytes: &[u8] = bytemuck::cast_slice(&self.rich_verts);
        if rbytes.len() as u64 > self.rich_cap {
            self.rich_cap = (rbytes.len() as u64).next_power_of_two();
            self.rich_buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rich-verts"),
                size: self.rich_cap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !rbytes.is_empty() {
            self.gpu.queue.write_buffer(&self.rich_buf, 0, rbytes);
        }
        let ibytes: &[u8] = bytemuck::cast_slice(&self.indices);
        if ibytes.len() as u64 > self.index_cap {
            self.index_cap = (ibytes.len() as u64).next_power_of_two();
            self.index_buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad-indices"),
                size: self.index_cap,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !ibytes.is_empty() {
            self.gpu.queue.write_buffer(&self.index_buf, 0, ibytes);
        }
    }

    fn fullscreen(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        source: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blur-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, source, &[]);
        pass.draw(0..3, 0..1);
    }

    fn blur_step(&self, step: [f32; 2]) -> wgpu::Buffer {
        let buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur-uniform"),
            size: std::mem::size_of::<LayerUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut params = [0.0; 16];
        params[..2].copy_from_slice(&step);
        let uniform = LayerUniform {
            screen: [0.0; 2],
            time: 0.0,
            pad: 0.0,
            params,
        };
        self.gpu
            .queue
            .write_buffer(&buffer, 0, bytemuck::bytes_of(&uniform));
        buffer
    }

    fn blur_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blur-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.gpu.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn ensure_blur(&mut self) {
        let size = (self.gpu.config.width.max(1), self.gpu.config.height.max(1));
        if self.blur_targets.as_ref().is_some_and(|b| b.size == size) {
            return;
        }
        let (small_w, small_h) = (
            (size.0 / BLUR_DIVISOR).max(1),
            (size.1 / BLUR_DIVISOR).max(1),
        );
        let scene = self.blur_texture(size.0, size.1);
        let first = self.blur_texture(small_w, small_h);
        let second = self.blur_texture(small_w, small_h);
        let across = self.blur_step([1.0 / small_w as f32, 0.0]);
        let down = self.blur_step([0.0, 1.0 / small_h as f32]);
        let still = self.blur_step([0.0, 0.0]);
        let group = |view: &wgpu::TextureView, uniform: &wgpu::Buffer| {
            bind_group(
                &self.gpu.device,
                &self.layer_layout,
                uniform,
                view,
                &self.layer_sampler,
            )
        };
        let passes = [
            (first.clone(), group(&scene, &across)),
            (second.clone(), group(&first, &down)),
        ];
        let blit = group(&scene, &still);
        let glass = bind_group(
            &self.gpu.device,
            &self.quad_layout_group,
            &self.uniform,
            &second,
            &self.linear_sampler,
        );
        self.blur_targets = Some(BlurTargets {
            size,
            scene,
            passes,
            blit,
            glass,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ranges(
        &self,
        pass: &mut wgpu::RenderPass,
        ranges: &[Range],
        start: usize,
        end: usize,
        composite_at: Option<usize>,
        world_ranges: usize,
        pixel_world: bool,
    ) {
        let mut bound_rich = false;
        if start < end {
            pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        }
        let (width, height) = (self.gpu.config.width, self.gpu.config.height);
        for i in start..=end {
            if composite_at == Some(i) {
                for program in &self.layer_programs {
                    pass.set_scissor_rect(0, 0, width, height);
                    pass.set_pipeline(&program.pipeline);
                    pass.set_bind_group(0, &program.group, &[]);
                    pass.draw(0..3, 0..1);
                }
            }
            if i == end {
                break;
            }
            let Some(r) = ranges.get(i) else {
                break;
            };
            if r.scissor[2] == 0 || r.scissor[3] == 0 || r.count == 0 {
                continue;
            }
            if r.rich != bound_rich {
                let buffer = if r.rich {
                    &self.rich_buf
                } else {
                    &self.vertex_buf
                };
                pass.set_vertex_buffer(0, buffer.slice(..));
                bound_rich = r.rich;
            }
            let glass = self.blur_targets.as_ref().filter(|_| r.glass);
            if let Some(blur) = glass {
                pass.set_pipeline(&self.rich_glass_pipeline);
                pass.set_bind_group(0, &blur.glass, &[]);
            } else if r.tex == 0 {
                pass.set_pipeline(if r.rich {
                    &self.rich_mask_pipeline
                } else {
                    &self.mask_pipeline
                });
                pass.set_bind_group(0, &self.atlas_bind_group, &[]);
            } else if let Some(bg) = self.sprite_bind_groups.get(r.tex as usize - 1) {
                pass.set_pipeline(if r.rich && pixel_world {
                    &self.rich_rgba_pipeline
                } else if r.rich {
                    &self.rich_smooth_pipeline
                } else if !pixel_world {
                    &self.smooth_pipeline
                } else if i < world_ranges {
                    &self.pixel_pipeline
                } else {
                    &self.rgba_pipeline
                });
                pass.set_bind_group(0, bg, &[]);
            } else {
                continue;
            }
            let [x, y, w, h] = r.scissor;
            pass.set_scissor_rect(x, y, w, h);
            pass.draw_indexed(r.start..r.start + r.count, 0, 0..1);
        }
    }

    fn layer_target(&self, uniform: &wgpu::Buffer) -> (wgpu::TextureView, wgpu::BindGroup) {
        let device = &self.gpu.device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer-target"),
            size: wgpu::Extent3d {
                width: self.gpu.config.width.max(1),
                height: self.gpu.config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: LAYER_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = bind_group(
            device,
            &self.layer_layout,
            uniform,
            &target,
            &self.layer_sampler,
        );
        (target, group)
    }

    fn sync_layers(&mut self, layers: &LayerFrame) {
        let size = (self.gpu.config.width, self.gpu.config.height);
        self.layer_programs.retain(|p| {
            layers
                .shaders
                .iter()
                .any(|s| s.layer == p.layer && s.source == p.source)
        });
        for shader in layers.shaders {
            let known = self.layer_programs.iter().any(|p| p.layer == shader.layer);
            if !known {
                let program = self.compile_layer(shader);
                self.layer_programs.push(program);
            }
            let Some(at) = self
                .layer_programs
                .iter()
                .position(|p| p.layer == shader.layer)
            else {
                continue;
            };
            if self.layer_programs[at].size != size {
                let (target, group) = self.layer_target(&self.layer_programs[at].uniform);
                let program = &mut self.layer_programs[at];
                program.target = target;
                program.group = group;
                program.size = size;
            }
            let uniform = LayerUniform {
                screen: [size.0 as f32, size.1 as f32],
                time: layers.time,
                pad: 0.0,
                params: shader.params,
            };
            self.gpu.queue.write_buffer(
                &self.layer_programs[at].uniform,
                0,
                bytemuck::bytes_of(&uniform),
            );
        }
    }

    fn compile_layer(&self, shader: &LayerShader) -> LayerProgram {
        let device = &self.gpu.device;
        if let Err(error) = shader.validate() {
            log::error!("layer shader {}: {error}", shader.layer);
        }
        let source = format!("{LAYER_PRELUDE}\n{}", shader.source);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("layer-shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("layer-pipeline"),
            layout: Some(&self.layer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.gpu.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("layer-uniform"),
            size: std::mem::size_of::<LayerUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (target, group) = self.layer_target(&uniform);
        LayerProgram {
            layer: shader.layer,
            source: shader.source,
            pipeline,
            uniform,
            target,
            group,
            size: (self.gpu.config.width, self.gpu.config.height),
        }
    }

    pub fn render(
        &mut self,
        clear: Color,
        world: &[DrawRect],
        ui: &[DrawRect],
        pixel_world: bool,
        layers: &LayerFrame,
    ) {
        self.sync_layers(layers);
        self.sync_atlas();
        let dims = [
            self.gpu.config.width as f32,
            self.gpu.config.height as f32,
            0.0,
            0.0,
        ];
        self.gpu
            .queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&dims));

        self.verts.clear();
        self.rich_verts.clear();
        self.indices.clear();
        let mut ranges = Vec::new();

        self.batch(world, 0..world.len(), &mut ranges);
        let drawn_world = ranges.len();
        let world_ranges = if pixel_world { drawn_world } else { 0 };

        let mut layer_ranges: Vec<(usize, Vec<Range>)> = Vec::new();
        for (layer, draws) in layers.draws {
            let Some(at) = self.layer_programs.iter().position(|p| p.layer == *layer) else {
                continue;
            };
            let mut own = Vec::new();
            self.batch(draws, 0..draws.len(), &mut own);
            layer_ranges.push((at, own));
        }

        let mut ui_order: Vec<usize> = (0..ui.len()).collect();
        ui_order.sort_by(|&a, &b| {
            ui[a]
                .z
                .partial_cmp(&ui[b].z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.batch(ui, ui_order.into_iter(), &mut ranges);
        self.upload();

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for program in &self.layer_programs {
            let own = layer_ranges
                .iter()
                .find(|(at, _)| std::ptr::eq(&self.layer_programs[*at], program));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("layer-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &program.target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let Some((_, own)) = own else {
                continue;
            };
            pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            for r in own {
                if r.tex == 0 {
                    pass.set_pipeline(&self.add_mask_pipeline);
                    pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                } else if let Some(bg) = self.sprite_bind_groups.get(r.tex as usize - 1) {
                    pass.set_pipeline(&self.add_smooth_pipeline);
                    pass.set_bind_group(0, bg, &[]);
                } else {
                    continue;
                }
                pass.draw_indexed(r.start..r.start + r.count, 0, 0..1);
            }
        }
        let has_glass = ranges.iter().any(|r| r.glass);
        if has_glass {
            self.ensure_blur();
        }
        let split = if has_glass { drawn_world } else { ranges.len() };
        {
            let c = clear.to_array();
            let target = match (&self.blur_targets, has_glass) {
                (Some(blur), true) => &blur.scene,
                _ => &view,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: c[0] as f64,
                            g: c[1] as f64,
                            b: c[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.draw_ranges(
                &mut pass,
                &ranges,
                0,
                split,
                Some(drawn_world),
                world_ranges,
                pixel_world,
            );
        }
        if let (true, Some(blur)) = (has_glass, &self.blur_targets) {
            for (target, source) in &blur.passes {
                self.fullscreen(&mut encoder, target, source);
            }
            self.fullscreen(&mut encoder, &view, &blur.blit);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.draw_ranges(
                &mut pass,
                &ranges,
                split,
                ranges.len(),
                None,
                world_ranges,
                pixel_world,
            );
        }
        self.gpu.queue.submit([encoder.finish()]);
        frame.present();
    }
}

#[cfg(test)]
mod quad_shader_tests {
    fn validate(source: &str) {
        use wgpu::naga;
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)
        .unwrap();
    }

    #[test]
    fn both_quad_shaders_are_valid() {
        validate(super::QUAD_WGSL);
        validate(super::RICH_WGSL);
        validate(&format!("{}\n{}", super::LAYER_PRELUDE, super::BLUR_WGSL));
    }

    #[test]
    fn plain_quads_pay_nothing_for_css_features() {
        assert_eq!(std::mem::size_of::<super::QuadVertex>(), 36);
        assert!(!super::QUAD_WGSL.contains("round_paint"));
    }
}
