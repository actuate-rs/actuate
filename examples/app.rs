use actuate::{use_mut, Compose, Data, Memo, Mut, Ref, Scope};
use std::ops::Deref;

#[derive(Hash)]
struct Button<'a, T> {
    label: T,
    count: Mut<'a, i32>,
}

unsafe impl<T> Data for Button<'_, T> {}

impl<T> Compose for Button<'_, T>
where
    T: Deref<Target = str>,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!(&*cx.me().label);

        cx.me().count.update(|x| *x += 1)
    }
}

#[derive(Data)]
struct Counter {
    label: String,
    initial: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().initial);

        dbg!(*count);

        let label = Ref::map(cx.me(), |me| &*me.label);

        (Memo::new(Button { label, count }), Button { count, label })
    }
}

#[tokio::main]
async fn main() {
    actuate::run(Counter {
        initial: 0,
        label: String::from("foo"),
    })
    .await;
}
