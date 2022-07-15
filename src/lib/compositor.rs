use super::{app::AppState, gpu_state};

pub struct Compositor {
    size: winit::dpi::PhysicalSize<u32>,
    time: instant::Duration,
}

impl Compositor {
    pub fn new(gpu_state: &mut gpu_state::GpuState) -> Self {
        Self {
            size: gpu_state.size(),
            time: instant::Duration::default(),
        }
    }

    pub fn time(&self) -> instant::Duration {
        self.time
    }
}

impl AppState for Compositor {
    fn resize(
        &mut self,
        _gpu_state: &mut super::gpu_state::GpuState,
        new_size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.size = new_size;
    }

    fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    fn input(
        &mut self,
        _event: Option<&winit::event::WindowEvent>,
        _mouse_motion: Option<(f64, f64)>,
    ) -> bool {
        false
    }

    fn update(&mut self, _gpu_state: &mut super::gpu_state::GpuState, dt: instant::Duration) {
        self.time += dt;
    }

    fn render(
        &mut self,
        _gpu_state: &mut gpu_state::GpuState,
        _encoder: &mut wgpu::CommandEncoder,
        _output: &wgpu::SurfaceTexture,
    ) {
    }
}

// let view = output
//     .texture
//     .create_view(&wgpu::TextureViewDescriptor::default());
