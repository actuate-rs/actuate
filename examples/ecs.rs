use actuate::{Ref, World};

fn main() {
    let mut world = World::default();
    let entity = world.spawn().insert(42i32).id();

    dbg!(*world.query::<Ref<i32>>(entity));
}
