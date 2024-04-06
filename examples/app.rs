use actuate::{Element, ElementHandle, Query, World};

fn task(element: ElementHandle) -> impl FnMut(Query<&i32>) {
    move |query| {
        dbg!(query.get(element));
    }
}

fn main() {
    let mut elem = Element::default();
    elem.insert(0);

    let mut world = World::default();
    let a = world.add_element(elem);

    world.spawn(task(a));
}
