use actuate::{lazy, Scope, View};
use bevy::prelude::*;

fn app(mut scope: Scope) -> impl View {
    scope.use_effect(
        (|| {
            dbg!("A");
        })
        .run_if(run_once()),
    );

    lazy(|| {
        lazy(|| {
            dbg!("B");
        })
    })
}

fn main() {
   actuate::run(app);
}
