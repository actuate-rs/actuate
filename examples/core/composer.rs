// Example using the core `Composer` struct.

use actuate::{composer::Composer, prelude::*};

#[derive(Data)]
struct A;

impl Compose for A {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        dbg!("A");
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        dbg!("App!");

        A
    }
}

fn main() {
    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::level_filters::LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    let mut composer = Composer::new(App);
    composer.compose();
    composer.compose();
    composer.compose();
}
