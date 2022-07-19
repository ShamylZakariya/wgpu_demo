pub struct GpuState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: tao::dpi::PhysicalSize<u32>,
    pub pipeline_vendor: super::render_pipeline::RenderPipelineVendor,
    pub depth_attachment: super::texture::Texture,
    pub color_attachment: super::texture::Texture,
}

impl GpuState {
    pub async fn new(window: &tao::window::Window) -> Self {
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
            format: *surface
                .get_supported_formats(&adapter)
                .first()
                .expect("Unable to find a surface compatible with the adapter"),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        // create depth texture
        let depth_attachment =
            super::texture::Texture::create_depth_texture(&device, &config, "Depth Attachment");

        let color_attachment =
            super::texture::Texture::create_color_texture(&device, &config, "Color Attachment");

        Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline_vendor: super::render_pipeline::RenderPipelineVendor::default(),
            depth_attachment,
            color_attachment,
        }
    }

    pub fn resize(&mut self, new_size: tao::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_attachment = super::texture::Texture::create_depth_texture(
                &self.device,
                &self.config,
                "Depth Attachment",
            );
            self.color_attachment = super::texture::Texture::create_color_texture(
                &self.device,
                &self.config,
                "Color Attachment",
            );
        }
    }

    pub fn size(&self) -> tao::dpi::PhysicalSize<u32> {
        self.size
    }
}
