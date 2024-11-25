use actuate::{
    executor::ExecutorContext,
    prelude::{Mut, Ref, *},
};
use serde::Deserialize;
use std::collections::HashMap;

// Dog breed composable.
#[derive(Data)]
struct Breed<'a> {
    name: &'a String,
    families: &'a Vec<String>,
}

impl Compose for Breed<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        spawn_with(
            Node {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            (
                spawn((
                    Text::new(cx.me().name),
                    Node {
                        width: Val::Px(300.0),
                        ..default()
                    },
                )),
                spawn_with(
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    compose::from_iter(Ref::map(cx.me(), |me| me.families), |family| {
                        spawn(Text::from(family.to_string()))
                    }),
                ),
            ),
        )
    }
}

#[derive(Deserialize)]
struct Response {
    message: HashMap<String, Vec<String>>,
}

// Dog breed list composable.
#[derive(Data)]
struct BreedList;

impl Compose for BreedList {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let breeds = use_mut(&cx, HashMap::new);

        // Spawn a task that loads dog breeds from an HTTP API.
        use_task(&cx, move || async move {
            let json: Response = reqwest::get("https://dog.ceo/api/breeds/list/all")
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            Mut::update(breeds, |breeds| *breeds = json.message);
        });

        // Render the currently loaded breeds.
        spawn_with(
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(30.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            compose::from_iter(breeds, |breed| Breed {
                name: breed.0,
                families: breed.1,
            }),
        )
    }
}

#[derive(Data)]
struct Example;

impl Compose for Example {
    fn compose(cx: Scope<Self>) -> impl Compose {
        // Setup the Tokio executor.
        use_provider(&cx, ExecutorContext::default);

        BreedList
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

    // Spawn a composition with a `BreedList`, adding it to the Actuate runtime.
    commands.spawn((Node::default(), Composition::new(Example)));
}
