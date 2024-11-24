use actuate::prelude::*;

#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        Window::new((
            Text::new(format!("High five count: {}", *count))
                .font(GenericFamily::Cursive)
                .font_size(60.),
            Text::new("Up high")
                .on_click(move || Mut::update(count, |x| *x += 1))
                .background_color(Color::BLUE)
                .border_radius(8.),
            Text::new("Down low")
                .on_click(move || Mut::update(count, |x| *x -= 1))
                .background_color(Color::RED)
                .border_radius(8.),
            if *count == 0 {
                Some(Text::new("Gimme five!"))
            } else {
                None
            },
        ))
        .font_size(40.)
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

    actuate::run(Counter { start: 0 })
}
