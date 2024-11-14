use actuate::{prelude::*, Canvas};
use taffy::{Size, Style};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::FmtSubscriber;
use vello::{
    kurbo::{Affine, Circle},
    peniko::{Color, Fill},
};

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Window {
            attributes: WindowAttributes::default(),
            content: (
                Canvas::new(
                    Style {
                        size: Size::from_lengths(100., 100.),
                        ..Default::default()
                    },
                    |scene| {
                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            Color::RED,
                            None,
                            &Circle::new((50.0, 50.0), 50.0),
                        );
                    },
                ),
                Canvas::new(
                    Style {
                        size: Size::from_lengths(100., 100.),
                        ..Default::default()
                    },
                    |scene| {
                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            Color::BLUE,
                            None,
                            &Circle::new((50.0, 50.0), 50.0),
                        );
                    },
                ),
                Canvas::new(
                    Style {
                        size: Size::from_lengths(100., 100.),
                        ..Default::default()
                    },
                    |scene| {
                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            Color::YELLOW,
                            None,
                            &Circle::new((50.0, 50.0), 50.0),
                        );
                    },
                ),
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
