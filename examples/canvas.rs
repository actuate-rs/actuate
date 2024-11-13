use actuate::{prelude::*, Canvas};
use taffy::{Size, Style};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;
use vello::{
    kurbo::{self, Affine},
    peniko::{Color, Fill},
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
        }
    }
}

fn main() {
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish(),
    )
    .unwrap();

    actuate::run(App);
}
