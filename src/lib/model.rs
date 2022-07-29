use std::{collections::HashMap, rc::Rc};

use cgmath::prelude::*;
use wgpu::{util::DeviceExt, vertex_attr_array};

use super::{
    camera,
    gpu_state::GpuState,
    light,
    render_pipeline::{self, RenderPipelineVendor},
    resources, texture,
    util::*,
};

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

static MODEL_VERTEX_ATTRIBS: [wgpu::VertexAttribute; 5] = vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 => Float32x3, 4 => Float32x3];
static MODEL_INSTANCE_ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x3, 10 => Float32x3, 11 => Float32x3, ];

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ModelVertex {
    pub position: Point3,
    pub tex_coords: Vec2,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
}

unsafe impl bytemuck::Pod for ModelVertex {}
unsafe impl bytemuck::Zeroable for ModelVertex {}

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
    position: Point3,
    rotation: Quat,
}

impl Instance {
    pub fn new<P, R>(position: P, rotation: R) -> Self
    where
        P: Into<Point3>,
        R: Into<Quat>,
    {
        Self {
            position: position.into(),
            rotation: rotation.into(),
        }
    }

    fn as_data(&self) -> InstanceData {
        InstanceData {
            model: Mat4::from_translation(self.position.to_vec()) * Mat4::from(self.rotation),
            normal_matrix: Mat3::from(self.rotation),
        }
    }

    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &MODEL_INSTANCE_ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct InstanceData {
    model: Mat4,
    normal_matrix: Mat3,
}

unsafe impl bytemuck::Pod for InstanceData {}
unsafe impl bytemuck::Zeroable for InstanceData {}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            model: Mat4::identity(),
            normal_matrix: Mat3::identity(),
        }
    }
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
#[derive(Copy, Clone)]
pub struct MaterialUniform {
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    glossiness: f32,
    _padding: [f32; 3],
    has_diffuse_normal_glossiness_textures: Vec4,
}

unsafe impl bytemuck::Pod for MaterialUniform {}
unsafe impl bytemuck::Zeroable for MaterialUniform {}

impl Default for MaterialUniform {
    fn default() -> Self {
        let one = Vec4::new(1.0, 1.0, 1.0, 1.0);
        Self {
            ambient: one,
            diffuse: one,
            specular: one,
            glossiness: 0.0,
            has_diffuse_normal_glossiness_textures: Vec4::new(0.0, 0.0, 0.0, 0.0),
            _padding: Default::default(),
        }
    }
}

pub struct MaterialProperties<'a> {
    pub name: &'a str,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
    pub glossiness: f32,
    pub environment_map: Option<Rc<texture::Texture>>,
    pub diffuse_texture: Option<texture::Texture>,
    pub normal_texture: Option<texture::Texture>,
    pub glossiness_texture: Option<texture::Texture>,
}

impl<'a> Default for MaterialProperties<'a> {
    fn default() -> Self {
        Self {
            name: Default::default(),
            ambient: Vec4::new(1.0, 1.0, 1.0, 1.0),
            diffuse: Vec4::new(1.0, 1.0, 1.0, 1.0),
            specular: Vec4::new(1.0, 1.0, 1.0, 1.0),
            glossiness: 1.0,
            environment_map: None,
            diffuse_texture: None,
            normal_texture: None,
            glossiness_texture: None,
        }
    }
}

pub struct Material {
    pub name: String,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub specular: Vec4,
    pub glossiness: f32,
    pub environment_map: Option<Rc<texture::Texture>>,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub glossiness_texture: texture::Texture,
    pub material_uniform: MaterialUniform, // represents non-texture uniforms
    pub material_uniform_buffer: wgpu::Buffer, // represents non-texture uniforms
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub ambient_pipeline_id: String,
    pub lit_pipeline_id: String,
}

impl Material {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, properties: MaterialProperties) -> Self {
        let mut bind_group_layout_entries = Vec::new();
        let mut bind_group_entries = Vec::new();
        let mut base_id = String::new();

        let material_uniform = MaterialUniform {
            ambient: color4(properties.ambient),
            diffuse: color4(properties.diffuse),
            specular: color4(properties.specular),
            glossiness: properties.glossiness,
            has_diffuse_normal_glossiness_textures: Vec4::new(
                if properties.diffuse_texture.is_some() {
                    1_f32
                } else {
                    0_f32
                },
                if properties.normal_texture.is_some() {
                    1_f32
                } else {
                    0_f32
                },
                if properties.glossiness_texture.is_some() {
                    1_f32
                } else {
                    0_f32
                },
                0.0,
            ),
            ..Default::default()
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

        if let Some(texture) = &properties.environment_map {
            base_id = format!("(environment-map-{})", offset);
            offset += Self::create_bind_groups_for(
                texture,
                offset,
                &mut bind_group_layout_entries,
                &mut bind_group_entries,
            );
        }

        let diffuse_texture =
            properties
                .diffuse_texture
                .unwrap_or(texture::Texture::new_placeholder(
                    device,
                    queue,
                    Some("Diffuse Placeholder"),
                ));
        base_id = format!("{}(diffuse-{})", base_id, offset);
        offset += Self::create_bind_groups_for(
            &diffuse_texture,
            offset,
            &mut bind_group_layout_entries,
            &mut bind_group_entries,
        );

        let normal_texture =
            properties
                .normal_texture
                .unwrap_or(texture::Texture::new_placeholder(
                    device,
                    queue,
                    Some("Normal Placeholder"),
                ));

        base_id = format!("{}(normal-{})", base_id, offset);
        offset += Self::create_bind_groups_for(
            &normal_texture,
            offset,
            &mut bind_group_layout_entries,
            &mut bind_group_entries,
        );

        let glossiness_texture =
            properties
                .glossiness_texture
                .unwrap_or(texture::Texture::new_placeholder(
                    device,
                    queue,
                    Some("Glossiness Placeholder"),
                ));

        base_id = format!("{}(glossiness-{})", base_id, offset);
        Self::create_bind_groups_for(
            &glossiness_texture,
            offset,
            &mut bind_group_layout_entries,
            &mut bind_group_entries,
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &bind_group_layout_entries,
            label: Some(properties.name),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some(properties.name),
        });

        Self {
            name: properties.name.to_owned(),
            ambient: properties.ambient,
            diffuse: properties.diffuse,
            specular: properties.specular,
            glossiness: properties.glossiness,
            environment_map: properties.environment_map,
            diffuse_texture: diffuse_texture,
            normal_texture: normal_texture,
            glossiness_texture: glossiness_texture,
            material_uniform,
            material_uniform_buffer,
            bind_group,
            bind_group_layout,
            ambient_pipeline_id: format!("model_ambient_[{base_id}]"),
            lit_pipeline_id: format!("model_lit_[{base_id}]"),
        }
    }

    pub fn prepare_pipelines(&self, gpu_state: &mut GpuState) {
        for pass in vec![render_pipeline::Pass::Ambient, render_pipeline::Pass::Lit].iter() {
            if !gpu_state
                .pipeline_vendor
                .has_pipeline(self.pipeline_id(pass))
            {
                let layout =
                    gpu_state
                        .device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some(self.pipeline_id(pass)),
                            bind_group_layouts: &[
                                &self.bind_group_layout,
                                &camera::Camera::bind_group_layout(&gpu_state.device),
                                &light::Light::bind_group_layout(&gpu_state.device),
                            ],
                            push_constant_ranges: &[],
                        });

                let shader = wgpu::ShaderModuleDescriptor {
                    label: Some(self.shader(pass)),
                    source: wgpu::ShaderSource::Wgsl(
                        resources::load_string_sync(self.shader(pass))
                            .unwrap()
                            .into(),
                    ),
                };

                gpu_state.pipeline_vendor.create_render_pipeline(
                    self.pipeline_id(pass),
                    &gpu_state.device,
                    render_pipeline::Properties {
                        vs_main: self.vertex_main(pass),
                        fs_main: self.fragment_main(pass),
                        layout: &layout,
                        color_format: texture::Texture::COLOR_FORMAT,
                        depth_format: Some(texture::Texture::DEPTH_FORMAT),
                        vertex_layouts: &Model::vertex_layout(),
                        shader,
                        pass: *pass,
                    },
                );
            }
        }
    }

    pub fn pipeline_id(&self, pass: &render_pipeline::Pass) -> &str {
        match pass {
            render_pipeline::Pass::Ambient => &self.ambient_pipeline_id,
            render_pipeline::Pass::Lit => &self.lit_pipeline_id,
        }
    }

    fn vertex_main(&self, _pass: &render_pipeline::Pass) -> &'static str {
        "vs_main"
    }

    fn fragment_main(&self, pass: &render_pipeline::Pass) -> &'static str {
        match pass {
            render_pipeline::Pass::Ambient => "fs_main_ambient",
            render_pipeline::Pass::Lit => "fs_main_lit",
        }
    }

    fn shader(&self, _pass: &render_pipeline::Pass) -> &'static str {
        "shaders/model.wgsl"
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
                view_dimension: texture.view_dimension,
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
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    instances: Vec<Instance>,
    instance_data: Vec<InstanceData>,
    is_dirty: bool,
    instance_buffer: wgpu::Buffer,
}

impl Model {
    pub fn new(
        device: &wgpu::Device,
        meshes: Vec<Mesh>,
        materials: Vec<Material>,
        instances: &[Instance],
    ) -> Self {
        let instance_data: Vec<InstanceData> = instances.iter().map(Instance::as_data).collect();
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
            is_dirty: true,
            instance_buffer,
        }
    }

    pub fn prepare_pipelines(&self, gpu_state: &mut GpuState) {
        for material in self.materials.iter() {
            material.prepare_pipelines(gpu_state);
        }
    }

    pub fn update_instance(&mut self, at: usize, to: Instance) {
        if at < self.instances.len() {
            self.instances[at] = to;
            self.is_dirty = true;
        }
    }

    pub fn update_instances(&mut self, updated_instances: &HashMap<usize, Instance>) {
        let mut did_mutate = false;
        for (idx, value) in updated_instances.iter() {
            if *idx < self.instances.len() {
                self.instances[*idx] = *value;
                did_mutate = true;
            }
        }
        if did_mutate {
            self.is_dirty = true;
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if !self.is_dirty {
            return;
        }

        // update the instance buffer in place
        for (instance, data) in self.instances.iter().zip(self.instance_data.iter_mut()) {
            *data = instance.as_data();
        }

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instance_data),
        );
        self.is_dirty = false;
    }

    pub fn vertex_layout<'a>() -> Vec<wgpu::VertexBufferLayout<'a>> {
        vec![
            ModelVertex::vertex_buffer_layout(),
            Instance::vertex_buffer_layout(),
        ]
    }
}

///////////////////////////

pub fn draw_model<'a, 'b>(
    render_pass: &'b mut wgpu::RenderPass<'a>,
    pipeline_vendor: &'a RenderPipelineVendor,
    model: &'a Model,
    camera: &'a camera::Camera,
    light: &'a light::Light,
    pass: &render_pipeline::Pass,
) where
    'a: 'b, // 'a lifetime at least as long as 'b
{
    let instances = 0..model.instances.len() as u32;
    for mesh in &model.meshes {
        let material = &model.materials[mesh.material];

        if let Some(pipeline) = pipeline_vendor.get_pipeline(material.pipeline_id(pass)) {
            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, model.instance_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &material.bind_group, &[]);
            render_pass.set_bind_group(1, camera.bind_group(), &[]);
            render_pass.set_bind_group(2, light.bind_group(), &[]);
            render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
        } else {
            eprintln!(
                "No pipeline available to render material id: {}",
                material.pipeline_id(pass)
            );
        }
    }
}
