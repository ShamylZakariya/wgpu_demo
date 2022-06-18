use lib::scene;

#[allow(dead_code)]
mod lib;

fn main() {
    pollster::block_on(lib::app::run(|window, gpu_state| {
        scene::Scene::new(window, gpu_state)
    }));
}
