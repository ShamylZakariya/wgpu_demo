use super::util::*;
use cgmath::prelude::*;
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use std::ops::Mul;
use winit::dpi::PhysicalPosition;
use winit::event::*;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

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
}

impl Camera {
    pub fn new<R: Into<Rad>>(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        fov_y: R,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let uniform = CameraUniform::new(device);

        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            look: Mat3::identity(),
            aspect: width as f32 / height as f32,
            fov_y: fov_y.into(),
            z_near,
            z_far,
            is_dirty: true,
            uniform,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.is_dirty {
            let position = self.position;
            let projection = self.projection_matrix();
            let view = self.view_matrix();
            self.uniform
                .edit()
                .update_view_proj(position, projection, view);
            self.uniform.write(queue);
            self.is_dirty = false;
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
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

pub struct CameraController {
    keyboard_horizontal: f32,
    keyboard_forward: f32,
    keyboard_vertical: f32,
    keyboard_yaw: f32,
    keyboard_pitch: f32,
    keyboard_shift_down: bool,
    mouse_yaw: f32,
    mouse_pitch: f32,
    zoom: f32,
    speed: f32,
    sensitivity: f32,
    camera: Camera,
}

impl CameraController {
    pub fn new(camera: Camera, speed: f32, sensitivity: f32) -> Self {
        Self {
            keyboard_horizontal: 0.0,
            keyboard_forward: 0.0,
            keyboard_vertical: 0.0,
            keyboard_yaw: 0.0,
            keyboard_pitch: 0.0,
            keyboard_shift_down: false,
            mouse_yaw: 0.0,
            mouse_pitch: 0.0,
            zoom: 0.0,
            speed,
            sensitivity,
            camera,
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let (amount, pressed) = if state == ElementState::Pressed {
            (1.0, true)
        } else {
            (0.0, false)
        };
        match key {
            VirtualKeyCode::W => {
                self.keyboard_forward = amount;
                true
            }
            VirtualKeyCode::S => {
                self.keyboard_forward = -amount;
                true
            }
            VirtualKeyCode::A => {
                self.keyboard_horizontal = -amount;
                true
            }
            VirtualKeyCode::D => {
                self.keyboard_horizontal = amount;
                true
            }
            VirtualKeyCode::E => {
                self.keyboard_vertical = amount;
                true
            }
            VirtualKeyCode::Q => {
                self.keyboard_vertical = -amount;
                true
            }
            VirtualKeyCode::Up => {
                self.keyboard_pitch = amount;
                true
            }
            VirtualKeyCode::Down => {
                self.keyboard_pitch = -amount;
                true
            }
            VirtualKeyCode::Left => {
                self.keyboard_yaw = amount;
                true
            }
            VirtualKeyCode::Right => {
                self.keyboard_yaw = -amount;
                true
            }
            VirtualKeyCode::LShift => {
                self.keyboard_shift_down = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.mouse_yaw = mouse_dx as f32;
        self.mouse_pitch = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.zoom += match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll * 20_f32,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
        self.zoom = self.zoom.min(100f32).max(-100f32);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera.resize(new_size.width, new_size.height);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Update camera position
        let linear_vel = self.speed * dt * if self.keyboard_shift_down { 3.0 } else { 1.0 };
        let local_camera_translation = Vec3::new(
            self.keyboard_horizontal * linear_vel,
            self.keyboard_vertical * linear_vel,
            self.keyboard_forward * -linear_vel,
        );
        if local_camera_translation.magnitude2() > 1e-4 {
            self.camera.local_translate(local_camera_translation);
        }

        // Update camera rotation
        if self.mouse_yaw.abs() > 0.0 || self.mouse_pitch.abs() > 0.0 {
            let mouse_angular_vel = self.sensitivity * dt;
            self.camera.rotate_by(
                rad(-self.mouse_yaw) * mouse_angular_vel,
                rad(-self.mouse_pitch) * mouse_angular_vel,
            );
        }

        if self.keyboard_yaw.abs() > 0.0 || self.keyboard_pitch.abs() > 0.0 {
            let keyboard_angular_vel = self.speed * self.sensitivity * dt;
            self.camera.rotate_by(
                rad(self.keyboard_yaw) * keyboard_angular_vel,
                rad(self.keyboard_pitch) * keyboard_angular_vel,
            );
        }

        // Zero out mouse motion
        self.mouse_yaw = 0.0;
        self.mouse_pitch = 0.0;

        // Update Field of View
        let fov: Rad = (deg(45.) + deg((self.zoom / 100_f32) * 30f32)).into();
        self.camera.set_fov_y(fov);

        self.camera.update(queue);
    }
}
