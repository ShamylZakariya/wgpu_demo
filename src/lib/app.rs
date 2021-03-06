use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::lib::gpu_state;

use super::scene::Scene;
use super::{compositor, gpu_state::GpuState};

pub async fn run<F, U>(factory: F, update: U)
where
    F: Fn(&winit::window::Window, &mut GpuState) -> Scene,
    U: 'static + Fn(&mut Scene),
{
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_title("WGPU Demo")
        .build(&event_loop)
        .unwrap();

    let mut gpu_state = gpu_state::GpuState::new(&window).await;
    let mut scene = factory(&window, &mut gpu_state);
    let mut compositor = compositor::Compositor::new(
        &mut gpu_state,
        &scene.camera.render_buffers,
        scene.environment_map.clone(),
    );

    // start even loop
    let mut last_render_time = instant::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => {
                if !scene.input(None, Some(delta)) {
                    compositor.input(None, Some(delta));
                }
            }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            let now = instant::Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            update(&mut scene);
            scene.update( &mut gpu_state, dt);

            compositor.update(&mut gpu_state, &scene.camera, dt);

            match gpu_state.surface.get_current_texture() {
                Ok(output) => {

                    let mut encoder =
                            gpu_state
                                .device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                });

                    scene.render(&mut gpu_state, &mut encoder);
                    compositor.render(&mut gpu_state, &scene.camera, &mut encoder, &output);

                    gpu_state.queue.submit(std::iter::once(encoder.finish()));
                    output.present();

                },
                Err(wgpu::SurfaceError::Lost) => {
                    let size = gpu_state.size();
                    gpu_state.resize(size);
                    scene.resize(&mut gpu_state, size);
                    compositor.resize(&mut gpu_state, &scene.camera.render_buffers, size);
                }
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() && !scene.input(Some(event), None) => {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        gpu_state.resize(*physical_size);
                        scene.resize(&mut gpu_state, *physical_size);
                        compositor.resize(&mut gpu_state, &scene.camera.render_buffers, *physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        gpu_state.resize(**new_inner_size);
                        scene.resize(&mut gpu_state, **new_inner_size);
                        compositor.resize(&mut gpu_state, &scene.camera.render_buffers, **new_inner_size);
                    }
                    _ => {}
                }
            }
        _ => {}
    });
}
