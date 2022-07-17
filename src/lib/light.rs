use cgmath::prelude::*;
use wgpu::util::DeviceExt;

use super::util::*;

const EPSILON: f32 = 1e-4;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct LightUniform {
    position: [f32; 3],
    _padding1: u32, // uniforms require 16-byte (4 float field spacing)
    direction: [f32; 3],
    _padding2: u32,
    ambient: [f32; 3],
    _padding3: u32,
    color: [f32; 3],
    _padding4: u32,
    // x: constant, y: linear, z: exponential, w: dot spot breadth
    attenuation: [f32; 4],
    light_type: i32,
    _padding5: [u32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightType {
    Ambient,
    Point,
    Spot,
    Directional,
}

impl LightType {
    fn value(&self) -> i32 {
        match self {
            LightType::Ambient => 0,
            LightType::Point => 1,
            LightType::Spot => 2,
            LightType::Directional => 3,
        }
    }
}

impl From<LightType> for i32 {
    fn from(light_type: LightType) -> Self {
        light_type.value() as i32
    }
}

pub struct AmbientLightDescriptor {
    pub ambient: Vec3,
}

pub struct PointLightDescriptor {
    pub position: Point3,
    pub ambient: Vec3,
    pub color: Vec3,
    pub constant_attenuation: f32,
    pub linear_attenuation: f32,
    pub exponential_attenuation: f32,
}

pub struct SpotLightDescriptor {
    pub position: Point3,
    pub direction: Vec3,
    pub ambient: Vec3,
    pub color: Vec3,
    pub constant_attenuation: f32,
    pub linear_attenuation: f32,
    pub exponential_attenuation: f32,
    pub spot_breadth: Deg,
}

pub struct DirectionalLightDescriptor {
    pub direction: Vec3,
    pub ambient: Vec3,
    pub color: Vec3,
    pub constant_attenuation: f32,
}

pub struct Light {
    light_type: LightType,
    position: Point3,
    direction: Vec3,
    ambient: Vec3,
    color: Vec3,
    constant_attenuation: f32,
    linear_attenuation: f32,
    exponential_attenuation: f32,
    spot_breadth: Deg,

    is_dirty: bool,
    buffer: wgpu::Buffer,
    uniform: LightUniform,
    bind_group: wgpu::BindGroup,
}

impl Light {
    pub fn new_ambient(device: &wgpu::Device, desc: &AmbientLightDescriptor) -> Self {
        let (buffer, bind_group, uniform) = Self::create_resources(device);

        Self {
            light_type: LightType::Ambient,
            position: [0_f32; 3].into(),
            direction: Vec3::zero(),
            ambient: desc.ambient,
            color: Vec3::zero(),
            constant_attenuation: 1_f32,
            linear_attenuation: 0_f32,
            exponential_attenuation: 0_f32,
            spot_breadth: deg(0_f32),
            is_dirty: true,
            buffer,
            uniform,
            bind_group,
        }
    }

    pub fn new_point(device: &wgpu::Device, desc: &PointLightDescriptor) -> Self {
        let (buffer, bind_group, uniform) = Self::create_resources(device);

        Self {
            light_type: LightType::Point,
            position: desc.position,
            direction: Vec3::zero(),
            ambient: desc.ambient,
            color: desc.color,
            constant_attenuation: desc.constant_attenuation.max(0_f32),
            linear_attenuation: desc.linear_attenuation.max(0_f32),
            exponential_attenuation: desc.exponential_attenuation.max(0_f32),
            spot_breadth: deg(0_f32),
            is_dirty: true,
            buffer,
            uniform,
            bind_group,
        }
    }

    pub fn new_spot(device: &wgpu::Device, desc: &SpotLightDescriptor) -> Self {
        let (buffer, bind_group, uniform) = Self::create_resources(device);

        Self {
            light_type: LightType::Spot,
            position: desc.position,
            direction: desc.direction.normalize(),
            ambient: desc.ambient,
            color: desc.color,
            constant_attenuation: desc.constant_attenuation.max(0_f32),
            linear_attenuation: desc.linear_attenuation.max(0_f32),
            exponential_attenuation: desc.exponential_attenuation.max(0_f32),
            spot_breadth: desc.spot_breadth,
            is_dirty: true,
            buffer,
            uniform,
            bind_group,
        }
    }

    pub fn new_directional(device: &wgpu::Device, desc: &DirectionalLightDescriptor) -> Self {
        let (buffer, bind_group, uniform) = Self::create_resources(device);

        Self {
            light_type: LightType::Directional,
            position: [0_f32; 3].into(),
            direction: desc.direction.normalize(),
            ambient: desc.ambient,
            color: desc.color,
            constant_attenuation: desc.constant_attenuation.max(0_f32),
            linear_attenuation: 0_f32,
            exponential_attenuation: 0_f32,
            spot_breadth: deg(0_f32),
            is_dirty: true,
            buffer,
            uniform,
            bind_group,
        }
    }

    fn create_resources(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::BindGroup, LightUniform) {
        // we can an empty uniform buffer here because we'll update it later in ::update()
        let uniform = LightUniform::default();
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

        (buffer, bind_group, uniform)
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub fn light_type(&self) -> LightType {
        self.light_type
    }

    pub fn ambient(&self) -> Vec3 {
        self.ambient
    }

    pub fn set_ambient<V: Into<Vec3>>(&mut self, ambient: V) {
        let new_ambient: Vec3 = ambient.into();
        if new_ambient.distance2(self.ambient) > EPSILON {
            self.ambient = new_ambient;
            self.is_dirty = true;
        }
    }

    pub fn position(&self) -> Point3 {
        self.position
    }

    pub fn set_position<P: Into<Point3>>(&mut self, position: P) {
        let new_position: Point3 = position.into();
        if new_position.distance2(self.position) > EPSILON {
            self.position = new_position;
            self.is_dirty = true;
        }
    }

    pub fn direction(&self) -> Vec3 {
        self.direction
    }

    pub fn set_direction<V: Into<Vec3>>(&mut self, dir: V) {
        let new_dir: Vec3 = dir.into();
        if new_dir.distance2(self.direction) > EPSILON {
            self.direction = new_dir;
            self.is_dirty = true;
        }
    }

    pub fn color(&self) -> Vec3 {
        self.color
    }

    pub fn set_color<V: Into<Vec3>>(&mut self, color: V) {
        let new_color: Vec3 = color.into();
        if new_color.distance2(self.color) > EPSILON {
            self.color = new_color;
            self.is_dirty = true;
        }
    }

    pub fn constant_attenuation(&self) -> f32 {
        self.constant_attenuation
    }

    pub fn set_constant_attenuation(&mut self, constant_attenuation: f32) {
        if (constant_attenuation - self.constant_attenuation).abs() > EPSILON {
            self.constant_attenuation = constant_attenuation;
            self.is_dirty = true;
        }
    }

    pub fn linear_attenuation(&self) -> f32 {
        self.linear_attenuation
    }

    pub fn set_linear_attenuation(&mut self, linear_attenuation: f32) {
        if (linear_attenuation - self.linear_attenuation).abs() > EPSILON {
            self.linear_attenuation = linear_attenuation;
            self.is_dirty = true;
        }
    }

    pub fn exponential_attenuation(&self) -> f32 {
        self.exponential_attenuation
    }

    pub fn set_exponential_attenuation(&mut self, exponential_attenuation: f32) {
        if (exponential_attenuation - self.exponential_attenuation).abs() > EPSILON {
            self.exponential_attenuation = exponential_attenuation;
            self.is_dirty = true;
        }
    }

    pub fn spot_breadth(&self) -> Deg {
        self.spot_breadth
    }

    pub fn set_spot_breadth(&mut self, spot_breadth: Deg) {
        if spot_breadth != self.spot_breadth {
            self.spot_breadth = spot_breadth;
            self.is_dirty = true;
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if !self.is_dirty {
            return;
        }

        // Update the uniform buffer and write it
        self.uniform.light_type = self.light_type.into();
        self.uniform.ambient = color3(self.ambient).into();
        self.uniform.color = color3(self.color).into();

        match self.light_type {
            LightType::Ambient => {
                self.uniform.attenuation = [1_f32, 0_f32, 0_f32, 0_f32];
            }
            LightType::Point => {
                self.uniform.position = self.position.into();
                self.uniform.attenuation = [
                    self.constant_attenuation.max(0_f32),
                    self.linear_attenuation.max(0_f32),
                    self.exponential_attenuation.max(0_f32),
                    0_f32,
                ];
            }
            LightType::Spot => {
                self.uniform.position = self.position.into();
                self.uniform.direction = self.direction.normalize().into();
                self.uniform.attenuation = [
                    self.constant_attenuation.max(0_f32),
                    self.linear_attenuation.max(0_f32),
                    self.exponential_attenuation.max(0_f32),
                    self.spot_breadth.cos(),
                ];
            }
            LightType::Directional => {
                self.uniform.direction = self.direction.normalize().into();
                self.uniform.attenuation =
                    [self.constant_attenuation.max(0_f32), 0_f32, 0_f32, 0_f32];
            }
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
        self.is_dirty = false;
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
