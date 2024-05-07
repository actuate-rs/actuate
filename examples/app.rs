use actuate::{use_state, view, View};

fn counter() -> impl View {
    view::from_fn(|cx| {
        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        dbg!(count);
    })
}

fn app() -> impl View {
    (counter(), counter())
}

#[tokio::main]
async fn main() {
    tokio::spawn(async move {
        actuate::run(app()).await;
    })
    .await
    .unwrap();
}
