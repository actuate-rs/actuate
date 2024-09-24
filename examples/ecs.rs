use actuate::{Mut, Query, Ref, World};

fn main() {
    let mut world = World::default();
    let entity = world.spawn().insert(42i32).id();

    world.add_system(move |query: Query<Ref<i32>>| {
        dbg!(*query.get(entity));
    });
    world.run();
    world.run();

    *world.query::<Mut<i32>>().get(entity) = 43;
    world.run();

    world.spawn().insert(42i32).id();
    world.run();
}
