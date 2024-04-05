use actuate::{Component, World};

struct X(i32);

impl Component for X {}

fn main() {
    let mut world = World::default();
    let x = world.spawn(X(0));
    let n = x.query::<&X>(&mut world);
    dbg!(n.0);
}
