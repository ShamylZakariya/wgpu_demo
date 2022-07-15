use std::collections::HashMap;

use cgmath::*;
use winit::event::{ElementState, KeyboardInput, MouseButton, WindowEvent};

use super::{app, camera, gpu_state, light, model, render_pipeline};

//////////////////////////////////////////////

pub struct Scene {
    size: winit::dpi::PhysicalSize<u32>,
    camera_controller: camera::CameraController,
    ambient_light: light::Light,
    pub lights: HashMap<usize, light::Light>,
    pub models: HashMap<usize, model::Model>,
    mouse_pressed: bool,
    time: instant::Duration,
}

impl Scene {
    pub fn new(
        gpu_state: &mut gpu_state::GpuState,
        camera: camera::Camera,
        lights: HashMap<usize, light::Light>,
        models: HashMap<usize, model::Model>,
    ) -> Self {
        let mut camera_controller = camera::CameraController::new(camera, 4.0, 0.4);
        camera_controller.resize(gpu_state.size());

        // create a pipeline (if needed) for each material
        for model in models.values() {
            model.prepare_pipelines(gpu_state);
        }

        // Create an ambient light which is the sum of all the ambient terms of the light sources provided
        let ambient_term = lights
            .values()
            .fold(Vector3::zero(), |total, light| total + light.ambient());

        let ambient_light = light::Light::new_ambient(
            &gpu_state.device,
            &light::AmbientLightDescriptor {
                ambient: ambient_term,
            },
        );

        Self {
            size: gpu_state.size(),
            camera_controller,
            ambient_light,
            lights,
            models,
            mouse_pressed: false,
            time: instant::Duration::default(),
        }
    }

    pub fn time(&self) -> instant::Duration {
        self.time
    }
}

impl app::AppState for Scene {
    fn resize(
        &mut self,
        _gpu_state: &mut gpu_state::GpuState,
        new_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.size = new_size;
        self.camera_controller.resize(new_size);
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
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

    fn update(&mut self, gpu_state: &mut gpu_state::GpuState, dt: instant::Duration) {
        self.camera_controller.update(&gpu_state.queue, dt);

        self.ambient_light.set_ambient(
            self.lights
                .values()
                .fold(Vector3::zero(), |total, light| total + light.ambient()),
        );
        self.ambient_light.update(&gpu_state.queue);

        for light in self.lights.values_mut() {
            light.update(&gpu_state.queue);
        }
        for model in self.models.values_mut() {
            model.update(&gpu_state.queue);
        }

        self.time += dt;
    }

    fn render(&mut self, gpu_state: &mut gpu_state::GpuState) -> Result<(), wgpu::SurfaceError> {
        let output = gpu_state.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            gpu_state
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
                    Some(wgpu::RenderPassColorAttachment {
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
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &gpu_state.depth_attachment.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            // Render ambient pass
            for model in self.models.values() {
                model::draw_model(
                    &mut render_pass,
                    &gpu_state.pipeline_vendor,
                    model,
                    &self.camera_controller,
                    &self.ambient_light,
                    &render_pipeline::Pass::Ambient,
                );
            }

            // Render lit passes (skipping ambient since they're rolled into self.ambient_light)
            for light in self
                .lights
                .values()
                .filter(|l| l.light_type() != light::LightType::Ambient)
            {
                for model in self.models.values() {
                    model::draw_model(
                        &mut render_pass,
                        &gpu_state.pipeline_vendor,
                        model,
                        &self.camera_controller,
                        light,
                        &render_pipeline::Pass::Lit,
                    );
                }
            }
        }

        gpu_state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
