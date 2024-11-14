use actuate::prelude::*;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        Window::new((
            Text::new(format!("High five count: {}", *count)),
            Text::new("Up high!"),
            Text::new("Down low!"),
        ))
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
