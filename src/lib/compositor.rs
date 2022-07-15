use super::{app::AppState, gpu_state, resources};

pub struct Compositor {
    size: winit::dpi::PhysicalSize<u32>,
    time: instant::Duration,
    render_pipeline: wgpu::RenderPipeline,
}

impl Compositor {
    pub fn new(gpu_state: &mut gpu_state::GpuState) -> Self {
        let shader = gpu_state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    resources::load_string_sync("shaders/compositor.wgsl")
                        .unwrap()
                        .into(),
                ),
            });

        let render_pipeline_layout =
            gpu_state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
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
            render_pipeline,
        }
    }

    pub fn time(&self) -> instant::Duration {
        self.time
    }
}

impl AppState for Compositor {
    fn resize(
        &mut self,
        _gpu_state: &mut super::gpu_state::GpuState,
        new_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.size = new_size;
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

    fn update(&mut self, _gpu_state: &mut super::gpu_state::GpuState, dt: instant::Duration) {
        self.time += dt;
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

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Compositor FSQ Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }
    }
}
