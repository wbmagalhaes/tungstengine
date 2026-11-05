use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::color::Color;
use crate::sprites::Sprites;
use crate::ui::DrawRect;

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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    silhouette: f32,
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

const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

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

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    fragment: &str,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let attrs =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<QuadVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attrs,
            }],
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
        );
        let pixel_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_pixel",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let rgba_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_rgba",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let smooth_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_smooth",
            config.format,
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let add_mask_pipeline =
            pipeline(device, &layout, &shader, "fs_mask", LAYER_FORMAT, ADDITIVE);
        let add_smooth_pipeline = pipeline(
            device,
            &layout,
            &shader,
            "fs_smooth",
            LAYER_FORMAT,
            ADDITIVE,
        );
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

        let vertex_cap = 4096 * std::mem::size_of::<QuadVertex>() as u64;
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad-verts"),
            size: vertex_cap,
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
        let mut cur: Option<(u32, [u32; 4])> = None;
        let mut start = self.indices.len() as u32;
        for qi in order {
            let q = &quads[qi];
            let sc = self.scissor(q.clip);
            if cur != Some((q.tex, sc)) {
                if let Some((tex, scissor)) = cur {
                    ranges.push(Range {
                        tex,
                        start,
                        count: self.indices.len() as u32 - start,
                        scissor,
                    });
                }
                cur = Some((q.tex, sc));
                start = self.indices.len() as u32;
            }
            let [x0, y0] = q.min;
            let [x1, y1] = q.max;
            let pts = q.quad.unwrap_or([[x0, y0], [x1, y0], [x0, y1], [x1, y1]]);
            let [u0, v0] = q.uv_min;
            let [u1, v1] = q.uv_max;
            let uvs = q.uvs.unwrap_or([[u0, v0], [u1, v0], [u0, v1], [u1, v1]]);
            let base = self.verts.len() as u32;
            for (pos, uv) in pts.into_iter().zip(uvs) {
                self.verts.push(QuadVertex {
                    pos,
                    uv,
                    color: q.color,
                    silhouette: f32::from(u8::from(q.silhouette)),
                });
            }
            self.indices
                .extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
        }
        if let Some((tex, scissor)) = cur {
            ranges.push(Range {
                tex,
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
        {
            let c = clear.to_array();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            if !ranges.is_empty() {
                pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            }
            let (width, height) = (self.gpu.config.width, self.gpu.config.height);
            for i in 0..=ranges.len() {
                if i == drawn_world {
                    for program in &self.layer_programs {
                        pass.set_scissor_rect(0, 0, width, height);
                        pass.set_pipeline(&program.pipeline);
                        pass.set_bind_group(0, &program.group, &[]);
                        pass.draw(0..3, 0..1);
                    }
                }
                let Some(r) = ranges.get(i) else {
                    break;
                };
                if r.scissor[2] == 0 || r.scissor[3] == 0 || r.count == 0 {
                    continue;
                }
                if r.tex == 0 {
                    pass.set_pipeline(&self.mask_pipeline);
                    pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                } else if let Some(bg) = self.sprite_bind_groups.get(r.tex as usize - 1) {
                    pass.set_pipeline(if !pixel_world {
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
        self.gpu.queue.submit([encoder.finish()]);
        frame.present();
    }
}
