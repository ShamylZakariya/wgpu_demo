use cgmath::Rotation3;
use lib::{model, resources, scene};

#[allow(dead_code)]
mod lib;

fn main() {
    pollster::block_on(lib::app::run(|_window, gpu_state| {
        // TODO: We shouldn't have to create a material bind group layout to load an obj model
        // Perhaps it would be smarter to have load_model_sync return a Model with a configured
        // bind_group_layout and bind_group

        let material_bind_group_layout = model::Material::bind_group_layout(
            &gpu_state.device,
            "Model Material Bind Group Layout",
        );

        let instance = model::Instance::new(
            (0.0, 0.0, 0.0),
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
        );

        let obj_model = resources::load_model_sync(
            "cube.obj",
            &gpu_state.device,
            &gpu_state.queue,
            &material_bind_group_layout,
            &[instance],
        )
        .unwrap();

        scene::Scene::new(gpu_state, obj_model)
    }));
}
