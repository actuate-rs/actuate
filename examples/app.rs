use actuate::{Compose, Data, Composer, Mut, Scope};

struct Button<'a> {
    count: Mut<'a, i32>,
}

unsafe impl Data for Button<'_> {}

impl Compose for Button<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.me.count.update(|x| *x += 1)
    }
}

struct Counter {
    initial: i32,
}

unsafe impl Data for Counter {}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = cx.use_mut(|| cx.me.initial);

        dbg!(*count);

        (Button { count }, Button { count })
    }
}

fn main() {
    let mut composer = Composer::new(Counter { initial: 0 });
    composer.compose();
    composer.recompose();
}
