use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::ecs::prelude::*;
use crate::ui::DrawRect;

use super::post::{FULLSCREEN_VERTEX, PASSTHROUGH_FRAGMENT, PostRegistry};
use super::shader::{MATERIAL_FLOATS, ShaderRegistry};
use super::transition::TransitionView;
use super::view::{self, Scene};
use crate::texture::Texture2D;

#[derive(Resource)]
pub struct GfxConfig {
    pub shadow_size: u32,
    pub msaa: u32,
}

impl Default for GfxConfig {
    fn default() -> Self {
        GfxConfig {
            shadow_size: 2048,
            msaa: 1,
        }
    }
}

const UI_WGSL: &str = r#"
struct Globals { screen: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

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
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas, samp, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * a);
}
"#;

const MASK_WGSL: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) shader: u32,
};

@vertex
fn vs(in: VsIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    return camera.view_proj * model * vec4<f32>(in.position, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

const SHADOW_WGSL: &str = r#"
struct Light { light_vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> light: Light;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
};

@vertex
fn vs(in: VsIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    return light.light_vp * model * vec4<f32>(in.position, 1.0);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(p.z, 0.0, 0.0, 1.0);
}
"#;

const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

const SPRITE_UI_WGSL: &str = r#"
struct Globals { screen: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

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
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(atlas, samp, in.uv) * in.color;
}
"#;

const OUTLINE_WGSL: &str = r#"
struct O { color: vec4<f32>, params: vec4<f32> };
@group(0) @binding(0) var<uniform> o: O;
@group(0) @binding(1) var mask: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[i];
    var out: VsOut;
    out.clip = vec4<f32>(xy, 0.0, 1.0);
    out.uv = xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let center = textureSample(mask, samp, in.uv).r;
    let texel = o.params.xy;
    let r = o.params.z;
    var m = 0.0;
    for (var k = 0; k < 16; k = k + 1) {
        let ang = f32(k) / 16.0 * 6.2831853;
        let off = vec2<f32>(cos(ang), sin(ang)) * texel * r;
        m = max(m, textureSample(mask, samp, in.uv + off).r);
    }
    let outside = 1.0 - step(0.5, center);
    let edge = step(0.5, m);
    let a = outside * edge * o.color.a;
    return vec4<f32>(o.color.rgb, a);
}
"#;

const TRANSITION_WGSL: &str = r#"
struct T {
    color: vec4<f32>,
    cutoff: f32,
    soft: f32,
    aspect: f32,
    pad: f32,
};
@group(0) @binding(0) var<uniform> t: T;
@group(0) @binding(1) var mask: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[i];
    var out: VsOut;
    out.clip = vec4<f32>(xy, 0.0, 1.0);
    out.uv = xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var p = in.uv - vec2<f32>(0.5, 0.5);
    if (t.aspect >= 1.0) {
        p.y = p.y / t.aspect;
    } else {
        p.x = p.x * t.aspect;
    }
    let uv = p + vec2<f32>(0.5, 0.5);
    let m = textureSample(mask, samp, uv).r;
    let edge = 1.0 - t.cutoff;
    let coverage = smoothstep(edge - t.soft, edge, m);
    return vec4<f32>(t.color.rgb, t.color.a * coverage);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    shader: u32,
    mr: [f32; 2],
    uvt: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient: [f32; 4],
    sky: [f32; 4],
    cam_pos: [f32; 4],
}

type DrawKey = (usize, usize, usize, usize, usize, usize);
type SceneDraw = (usize, usize, usize, usize, usize, usize, wgpu::Buffer, u32);

struct ShaderProgram {
    pipelines: [wgpu::RenderPipeline; 3],
    pipelines_no_depth: [wgpu::RenderPipeline; 3],
    has_params: bool,
}

struct PostProgram {
    pipeline: wgpu::RenderPipeline,
    has_params: bool,
}

struct MeshBuf {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

fn make_depth(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn glsl_module(
    device: &wgpu::Device,
    src: &str,
    stage: wgpu::naga::ShaderStage,
) -> wgpu::ShaderModule {
    let define = match stage {
        wgpu::naga::ShaderStage::Vertex => "VERTEX",
        _ => "FRAGMENT",
    };
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("glsl"),
        source: wgpu::ShaderSource::Glsl {
            shader: src.into(),
            stage,
            defines: &[(define, "1")],
        },
    })
}

const DEPTH_TARGET: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

const EXAMINE_FRAGMENT: &str = r#"
#version 450
layout(set = 0, binding = 0) uniform texture2D scene_tex;
layout(set = 0, binding = 1) uniform sampler scene_samp;
layout(set = 0, binding = 2) uniform Globals { vec4 p0; vec4 p1; } g;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 frag_out;
void main() {
    vec4 c = texture(sampler2D(scene_tex, scene_samp), v_uv);
    float intro = g.p0.x;
    float dim = 0.62;
    frag_out = vec4(c.rgb * c.a, max(c.a, dim)) * intro;
}
"#;

#[derive(Default)]
struct MsTargets {
    color: Option<wgpu::TextureView>,
    depth_target: Option<wgpu::TextureView>,
    depth: Option<wgpu::TextureView>,
    mask: Option<wgpu::TextureView>,
    textures: Vec<wgpu::Texture>,
}

fn make_msaa(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    samples: u32,
) -> MsTargets {
    if samples <= 1 {
        return MsTargets::default();
    }
    let mut textures = Vec::new();
    let mut mk = |format: wgpu::TextureFormat| {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa"),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        textures.push(tex);
        view
    };
    let color = Some(mk(HDR_FORMAT));
    let depth_target = Some(mk(DEPTH_TARGET));
    let depth = Some(mk(wgpu::TextureFormat::Depth32Float));
    let mask = Some(mk(wgpu::TextureFormat::R8Unorm));
    MsTargets {
        color,
        depth_target,
        depth,
        mask,
        textures,
    }
}

fn make_offscreen(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

fn make_mask(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("outline-mask"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn make_outline_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    mask_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("outline-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(mask_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub struct Gfx {
    gpu: GpuContext,

    ui_pipeline: wgpu::RenderPipeline,
    ui_bind_group: wgpu::BindGroup,
    ui_uniform: wgpu::Buffer,
    ui_vertex_buf: wgpu::Buffer,
    ui_vertex_cap: u64,
    ui_index_buf: wgpu::Buffer,
    ui_index_cap: u64,
    ui_verts: Vec<UiVertex>,
    ui_indices: Vec<u32>,
    ui_order: Vec<usize>,
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_bind_groups: Vec<wgpu::BindGroup>,
    _sprite_textures: Vec<wgpu::Texture>,
    atlas: wgpu::Texture,
    _atlas_view: wgpu::TextureView,
    _sampler: wgpu::Sampler,

    programs: Vec<ShaderProgram>,
    camera_bgl: wgpu::BindGroupLayout,
    material_bgl: wgpu::BindGroupLayout,
    default_material_bg: wgpu::BindGroup,
    texture_bgl: wgpu::BindGroupLayout,
    mesh_textures: Vec<wgpu::BindGroup>,
    _model_textures: Vec<wgpu::Texture>,
    camera_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    shadow_enabled: bool,
    shadow_buffer: wgpu::Buffer,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_pass_bg: wgpu::BindGroup,
    shadow_depth_view: wgpu::TextureView,
    shadow_color_view: wgpu::TextureView,
    _shadow_color: wgpu::Texture,
    _shadow_depth: wgpu::Texture,

    samples: u32,
    ms_color: Option<wgpu::TextureView>,
    ms_depth_target: Option<wgpu::TextureView>,
    ms_depth: Option<wgpu::TextureView>,
    ms_mask: Option<wgpu::TextureView>,
    _ms_textures: Vec<wgpu::Texture>,

    offscreen: [wgpu::Texture; 2],
    offscreen_views: [wgpu::TextureView; 2],
    offscreen_depth: wgpu::Texture,
    offscreen_depth_view: wgpu::TextureView,
    post_bgl: wgpu::BindGroupLayout,
    post_sampler: wgpu::Sampler,
    post_globals: wgpu::Buffer,
    post_programs: Vec<PostProgram>,
    passthrough_pipeline: wgpu::RenderPipeline,
    examine_pipeline: wgpu::RenderPipeline,

    meshes: Vec<MeshBuf>,
    depth_view: wgpu::TextureView,

    outline_mask_view: wgpu::TextureView,
    outline_mask_pipeline: wgpu::RenderPipeline,
    outline_pipeline: wgpu::RenderPipeline,
    outline_uniform: wgpu::Buffer,
    outline_sampler: wgpu::Sampler,
    outline_bgl: wgpu::BindGroupLayout,
    outline_bind_group: wgpu::BindGroup,

    transition_pipeline: wgpu::RenderPipeline,
    transition_uniform: wgpu::Buffer,
    transition_bind_group: wgpu::BindGroup,
    _mask: wgpu::Texture,
    _mask_view: wgpu::TextureView,
    _mask_sampler: wgpu::Sampler,

    last_stats: view::RenderStats,
    vram_bytes: u64,
}

pub use crate::render::core::GfxError;
use crate::render::core::{GpuContext, GpuLimits};

impl Gfx {
    pub async fn new(
        window: Arc<Window>,
        registry: &view::MeshRegistry,
        shaders: &ShaderRegistry,
        sprites: &crate::sprites::Sprites,
        mask_img: &Texture2D,
        shadow_size: u32,
        samples: u32,
    ) -> Result<Gfx, GfxError> {
        let samples = samples.max(1);
        let shadow_enabled = shadow_size > 0;
        let shadow_size = shadow_size.max(1);
        let gpu = GpuContext::new(window, GpuLimits::Default).await?;
        let (device, queue, config) = (&gpu.device, &gpu.queue, &gpu.config);

        let atlas = gpu.create_ui_atlas();
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let ui_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ui_vertex_cap = 4096 * std::mem::size_of::<UiVertex>() as u64;
        let ui_vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-verts"),
            size: ui_vertex_cap,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_index_cap = 6144 * std::mem::size_of::<u32>() as u64;
        let ui_index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui-indices"),
            size: ui_index_cap,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let ui_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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

        let ui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui-bg"),
            layout: &ui_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ui_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui-shader"),
            source: wgpu::ShaderSource::Wgsl(UI_WGSL.into()),
        });
        let ui_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui-layout"),
            bind_group_layouts: &[Some(&ui_bgl)],
            immediate_size: 0,
        });
        let ui_attrs = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui-pipeline"),
            layout: Some(&ui_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ui_attrs,
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_UI_WGSL.into()),
        });
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite-pipeline"),
            layout: Some(&ui_layout),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ui_attrs,
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &sprite_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sprite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mut sprite_textures = Vec::new();
        let mut sprite_bind_groups = Vec::new();
        for img in sprites.images() {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sprite-texture"),
                size: wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
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
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            sprite_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sprite-bg"),
                layout: &ui_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ui_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sprite_sampler),
                    },
                ],
            }));
            sprite_textures.push(tex);
        }

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("light"),
            size: std::mem::size_of::<LightUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-vp"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow-color"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_color_view = shadow_color.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow-depth"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let shadow_depth_view = shadow_depth.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-bgl"),
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &camera_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shadow_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&shadow_color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_pass_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow-pass-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let shadow_pass_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow-pass-bg"),
            layout: &shadow_pass_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_buffer.as_entire_binding(),
            }],
        });

        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let default_material_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("default-material"),
                contents: bytemuck::cast_slice(&[0.0f32; MATERIAL_FLOATS]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let default_material_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("default-material-bg"),
            layout: &material_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: default_material_buffer.as_entire_binding(),
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let model_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("model-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let upload_texture = |img: &Texture2D| -> wgpu::Texture {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("model-texture"),
                size: wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
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
            tex
        };
        let white = Texture2D {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        };
        let mut model_textures: Vec<wgpu::Texture> = Vec::new();
        let mut model_views: Vec<wgpu::TextureView> = Vec::new();
        model_textures.push(upload_texture(&white));
        for img in registry.images() {
            model_textures.push(upload_texture(img));
        }
        for tex in &model_textures {
            model_views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        let make_texture_bg = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("texture-bg"),
                layout: &texture_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&model_sampler),
                    },
                ],
            })
        };
        let mesh_textures: Vec<wgpu::BindGroup> = registry
            .mesh_textures()
            .iter()
            .map(|slot| {
                let view = match slot {
                    Some(i) => &model_views[i + 1],
                    None => &model_views[0],
                };
                make_texture_bg(view)
            })
            .collect();

        let post_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let post_globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post-globals"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (offscreen_a, offscreen_a_view) = make_offscreen(device, config, HDR_FORMAT);
        let (offscreen_b, offscreen_b_view) = make_offscreen(device, config, HDR_FORMAT);
        let (offscreen_depth, offscreen_depth_view) = make_offscreen(device, config, DEPTH_TARGET);
        let ms = make_msaa(device, config, samples);

        let passthrough_pipeline = {
            let vs = glsl_module(device, FULLSCREEN_VERTEX, wgpu::naga::ShaderStage::Vertex);
            let fs = glsl_module(
                device,
                PASSTHROUGH_FRAGMENT,
                wgpu::naga::ShaderStage::Fragment,
            );
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("passthrough-layout"),
                bind_group_layouts: &[Some(&post_bgl)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("passthrough-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let post_programs: Vec<PostProgram> = Vec::new();

        let examine_pipeline = {
            let vs = glsl_module(device, FULLSCREEN_VERTEX, wgpu::naga::ShaderStage::Vertex);
            let fs = glsl_module(device, EXAMINE_FRAGMENT, wgpu::naga::ShaderStage::Fragment);
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("examine-layout"),
                bind_group_layouts: &[Some(&post_bgl)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("examine-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene-layout"),
            bind_group_layouts: &[Some(&camera_bgl)],
            immediate_size: 0,
        });
        let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 8 => Float32x4, 9 => Float32x2];
        let inst_attrs = wgpu::vertex_attr_array![
            2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Uint32
        ];
        let programs: Vec<ShaderProgram> = Vec::new();

        let shadow_pipeline = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shadow-shader"),
                source: wgpu::ShaderSource::Wgsl(SHADOW_WGSL.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow-layout"),
                bind_group_layouts: &[Some(&shadow_pass_bgl)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shadow-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<view::Vertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &mesh_attrs,
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Instance>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &inst_attrs,
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: SHADOW_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        let make_mesh = |verts: &[view::Vertex], indices: &[u32]| MeshBuf {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh-verts"),
                contents: bytemuck::cast_slice(verts),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: indices.len() as u32,
        };

        let meshes: Vec<MeshBuf> = registry
            .data()
            .iter()
            .map(|(verts, indices)| make_mesh(verts, indices))
            .collect();

        let vram_bytes: u64 = registry
            .data()
            .iter()
            .map(|(v, i)| (v.len() * std::mem::size_of::<view::Vertex>() + i.len() * 4) as u64)
            .sum::<u64>()
            + registry
                .images()
                .iter()
                .map(|t| t.rgba.len() as u64)
                .sum::<u64>();

        let depth_view = make_depth(device, config);

        let mask_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("outline-mask-shader"),
            source: wgpu::ShaderSource::Wgsl(MASK_WGSL.into()),
        });
        let outline_mask_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("outline-mask-pipeline"),
                layout: Some(&scene_layout),
                vertex: wgpu::VertexState {
                    module: &mask_shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<view::Vertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &mesh_attrs,
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Instance>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &inst_attrs,
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &mask_shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        let outline_mask_view = make_mask(device, config);
        let outline_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("outline-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let outline_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline-uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let outline_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("outline-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
        let outline_bind_group = make_outline_bind_group(
            device,
            &outline_bgl,
            &outline_uniform,
            &outline_mask_view,
            &outline_sampler,
        );
        let outline_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("outline-shader"),
            source: wgpu::ShaderSource::Wgsl(OUTLINE_WGSL.into()),
        });
        let outline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("outline-layout"),
            bind_group_layouts: &[Some(&outline_bgl)],
            immediate_size: 0,
        });
        let outline_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("outline-pipeline"),
            layout: Some(&outline_layout),
            vertex: wgpu::VertexState {
                module: &outline_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &outline_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let mask = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("transition-mask"),
            size: wgpu::Extent3d {
                width: mask_img.width,
                height: mask_img.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &mask,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &mask_img.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(mask_img.width * 4),
                rows_per_image: Some(mask_img.height),
            },
            wgpu::Extent3d {
                width: mask_img.width,
                height: mask_img.height,
                depth_or_array_layers: 1,
            },
        );
        let mask_view = mask.create_view(&wgpu::TextureViewDescriptor::default());
        let mask_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("transition-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let transition_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transition-uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let transition_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transition-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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

        let transition_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transition-bg"),
            layout: &transition_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: transition_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mask_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&mask_sampler),
                },
            ],
        });

        let transition_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transition-shader"),
            source: wgpu::ShaderSource::Wgsl(TRANSITION_WGSL.into()),
        });
        let transition_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transition-layout"),
            bind_group_layouts: &[Some(&transition_bgl)],
            immediate_size: 0,
        });
        let transition_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transition-pipeline"),
            layout: Some(&transition_layout),
            vertex: wgpu::VertexState {
                module: &transition_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &transition_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let mut gfx = Gfx {
            gpu,
            ui_pipeline,
            ui_bind_group,
            ui_uniform,
            ui_vertex_buf,
            ui_vertex_cap,
            ui_index_buf,
            ui_index_cap,
            ui_verts: Vec::new(),
            ui_indices: Vec::new(),
            ui_order: Vec::new(),
            sprite_pipeline,
            sprite_bind_groups,
            _sprite_textures: sprite_textures,
            atlas,
            _atlas_view: atlas_view,
            _sampler: sampler,
            programs,
            camera_bgl,
            material_bgl,
            default_material_bg,
            texture_bgl,
            mesh_textures,
            _model_textures: model_textures,
            camera_buffer,
            light_buffer,
            camera_bind_group,
            shadow_enabled,
            shadow_buffer,
            shadow_pipeline,
            shadow_pass_bg,
            shadow_depth_view,
            shadow_color_view,
            _shadow_color: shadow_color,
            _shadow_depth: shadow_depth,
            samples,
            ms_color: ms.color,
            ms_depth_target: ms.depth_target,
            ms_depth: ms.depth,
            ms_mask: ms.mask,
            _ms_textures: ms.textures,
            offscreen: [offscreen_a, offscreen_b],
            offscreen_views: [offscreen_a_view, offscreen_b_view],
            offscreen_depth,
            offscreen_depth_view,
            post_bgl,
            post_sampler,
            post_globals,
            post_programs,
            passthrough_pipeline,
            examine_pipeline,
            meshes,
            depth_view,
            outline_mask_view,
            outline_mask_pipeline,
            outline_pipeline,
            outline_uniform,
            outline_sampler,
            outline_bgl,
            outline_bind_group,
            transition_pipeline,
            transition_uniform,
            transition_bind_group,
            _mask: mask,
            _mask_view: mask_view,
            _mask_sampler: mask_sampler,
            last_stats: view::RenderStats::default(),
            vram_bytes,
        };
        gfx.ensure_programs(shaders);
        Ok(gfx)
    }

    fn build_program(
        &self,
        vertex_src: &str,
        fragment_src: &str,
        has_params: bool,
    ) -> ShaderProgram {
        let glsl = |src: &str, stage: wgpu::naga::ShaderStage| {
            let define = match stage {
                wgpu::naga::ShaderStage::Vertex => "VERTEX",
                _ => "FRAGMENT",
            };
            self.gpu
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("scene-shader"),
                    source: wgpu::ShaderSource::Glsl {
                        shader: src.into(),
                        stage,
                        defines: &[(define, "1")],
                    },
                })
        };
        let vs = glsl(vertex_src, wgpu::naga::ShaderStage::Vertex);
        let fs = glsl(fragment_src, wgpu::naga::ShaderStage::Fragment);

        let layouts: Vec<&wgpu::BindGroupLayout> =
            vec![&self.camera_bgl, &self.material_bgl, &self.texture_bgl];
        let layout_refs: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().map(|l| Some(*l)).collect();
        let layout = self
            .gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("scene-program-layout"),
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            });

        let mesh_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 8 => Float32x4, 9 => Float32x2];
        let inst_attrs = wgpu::vertex_attr_array![
            2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Uint32, 10 => Float32x2, 11 => Float32x4
        ];
        let make = |cull: Option<wgpu::Face>, depth_write: bool| {
            self.gpu
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("scene-pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &vs,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        buffers: &[
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<view::Vertex>() as u64,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &mesh_attrs,
                            },
                            wgpu::VertexBufferLayout {
                                array_stride: std::mem::size_of::<Instance>() as u64,
                                step_mode: wgpu::VertexStepMode::Instance,
                                attributes: &inst_attrs,
                            },
                        ],
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: cull,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: Some(depth_write),
                        depth_compare: Some(wgpu::CompareFunction::Less),
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: self.samples,
                        ..Default::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &fs,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: HDR_FORMAT,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                            Some(wgpu::ColorTargetState {
                                format: DEPTH_TARGET,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                        ],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        };

        ShaderProgram {
            pipelines: [
                make(Some(wgpu::Face::Back), true),
                make(Some(wgpu::Face::Front), true),
                make(None, true),
            ],
            pipelines_no_depth: [
                make(Some(wgpu::Face::Back), false),
                make(Some(wgpu::Face::Front), false),
                make(None, false),
            ],
            has_params,
        }
    }

    pub fn ensure_programs(&mut self, shaders: &ShaderRegistry) {
        while self.programs.len() < shaders.len() {
            let i = self.programs.len();
            let program = self.build_program(
                shaders.vertex_source(i),
                shaders.fragment_source(i),
                shaders.has_params(i),
            );
            self.programs.push(program);
        }
    }

    pub fn reload_program(&mut self, i: usize, shaders: &ShaderRegistry) {
        if i < self.programs.len() {
            self.programs[i] = self.build_program(
                shaders.vertex_source(i),
                shaders.fragment_source(i),
                shaders.has_params(i),
            );
        }
    }

    fn build_post_program(&self, fragment_src: &str, has_params: bool) -> PostProgram {
        let vs = glsl_module(
            &self.gpu.device,
            FULLSCREEN_VERTEX,
            wgpu::naga::ShaderStage::Vertex,
        );
        let fs = glsl_module(
            &self.gpu.device,
            fragment_src,
            wgpu::naga::ShaderStage::Fragment,
        );
        let mut layouts: Vec<&wgpu::BindGroupLayout> = vec![&self.post_bgl];
        if has_params {
            layouts.push(&self.material_bgl);
        }
        let layout_refs: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().map(|l| Some(*l)).collect();
        let layout = self
            .gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("post-layout"),
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            });
        let pipeline = self
            .gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("post-pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        PostProgram {
            pipeline,
            has_params,
        }
    }

    pub fn ensure_post_programs(&mut self, post: &PostRegistry) {
        while self.post_programs.len() < post.len() {
            let i = self.post_programs.len();
            let program = self.build_post_program(post.fragment_source(i), post.has_params(i));
            self.post_programs.push(program);
        }
    }

    pub fn reload_post_program(&mut self, i: usize, post: &PostRegistry) {
        if i < self.post_programs.len() {
            self.post_programs[i] =
                self.build_post_program(post.fragment_source(i), post.has_params(i));
        }
    }

    fn prepare_draws(&self, scene: &Scene) -> (Vec<wgpu::BindGroup>, Vec<SceneDraw>) {
        let nprog = self.programs.len();
        let mut param_sets: Vec<[f32; MATERIAL_FLOATS]> = Vec::new();
        let mut param_lookup: HashMap<[u32; MATERIAL_FLOATS], usize> = HashMap::new();
        let mut groups: HashMap<DrawKey, Vec<Instance>> = HashMap::new();
        for item in &scene.items {
            let mesh = item.mesh.index();
            if mesh >= self.meshes.len() {
                continue;
            }
            let prog = (item.shader.0 as usize).min(nprog.saturating_sub(1));
            let param_id = if self
                .programs
                .get(prog)
                .map(|p| p.has_params)
                .unwrap_or(false)
            {
                let key: [u32; MATERIAL_FLOATS] = std::array::from_fn(|i| item.params[i].to_bits());
                *param_lookup.entry(key).or_insert_with(|| {
                    param_sets.push(item.params);
                    param_sets.len()
                })
            } else {
                0
            };
            groups
                .entry((
                    prog,
                    mesh,
                    item.faces.index(),
                    param_id,
                    item.cast_shadows as usize,
                    item.outline_occlude as usize,
                ))
                .or_default()
                .push(Instance {
                    model: item.model.to_cols_array_2d(),
                    color: item.color.to_array(),
                    shader: item.shader.0,
                    mr: [item.metallic, item.roughness],
                    uvt: [
                        item.uv_scale[0],
                        item.uv_scale[1],
                        item.uv_offset[0],
                        item.uv_offset[1],
                    ],
                });
        }

        let param_bgs: Vec<wgpu::BindGroup> = param_sets
            .iter()
            .map(|set| {
                let buf = self
                    .gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("material-params"),
                        contents: bytemuck::cast_slice(set),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                self.gpu
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("material-bg"),
                        layout: &self.material_bgl,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buf.as_entire_binding(),
                        }],
                    })
            })
            .collect();

        let mut scene_draws: Vec<SceneDraw> = groups
            .iter()
            .map(|(&(prog, mesh, face, pid, cast, occlude), insts)| {
                let buf = self
                    .gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("instances"),
                        contents: bytemuck::cast_slice(insts),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                (
                    prog,
                    mesh,
                    face,
                    pid,
                    cast,
                    occlude,
                    buf,
                    insts.len() as u32,
                )
            })
            .collect();
        scene_draws.sort_by_key(|d| std::cmp::Reverse(d.5));
        (param_bgs, scene_draws)
    }

    fn write_camera_light(&self, scene: &Scene) {
        let aspect = self.gpu.config.width as f32 / self.gpu.config.height.max(1) as f32;
        let cam = &scene.camera;
        let proj = Mat4::perspective_rh(cam.fov, aspect, cam.near, cam.far);
        let view = cam.world.inverse();
        let view_proj = (proj * view).to_cols_array();
        self.gpu
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&view_proj));

        let lt = &scene.lighting;
        let cam_pos = cam.world.w_axis.truncate();
        let sun = lt.sun_color.to_array();
        let amb = lt.ambient.to_array();
        let sky = lt.sky.to_array();
        let light = LightUniform {
            sun_dir: [lt.sun_dir.x, lt.sun_dir.y, lt.sun_dir.z, lt.shininess],
            sun_color: [sun[0], sun[1], sun[2], lt.sun_intensity],
            ambient: [amb[0], amb[1], amb[2], lt.specular],
            sky: [sky[0], sky[1], sky[2], lt.fresnel],
            cam_pos: [cam_pos.x, cam_pos.y, cam_pos.z, scene.time],
        };
        self.gpu
            .queue
            .write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light));
    }

    fn render_examine(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        scene_draws: &[SceneDraw],
        param_bgs: &[wgpu::BindGroup],
    ) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("examine-shadow-clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.shadow_color_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let (color_view, color_resolve) = match &self.ms_color {
            Some(ms) => (ms, Some(&self.offscreen_views[0])),
            None => (&self.offscreen_views[0], None),
        };
        let (dtgt_view, dtgt_resolve) = match &self.ms_depth_target {
            Some(ms) => (ms, Some(&self.offscreen_depth_view)),
            None => (&self.offscreen_depth_view, None),
        };
        let scene_depth = self.ms_depth.as_ref().unwrap_or(&self.depth_view);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("examine-scene-pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: color_resolve,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: dtgt_view,
                    resolve_target: dtgt_resolve,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 10000.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: scene_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        for (prog, mesh_idx, face, pid, _cast, occlude, inst, count) in scene_draws {
            let program = &self.programs[*prog];
            let mesh = &self.meshes[*mesh_idx];
            if *occlude == 0 {
                pass.set_pipeline(&program.pipelines_no_depth[*face]);
            } else {
                pass.set_pipeline(&program.pipelines[*face]);
            }
            if program.has_params && *pid >= 1 {
                pass.set_bind_group(1, &param_bgs[*pid - 1], &[]);
            } else {
                pass.set_bind_group(1, &self.default_material_bg, &[]);
            }
            if let Some(tex_bg) = self.mesh_textures.get(*mesh_idx) {
                pass.set_bind_group(2, tex_bg, &[]);
            }
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(1, inst.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.count, 0, 0..*count);
        }
    }

    fn post_bind_group(&self, src: &wgpu::TextureView) -> wgpu::BindGroup {
        self.gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post-bg"),
                layout: &self.post_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.post_globals.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.offscreen_depth_view),
                    },
                ],
            })
    }

    fn param_bind_group(&self, params: &[f32; MATERIAL_FLOATS]) -> wgpu::BindGroup {
        let buf = self
            .gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("post-params"),
                contents: bytemuck::cast_slice(params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post-param-bg"),
                layout: &self.material_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            })
    }

    pub fn size(&self) -> (f32, f32) {
        self.gpu.size()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.gpu.resize(width, height) {
            self.depth_view = make_depth(&self.gpu.device, &self.gpu.config);
            self.outline_mask_view = make_mask(&self.gpu.device, &self.gpu.config);
            self.outline_bind_group = make_outline_bind_group(
                &self.gpu.device,
                &self.outline_bgl,
                &self.outline_uniform,
                &self.outline_mask_view,
                &self.outline_sampler,
            );
            let (a, av) = make_offscreen(&self.gpu.device, &self.gpu.config, HDR_FORMAT);
            let (b, bv) = make_offscreen(&self.gpu.device, &self.gpu.config, HDR_FORMAT);
            let (d, dv) = make_offscreen(&self.gpu.device, &self.gpu.config, DEPTH_TARGET);
            self.offscreen = [a, b];
            self.offscreen_views = [av, bv];
            self.offscreen_depth = d;
            self.offscreen_depth_view = dv;
            let ms = make_msaa(&self.gpu.device, &self.gpu.config, self.samples);
            self.ms_color = ms.color;
            self.ms_depth_target = ms.depth_target;
            self.ms_depth = ms.depth;
            self.ms_mask = ms.mask;
            self._ms_textures = ms.textures;
        }
    }

    pub fn stats(&self) -> view::RenderStats {
        self.last_stats
    }

    fn sync_atlas(&self) {
        self.gpu.sync_ui_atlas(&self.atlas);
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        examine: Option<&Scene>,
        examine_intro: f32,
        quads: &[DrawRect],
        transition: &TransitionView,
    ) {
        self.sync_atlas();
        // camera (from the MainCam entity)
        let aspect = self.gpu.config.width as f32 / self.gpu.config.height.max(1) as f32;
        let cam = &scene.camera;
        let proj = Mat4::perspective_rh(cam.fov, aspect, cam.near, cam.far);
        let view = cam.world.inverse();
        let view_proj = (proj * view).to_cols_array();
        self.gpu
            .queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&view_proj));

        let lt = &scene.lighting;
        let bg = lt.background.to_array();
        let cam_pos = cam.world.w_axis.truncate();
        let sun = lt.sun_color.to_array();
        let amb = lt.ambient.to_array();
        let sky = lt.sky.to_array();
        let light = LightUniform {
            sun_dir: [lt.sun_dir.x, lt.sun_dir.y, lt.sun_dir.z, lt.shininess],
            sun_color: [sun[0], sun[1], sun[2], lt.sun_intensity],
            ambient: [amb[0], amb[1], amb[2], lt.specular],
            sky: [sky[0], sky[1], sky[2], lt.fresnel],
            cam_pos: [cam_pos.x, cam_pos.y, cam_pos.z, scene.time],
        };
        self.gpu
            .queue
            .write_buffer(&self.light_buffer, 0, bytemuck::bytes_of(&light));

        // sun shadow: orthographic frustum looking down the sun direction at the scene center
        let light_dir = lt.sun_dir.normalize_or_zero();
        let up = if light_dir.y.abs() > 0.95 {
            glam::Vec3::Z
        } else {
            glam::Vec3::Y
        };
        let center = glam::Vec3::ZERO;
        let eye = center + light_dir * 40.0;
        let light_view = Mat4::look_at_rh(eye, center, up);
        let light_proj = Mat4::orthographic_rh(-22.0, 22.0, -22.0, 22.0, 1.0, 90.0);
        let light_vp = (light_proj * light_view).to_cols_array();
        self.gpu
            .queue
            .write_buffer(&self.shadow_buffer, 0, bytemuck::cast_slice(&light_vp));

        // group instances by (shader, mesh, faces, params): shader picks the program, faces the
        // cull pipeline, params dedupes per-material uniform blocks into bind groups.
        let nprog = self.programs.len();
        let mut param_sets: Vec<[f32; MATERIAL_FLOATS]> = Vec::new();
        let mut param_lookup: HashMap<[u32; MATERIAL_FLOATS], usize> = HashMap::new();
        let mut groups: HashMap<DrawKey, Vec<Instance>> = HashMap::new();
        for item in &scene.items {
            let mesh = item.mesh.index();
            if mesh >= self.meshes.len() {
                continue;
            }
            let prog = (item.shader.0 as usize).min(nprog.saturating_sub(1));
            let param_id = if self
                .programs
                .get(prog)
                .map(|p| p.has_params)
                .unwrap_or(false)
            {
                let key: [u32; MATERIAL_FLOATS] = std::array::from_fn(|i| item.params[i].to_bits());
                *param_lookup.entry(key).or_insert_with(|| {
                    param_sets.push(item.params);
                    param_sets.len()
                })
            } else {
                0
            };
            groups
                .entry((
                    prog,
                    mesh,
                    item.faces.index(),
                    param_id,
                    item.cast_shadows as usize,
                    item.outline_occlude as usize,
                ))
                .or_default()
                .push(Instance {
                    model: item.model.to_cols_array_2d(),
                    color: item.color.to_array(),
                    shader: item.shader.0,
                    mr: [item.metallic, item.roughness],
                    uvt: [
                        item.uv_scale[0],
                        item.uv_scale[1],
                        item.uv_offset[0],
                        item.uv_offset[1],
                    ],
                });
        }

        let param_bgs: Vec<wgpu::BindGroup> = param_sets
            .iter()
            .map(|set| {
                let buf = self
                    .gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("material-params"),
                        contents: bytemuck::cast_slice(set),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                self.gpu
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("material-bg"),
                        layout: &self.material_bgl,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buf.as_entire_binding(),
                        }],
                    })
            })
            .collect();

        let mut scene_draws: Vec<SceneDraw> = groups
            .iter()
            .map(|(&(prog, mesh, face, pid, cast, occlude), insts)| {
                let buf = self
                    .gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("instances"),
                        contents: bytemuck::cast_slice(insts),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
                (
                    prog,
                    mesh,
                    face,
                    pid,
                    cast,
                    occlude,
                    buf,
                    insts.len() as u32,
                )
            })
            .collect();
        scene_draws.sort_by_key(|d| std::cmp::Reverse(d.5));

        self.last_stats = view::RenderStats {
            draw_calls: scene_draws.len() as u32,
            instances: scene_draws.iter().map(|d| d.7).sum(),
            triangles: scene_draws
                .iter()
                .map(|d| self.meshes[d.1].count / 3 * d.7)
                .sum(),
            vram_bytes: self.vram_bytes,
        };

        // outline silhouette instances, grouped by mesh
        let mut outline_by_mesh: Vec<Vec<Instance>> =
            (0..self.meshes.len()).map(|_| Vec::new()).collect();
        for item in &scene.outlines {
            let idx = item.mesh.index();
            if idx < self.meshes.len() {
                outline_by_mesh[idx].push(Instance {
                    model: item.model.to_cols_array_2d(),
                    color: item.color.to_array(),
                    shader: item.shader.0,
                    mr: [item.metallic, item.roughness],
                    uvt: [
                        item.uv_scale[0],
                        item.uv_scale[1],
                        item.uv_offset[0],
                        item.uv_offset[1],
                    ],
                });
            }
        }
        let outline_bufs: Vec<Option<wgpu::Buffer>> = outline_by_mesh
            .iter()
            .map(|insts| {
                (!insts.is_empty()).then(|| {
                    self.gpu
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("outline-instances"),
                            contents: bytemuck::cast_slice(insts),
                            usage: wgpu::BufferUsages::VERTEX,
                        })
                })
            })
            .collect();
        let draw_outline = scene.outline_width > 0.0 && outline_bufs.iter().any(Option::is_some);
        if draw_outline {
            let u = [
                scene.outline_color[0],
                scene.outline_color[1],
                scene.outline_color[2],
                scene.outline_color[3],
                1.0 / self.gpu.config.width.max(1) as f32,
                1.0 / self.gpu.config.height.max(1) as f32,
                scene.outline_width,
                0.0,
            ];
            self.gpu
                .queue
                .write_buffer(&self.outline_uniform, 0, bytemuck::cast_slice(&u));
        }

        // UI geometry
        let dims = [
            self.gpu.config.width as f32,
            self.gpu.config.height as f32,
            0.0,
            0.0,
        ];

        self.gpu
            .queue
            .write_buffer(&self.ui_uniform, 0, bytemuck::cast_slice(&dims));

        let mut verts = std::mem::take(&mut self.ui_verts);
        let mut indices = std::mem::take(&mut self.ui_indices);
        let mut order = std::mem::take(&mut self.ui_order);
        verts.clear();
        indices.clear();
        order.clear();
        order.extend(0..quads.len());
        order.sort_by(|&a, &b| {
            quads[a].tex.cmp(&quads[b].tex).then_with(|| {
                quads[a]
                    .z
                    .partial_cmp(&quads[b].z)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        let sw = self.gpu.config.width as f32;
        let sh = self.gpu.config.height as f32;
        let scissor = |clip: [f32; 4]| -> [u32; 4] {
            let x0 = clip[0].clamp(0.0, sw);
            let y0 = clip[1].clamp(0.0, sh);
            let x1 = (clip[0] + clip[2]).clamp(0.0, sw);
            let y1 = (clip[1] + clip[3]).clamp(0.0, sh);
            [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]
        };

        let mut ui_ranges: Vec<(u32, u32, u32, [u32; 4])> = Vec::new();
        let mut cur: Option<(u32, [u32; 4])> = None;
        let mut range_start = 0u32;
        for &qi in &order {
            let q = &quads[qi];
            let sc = scissor(q.clip);
            if cur != Some((q.tex, sc)) {
                if let Some((t, s)) = cur {
                    ui_ranges.push((t, range_start, indices.len() as u32 - range_start, s));
                }
                cur = Some((q.tex, sc));
                range_start = indices.len() as u32;
            }
            let [x0, y0] = q.min;
            let [x1, y1] = q.max;
            let [p00, p10, p01, p11] = q.quad.unwrap_or([[x0, y0], [x1, y0], [x0, y1], [x1, y1]]);
            let [u0, v0] = q.uv_min;
            let [u1, v1] = q.uv_max;
            let c = q.color;
            let base = verts.len() as u32;
            verts.push(UiVertex {
                pos: p00,
                uv: [u0, v0],
                color: c,
            });
            verts.push(UiVertex {
                pos: p10,
                uv: [u1, v0],
                color: c,
            });
            verts.push(UiVertex {
                pos: p01,
                uv: [u0, v1],
                color: c,
            });
            verts.push(UiVertex {
                pos: p11,
                uv: [u1, v1],
                color: c,
            });
            indices.extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
        }
        if let Some((t, s)) = cur {
            ui_ranges.push((t, range_start, indices.len() as u32 - range_start, s));
        }

        let vbytes: &[u8] = bytemuck::cast_slice(&verts);
        if vbytes.len() as u64 > self.ui_vertex_cap {
            self.ui_vertex_cap = (vbytes.len() as u64).next_power_of_two();
            self.ui_vertex_buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui-verts"),
                size: self.ui_vertex_cap,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !vbytes.is_empty() {
            self.gpu.queue.write_buffer(&self.ui_vertex_buf, 0, vbytes);
        }
        let ibytes: &[u8] = bytemuck::cast_slice(&indices);
        if ibytes.len() as u64 > self.ui_index_cap {
            self.ui_index_cap = (ibytes.len() as u64).next_power_of_two();
            self.ui_index_buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui-indices"),
                size: self.ui_index_cap,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !ibytes.is_empty() {
            self.gpu.queue.write_buffer(&self.ui_index_buf, 0, ibytes);
        }

        self.ui_verts = verts;
        self.ui_indices = indices;
        self.ui_order = order;

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
        let view_tex = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // shadow pass: render scene depth from the sun's point of view
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.shadow_color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.shadow_enabled {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, &self.shadow_pass_bg, &[]);
                for (_prog, mesh_idx, _face, _pid, cast, _occlude, inst, count) in &scene_draws {
                    if *cast == 0 {
                        continue;
                    }
                    let mesh = &self.meshes[*mesh_idx];
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_vertex_buffer(1, inst.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.count, 0, 0..*count);
                }
            }
        }

        // 3D pass (clears color + depth)
        {
            let (color_view, color_resolve) = match &self.ms_color {
                Some(ms) => (ms, Some(&self.offscreen_views[0])),
                None => (&self.offscreen_views[0], None),
            };
            let (dtgt_view, dtgt_resolve) = match &self.ms_depth_target {
                Some(ms) => (ms, Some(&self.offscreen_depth_view)),
                None => (&self.offscreen_depth_view, None),
            };
            let scene_depth = self.ms_depth.as_ref().unwrap_or(&self.depth_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        resolve_target: color_resolve,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: bg[0] as f64,
                                g: bg[1] as f64,
                                b: bg[2] as f64,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: dtgt_view,
                        resolve_target: dtgt_resolve,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 10000.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: scene_depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for (prog, mesh_idx, face, pid, _cast, occlude, inst, count) in &scene_draws {
                let program = &self.programs[*prog];
                let mesh = &self.meshes[*mesh_idx];
                if *occlude == 0 {
                    pass.set_pipeline(&program.pipelines_no_depth[*face]);
                } else {
                    pass.set_pipeline(&program.pipelines[*face]);
                }
                if program.has_params && *pid >= 1 {
                    pass.set_bind_group(1, &param_bgs[*pid - 1], &[]);
                } else {
                    pass.set_bind_group(1, &self.default_material_bg, &[]);
                }
                if let Some(tex_bg) = self.mesh_textures.get(*mesh_idx) {
                    pass.set_bind_group(2, tex_bg, &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_vertex_buffer(1, inst.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.count, 0, 0..*count);
            }
        }

        // outline: render hovered silhouettes into a mask, then composite a rim over the scene
        if draw_outline {
            {
                let (mask_view, mask_resolve) = match &self.ms_mask {
                    Some(ms) => (ms, Some(&self.outline_mask_view)),
                    None => (&self.outline_mask_view, None),
                };
                let scene_depth = self.ms_depth.as_ref().unwrap_or(&self.depth_view);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("outline-mask-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: mask_view,
                        resolve_target: mask_resolve,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: scene_depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.outline_mask_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                for (idx, mesh) in self.meshes.iter().enumerate() {
                    if let Some(inst) = &outline_bufs[idx] {
                        pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                        pass.set_vertex_buffer(1, inst.slice(..));
                        pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..mesh.count, 0, 0..outline_by_mesh[idx].len() as u32);
                    }
                }
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("outline-composite-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.offscreen_views[0],
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
                pass.set_pipeline(&self.outline_pipeline);
                pass.set_bind_group(0, &self.outline_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }

        // post-process chain: scene sits in HDR offscreen[0]; passes ping-pong, then the present
        // pass resolves the final HDR offscreen to the swapchain.
        {
            let globals = [
                self.gpu.config.width as f32,
                self.gpu.config.height as f32,
                scene.time,
                0.0,
                cam.near,
                cam.far,
                0.0,
                0.0,
            ];
            self.gpu
                .queue
                .write_buffer(&self.post_globals, 0, bytemuck::cast_slice(&globals));

            let active: Vec<_> = scene
                .post
                .iter()
                .filter(|p| (p.shader as usize) < self.post_programs.len())
                .collect();

            let mut src = 0usize;
            for p in &active {
                let prog = &self.post_programs[p.shader as usize];
                let src_bg = self.post_bind_group(&self.offscreen_views[src]);
                let param_bg = prog.has_params.then(|| self.param_bind_group(&p.params));
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("post-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.offscreen_views[1 - src],
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
                pass.set_pipeline(&prog.pipeline);
                pass.set_bind_group(0, &src_bg, &[]);
                if let Some(pb) = &param_bg {
                    pass.set_bind_group(1, pb, &[]);
                }
                pass.draw(0..3, 0..1);
                drop(pass);
                src = 1 - src;
            }

            let present_bg = self.post_bind_group(&self.offscreen_views[src]);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_tex,
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
            pass.set_pipeline(&self.passthrough_pipeline);
            pass.set_bind_group(0, &present_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        self.gpu.queue.submit([encoder.finish()]);

        if let Some(ex) = examine {
            self.write_camera_light(ex);
            let (ex_param_bgs, ex_draws) = self.prepare_draws(ex);
            let mut enc = self
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            self.render_examine(&mut enc, &ex_draws, &ex_param_bgs);
            self.gpu.queue.submit([enc.finish()]);

            let g = [examine_intro, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
            self.gpu
                .queue
                .write_buffer(&self.post_globals, 0, bytemuck::cast_slice(&g));
            let comp_bg = self.post_bind_group(&self.offscreen_views[0]);
            let mut enc = self
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("examine-composite-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_tex,
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
                pass.set_pipeline(&self.examine_pipeline);
                pass.set_bind_group(0, &comp_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            self.gpu.queue.submit([enc.finish()]);
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // UI pass (loads, drawn on top, no depth)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_tex,
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
            if !ui_ranges.is_empty() {
                pass.set_vertex_buffer(0, self.ui_vertex_buf.slice(..));
                pass.set_index_buffer(self.ui_index_buf.slice(..), wgpu::IndexFormat::Uint32);
            }
            for (tex, start, count, sc) in &ui_ranges {
                if sc[2] == 0 || sc[3] == 0 {
                    continue;
                }
                if *tex == 0 {
                    pass.set_pipeline(&self.ui_pipeline);
                    pass.set_bind_group(0, &self.ui_bind_group, &[]);
                } else if let Some(bg) = self.sprite_bind_groups.get(*tex as usize - 1) {
                    pass.set_pipeline(&self.sprite_pipeline);
                    pass.set_bind_group(0, bg, &[]);
                } else {
                    continue;
                }
                pass.set_scissor_rect(sc[0], sc[1], sc[2], sc[3]);
                pass.draw_indexed(*start..*start + *count, 0, 0..1);
            }
        }

        // transition overlay (alpha mask over everything)
        if transition.cutoff > 0.0 {
            let aspect = self.gpu.config.width as f32 / self.gpu.config.height.max(1) as f32;
            let u = [
                transition.color[0],
                transition.color[1],
                transition.color[2],
                transition.color[3],
                transition.cutoff,
                transition.soft,
                aspect,
                0.0,
            ];
            self.gpu
                .queue
                .write_buffer(&self.transition_uniform, 0, bytemuck::cast_slice(&u));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("transition-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_tex,
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
            pass.set_pipeline(&self.transition_pipeline);
            pass.set_bind_group(0, &self.transition_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.gpu.queue.submit([encoder.finish()]);
        frame.present();
    }
}
