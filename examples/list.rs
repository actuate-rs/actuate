use actuate::prelude::*;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

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
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    actuate::run(List)
}
