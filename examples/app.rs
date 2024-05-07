use actuate::{use_context, use_provider, view, View, VirtualDom};

fn counter(initial: i32) -> impl View {
    view::from_fn(move |cx| {
        let count = use_context::<i32>(cx).unwrap();
        dbg!(count);
    })
}

fn app() -> impl View {
    view::from_fn(move |cx| {
        use_provider(cx, || 0);

        (counter(0), counter(100))
    })
}

#[tokio::main]
async fn main() {
    let mut vdom: VirtualDom<_, _, ()> = VirtualDom::new(app());

    tokio::spawn(async move {
        vdom.run().await;
    })
    .await
    .unwrap();
}
