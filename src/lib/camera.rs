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

    pub fn view_matrix(&self) -> Matrix4<f32> {
        let world_rotation = self.world_rotation();
        let world_rotation = Matrix4::from_cols(
            world_rotation[0].extend(0.),
            world_rotation[1].extend(0.),
            world_rotation[2].extend(0.),
            Vector4::unit_w(),
        );
        let world_translation = Matrix4::from_translation(self.position.to_vec());
        let world_transform = world_translation.mul(world_rotation);
        return world_transform.invert().unwrap();

        // let world_rotation = self.world_rotation();
        // let forward = world_rotation[2].normalize();
        // let up = world_rotation[1].normalize();
        // Matrix4::look_to_rh(self.position, forward, up)
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
    keyboard_left: f32,
    keyboard_right: f32,
    keyboard_forward: f32,
    keyboard_backward: f32,
    keyboard_up: f32,
    keyboard_down: f32,
    mouse_horizontal: f32,
    mouse_vertical: f32,
    zoom: f32,
    speed: f32,
    sensitivity: f32,

    camera: Camera,
    projection: Projection,
    buffer: wgpu::Buffer,
    uniform_data: CameraUniform,
    pub bind_group: wgpu::BindGroup,
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
            keyboard_left: 0.0,
            keyboard_right: 0.0,
            keyboard_forward: 0.0,
            keyboard_backward: 0.0,
            keyboard_up: 0.0,
            keyboard_down: 0.0,
            mouse_horizontal: 0.0,
            mouse_vertical: 0.0,
            zoom: 0.0,
            speed,
            sensitivity,
            camera,
            projection,
            buffer,
            uniform_data,
            bind_group,
        }
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
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.keyboard_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.keyboard_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.keyboard_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.keyboard_right = amount;
                true
            }
            VirtualKeyCode::E => {
                self.keyboard_up = amount;
                true
            }
            VirtualKeyCode::Q => {
                self.keyboard_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.mouse_horizontal = mouse_dx as f32;
        self.mouse_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.zoom += match delta {
            MouseScrollDelta::LineDelta(_, scroll) => *scroll * 20. as f32,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.projection.resize(new_size.width, new_size.height);
    }

    pub fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Update camera position
        let linear_vel = self.speed * dt;
        let mut camera_position = self.camera.position;
        let camera_rotation = self.camera.world_rotation();
        let camera_right = camera_rotation[0].normalize();
        let camera_up = camera_rotation[1].normalize();
        let camera_forward = camera_rotation[2].normalize() * -1.;

        camera_position += camera_forward * self.keyboard_forward * linear_vel;
        camera_position -= camera_forward * self.keyboard_backward * linear_vel;

        camera_position += camera_right * self.keyboard_right * linear_vel;
        camera_position -= camera_right * self.keyboard_left * linear_vel;

        camera_position += camera_up * self.keyboard_up * linear_vel;
        camera_position -= camera_up * self.keyboard_down * linear_vel;

        self.camera.position = camera_position;

        // Update camera rotation
        let angular_vel = self.sensitivity * dt;
        self.camera.yaw += Rad(-self.mouse_horizontal) * angular_vel;
        self.camera.pitch += Rad(-self.mouse_vertical) * angular_vel;

        // Zero out mouse motion
        self.mouse_horizontal = 0.0;
        self.mouse_vertical = 0.0;

        // Update Field of View
        let zoom = self.zoom.min(100f32).max(-100f32) / 100f32;
        let fov = Deg(45.) + Deg(zoom * 30f32);
        self.projection.fovy = fov.into();

        // Update the uniform buffer and write it
        self.uniform_data
            .update_view_proj(&self.camera, &self.projection);

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform_data]));
    }
}
