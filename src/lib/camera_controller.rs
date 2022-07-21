use super::util::*;
use cgmath::prelude::*;
use instant::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;

use super::camera::Camera;

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
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
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
        }
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

    pub fn update(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Update camera position
        let linear_vel = self.speed * dt * if self.keyboard_shift_down { 3.0 } else { 1.0 };
        let local_camera_translation = Vec3::new(
            self.keyboard_horizontal * linear_vel,
            self.keyboard_vertical * linear_vel,
            self.keyboard_forward * -linear_vel,
        );
        if local_camera_translation.magnitude2() > 1e-4 {
            camera.local_translate(local_camera_translation);
        }

        // Update camera rotation
        if self.mouse_yaw.abs() > 0.0 || self.mouse_pitch.abs() > 0.0 {
            let mouse_angular_vel = self.sensitivity * dt;
            camera.rotate_by(
                rad(-self.mouse_yaw) * mouse_angular_vel,
                rad(-self.mouse_pitch) * mouse_angular_vel,
            );
        }

        if self.keyboard_yaw.abs() > 0.0 || self.keyboard_pitch.abs() > 0.0 {
            let keyboard_angular_vel = self.speed * self.sensitivity * dt;
            camera.rotate_by(
                rad(self.keyboard_yaw) * keyboard_angular_vel,
                rad(self.keyboard_pitch) * keyboard_angular_vel,
            );
        }

        // Zero out mouse motion
        self.mouse_yaw = 0.0;
        self.mouse_pitch = 0.0;

        // Update Field of View
        let fov: Rad = (deg(45.) + deg((self.zoom / 100_f32) * 30f32)).into();
        camera.set_fov_y(fov);
    }
}
