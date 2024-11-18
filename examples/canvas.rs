use actuate::prelude::*;
use taffy::{Size, Style};
use vello::{
    kurbo::{self, Affine},
    peniko::Fill,
};

#[derive(Data)]
struct Circle {
    color: Color,
}

impl Compose for Circle {
    fn compose(cx: Scope<Self>) -> impl Compose {
        Canvas::new(
            Style {
                size: Size::from_lengths(100., 100.),
                ..Default::default()
            },
            move |_layout, scene| {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    cx.me().color,
                    None,
                    &kurbo::Circle::new((50.0, 50.0), 50.0),
                );
            },
        )
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Window {
            attributes: WindowAttributes::default(),
            content: (
                Circle { color: Color::RED },
                Circle { color: Color::BLUE },
                Circle {
                    color: Color::YELLOW,
                },
            ),
            background_color: Color::BLACK,
        }
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

    actuate::run(App);
}
