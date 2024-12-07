// Counter UI example.

use actuate::prelude::*;
use bevy::{prelude::*, winit::WinitSettings};

// Counter composable.
#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        (
            text::headline(format!("High five count: {}", count)),
            button(text::label("Up high")).on_click(move || SignalMut::update(count, |x| *x += 1)),
            button(text::label("Down low")).on_click(move || SignalMut::update(count, |x| *x -= 1)),
            if *count == 0 {
                Some(text::label("Gimme five!"))
            } else {
                None
            },
        )
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    // Spawn a composition with a `Counter`, adding it to the Actuate runtime.
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(10.),
            ..default()
        },
        Composition::new(Counter { start: 0 }),
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}
