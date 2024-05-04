use actuate::{use_state, SetState, View, VirtualDom};

#[derive(Clone, PartialEq)]
struct A {
    count: i32,
    set_count: SetState<i32>,
}

impl View for A {
    fn view(&self) -> impl View {
        self.set_count.set(self.count + 1);

        dbg!(self.count);
    }
}

#[derive(Clone, PartialEq)]
struct App;

impl View for App {
    fn view(&self) -> impl View {
        dbg!("App");

        let (count, set_count) = use_state(|| 0);

        set_count.set(count + 1);

        A { count, set_count }
    }
}

#[tokio::main]
async fn main() {
    let mut vdom = VirtualDom::new(App);
    vdom.run().await;
    vdom.run().await;

    dbg!(vdom.slice(0));
}
