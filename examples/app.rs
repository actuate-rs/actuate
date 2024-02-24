use actuate::{Builder, World};

fn a(x: &i32) {}
fn main() {
    let mut world = World::default();
    world.query::<&i32>();
    Builder::default().add_system(a);
}
