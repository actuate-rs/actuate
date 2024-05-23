use actuate::{use_state, Scope, View};

struct App;

impl View for App {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || 0);

        dbg!(count);

        set_count.set(count + 1)

        
    }
}

#[tokio::main]
async fn main() {
    actuate::run(App).await
}
