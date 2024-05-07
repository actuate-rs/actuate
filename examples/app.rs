use actuate::{use_state, view, View, VirtualDom};

fn counter(initial: i32) -> impl View {
    view::from_fn(move |cx| {
        let (count, set_count) = use_state(cx, || initial);

        set_count.set(count + 1);

        dbg!(count);

        (*count == 0).then(|| {
            view::from_fn(|_| {
                dbg!("test!");
            })
        })
    })
}

fn app() -> impl View {
    (counter(0), counter(100))
}

#[tokio::main]
async fn main() {
    let mut vdom: VirtualDom<_, _, ()> = VirtualDom::new(app());

    tokio::spawn(async move {
        vdom.run().await;
        vdom.run().await;
    })
    .await
    .unwrap();
}
