use actuate::World;

fn main() {
    let mut world = World::default();
    dbg!(world.spawn().insert(42i32).get_mut::<i32>());
}
