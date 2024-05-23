use actuate::{use_state, Scope, View};

#[derive(Clone, PartialEq)]
struct Child {
    count: i32,
}

impl View for Child {
    fn body(&self, _cx: &Scope) -> impl View {
        dbg!(self.count);
    }
}

struct App;

impl View for App {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        Child { count: *count }.memo()
    }
}

#[tokio::main]
async fn main() {
    actuate::run(App).await
}
