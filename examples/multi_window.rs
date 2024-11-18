use actuate::prelude::*;

#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        (
            Text::new(format!("High five count: {}", *count))
                .font(GenericFamily::Cursive)
                .font_size(60.),
            Text::new("Up high!")
                .on_click(move || count.update(|x| *x += 1))
                .background_color(Color::BLUE),
            Text::new("Down low!")
                .on_click(move || count.update(|x| *x -= 1))
                .background_color(Color::RED),
        )
            .font_size(40.)
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        (
            Window::new(Counter { start: 0 }),
            Window::new(Counter { start: 50 }),
        )
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

    actuate::run(App)
}
