use actuate::prelude::*;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

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

    actuate::run(App)
}
