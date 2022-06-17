use super::{app::AppState, texture};

struct GpuState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    depth_texture: texture::Texture,
}

impl GpuState {
    async fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        // create depth texture
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "Depth Texture");

        Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "Depth Texture");
        }
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }
}

pub struct Scene {
    gpu_state: Option<GpuState>,
}

impl Scene {
    pub fn new() -> Self {
        Self { gpu_state: None }
    }
}

impl AppState for Scene {
    fn build(&mut self, window: &winit::window::Window) {
        let gpu_state = pollster::block_on(GpuState::new(window));
        self.gpu_state = Some(gpu_state);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(gpu_state) = self.gpu_state.as_mut() {
            gpu_state.resize(new_size);
        }
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.gpu_state
            .as_ref()
            .map_or(winit::dpi::PhysicalSize::new(0u32, 0u32), |g| g.size())
    }

    fn input(
        &mut self,
        event: Option<&winit::event::WindowEvent>,
        mouse_motion: Option<(f64, f64)>,
    ) -> bool {
        false
    }

    fn update(&mut self, dt: instant::Duration) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if let Some(gpu_state) = self.gpu_state.as_mut() {
            let output = gpu_state.surface.get_current_texture()?;
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                gpu_state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[
                        // this is output [[location(0)]]
                        wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    g: 0.2,
                                    r: 0.1,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            },
                        },
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &gpu_state.depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                });
            }

            gpu_state.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }

        Ok(())
    }
}
