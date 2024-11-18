use actuate::{composer::Composer, prelude::*};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct A;

impl Compose for A {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        info!("A")
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        info!("App!");

        A
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    let mut composer = Composer::new(App);
    composer.compose();
    composer.compose();
    composer.compose();
}
