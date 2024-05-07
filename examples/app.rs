use actuate::{use_state, view, View, VirtualDom};

fn counter() -> impl View {
    view::from_fn(|cx| {
        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        dbg!(count);
    })
}

fn app() -> impl View {
    view::from_fn(|_| {
        dbg!("app");
        (counter(), counter())
    })
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
