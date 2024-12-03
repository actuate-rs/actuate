// Example using the core `Composer` struct.

use actuate::{composer::Composer, prelude::*};

#[derive(Data)]
struct A;

impl Compose for A {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        "".parse().map(|_: i32| ()).map_err(Error::new)
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        catch(
            |error| {
                dbg!(error);
            },
            A,
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

    let mut composer = Composer::new(App);
    composer.try_compose().unwrap();
}
