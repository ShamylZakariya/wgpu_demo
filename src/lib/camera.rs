use super::{gpu_state, util::*};
use cgmath::prelude::*;
use std::ops::Mul;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

///////////////////////////////////////////////

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CameraUniformData {
    view_position: Vec4,
    view_proj: Mat4,
}

unsafe impl bytemuck::Pod for CameraUniformData {}
unsafe impl bytemuck::Zeroable for CameraUniformData {}

impl Default for CameraUniformData {
    fn default() -> Self {
        Self {
            view_position: Vec4::zero(),
            view_proj: Mat4::identity(),
        }
    }
}

impl CameraUniformData {
    fn update_view_proj(
        &mut self,
        camera_position: Point3,
        camera_projection: Mat4,
        camera_view: Mat4,
    ) {
        self.view_position = camera_position.to_homogeneous();
        self.view_proj = camera_projection * camera_view;
    }
}

type CameraUniform = UniformWrapper<CameraUniformData>;

///////////////////////////////////////////////

pub struct RenderBuffers {
    pub color: Option<super::texture::Texture>,
    pub depth: Option<super::texture::Texture>,
}

///////////////////////////////////////////////

pub struct Camera {
    // world view
    position: Point3,
    look: Mat3,

    // projection
    aspect: f32,
    fov_y: Rad,
    z_near: f32,
    z_far: f32,

    // uniform storage
    is_dirty: bool,
    uniform: CameraUniform,

    // attachments
    pub render_buffers: RenderBuffers,
}

impl Camera {
    pub fn new<R: Into<Rad>>(
        gpu_state: &gpu_state::GpuState,
        fov_y: R,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let uniform = CameraUniform::new(&gpu_state.device);

        // create depth texture
        let depth_attachment = super::texture::Texture::create_depth_texture(
            &gpu_state.device,
            &gpu_state.config,
            "Depth Attachment",
        );

        let color_attachment = super::texture::Texture::create_color_texture(
            &gpu_state.device,
            &gpu_state.config,
            "Color Attachment",
        );

        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            look: Mat3::identity(),
            aspect: gpu_state.size.width as f32 / gpu_state.size.height as f32,
            fov_y: fov_y.into(),
            z_near,
            z_far,
            is_dirty: true,
            uniform,
            render_buffers: RenderBuffers {
                color: Some(color_attachment),
                depth: Some(depth_attachment),
            },
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.is_dirty {
            let position = self.position;
            let projection = self.projection_matrix();
            let view = self.view_matrix();
            self.uniform
                .get_mut()
                .update_view_proj(position, projection, view);
            self.uniform.write(queue);
            self.is_dirty = false;
        }
    }

    pub fn resize(&mut self, gpu_state: &gpu_state::GpuState, size: winit::dpi::PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;

        if self.render_buffers.depth.is_some() {
            self.render_buffers
                .depth
                .replace(super::texture::Texture::create_depth_texture(
                    &gpu_state.device,
                    &gpu_state.config,
                    "Depth Attachment",
                ));
        }

        if self.render_buffers.color.is_some() {
            self.render_buffers
                .color
                .replace(super::texture::Texture::create_color_texture(
                    &gpu_state.device,
                    &gpu_state.config,
                    "Color Attachment",
                ));
        }
        self.is_dirty = true;
    }

    pub fn fov_y(&self) -> Rad {
        self.fov_y
    }

    pub fn set_fov_y<R: Into<Rad>>(&mut self, new_fov_y: R) {
        let new_fov_y: Rad = new_fov_y.into();
        if new_fov_y != self.fov_y {
            self.fov_y = new_fov_y;
            self.is_dirty = true;
        }
    }

    pub fn depth_range(&self) -> (f32, f32) {
        (self.z_near, self.z_far)
    }

    pub fn set_depth_range(&mut self, z_near: f32, z_far: f32) {
        if (z_near - self.z_near).abs() > 1e-4 || (z_far - self.z_far).abs() > 1e-4 {
            self.z_near = z_near;
            self.z_far = z_far;
            self.is_dirty = true;
        }
    }

    pub fn look_at<P, V>(&mut self, position: P, at: P, up: V)
    where
        P: Into<Point3>,
        V: Into<Vec3>,
    {
        let position: Point3 = position.into();
        let at: Point3 = at.into();
        let up: Vec3 = up.into().normalize();

        let forward = -(at - position).normalize();
        let right = up.cross(forward).normalize();
        let up = forward.cross(right).normalize();

        self.look = Mat3::from_cols(right, up, forward);
        self.position = position;
        self.is_dirty = true;
    }

    pub fn local_translate<V: Into<Vec3>>(&mut self, translation: V) {
        let translation: Vec3 = translation.into();
        let world_translation = self.look * translation;
        self.position += world_translation;
        self.is_dirty = true;
    }

    pub fn rotate_by(&mut self, yaw: Rad, pitch: Rad) {
        // perform rotation about local right axis before rotating about global (0,1,0)
        self.look = Mat3::from_axis_angle(self.look[0], pitch) * self.look;
        self.look = Mat3::from_angle_y(yaw) * self.look;
        self.is_dirty = true;
    }

    pub fn world_rotation(&self) -> Mat3 {
        self.look
    }

    pub fn world_transform(&self) -> Mat4 {
        let world_rotation = self.world_rotation();
        let world_rotation = Mat4::from_cols(
            world_rotation[0].extend(0.),
            world_rotation[1].extend(0.),
            world_rotation[2].extend(0.),
            Vec4::unit_w(),
        );
        let world_translation = Mat4::from_translation(self.position.to_vec());
        world_translation.mul(world_rotation)
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.world_transform().invert().unwrap()
    }

    pub fn projection_matrix(&self) -> Mat4 {
        OPENGL_TO_WGPU_MATRIX
            * cgmath::perspective(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.uniform.bind_group
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        CameraUniform::bind_group_layout(device)
    }
}

///////////////////////////////////////////////
