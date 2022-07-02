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

#[derive(Debug)]
pub struct Camera {
    position: Point3<f32>,
    yaw: Rad<f32>,
    pitch: Rad<f32>,
}

impl Camera {
    pub fn new<P, A>(position: P, yaw: A, pitch: A) -> Self
    where
        P: Into<Point3<f32>>,
        A: Into<Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn world_rotation(&self) -> Matrix3<f32> {
        let yaw_rotation = Matrix3::from_axis_angle(Vector3::unit_y(), self.yaw);
        let right = yaw_rotation[0].normalize();
        let pitch_rotation = Matrix3::from_axis_angle(right, self.pitch);
        pitch_rotation.mul(yaw_rotation)
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
}

///////////////////////////////////////////////

#[derive(Debug)]
pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

///////////////////////////////////////////////

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

    fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.view_matrix()).into();
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
    projection: Projection,
    projection_is_dirty: bool,
    buffer: wgpu::Buffer,
    uniform_data: CameraUniform,
    bind_group: wgpu::BindGroup,
}

impl CameraController {
    pub fn new(
        device: &wgpu::Device,
        camera: Camera,
        projection: Projection,
        speed: f32,
        sensitivity: f32,
    ) -> Self {
        let mut uniform_data = CameraUniform::new();
        uniform_data.update_view_proj(&camera, &projection);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("CameraController::buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::bind_group_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("CameraController::bind_group"),
        });

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
            projection,
            projection_is_dirty: true,
            buffer,
            uniform_data,
            bind_group,
        }
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
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.projection.resize(new_size.width, new_size.height);
        self.projection_is_dirty = true;
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        let dt = dt.as_secs_f32();
        let mut camera_is_dirty = false;

        // Update camera position
        let linear_vel = self.speed * dt * if self.keyboard_shift_down { 3.0 } else { 1.0 };
        let mut camera_position = self.camera.position;
        let camera_rotation = self.camera.world_rotation();
        let camera_right = camera_rotation[0].normalize();
        let camera_up = camera_rotation[1].normalize();
        let camera_forward = camera_rotation[2].normalize() * -1.;

        camera_position += camera_forward * self.keyboard_forward * linear_vel;
        camera_position += camera_right * self.keyboard_horizontal * linear_vel;
        camera_position += camera_up * self.keyboard_vertical * linear_vel;
        if camera_position.distance2(self.camera.position) > 1e-4 {
            self.camera.position = camera_position;
            camera_is_dirty = true;
        }

        // Update camera rotation
        let mouse_angular_vel = self.sensitivity * dt;
        self.camera.yaw += Rad(-self.mouse_yaw) * mouse_angular_vel;
        self.camera.pitch += Rad(-self.mouse_pitch) * mouse_angular_vel;

        let keyboard_angular_vel = self.speed * self.sensitivity * dt;
        self.camera.yaw += Rad(self.keyboard_yaw) * keyboard_angular_vel;
        self.camera.pitch += Rad(self.keyboard_pitch) * keyboard_angular_vel;

        if self.mouse_yaw.abs() > 0.0
            || self.mouse_pitch.abs() > 0.0
            || self.keyboard_yaw.abs() > 0.0
            || self.keyboard_pitch.abs() > 0.0
        {
            camera_is_dirty = true
        }

        // Zero out mouse motion
        self.mouse_yaw = 0.0;
        self.mouse_pitch = 0.0;

        // Update Field of View
        let zoom = self.zoom.min(100f32).max(-100f32) / 100f32;
        let fov = Deg(45.) + Deg(zoom * 30f32);
        self.projection.fovy = fov.into();

        // Update the uniform buffer and write it
        if camera_is_dirty || self.projection_is_dirty {
            self.uniform_data
                .update_view_proj(&self.camera, &self.projection);

            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform_data]));
            self.projection_is_dirty = false;
        }
    }
}
