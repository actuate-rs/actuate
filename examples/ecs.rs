use actuate::{Component, ComponentsMut, Mut, Query, Ref, World};

struct A(i32);

impl Component for A {
    fn start(me: &mut ComponentsMut<Self>) {
        me.add_system(move |query: Query<Ref<A>>| {
            dbg!("HERE");
        });
    }
}

fn main() {
    let mut world = World::default();

    let mut entity = world.spawn();
    entity.insert(A(42));
    let id = entity.id();
    //entity.component_mut::<A>().remove();

    world.run();
    world.run();

    world.query::<Mut<A>>().get(id).0 = 43;
    world.run();
}
