use actuate::{use_mut, Compose, Composer, Data, Mut, Scope};

#[derive(Data)]
struct Button<'a> {
    count: Mut<'a, i32>,
}

impl Compose for Button<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.me.count.update(|x| *x += 1)
    }
}

#[derive(Data)]
struct Counter {
    initial: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me.initial);

        dbg!(*count);

        (Button { count }, Button { count })
    }
}

fn main() {
    let mut composer = Composer::new(Counter { initial: 0 });
    composer.compose();
    composer.recompose();
}
