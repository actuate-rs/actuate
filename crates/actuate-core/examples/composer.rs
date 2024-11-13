use actuate_core::{prelude::*, Composer};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        info!("run!");
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
}
