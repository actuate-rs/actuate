use actuate::{use_state, View, VirtualDom};

#[derive(Clone)]
struct A;

impl View for A {
    fn view(&self) -> impl View {
        dbg!("A");
    }
}

#[derive(Clone)]
struct B;

impl View for B {
    fn view(&self) -> impl View {
        dbg!("B");
    }
}

struct App;

impl View for App {
    fn view(&self) -> impl View {
        let (count, set_count) = use_state(|| 0);
        dbg!(count);

        set_count.set(count + 1);

        (A, B)
    }
}

#[tokio::main]
async fn main() {
    let mut vdom = VirtualDom::new(App);
    vdom.run().await;
    vdom.run().await;

    dbg!(vdom.slice(0));
}
