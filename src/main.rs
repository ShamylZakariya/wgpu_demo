use cgmath::Rotation3;
use lib::{model, resources, scene};

#[allow(dead_code)]
mod lib;

fn main() {
    env_logger::init();

    pollster::block_on(lib::app::run(|_window, gpu_state| {
        let instance = model::Instance::new(
            (0.0, 0.0, 0.0),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        );

        let obj_model = resources::load_model_sync(
            "cube.obj",
            Some("cobble.mtl"),
            &gpu_state.device,
            &gpu_state.queue,
            &[instance],
        )
        .unwrap();

        scene::Scene::new(gpu_state, obj_model)
    }));
}
