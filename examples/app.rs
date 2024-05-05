use actuate::{virtual_dom, View, ViewBuilder};

struct App;

impl View for App {
    fn body(&self) -> impl ViewBuilder {
        dbg!("view!");
    }
}

fn main() {
    let mut vdom = virtual_dom(App);
    vdom.build();
}
