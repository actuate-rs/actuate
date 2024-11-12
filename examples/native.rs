use actuate::{Compose, Data, Scope};

struct App;

unsafe impl Data for App {}

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        dbg!("Hello, world!");
    }
}

fn main() {
    actuate::native::run(App);
}
