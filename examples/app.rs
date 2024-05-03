use actuate::{use_state, View, VirtualDom};

struct A;

impl View for A {
    fn view(&self) -> impl View {
        dbg!("A");
    }
}

struct App;

impl View for App {
    fn view(&self) -> impl View {
        let count = use_state(|| 0);
        dbg!(count);

        A
    }
}

fn main() {
    let mut vdom = VirtualDom::new(App);
    vdom.run();

    dbg!(vdom.slice(0));
}
