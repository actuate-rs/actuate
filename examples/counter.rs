use actuate::prelude::{Mut, *};
use bevy::prelude::*;

// Counter composable.
#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        spawn_with(
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
            (
                spawn(Text::new(format!("High five count: {}", count))),
                spawn(Text::new("Up high")).observe(
                    move |_trigger: In<Trigger<Pointer<Click>>>| Mut::update(count, |x| *x += 1),
                ),
                spawn(Text::new("Down low")).observe(
                    move |_trigger: In<Trigger<Pointer<Click>>>| Mut::update(count, |x| *x -= 1),
                ),
                if *count == 0 {
                    Some(spawn(Text::new("Gimme five!")))
                } else {
                    None
                },
            ),
        )
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    // Spawn a composition with a `Counter`, adding it to the Actuate runtime.
    commands.spawn((Node::default(), Composition::new(Counter { start: 0 })));
}
