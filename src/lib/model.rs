use wgpu::{util::DeviceExt, vertex_attr_array};

use super::{camera, light, texture};

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

    fn to_data(&self) -> InstanceData {
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

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                // Diffuse
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                // Normal
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            label: Some(name),
        });
        Self {
            name: name.into(),
            diffuse_texture,
            normal_texture,
            bind_group,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device, name: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Diffuse
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
                // Normal
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some(name),
        })
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
        let instance_data = Self::instance_data(&instances);
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
            *data = instance.to_data();
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
        instances.iter().map(Instance::to_data).collect()
    }
}

///////////////////////////

pub fn draw_model<'a, 'b>(
    render_pass: &'b mut wgpu::RenderPass<'a>,
    model: &'a Model,
    camera: &'a camera::CameraController,
    light: &'a light::Light,
) where
    'a: 'b, // 'a lifetime at least as long as 'b
{
    render_pass.set_vertex_buffer(1, model.instance_buffer.slice(..));
    let instances = 0..model.instances.len() as u32;
    for mesh in &model.meshes {
        let material = &model.materials[mesh.material];

        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_bind_group(0, &material.bind_group, &[]);
        render_pass.set_bind_group(1, &camera.bind_group, &[]);
        render_pass.set_bind_group(2, &light.bind_group, &[]);
        render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
    }
}
