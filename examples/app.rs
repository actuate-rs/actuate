use actuate::{use_state, virtual_dom, Scope, View, ViewBuilder};

struct App;

impl View for App {
    fn body(&self, cx: &Scope) -> impl ViewBuilder {
        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        dbg!(count);
    }
}

fn main() {
    let mut vdom = virtual_dom(App);
    vdom.run();
    vdom.run();
}
