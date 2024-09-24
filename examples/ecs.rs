use actuate::{Component, ComponentMut, Mut, Query, Ref, World};

struct A(i32);

impl Component for A {
    fn start(me: &mut ComponentMut<Self>) {
        let entity = me.entity().id();
        me.add_system(move |query: Query<Ref<A>>| {
            dbg!(query.get(entity).0);
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
