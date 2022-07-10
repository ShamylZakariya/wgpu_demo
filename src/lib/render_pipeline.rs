use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub enum Pass {
    Ambient,
    Lit,
}

pub struct Properties<'a> {
    pub vs_main: &'a str,
    pub fs_main: &'a str,
    pub layout: &'a wgpu::PipelineLayout,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub vertex_layouts: &'a [wgpu::VertexBufferLayout<'a>],
    pub shader: wgpu::ShaderModuleDescriptor<'a>,
    pub pass: Pass,
}

#[derive(Default)]
pub struct RenderPipelineVendor {
    pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl RenderPipelineVendor {
    pub fn has_pipeline(&self, named: &str) -> bool {
        self.pipelines.contains_key(named)
    }

    pub fn get_pipeline(&self, named: &str) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(named)
    }

    pub fn create_render_pipeline(
        &mut self,
        named: &str,
        device: &wgpu::Device,
        properties: Properties,
    ) -> &wgpu::RenderPipeline {
        let shader = device.create_shader_module(properties.shader);
        let depth_write_enabled = match properties.pass {
            Pass::Ambient => true,
            Pass::Lit => false,
        };

        let blend_state = match properties.pass {
            Pass::Ambient => wgpu::BlendState::REPLACE,
            Pass::Lit => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("RenderPipeline: {}", named)),
            layout: Some(properties.layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: properties.vs_main,
                buffers: properties.vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: properties.fs_main,
                targets: &[Some(wgpu::ColorTargetState {
                    format: properties.color_format,
                    blend: Some(blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: properties
                .depth_format
                .map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        self.pipelines.insert(named.to_owned(), pipeline);
        self.pipelines.get(named).unwrap()
    }
}
