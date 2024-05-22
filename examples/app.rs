use actuate::View;

struct App;

impl View for App {
    fn body(&self) -> impl View {
        dbg!("Hello world!");
    }
}

fn main() {
    actuate::run(App)
}
