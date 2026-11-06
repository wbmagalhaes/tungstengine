use crate::ui::{DrawRect, RoundShape};

pub(super) const QUAD_WGSL: &str = r#"
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
pub(super) struct QuadVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub silhouette: f32,
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

pub(super) fn quad_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub(super) fn bind_group(
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

pub(super) fn quad_layout() -> wgpu::VertexBufferLayout<'static> {
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

pub(super) fn pipeline(
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

struct UiRange {
    tex: u32,
    rich: bool,
    glass: bool,
    start: u32,
    count: u32,
    scissor: [u32; 4],
}

pub struct UiRenderer {
    mask: wgpu::RenderPipeline,
    smooth: wgpu::RenderPipeline,
    rgba: wgpu::RenderPipeline,
    rich_mask: wgpu::RenderPipeline,
    rich_smooth: wgpu::RenderPipeline,
    rich_rgba: wgpu::RenderPipeline,
    glass: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    atlas_group: wgpu::BindGroup,
    sprite_groups: Vec<wgpu::BindGroup>,
    glass_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    verts: Vec<QuadVertex>,
    rich_verts: Vec<RichVertex>,
    indices: Vec<u32>,
    ranges: Vec<UiRange>,
    vertex_buf: wgpu::Buffer,
    vertex_cap: u64,
    rich_buf: wgpu::Buffer,
    rich_cap: u64,
    index_buf: wgpu::Buffer,
    index_cap: u64,
    size: (f32, f32),
}

fn grow(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    cap: &mut u64,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) {
    if bytes.len() as u64 > *cap {
        *cap = (bytes.len() as u64).next_power_of_two();
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-buffer"),
            size: *cap,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
    if !bytes.is_empty() {
        queue.write_buffer(buffer, 0, bytes);
    }
}

fn buffer(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ui-buffer"),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        atlas: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let layout = quad_bind_layout(device);
        let uniform = buffer(device, 16, wgpu::BufferUsages::UNIFORM);
        let atlas_group = bind_group(device, &layout, &uniform, atlas, sampler);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let plain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui-shader"),
            source: wgpu::ShaderSource::Wgsl(QUAD_WGSL.into()),
        });
        let rich_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui-rich-shader"),
            source: wgpu::ShaderSource::Wgsl(RICH_WGSL.into()),
        });
        let blend = wgpu::BlendState::ALPHA_BLENDING;
        let plain = |fragment: &str| {
            pipeline(
                device,
                &pipeline_layout,
                &plain_shader,
                fragment,
                format,
                blend,
                quad_layout(),
            )
        };
        let rich = |fragment: &str| {
            pipeline(
                device,
                &pipeline_layout,
                &rich_shader,
                fragment,
                format,
                blend,
                rich_layout(),
            )
        };
        let vertex_cap = 4096 * std::mem::size_of::<QuadVertex>() as u64;
        let rich_cap = 256 * std::mem::size_of::<RichVertex>() as u64;
        let index_cap = 6144 * std::mem::size_of::<u32>() as u64;
        UiRenderer {
            mask: plain("fs_mask"),
            smooth: plain("fs_smooth"),
            rgba: plain("fs_rgba"),
            rich_mask: rich("fs_mask"),
            rich_smooth: rich("fs_smooth"),
            rich_rgba: rich("fs_rgba"),
            glass: rich("fs_glass"),
            atlas_group,
            sprite_groups: Vec::new(),
            glass_group: None,
            sampler: sampler.clone(),
            verts: Vec::new(),
            rich_verts: Vec::new(),
            indices: Vec::new(),
            ranges: Vec::new(),
            vertex_buf: buffer(device, vertex_cap, wgpu::BufferUsages::VERTEX),
            vertex_cap,
            rich_buf: buffer(device, rich_cap, wgpu::BufferUsages::VERTEX),
            rich_cap,
            index_buf: buffer(device, index_cap, wgpu::BufferUsages::INDEX),
            index_cap,
            size: (1.0, 1.0),
            layout,
            uniform,
        }
    }

    pub fn add_sprite(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        let group = bind_group(device, &self.layout, &self.uniform, view, &self.sampler);
        self.sprite_groups.push(group);
    }

    pub fn set_glass(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        let group = bind_group(device, &self.layout, &self.uniform, view, &self.sampler);
        self.glass_group = Some(group);
    }

    pub fn wants_glass(&self) -> bool {
        self.ranges.iter().any(|r| r.glass)
    }

    fn scissor(&self, clip: [f32; 4]) -> [u32; 4] {
        let (w, h) = self.size;
        let x0 = clip[0].clamp(0.0, w);
        let y0 = clip[1].clamp(0.0, h);
        let x1 = (clip[0] + clip[2]).clamp(0.0, w);
        let y1 = (clip[1] + clip[3]).clamp(0.0, h);
        [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draws: &[DrawRect],
        size: (f32, f32),
    ) {
        self.size = size;
        let dims = [size.0, size.1, 0.0, 0.0];
        queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&dims));
        self.verts.clear();
        self.rich_verts.clear();
        self.indices.clear();
        self.ranges.clear();
        let mut order: Vec<usize> = (0..draws.len()).collect();
        order.sort_by(|&a, &b| {
            draws[a]
                .z
                .partial_cmp(&draws[b].z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut cur: Option<(u32, bool, bool, [u32; 4])> = None;
        let mut start = 0;
        for qi in order {
            let q = &draws[qi];
            let scissor = self.scissor(q.clip);
            let rich = q.round.is_some() || q.clip_round.is_some();
            let glass = q.round.is_some_and(|r| r.blur > 0.0);
            let key = (q.tex, rich, glass, scissor);
            if cur != Some(key) {
                self.close_range(cur, start);
                cur = Some(key);
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
        self.close_range(cur, start);
        let vertex = wgpu::BufferUsages::VERTEX;
        let plain: &[u8] = bytemuck::cast_slice(&self.verts);
        grow(
            device,
            queue,
            &mut self.vertex_buf,
            &mut self.vertex_cap,
            plain,
            vertex,
        );
        let fancy: &[u8] = bytemuck::cast_slice(&self.rich_verts);
        grow(
            device,
            queue,
            &mut self.rich_buf,
            &mut self.rich_cap,
            fancy,
            vertex,
        );
        let index: &[u8] = bytemuck::cast_slice(&self.indices);
        let usage = wgpu::BufferUsages::INDEX;
        grow(
            device,
            queue,
            &mut self.index_buf,
            &mut self.index_cap,
            index,
            usage,
        );
    }

    fn close_range(&mut self, cur: Option<(u32, bool, bool, [u32; 4])>, start: u32) {
        if let Some((tex, rich, glass, scissor)) = cur {
            self.ranges.push(UiRange {
                tex,
                rich,
                glass,
                start,
                count: self.indices.len() as u32 - start,
                scissor,
            });
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass, pixel_sprites: bool) {
        if self.ranges.is_empty() {
            return;
        }
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        let mut bound_rich = false;
        for r in &self.ranges {
            if r.scissor[2] == 0 || r.scissor[3] == 0 || r.count == 0 {
                continue;
            }
            if r.rich != bound_rich {
                let source = if r.rich {
                    &self.rich_buf
                } else {
                    &self.vertex_buf
                };
                pass.set_vertex_buffer(0, source.slice(..));
                bound_rich = r.rich;
            }
            let glass = self.glass_group.as_ref().filter(|_| r.glass);
            if let Some(group) = glass {
                pass.set_pipeline(&self.glass);
                pass.set_bind_group(0, group, &[]);
            } else if r.tex == 0 {
                pass.set_pipeline(if r.rich { &self.rich_mask } else { &self.mask });
                pass.set_bind_group(0, &self.atlas_group, &[]);
            } else if let Some(group) = self.sprite_groups.get(r.tex as usize - 1) {
                pass.set_pipeline(match (r.rich, pixel_sprites) {
                    (true, true) => &self.rich_rgba,
                    (true, false) => &self.rich_smooth,
                    (false, true) => &self.rgba,
                    (false, false) => &self.smooth,
                });
                pass.set_bind_group(0, group, &[]);
            } else {
                continue;
            }
            let [x, y, w, h] = r.scissor;
            pass.set_scissor_rect(x, y, w, h);
            pass.draw_indexed(r.start..r.start + r.count, 0, 0..1);
        }
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
    }

    #[test]
    fn plain_quads_pay_nothing_for_css_features() {
        assert_eq!(std::mem::size_of::<super::QuadVertex>(), 36);
        assert!(!super::QUAD_WGSL.contains("round_paint"));
    }
}
