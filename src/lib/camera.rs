use cgmath::*;
use instant::Duration;
use std::f32::consts::FRAC_PI_2;
use std::ops::Mul;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

///////////////////////////////////////////////

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: cgmath::Vector4::zero().into(),
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(
        &mut self,
        camera_position: Point3<f32>,
        camera_projection: Matrix4<f32>,
        camera_view: Matrix4<f32>,
    ) {
        self.view_position = camera_position.to_homogeneous().into();
        self.view_proj = (camera_projection * camera_view).into();
    }
}

///////////////////////////////////////////////

#[derive(Debug)]
pub struct Camera {
    // world view
    position: Point3<f32>,
    look: Matrix3<f32>,

    // projection
    aspect: f32,
    fov_y: Rad<f32>,
    znear: f32,
    zfar: f32,

    // uniform storage
    is_dirty: bool,
    buffer: wgpu::Buffer,
    uniform_data: CameraUniform,
    bind_group: wgpu::BindGroup,
}

impl Camera {
    pub fn new<R: Into<Rad<f32>>>(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        fovy: R,
        znear: f32,
        zfar: f32,
    ) -> Self {
        let uniform_data = CameraUniform::new();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera::buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Camera::bind_group"),
        });

        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            look: Matrix3::identity(),
            aspect: width as f32 / height as f32,
            fov_y: fovy.into(),
            znear,
            zfar,
            is_dirty: true,
            buffer,
            uniform_data,
            bind_group,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.is_dirty {
            self.uniform_data.update_view_proj(
                self.position,
                self.projection_matrix(),
                self.view_matrix(),
            );
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform_data]));
            self.is_dirty = false;
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
        self.is_dirty = true;
    }

    pub fn fov_y(&self) -> Rad<f32> {
        self.fov_y
    }

    pub fn set_fov_y<R: Into<Rad<f32>>>(&mut self, new_fov_y: R) {
        let new_fov_y: Rad<f32> = new_fov_y.into();
        if new_fov_y != self.fov_y {
            self.fov_y = new_fov_y;
            self.is_dirty = true;
        }
    }

    pub fn look_at<P, V>(&mut self, position: P, at: P, up: V)
    where
        P: Into<Point3<f32>>,
        V: Into<Vector3<f32>>,
    {
        let position: Point3<f32> = position.into();
        let at: Point3<f32> = at.into();
        let up: Vector3<f32> = up.into().normalize();

        let forward = -(at - position).normalize();
        let right = up.cross(forward).normalize();
        let up = forward.cross(right).normalize();

        self.look = Matrix3::from_cols(right, up, forward);
        self.position = position;
        self.is_dirty = true;
    }

    pub fn local_translate<V: Into<Vector3<f32>>>(&mut self, translation: V) {
        let translation: Vector3<f32> = translation.into();
        let world_translation = self.look * translation;
        self.position += world_translation;
        self.is_dirty = true;
    }

    pub fn rotate_by(&mut self, yaw: Rad<f32>, pitch: Rad<f32>) {
        // perform rotation about local right axis before rotating about global (0,1,0)
        self.look = Matrix3::from_axis_angle(self.look[0], pitch) * self.look;
        self.look = Matrix3::from_angle_y(yaw) * self.look;
        self.is_dirty = true;
    }

    pub fn world_rotation(&self) -> Matrix3<f32> {
        self.look
    }

    pub fn world_transform(&self) -> Matrix4<f32> {
        let world_rotation = self.world_rotation();
        let world_rotation = Matrix4::from_cols(
            world_rotation[0].extend(0.),
            world_rotation[1].extend(0.),
            world_rotation[2].extend(0.),
            Vector4::unit_w(),
        );
        let world_translation = Matrix4::from_translation(self.position.to_vec());
        world_translation.mul(world_rotation)
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        self.world_transform().invert().unwrap()
    }

    pub fn projection_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fov_y, self.aspect, self.znear, self.zfar)
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
            label: Some("CameraController::bind_group_layout"),
        })
    }
}

///////////////////////////////////////////////

#[derive(Debug)]
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
        let local_camera_translation = Vector3::new(
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
                Rad(-self.mouse_yaw) * mouse_angular_vel,
                Rad(-self.mouse_pitch) * mouse_angular_vel,
            );
        }

        if self.keyboard_yaw.abs() > 0.0 || self.keyboard_pitch.abs() > 0.0 {
            let keyboard_angular_vel = self.speed * self.sensitivity * dt;
            self.camera.rotate_by(
                Rad(self.keyboard_yaw) * keyboard_angular_vel,
                Rad(self.keyboard_pitch) * keyboard_angular_vel,
            );
        }

        // Zero out mouse motion
        self.mouse_yaw = 0.0;
        self.mouse_pitch = 0.0;

        // Update Field of View
        let fov: Rad<f32> = (Deg(45.) + Deg((self.zoom / 100_f32) * 30f32)).into();
        self.camera.set_fov_y(fov);

        self.camera.update(queue);
    }
}
