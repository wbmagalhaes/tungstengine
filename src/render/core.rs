use std::sync::Arc;

use winit::window::Window;

use crate::color::Color;
use crate::ecs::Resource;
use crate::ui::{atlas_pixels, atlas_size, take_dirty_rows};

#[derive(Resource, Clone, Copy)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        ClearColor(Color::rgb(20, 18, 28))
    }
}

#[derive(Debug)]
pub struct GfxError(pub String);

impl std::fmt::Display for GfxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GfxError {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GpuLimits {
    WebGl2,
    Default,
}

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    pub async fn new(window: Arc<Window>, limits: GpuLimits) -> Result<GpuContext, GfxError> {
        let size = window.inner_size();
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|e| GfxError(format!("create_surface: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| GfxError(format!("request_adapter: {e}")))?;

        let required_limits = match limits {
            GpuLimits::WebGl2 => {
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
            }
            GpuLimits::Default => wgpu::Limits::default(),
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits,
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(|e| GfxError(format!("request_device: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let present_mode = [wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
            .into_iter()
            .find(|m| caps.present_modes.contains(m))
            .unwrap_or(wgpu::PresentMode::Fifo);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(GpuContext {
            surface,
            device,
            queue,
            config,
        })
    }

    pub fn size(&self) -> (f32, f32) {
        (self.config.width as f32, self.config.height as f32)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 {
            return false;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        true
    }

    pub fn create_ui_atlas(&self) -> wgpu::Texture {
        let (w, h) = atlas_size();
        let extent = wgpu::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        };
        let atlas = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui-atlas"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w as u32),
                rows_per_image: Some(h as u32),
            },
            extent,
        );
        atlas
    }

    pub fn sync_ui_atlas(&self, atlas: &wgpu::Texture) {
        let Some((y0, y1, rows)) = take_dirty_rows() else {
            return;
        };
        let (w, _) = atlas_size();
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: atlas,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: y0 as u32,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &rows,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w as u32),
                rows_per_image: Some((y1 - y0) as u32),
            },
            wgpu::Extent3d {
                width: w as u32,
                height: (y1 - y0) as u32,
                depth_or_array_layers: 1,
            },
        );
    }
}
