use actuate::{prelude::*, Text};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Window {
            attributes: WindowAttributes::default(),
            content: Text("Hello, World!"),
        }
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::ERROR)
            .finish(),
    )
    .unwrap();

    actuate::run(App);
}
