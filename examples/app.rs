use actuate::{lazy, Scope, View};
use bevy::prelude::*;

#[derive(Component)]
struct Count(i32);

fn app(mut scope: Scope) -> impl View {
    let count_entity = scope.use_bundle(|| Count(5));

    lazy(move |count_query: Query<&Count>| {
        if let Ok(count) = count_query.get(count_entity) {
            dbg!(count.0);
        }
    })
}

fn main() {
    actuate::run(app);
}
