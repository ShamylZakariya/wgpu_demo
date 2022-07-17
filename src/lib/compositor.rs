use super::{app::AppState, camera::Camera, gpu_state, util::*};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CompositorUniformData {
    camera_z_near_far: [f32; 4],
}

type CompositorUniform = UniformWrapper<CompositorUniformData>;

pub struct Compositor {
    size: winit::dpi::PhysicalSize<u32>,
    time: instant::Duration,
    uniform: CompositorUniform,
    uniform_is_dirty: bool,
    textures_bind_group_layout: wgpu::BindGroupLayout,
    textures_bind_group: wgpu::BindGroup,
    depth_attachment_sampler: wgpu::Sampler,
    render_pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new(gpu_state: &mut gpu_state::GpuState) -> Self {
        let uniform = CompositorUniform::new(&gpu_state.device);

        let textures_bind_group_layout =
            gpu_state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compositor Bind Group Layout"),
                    entries: &[
                        // Color atachment
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
                        // Color Attachment Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // Depth atachment
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Depth Attachment Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let depth_attachment_sampler = gpu_state.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let textures_bind_group = Self::create_textures_bind_group(
            gpu_state,
            &textures_bind_group_layout,
            &depth_attachment_sampler,
        );

        let render_pipeline_layout =
            gpu_state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&textures_bind_group_layout, &uniform.bind_group_layout],
                    push_constant_ranges: &[],
                });

        let shader = gpu_state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    super::resources::load_string_sync("shaders/compositor.wgsl")
                        .unwrap()
                        .into(),
                ),
            });

        let render_pipeline =
            gpu_state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "compositor_vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "compositor_fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu_state.config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent::REPLACE,
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        Self {
            size: gpu_state.size(),
            time: instant::Duration::default(),
            uniform,
            uniform_is_dirty: true,
            textures_bind_group_layout,
            textures_bind_group,
            depth_attachment_sampler,
            render_pipeline,
        }
    }

    pub fn time(&self) -> instant::Duration {
        self.time
    }

    pub fn read_camera_properties(&mut self, camera: &Camera) {
        let (z_near, z_far) = camera.depth_range();
        self.uniform.data.camera_z_near_far = Vec4::new(z_near, z_far, 0.0, 0.0).into();
        self.uniform_is_dirty = true;
    }

    fn create_textures_bind_group(
        gpu_state: &gpu_state::GpuState,
        texture_layout: &wgpu::BindGroupLayout,
        depth_attachment_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        gpu_state
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &gpu_state.color_attachment.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            &gpu_state.color_attachment.sampler,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &gpu_state.depth_attachment.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(depth_attachment_sampler),
                    },
                ],
                label: Some("Compositor Bind Group"),
            })
    }
}

impl AppState for Compositor {
    fn resize(
        &mut self,
        gpu_state: &mut super::gpu_state::GpuState,
        new_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.size = new_size;
        self.textures_bind_group = Self::create_textures_bind_group(
            gpu_state,
            &self.textures_bind_group_layout,
            &self.depth_attachment_sampler,
        );
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    fn input(
        &mut self,
        _event: Option<&winit::event::WindowEvent>,
        _mouse_motion: Option<(f64, f64)>,
    ) -> bool {
        false
    }

    fn update(&mut self, gpu_state: &mut super::gpu_state::GpuState, dt: instant::Duration) {
        self.time += dt;
        if self.uniform_is_dirty {
            self.uniform.write(&mut gpu_state.queue);
            self.uniform_is_dirty = false;
        }
    }

    fn render(
        &mut self,
        _gpu_state: &mut gpu_state::GpuState,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::SurfaceTexture,
    ) {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Compositor FSQ Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // FSQ doens't need to clear
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.textures_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
