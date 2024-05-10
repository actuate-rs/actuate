use actuate::{use_state, Scope, View, VirtualDom};

struct Counter {
    start: i32,
}

impl View for Counter {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || self.start);

        set_count.set(count + 1);

        dbg!(count);
    }
}

struct App;

impl View for App {
    fn body(&self, _cx: &Scope) -> impl View {
        (Counter { start: 0 }, Counter { start: 100 })
    }
}

#[tokio::main]
async fn main() {
    let mut vdom = VirtualDom::new(App.into_node());

    tokio::spawn(async move {
        vdom.run().await;
        vdom.run().await;
    })
    .await
    .unwrap();
}
