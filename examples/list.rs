use actuate::prelude::*;

#[derive(Data)]
struct List;

impl Compose for List {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let items = use_mut(&cx, Vec::new);

        Window::new((
            Flex::row((
                Text::new("Push!").on_click(move || items.update(|items| items.push("item"))),
                Text::new("Pop!").on_click(move || {
                    items.update(|items| {
                        items.pop();
                    })
                }),
            ))
            .color(Color::WHITE)
            .background_color(Color::BLACK),
            compose::from_iter(items, |label| Text::new(label)),
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

    actuate::run(List)
}
