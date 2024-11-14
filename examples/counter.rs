use actuate::{prelude::*, View};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        tracing::info!("{}", *count);

        Window::new((
            Text::new(format!("High five count: {}", *count)),
            Text::new("Up high!").on_click(move || count.update(|x| *x += 1)),
            Text::new("Down low!").on_click(move || count.update(|x| *x -= 1)),
        ))
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::INFO)
            .finish(),
    )
    .unwrap();

    actuate::run(App);
}
