use winit::event::{ElementState, KeyboardInput, MouseButton, WindowEvent};

use super::{app, camera, gpu_state, light};

pub struct Scene {
    gpu_state: gpu_state::GpuState,
    camera: camera::CameraController,
    light: light::Light,
    mouse_pressed: bool,
}

impl Scene {
    pub fn new(window: &winit::window::Window, gpu_state: gpu_state::GpuState) -> Self {
        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = camera::Projection::new(
            window.inner_size().width,
            window.inner_size().height,
            cgmath::Deg(45.0),
            0.1,
            100.0,
        );

        let camera = camera::CameraController::new(&gpu_state.device, camera, projection, 4.0, 0.4);

        let light = light::Light::new(&gpu_state.device, (2.0, 2.0, 2.0), (1.0, 1.0, 1.0));

        Self {
            gpu_state,
            camera,
            light,
            mouse_pressed: false,
        }
    }
}

impl app::AppState for Scene {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu_state.resize(new_size);
        self.camera.resize(new_size);
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.gpu_state.size()
    }

    fn input(
        &mut self,
        event: Option<&winit::event::WindowEvent>,
        mouse_motion: Option<(f64, f64)>,
    ) -> bool {
        if let Some(event) = event {
            match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(key),
                            state,
                            ..
                        },
                    ..
                } => {
                    return self.camera.process_keyboard(*key, *state);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    self.camera.process_scroll(delta);
                    return true;
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    self.mouse_pressed = *state == ElementState::Pressed;
                    return true;
                }
                _ => {}
            }
        }

        if let Some(mouse_motion) = mouse_motion {
            if self.mouse_pressed {
                self.camera.process_mouse(mouse_motion.0, mouse_motion.1);
                return true;
            }
        }

        false
    }

    fn update(&mut self, dt: instant::Duration) {
        self.camera.update(&mut self.gpu_state.queue, dt);
        self.light.update(&mut self.gpu_state.queue, dt);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.gpu_state.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.gpu_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // this is output [[location(0)]]
                    wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                g: 0.2,
                                r: 0.1,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    },
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu_state.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        }

        self.gpu_state
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
