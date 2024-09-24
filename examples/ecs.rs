use actuate::{Mut, Query, Ref, World};

fn main() {
    let mut world = World::default();
    let mut entity_ref = world.spawn();
    let entity = entity_ref.id();
    entity_ref
        .insert(42i32)
        .add_system(move |query: Query<Ref<i32>>| {
            dbg!(*query.get(entity));
        });
    entity_ref.component_mut::<i32>().remove();
    world.run();
    world.run();

    //*world.query::<Mut<i32>>().get(entity) = 43;
    //world.run();
}
