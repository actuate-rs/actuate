use actuate::prelude::*;
use actuate_core::use_task;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_task(&cx, || async {
            dbg!("Hello, world!");
        });

        Window::new(Text::new("Hello, world!"))
    }
}

fn main() {
    actuate::run(App)
}
