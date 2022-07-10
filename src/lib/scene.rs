use cgmath::*;
use winit::event::{ElementState, KeyboardInput, MouseButton, WindowEvent};

use super::{app, camera, gpu_state, light, model, render_pipeline};

//////////////////////////////////////////////

pub struct Scene {
    gpu_state: gpu_state::GpuState,
    camera_controller: camera::CameraController,
    ambient_light: light::Light,
    lights: Vec<light::Light>,
    models: Vec<model::Model>,
    mouse_pressed: bool,
}

impl Scene {
    pub fn new(
        mut gpu_state: gpu_state::GpuState,
        lights: Vec<light::Light>,
        camera: camera::Camera,
        models: Vec<model::Model>,
    ) -> Self {
        let projection = camera::Projection::new(
            gpu_state.size().width,
            gpu_state.size().height,
            Deg(45.0),
            0.5,
            500.0,
        );

        let camera_controller =
            camera::CameraController::new(&gpu_state.device, camera, projection, 4.0, 0.4);

        // create a pipeline (if needed) for each material
        for model in models.iter() {
            model.prepare_pipelines(&mut gpu_state);
        }

        // Create an ambient light which is the sum of all the ambient terms of the light sources provided
        let ambient_term = lights
            .iter()
            .fold(Vector3::zero(), |total, light| total + light.ambient());

        let ambient_light = light::Light::new_ambient(
            &gpu_state.device,
            &light::AmbientLightDescriptor {
                ambient: ambient_term,
            },
        );

        Self {
            gpu_state,
            camera_controller,
            ambient_light,
            lights,
            models,
            mouse_pressed: false,
        }
    }
}

impl app::AppState for Scene {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu_state.resize(new_size);
        self.camera_controller.resize(new_size);
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
                    return self.camera_controller.process_keyboard(*key, *state);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    self.camera_controller.process_scroll(delta);
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
                self.camera_controller
                    .process_mouse(mouse_motion.0, mouse_motion.1);
                return true;
            }
        }

        false
    }

    fn update(&mut self, dt: instant::Duration) {
        self.camera_controller.update(&self.gpu_state.queue, dt);

        self.ambient_light.set_ambient(
            self.lights
                .iter()
                .fold(Vector3::zero(), |total, light| total + light.ambient()),
        );
        self.ambient_light.update(&self.gpu_state.queue);

        for light in self.lights.iter_mut() {
            light.update(&self.gpu_state.queue);
        }
        for model in self.models.iter_mut() {
            model.update(&self.gpu_state.queue);
        }
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

        // Render solid passes
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ambient Render Pass"),
                color_attachments: &[
                    // this is output [[location(0)]]
                    wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                g: 0.1,
                                r: 0.1,
                                b: 0.1,
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

            // Render ambient pass
            for model in self.models.iter() {
                model::draw_model(
                    &mut render_pass,
                    &self.gpu_state.pipeline_vendor,
                    model,
                    &self.camera_controller,
                    &self.ambient_light,
                    &render_pipeline::Pass::Ambient,
                );
            }

            // Render lit passes (skipping ambient since they're rolled into self.ambient_light)
            for light in self
                .lights
                .iter()
                .filter(|l| l.light_type() != light::LightType::Ambient)
            {
                for model in self.models.iter() {
                    model::draw_model(
                        &mut render_pass,
                        &self.gpu_state.pipeline_vendor,
                        model,
                        &self.camera_controller,
                        light,
                        &render_pipeline::Pass::Lit,
                    );
                }
            }
        }

        self.gpu_state
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
