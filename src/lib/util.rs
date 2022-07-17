use wgpu::util::DeviceExt;

// Some type aliases to make stuff a little less verbose
pub type Vec2 = cgmath::Vector2<f32>;
pub type Vec3 = cgmath::Vector3<f32>;
pub type Vec4 = cgmath::Vector4<f32>;
pub type Mat3 = cgmath::Matrix3<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;
pub type Point3 = cgmath::Point3<f32>;
pub type Rad = cgmath::Rad<f32>;
pub type Deg = cgmath::Deg<f32>;
pub type Quat = cgmath::Quaternion<f32>;

pub fn deg(degrees: f32) -> Deg {
    cgmath::Deg(degrees)
}

pub fn rad(degrees: f32) -> Rad {
    cgmath::Rad(degrees)
}

pub fn color3<V>(color: V) -> Vec3
where
    V: Into<Vec3>,
{
    let v: Vec3 = color.into();
    Vec3::new(v.x * v.x, v.y * v.y, v.z * v.z)
}

pub fn color4<V>(color: V) -> Vec4
where
    V: Into<Vec4>,
{
    let v: Vec4 = color.into();
    Vec4::new(v.x * v.x, v.y * v.y, v.z * v.z, v.w)
}

/// Uniforms is a generic "holder" for uniform data types.
pub struct UniformWrapper<D> {
    pub data: D,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl<D> UniformWrapper<D>
where
    D: bytemuck::Pod + bytemuck::Zeroable + Default,
{
    pub fn new(device: &wgpu::Device) -> Self {
        let data = D::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("Uniform Bind Group Layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Uniform Bind Group"),
        });

        Self {
            data,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn write(&self, queue: &mut wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
