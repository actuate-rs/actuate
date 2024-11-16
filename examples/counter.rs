use actuate::prelude::*;
use actuate_core::use_drop;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct A;

impl Compose for A {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_drop(&cx, || {
            dbg!("Dropped");
        });
    }
}

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
            Text::new("Up high!")
                .on_click(move || count.update(|x| *x += 1))
                .background_color(Color::BLUE),
            Text::new("Down low!")
                .on_click(move || count.update(|x| *x -= 1))
                .background_color(Color::RED),
            if *count == 1 {
                Some(Text::new("A"))
            } else {
                None
            },
        ))
        .font_size(40.)
    }
}

fn main() {
    actuate::run(Counter { start: 0 })
}
