use actuate::{ActuatePlugin, Scope};
use bevy::{
    app::{App, Update},
    ecs::schedule::{common_conditions::run_once, IntoSystemConfigs},
    DefaultPlugins,
};

fn app(mut scope: Scope) {
    scope.use_effect(
        (|| {
            dbg!("A");
        })
        .run_if(run_once()),
    );
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(Update, app)
        .run();
}
