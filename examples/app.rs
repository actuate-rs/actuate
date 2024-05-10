use actuate::View;

struct App;

impl View for App {
    fn body(&self) -> impl View {
        dbg!("Wat!");
    }
}

fn main() {
    actuate::run(App);
}