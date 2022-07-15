use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::lib::gpu_state;

use super::gpu_state::GpuState;
use super::scene::Scene;

pub trait AppState {
    fn resize(
        &mut self,
        gpu_state: &mut gpu_state::GpuState,
        new_size: winit::dpi::PhysicalSize<u32>,
    );
    fn size(&self) -> PhysicalSize<u32>;
    fn input(
        &mut self,
        event: Option<&winit::event::WindowEvent>,
        mouse_motion: Option<(f64, f64)>,
    ) -> bool;
    fn update(&mut self, gpu_state: &mut gpu_state::GpuState, dt: instant::Duration);
    fn render(&mut self, gpu_state: &mut gpu_state::GpuState) -> Result<(), wgpu::SurfaceError>;
}

pub async fn run<F, U>(factory: F, update: U)
where
    F: Fn(&winit::window::Window, &mut GpuState) -> Scene,
    U: 'static + Fn(&mut Scene),
{
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_title("Hello WGPU")
        .build(&event_loop)
        .unwrap();

    let mut gpu_state = gpu_state::GpuState::new(&window).await;
    let mut scene = factory(&window, &mut gpu_state);

    // start even loop
    let mut last_render_time = instant::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => {
                scene.input(None, Some(delta));
            }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            let now = instant::Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            update(&mut scene);
            scene.update( &mut gpu_state, dt);
            match scene.render(&mut gpu_state) {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => {
                    let size = gpu_state.size();
                    gpu_state.resize(size);
                    scene.resize(&mut gpu_state, size);
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
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        gpu_state.resize(**new_inner_size);
                        scene.resize(&mut gpu_state, **new_inner_size);
                    }
                    _ => {}
                }
            }
        _ => {}
    });
}
