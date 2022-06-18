use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub trait AppState {
    fn build(&mut self, window: &winit::window::Window);
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    fn size(&self) -> PhysicalSize<u32>;
    fn input(
        &mut self,
        event: Option<&winit::event::WindowEvent>,
        mouse_motion: Option<(f64, f64)>,
    ) -> bool;
    fn update(&mut self, dt: instant::Duration);
    fn render(&mut self) -> Result<(), wgpu::SurfaceError>;
}

pub async fn run(mut app: Box<dyn AppState>) {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_title("Hello WGPU")
        .build(&event_loop)
        .unwrap();

    todo!("Make this take a closure");
    app.build(&window);

    // start even loop
    let mut last_render_time = instant::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => {
                app.input(None, Some(delta));
            }
        Event::RedrawRequested(window_id) if window_id == window.id() => {
            let now = instant::Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            app.update(dt);
            match app.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => app.resize(app.size()),
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
            } if window_id == window.id() && !app.input(Some(event), None) => {
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
                        app.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        app.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        _ => {}
    });
}
