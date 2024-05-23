use actuate::{use_context, use_provider, use_state, Scope, View};

#[derive(Clone, PartialEq)]
struct Child {
    count: i32,
}

impl View for Child {
    fn body(&self, cx: &Scope) -> impl View {
        dbg!(use_context::<&'static str>(cx));

        dbg!(self.count);
    }
}

struct App;

impl View for App {
    fn body(&self, cx: &Scope) -> impl View {
        use_provider(cx, || "Hi!");

        let (count, set_count) = use_state(cx, || 0);

        set_count.set(count + 1);

        Child { count: *count }.memo()
    }
}

#[tokio::main]
async fn main() {
    actuate::run(App).await
}
