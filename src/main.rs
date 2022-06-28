use cgmath::Rotation3;
use lib::{gpu_state::GpuState, model, resources, scene};

#[allow(dead_code)]
mod lib;

fn load_model<V>(
    obj_file: &str,
    mtl_file: Option<&str>,
    position: V,
    gpu_state: &GpuState,
) -> model::Model
where
    V: Into<cgmath::Vector3<f32>>,
{
    let instance = model::Instance::new(
        position,
        cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
    );

    resources::load_model_sync(
        obj_file,
        mtl_file,
        &gpu_state.device,
        &gpu_state.queue,
        &[instance],
        true,
    )
    .unwrap()
}

fn main() {
    env_logger::init();

    pollster::block_on(lib::app::run(|_window, gpu_state| {
        let models = vec![
            load_model("cube.obj", None, (0.0, 0.0, 0.0), &gpu_state),
            load_model("cube.obj", Some("cobble.mtl"), (2.5, 0.0, 0.0), &gpu_state),
        ];

        scene::Scene::new(gpu_state, models)
    }));
}
