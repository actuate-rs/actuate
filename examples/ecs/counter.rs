// Counter UI example.

use actuate::prelude::*;

// Counter composable.
#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .content((
            spawn(Text::new(format!("High five count: {}", count))),
            spawn(Text::new("Up high")).observe(move |_trigger: In<Trigger<Pointer<Click>>>| {
                Mut::update(count, |x| *x += 1)
            }),
            spawn(Text::new("Down low")).observe(move |_trigger: In<Trigger<Pointer<Click>>>| {
                Mut::update(count, |x| *x -= 1)
            }),
            if *count == 0 {
                Some(spawn(Text::new("Gimme five!")))
            } else {
                None
            },
        ))
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    // Spawn a composition with a `Counter`, adding it to the Actuate runtime.
    commands.spawn((Node::default(), Composition::new(Counter { start: 0 })));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(Startup, setup)
        .run();
}
