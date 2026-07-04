//! WGPU sprite renderer: one instanced draw call per contiguous run of
//! sprites sharing a texture, sorted by layer (painter's algorithm).

use std::sync::Arc;

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{Color, Viewport};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible GPU adapter found")]
    NoAdapter,
    #[error("failed to request device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("surface is not supported by the adapter")]
    UnsupportedSurface,
}

/// Handle to a texture uploaded to the GPU. `Default` yields the first
/// texture ever created (useful as a placeholder in tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureId(u32);

/// One sprite to draw this frame, in world coordinates.
#[derive(Debug, Clone, Copy)]
pub struct SpriteInstance {
    pub x: f32,
    pub y: f32,
    /// Rotation in radians, counter-clockwise.
    pub rotation: f32,
    /// Final half extents in world units (texture size x scale / 2).
    pub half_width: f32,
    pub half_height: f32,
    pub opacity: f32,
    /// Higher layers draw on top.
    pub layer: i32,
    pub texture: TextureId,
    /// Texture sub-rectangle `(u0, v0, u1, v1)`; `FULL_TEXTURE` for all of it.
    pub uv: [f32; 4],
}

/// UV rect covering the whole texture.
pub const FULL_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// Camera state used to build the view-projection matrix.
#[derive(Debug, Clone, Copy)]
pub struct CameraView {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Default for CameraView {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RawInstance {
    center: [f32; 2],
    half_size: [f32; 2],
    rotation: f32,
    opacity: f32,
    uv_rect: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

struct GpuTexture {
    bind_group: wgpu::BindGroup,
}

/// The 2D renderer. Owns the WGPU device and the window surface.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: Vec<GpuTexture>,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
}

impl Renderer {
    /// Creates a renderer drawing to `window`. Blocks on GPU adapter setup.
    pub fn new(window: Arc<Window>) -> Result<Self, RenderError> {
        pollster::block_on(Self::new_async(window))
    }

    async fn new_async(window: Arc<Window>) -> Result<Self, RenderError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("aigs-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await?;

        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(RenderError::UnsupportedSurface)?;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aigs-sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aigs-camera-layout"),
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
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("aigs-camera-buffer"),
            contents: bytemuck::bytes_of(&CameraUniform {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aigs-camera-bind-group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aigs-texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aigs-sprite-pipeline-layout"),
            bind_group_layouts: &[&camera_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let instance_attributes = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32,
            3 => Float32,
            4 => Float32x4,
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aigs-sprite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RawInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance_attributes,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("aigs-sprite-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let instance_capacity = 1024;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aigs-instance-buffer"),
            size: (instance_capacity * std::mem::size_of::<RawInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_layout,
            sampler,
            textures: Vec::new(),
            instance_buffer,
            instance_capacity,
        })
    }

    pub fn viewport(&self) -> Viewport {
        Viewport {
            width: self.config.width,
            height: self.config.height,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Uploads an RGBA8 texture (`pixels` is `width * height * 4` bytes).
    pub fn create_texture_rgba(&mut self, width: u32, height: u32, pixels: &[u8]) -> TextureId {
        assert_eq!(
            pixels.len(),
            (width * height * 4) as usize,
            "pixel buffer size mismatch"
        );
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("aigs-sprite-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aigs-sprite-texture-bind-group"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.push(GpuTexture { bind_group });
        TextureId(self.textures.len() as u32 - 1)
    }

    /// Draws one frame. Sorts `sprites` in place by layer then texture and
    /// issues one instanced draw call per texture run.
    pub fn render(
        &mut self,
        clear: Color,
        camera: CameraView,
        sprites: &mut [SpriteInstance],
    ) -> Result<(), wgpu::SurfaceError> {
        sprites.sort_unstable_by_key(|sprite| (sprite.layer, sprite.texture.0));

        let uniform = CameraUniform {
            view_proj: self.view_projection(camera).to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        if sprites.len() > self.instance_capacity {
            self.instance_capacity = sprites.len().next_power_of_two();
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("aigs-instance-buffer"),
                size: (self.instance_capacity * std::mem::size_of::<RawInstance>())
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let raw: Vec<RawInstance> = sprites
            .iter()
            .map(|sprite| RawInstance {
                center: [sprite.x, sprite.y],
                half_size: [sprite.half_width, sprite.half_height],
                rotation: sprite.rotation,
                opacity: sprite.opacity,
                uv_rect: sprite.uv,
            })
            .collect();
        if !raw.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&raw));
        }

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aigs-frame-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aigs-sprite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear.r),
                            g: f64::from(clear.g),
                            b: f64::from(clear.b),
                            a: f64::from(clear.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

            let mut start = 0;
            while start < sprites.len() {
                let texture = sprites[start].texture;
                let mut end = start + 1;
                while end < sprites.len() && sprites[end].texture == texture {
                    end += 1;
                }
                if let Some(gpu_texture) = self.textures.get(texture.0 as usize) {
                    pass.set_bind_group(1, &gpu_texture.bind_group, &[]);
                    pass.draw(0..6, start as u32..end as u32);
                }
                start = end;
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn view_projection(&self, camera: CameraView) -> Mat4 {
        let zoom = camera.zoom.max(0.0001);
        let half_width = self.config.width as f32 / (2.0 * zoom);
        let half_height = self.config.height as f32 / (2.0 * zoom);
        Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -1.0,
            1.0,
        ) * Mat4::from_translation(glam::Vec3::new(-camera.x, -camera.y, 0.0))
    }
}
