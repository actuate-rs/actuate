use actuate::{use_state, Node, Scope, View, VirtualDom};

struct App;

impl View for App {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        dbg!(count);
    }
}

#[tokio::main]
async fn main() {
    let mut vdom: VirtualDom<_, _, ()> = VirtualDom::new(App.into_node());

    vdom.run().await;
    vdom.run().await;
}
