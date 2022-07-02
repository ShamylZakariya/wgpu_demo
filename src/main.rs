use cgmath::*;
use lib::{camera, gpu_state::GpuState, light, model, resources, scene};

#[allow(dead_code)]
mod lib;

fn load_model<P>(
    obj_file: &str,
    mtl_file: Option<&str>,
    positions: &[P],
    gpu_state: &GpuState,
) -> model::Model
where
    P: Into<cgmath::Point3<f32>> + Copy,
{
    let instances: Vec<_> = positions
        .iter()
        .map(|p| {
            model::Instance::new(
                (*p).into(),
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            )
        })
        .collect();

    resources::load_model_sync(
        obj_file,
        mtl_file,
        &gpu_state.device,
        &gpu_state.queue,
        &instances,
        false,
    )
    .unwrap()
}

fn main() {
    env_logger::init();

    pollster::block_on(lib::app::run(|_window, gpu_state| {
        let mut positions = vec![];
        for x in 0..50 {
            for z in 0..50 {
                positions.push(((x as f32 * 2.5) as f32, 0_f32, (z as f32 * 2.5) as f32))
            }
        }

        let models = vec![
            load_model("cube.obj", Some("plain.mtl"), &positions, &gpu_state),
            // load_model(
            //     "cube.obj",
            //     Some("cobble.mtl"),
            //     &[Point3::new(70.5, 5.0, 62.5)],
            //     &gpu_state,
            // ),
            // load_model(
            //     "cube.obj",
            //     None,
            //     &[Point3::new(70.5, 5.0, 65.0)],
            //     &gpu_state,
            // ),
        ];

        let ambient_light = light::Light::new_ambient(
            &gpu_state.device,
            &light::AmbientLightDescriptor {
                ambient: (0.3, 0.3, 0.3).into(),
            },
        );

        let point_light = light::Light::new_point(
            &gpu_state.device,
            &light::PointLightDescriptor {
                position: (62.5, 4.0, 62.5).into(),
                ambient: (0.0, 0.0, 0.0).into(),
                color: (1.0, 0.0, 0.0).into(),
                constant_attenuation: 1_f32,
                linear_attenuation: 0_f32,
                exponential_attenuation: 0.05_f32,
            },
        );

        let directional_light = light::Light::new_directional(
            &gpu_state.device,
            &light::DirectionalLightDescriptor {
                direction: (1.0, 1.0, 1.0).into(),
                ambient: (0.0, 0.0, 0.0).into(),
                color: (0.0, 0.0, 1.0).into(),
                constant_attenuation: 2.0,
            },
        );

        let spot_light = light::Light::new_spot(
            &&gpu_state.device,
            &light::SpotLightDescriptor {
                position: (62.5, 4.0, 62.5).into(),
                direction: (1.0, -1.0, 0.0).into(),
                ambient: (0.0, 0.0, 0.0).into(),
                color: (0.0, 1.0, 0.0).into(),
                constant_attenuation: 1_f32,
                linear_attenuation: 0_f32,
                exponential_attenuation: 0_f32,
                spot_breadth: Deg(75_f32),
            },
        );

        let lights = vec![ambient_light, spot_light, directional_light, point_light];

        let camera = camera::Camera::new((60.0, 4.0, 60.0), Deg(180.), Deg(0.));

        scene::Scene::new(gpu_state, lights, camera, models)
    }));
}
