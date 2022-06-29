use wgpu::{util::DeviceExt, vertex_attr_array};

use super::{
    camera,
    gpu_state::GpuState,
    light,
    render_pipeline::{self, RenderPipelineVendor},
    resources, texture,
};

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static MODEL_VERTEX_ATTRIBS: [wgpu::VertexAttribute; 5] = vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3, 4 => Float32x3];
static MODEL_INSTANCE_ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x3, 10 => Float32x3, 11 => Float32x3, ];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl ModelVertex {
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MODEL_VERTEX_ATTRIBS,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn new<P, R>(position: P, rotation: R) -> Self
    where
        P: Into<cgmath::Vector3<f32>>,
        R: Into<cgmath::Quaternion<f32>>,
    {
        Self {
            position: position.into(),
            rotation: rotation.into(),
        }
    }

    fn as_data(&self) -> InstanceData {
        let model =
            cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);
        InstanceData {
            model: model.into(),
            normal_matrix: cgmath::Matrix3::from(self.rotation).into(),
        }
    }

    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &MODEL_INSTANCE_ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    model: [[f32; 4]; 4],
    normal_matrix: [[f32; 3]; 3],
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
    shininess: f32,
    _padding: [f32; 3],
}

pub struct MaterialProperties<'a> {
    pub name: &'a str,
    pub ambient: cgmath::Vector4<f32>,
    pub diffuse: cgmath::Vector4<f32>,
    pub specular: cgmath::Vector4<f32>,
    pub shininess: f32,
    pub diffuse_texture: Option<texture::Texture>,
    pub normal_texture: Option<texture::Texture>,
    pub shininess_texture: Option<texture::Texture>,
}

impl<'a> Default for MaterialProperties<'a> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            ambient: cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0),
            diffuse: cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0),
            specular: cgmath::Vector4::new(1.0, 1.0, 1.0, 1.0),
            shininess: 1.0,
            diffuse_texture: None,
            normal_texture: None,
            shininess_texture: None,
        }
    }
}

pub struct Material {
    pub name: String,
    pub ambient: cgmath::Vector4<f32>,
    pub diffuse: cgmath::Vector4<f32>,
    pub specular: cgmath::Vector4<f32>,
    pub shininess: f32,
    pub diffuse_texture: Option<texture::Texture>,
    pub normal_texture: Option<texture::Texture>,
    pub shininess_texture: Option<texture::Texture>,
    pub material_uniform: MaterialUniform, // represents non-texture uniforms
    pub material_uniform_buffer: wgpu::Buffer, // represents non-texture uniforms
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub id: String,
}

impl Material {
    pub fn new(device: &wgpu::Device, properties: MaterialProperties) -> Self {
        let mut bind_group_layout_entries = Vec::new();
        let mut bind_group_entries = Vec::new();
        let mut id = String::new();

        let material_uniform = MaterialUniform {
            ambient: properties.ambient.into(),
            diffuse: properties.diffuse.into(),
            specular: properties.specular.into(),
            shininess: properties.shininess,
            _padding: [0.0; 3],
        };

        let material_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Material::uniform_buffer"),
                contents: bytemuck::cast_slice(&[material_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: 0,
            resource: material_uniform_buffer.as_entire_binding(),
        });

        let mut offset = 1u32;
        if let Some(texture) = &properties.diffuse_texture {
            id = format!("(diffuse-{})", offset);
            offset += Self::create_bind_groups_for(
                texture,
                offset,
                &mut bind_group_layout_entries,
                &mut bind_group_entries,
            );
        }

        if let Some(texture) = &properties.normal_texture {
            id = format!("{}(normal-{})", id, offset);
            offset += Self::create_bind_groups_for(
                texture,
                offset,
                &mut bind_group_layout_entries,
                &mut bind_group_entries,
            );
        }

        if let Some(texture) = &properties.shininess_texture {
            id = format!("{}(shininess-{})", id, offset);
            Self::create_bind_groups_for(
                texture,
                offset,
                &mut bind_group_layout_entries,
                &mut bind_group_entries,
            );
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &bind_group_layout_entries,
            label: Some(properties.name),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some(properties.name),
        });

        println!("Material id: {id}");

        Self {
            name: properties.name.to_owned(),
            ambient: properties.ambient,
            diffuse: properties.diffuse,
            specular: properties.specular,
            shininess: properties.shininess,
            diffuse_texture: properties.diffuse_texture,
            normal_texture: properties.normal_texture,
            shininess_texture: properties.shininess_texture,
            material_uniform,
            material_uniform_buffer,
            bind_group,
            bind_group_layout,
            id,
        }
    }

    pub fn prepare_pipeline(&self, gpu_state: &mut GpuState) {
        if !gpu_state.pipeline_vendor.has_pipeline(&self.id) {
            let layout = gpu_state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&self.id),
                    bind_group_layouts: &[
                        &self.bind_group_layout,
                        &camera::CameraController::bind_group_layout(&gpu_state.device),
                        &light::Light::bind_group_layout(&gpu_state.device),
                    ],
                    push_constant_ranges: &[],
                });

            let shader_source = resources::load_string_sync(self.shader()).unwrap();

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("ModelPipeline Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            };

            gpu_state.pipeline_vendor.create_render_pipeline(
                &self.id,
                &gpu_state.device,
                render_pipeline::Properties {
                    layout: &layout,
                    color_format: gpu_state.config.format,
                    depth_format: Some(texture::Texture::DEPTH_FORMAT),
                    vertex_layouts: &Model::vertex_layout(),
                    shader,
                },
            );
        }
    }

    pub fn shader(&self) -> &'static str {
        match (
            &self.diffuse_texture,
            &self.normal_texture,
            &self.shininess_texture,
        ) {
            (None, None, None) => "shaders/model/notex.wgsl",
            (Some(_), None, None) => "shaders/model/diffuse.wgsl",
            (Some(_), Some(_), None) => "shaders/model/diffuse_normal.wgsl",
            (Some(_), Some(_), Some(_)) => "shaders/model/diffuse_normal_shininess.wgsl",
            _ => unimplemented!("Material::shader doesn't support texture conbination specified"),
        }
    }

    fn create_bind_groups_for<'a: 'b, 'b>(
        texture: &'a texture::Texture,
        offset: u32,
        bind_group_layout_entries: &'b mut Vec<wgpu::BindGroupLayoutEntry>,
        bind_group_entries: &'b mut Vec<wgpu::BindGroupEntry<'a>>,
    ) -> u32 {
        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: offset,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        });

        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: offset,
            resource: wgpu::BindingResource::TextureView(&texture.view),
        });

        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: offset + 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });

        bind_group_entries.push(wgpu::BindGroupEntry {
            binding: offset + 1,
            resource: wgpu::BindingResource::Sampler(&texture.sampler),
        });

        2
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub instances: Vec<Instance>,
    instance_data: Vec<InstanceData>,
    pub instance_buffer: wgpu::Buffer,
}

impl Model {
    pub fn new(
        device: &wgpu::Device,
        meshes: Vec<Mesh>,
        materials: Vec<Material>,
        instances: &[Instance],
    ) -> Self {
        let instance_data = Self::instance_data(instances);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model::instance_buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Model {
            meshes,
            materials,
            instances: instances.to_vec(),
            instance_data,
            instance_buffer,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        for (instance, data) in self.instances.iter().zip(self.instance_data.iter_mut()) {
            *data = instance.as_data();
        }

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instance_data),
        );
    }

    pub fn vertex_layout<'a>() -> Vec<wgpu::VertexBufferLayout<'a>> {
        vec![
            ModelVertex::vertex_buffer_layout(),
            Instance::vertex_buffer_layout(),
        ]
    }

    fn instance_data(instances: &[Instance]) -> Vec<InstanceData> {
        instances.iter().map(Instance::as_data).collect()
    }
}

///////////////////////////

pub fn draw_model<'a, 'b>(
    render_pass: &'b mut wgpu::RenderPass<'a>,
    pipeline_vendor: &'a RenderPipelineVendor,
    model: &'a Model,
    camera: &'a camera::CameraController,
    light: &'a light::Light,
) where
    'a: 'b, // 'a lifetime at least as long as 'b
{
    let instances = 0..model.instances.len() as u32;
    for mesh in &model.meshes {
        let material = &model.materials[mesh.material];

        if let Some(pipeline) = pipeline_vendor.get_pipeline(&material.id) {
            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, model.instance_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            render_pass.set_bind_group(1, &camera.bind_group, &[]);
            render_pass.set_bind_group(2, &light.bind_group, &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
        } else {
            eprintln!(
                "No pipeline available to render material id: {}",
                material.id
            );
        }
    }
}
