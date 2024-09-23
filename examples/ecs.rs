use actuate::{Query, Ref, World};

fn main() {
    let mut world = World::default();
    let entity = world.spawn().insert(42i32).id();

    let id = world.add_system(move |query: Query<Ref<i32>>| {
        dbg!(*query.get(entity));
    });
    world.run_system(id);
}
