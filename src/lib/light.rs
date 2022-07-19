use super::util::*;
use cgmath::prelude::*;

const EPSILON: f32 = 1e-4;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct LightUniformData {
    position: Point3,
    _padding1: u32, // uniforms require 16-byte (4 float field spacing)
    direction: Vec3,
    _padding2: u32,
    ambient: Vec3,
    _padding3: u32,
    color: Vec3,
    _padding4: u32,
    // x: constant, y: linear, z: exponential, w: dot spot breadth
    attenuation: Vec4,
    light_type: i32,
    _padding5: [u32; 3],
}

unsafe impl bytemuck::Pod for LightUniformData {}
unsafe impl bytemuck::Zeroable for LightUniformData {}

impl Default for LightUniformData {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            direction: Vec3::zero(),
            ambient: Vec3::zero(),
            color: Vec3::zero(),
            attenuation: Vec4::zero(),
            light_type: 0,
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
            _padding4: 0,
            _padding5: [0; 3],
        }
    }
}

impl LightUniformData {
    fn set_position(&mut self, position: Point3) -> &mut Self {
        self.position = position;
        self
    }

    fn set_direction(&mut self, direction: Vec3) -> &mut Self {
        self.direction = direction.normalize();
        self
    }

    fn set_ambient(&mut self, ambient: Vec3) -> &mut Self {
        self.ambient = ambient;
        self
    }

    fn set_color(&mut self, color: Vec3) -> &mut Self {
        self.color = color;
        self
    }

    fn set_attenuation(&mut self, attenuation: Vec4) -> &mut Self {
        self.attenuation = attenuation;
        self.attenuation.x = self.attenuation.x.max(0.0);
        self.attenuation.y = self.attenuation.y.max(0.0);
        self.attenuation.z = self.attenuation.z.max(0.0);
        self.attenuation.w = self.attenuation.w.max(0.0);
        self
    }

    fn set_light_type(&mut self, light_type: LightType) -> &mut Self {
        self.light_type = light_type.into();
        self
    }
}

type LightUniform = UniformWrapper<LightUniformData>;

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
    uniform: LightUniform,
}

impl Light {
    pub fn new_ambient(device: &wgpu::Device, desc: &AmbientLightDescriptor) -> Self {
        let mut uniform = LightUniform::new(device);
        uniform
            .get_mut()
            .set_light_type(LightType::Ambient)
            .set_ambient(desc.ambient)
            .set_attenuation(Vec4::new(1.0, 0.0, 0.0, 0.0));
        Self {
            light_type: LightType::Ambient,
            uniform,
        }
    }

    pub fn new_point(device: &wgpu::Device, desc: &PointLightDescriptor) -> Self {
        let mut uniform = LightUniform::new(device);
        uniform
            .get_mut()
            .set_light_type(LightType::Point)
            .set_position(desc.position)
            .set_ambient(desc.ambient)
            .set_color(desc.color)
            .set_attenuation(Vec4::new(
                desc.constant_attenuation,
                desc.linear_attenuation,
                desc.exponential_attenuation,
                0.0,
            ));
        Self {
            light_type: LightType::Point,
            uniform,
        }
    }

    pub fn new_spot(device: &wgpu::Device, desc: &SpotLightDescriptor) -> Self {
        let mut uniform = LightUniform::new(device);
        uniform
            .get_mut()
            .set_light_type(LightType::Spot)
            .set_position(desc.position)
            .set_direction(desc.direction)
            .set_ambient(desc.ambient)
            .set_color(desc.color)
            .set_attenuation(Vec4::new(
                desc.constant_attenuation,
                desc.linear_attenuation,
                desc.exponential_attenuation,
                desc.spot_breadth.cos(),
            ));
        Self {
            light_type: LightType::Spot,
            uniform,
        }
    }

    pub fn new_directional(device: &wgpu::Device, desc: &DirectionalLightDescriptor) -> Self {
        let mut uniform = LightUniform::new(device);
        uniform
            .get_mut()
            .set_light_type(LightType::Directional)
            .set_direction(desc.direction)
            .set_ambient(desc.ambient)
            .set_color(desc.color)
            .set_attenuation(Vec4::new(desc.constant_attenuation, 0.0, 0.0, 0.0));
        Self {
            light_type: LightType::Directional,
            uniform,
        }
    }

    pub fn light_type(&self) -> LightType {
        self.light_type
    }

    pub fn ambient(&self) -> Vec3 {
        self.uniform.get().ambient
    }

    pub fn set_ambient<V: Into<Vec3>>(&mut self, ambient: V) {
        let new_ambient: Vec3 = ambient.into();
        if new_ambient.distance2(self.ambient()) > EPSILON {
            self.uniform.get_mut().set_ambient(new_ambient);
        }
    }

    pub fn position(&self) -> Point3 {
        self.uniform.get().position
    }

    pub fn set_position<P: Into<Point3>>(&mut self, position: P) {
        let new_position: Point3 = position.into();
        if new_position.distance2(self.position()) > EPSILON {
            self.uniform.get_mut().set_position(new_position);
        }
    }

    pub fn direction(&self) -> Vec3 {
        self.uniform.get().direction
    }

    pub fn set_direction<V: Into<Vec3>>(&mut self, dir: V) {
        let new_dir: Vec3 = dir.into();
        if new_dir.distance2(self.direction()) > EPSILON {
            self.uniform.get_mut().set_direction(new_dir);
        }
    }

    pub fn color(&self) -> Vec3 {
        self.uniform.get().color
    }

    pub fn set_color<V: Into<Vec3>>(&mut self, color: V) {
        let new_color: Vec3 = color.into();
        if new_color.distance2(self.color()) > EPSILON {
            self.uniform.get_mut().set_color(new_color);
        }
    }

    pub fn constant_attenuation(&self) -> f32 {
        self.uniform.get().attenuation.x
    }

    pub fn set_constant_attenuation(&mut self, constant_attenuation: f32) {
        let mut attenuation = self.uniform.get().attenuation;
        if (constant_attenuation - attenuation.x).abs() > EPSILON {
            attenuation.x = constant_attenuation;
            self.uniform.get_mut().set_attenuation(attenuation);
        }
    }

    pub fn linear_attenuation(&self) -> f32 {
        self.uniform.get().attenuation.y
    }

    pub fn set_linear_attenuation(&mut self, linear_attenuation: f32) {
        let mut attenuation = self.uniform.get().attenuation;
        if (linear_attenuation - attenuation.x).abs() > EPSILON {
            attenuation.y = linear_attenuation;
            self.uniform.get_mut().set_attenuation(attenuation);
        }
    }

    pub fn exponential_attenuation(&self) -> f32 {
        self.uniform.get().attenuation.z
    }

    pub fn set_exponential_attenuation(&mut self, exponential_attenuation: f32) {
        let mut attenuation = self.uniform.get().attenuation;
        if (exponential_attenuation - attenuation.x).abs() > EPSILON {
            attenuation.z = exponential_attenuation;
            self.uniform.get_mut().set_attenuation(attenuation);
        }
    }

    pub fn spot_breadth(&self) -> Deg {
        deg(self.uniform.get().attenuation.w.acos())
    }

    pub fn set_spot_breadth(&mut self, spot_breadth: Deg) {
        if spot_breadth != self.spot_breadth() {
            let mut attenuation = self.uniform.get().attenuation;
            attenuation.w = spot_breadth.cos();
            self.uniform.get_mut().attenuation = attenuation;
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.write(queue);
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.uniform.bind_group
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        LightUniform::bind_group_layout(device)
    }
}
