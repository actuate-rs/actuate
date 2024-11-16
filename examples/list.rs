use actuate::prelude::*;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let items = use_mut(&cx, Vec::new);

        dbg!(items.len());

        Window::new((
            actuate::core::from_iter((*items).clone(), |label| Text::new(label)),
            Text::new("Push!").on_click(move || items.update(|items| items.push("item"))),
            Text::new("Pop!").on_click(move || {
                items.update(|items| {
                    items.pop();
                })
            }),
        ))
        .font_size(40.)
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    actuate::run(Counter { start: 0 })
}
