use actuate::View;

struct A;

impl View for A {
    fn body(&self) -> impl View {
        dbg!("A!");
    }
}

struct App;

impl View for App {
    fn body(&self) -> impl View {
        (A, A)
    }
}

#[tokio::main]
async fn main() {
    actuate::run(App).await;
}
