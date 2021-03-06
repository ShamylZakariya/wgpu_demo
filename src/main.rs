use std::{collections::HashMap, rc::Rc};

use cgmath::prelude::*;
use lib::{camera, gpu_state::GpuState, light, model, resources, scene, texture, util::*};

#[allow(dead_code)]
mod lib;

fn load_model<P>(
    obj_file: &str,
    mtl_file: Option<&str>,
    positions: &[P],
    gpu_state: &GpuState,
    environment_map: Rc<texture::Texture>,
) -> model::Model
where
    P: Into<Point3> + Copy,
{
    let instances: Vec<_> = positions
        .iter()
        .map(|p| model::Instance::new((*p).into(), Quat::from_axis_angle(Vec3::unit_z(), deg(0.0))))
        .collect();

    resources::load_model_sync(
        obj_file,
        mtl_file,
        &gpu_state.device,
        &gpu_state.queue,
        &instances,
        environment_map,
        false,
    )
    .unwrap()
}

const ID_LIGHT_AMBIENT: usize = 0;
const ID_LIGHT_PRIMARY: usize = 1;
const ID_LIGHT_POINT: usize = 2;
const ID_LIGHT_SPOT: usize = 3;

const ID_MODEL_CUBE_FLOOR: usize = 0;

fn main() {
    env_logger::init();

    pollster::block_on(lib::app::run(
        |_window, gpu_state| {
            let environment_map = Rc::new(
                resources::load_cubemap_texture_sync(
                    "env-map.dds",
                    &gpu_state.device,
                    &gpu_state.queue,
                )
                .unwrap(),
            );

            let mut positions = vec![];
            for x in 0..50 {
                for z in 0..50 {
                    positions.push(((x as f32 * 2.5) as f32, 0_f32, (z as f32 * 2.5) as f32))
                }
            }

            let models = HashMap::from([(
                ID_MODEL_CUBE_FLOOR,
                load_model(
                    "cube.obj",
                    Some("untextured.mtl"),
                    &positions,
                    gpu_state,
                    environment_map.clone(),
                ),
            )]);

            let ambient_light = light::Light::new_ambient(
                &gpu_state.device,
                &light::AmbientLightDescriptor {
                    ambient: [0.05; 3].into(),
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
                    direction: (1.0, 1.0, 0.0).into(),
                    ambient: (0.0, 0.0, 0.0).into(),
                    color: (0.0, 0.0, 1.0).into(),
                    constant_attenuation: 1.0,
                },
            );

            let spot_light = light::Light::new_spot(
                &gpu_state.device,
                &light::SpotLightDescriptor {
                    position: (62.5, 4.0, 62.5).into(),
                    direction: (1.0, -1.0, 0.0).into(),
                    ambient: (0.0, 0.0, 0.0).into(),
                    color: (0.0, 1.0, 0.0).into(),
                    constant_attenuation: 1_f32,
                    linear_attenuation: 0_f32,
                    exponential_attenuation: 0_f32,
                    spot_breadth: deg(75_f32),
                },
            );

            let lights = HashMap::from([
                (ID_LIGHT_AMBIENT, ambient_light),
                (ID_LIGHT_PRIMARY, directional_light),
                (ID_LIGHT_POINT, point_light),
                (ID_LIGHT_SPOT, spot_light),
            ]);

            let mut camera = camera::Camera::new(gpu_state, deg(45.0), 0.5, 500.0);
            camera.look_at((60.0, 4.0, 60.0), (62.5, 0.0, 62.5), (0.0, 1.0, 0.0));

            scene::Scene::new(gpu_state, camera, environment_map, lights, models)
        },
        |scene| {
            let seconds = scene.time().as_secs_f32();
            let cycle = (seconds).cos();

            if let Some(point_light) = scene.lights.get_mut(&ID_LIGHT_POINT) {
                let mut light_pos = point_light.position();
                light_pos.y = 4.0 + cycle * 3.0;

                point_light.set_position(light_pos);
            }
        },
    ));
}
