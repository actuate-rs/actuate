// Inspector UI example.

use actuate::{inspector::Inspector, prelude::*};
use bevy::prelude::*;

#[derive(Data)]
struct Example;

impl Compose for Example {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Inspector {}
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands.spawn((Node::default(), Composition::new(Example)));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(Startup, setup)
        .run();
}
