use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::color::Color;
use crate::sprites::Sprites;
use crate::ui::DrawRect;

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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

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
        let mask_pipeline = pipeline(device, &layout, &shader, "fs_mask", config.format);
        let pixel_pipeline = pipeline(device, &layout, &shader, "fs_pixel", config.format);
        let rgba_pipeline = pipeline(device, &layout, &shader, "fs_rgba", config.format);

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

    pub fn render(&mut self, clear: Color, world: &[DrawRect], ui: &[DrawRect], pixel_world: bool) {
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
        let world_ranges = if pixel_world { ranges.len() } else { 0 };

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
            for (i, r) in ranges.iter().enumerate() {
                if r.scissor[2] == 0 || r.scissor[3] == 0 || r.count == 0 {
                    continue;
                }
                if r.tex == 0 {
                    pass.set_pipeline(&self.mask_pipeline);
                    pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                } else if let Some(bg) = self.sprite_bind_groups.get(r.tex as usize - 1) {
                    pass.set_pipeline(if i < world_ranges {
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
