// Example using the core `Composer` struct.

use actuate::{composer::Composer, prelude::*};

#[derive(Data)]
struct A;

impl Compose for A {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        dbg!("A");
    }
}

#[derive(Data)]
struct B;

impl Compose for B {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        dbg!("B");
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        SignalMut::update(count, |x| *x += 1);

        if *count == 0 {
            DynCompose::new(A)
        } else {
            DynCompose::new(B)
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

    let mut composer = Composer::new(App);
    for _ in 0..3 {
        composer.try_compose().unwrap().unwrap();
    }
}
