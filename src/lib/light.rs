use cgmath::*;
use instant::Duration;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    _padding: u32, // uniforms require 16-byte (4 float field spacing)
    color: [f32; 3],
    _padding2: u32,
}

pub struct Light {
    position: Point3<f32>,
    buffer: wgpu::Buffer,
    uniform: LightUniform,
    pub bind_group: wgpu::BindGroup,
}

impl Light {
    pub fn new<V>(device: &wgpu::Device, position: V, color: V) -> Self
    where
        V: Into<Point3<f32>>,
    {
        let position: Point3<f32> = position.into();
        let color: Point3<f32> = color.into();

        let uniform = LightUniform {
            position: [position.x, position.y, position.z],
            _padding: 0,
            color: [color.x, color.y, color.z],
            _padding2: 0,
        };

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light::buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Light::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Light::bind_group"),
        });

        Self {
            position,
            buffer,
            uniform,
            bind_group,
        }
    }

    pub fn update(&mut self, queue: &mut wgpu::Queue, _dt: Duration) {
        // Update the uniform buffer and write it
        self.uniform.position = self.position.into();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Light::bind_group_layout"),
        })
    }
}
