#[allow(dead_code)]
mod lib;

fn main() {
    let scene = Box::new(lib::scene::Scene::new());
    pollster::block_on(lib::app::run(scene));
}
