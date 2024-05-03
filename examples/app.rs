use actuate::{use_state, View};

struct App;

impl View for App {
    fn view(&self) -> impl View {
        let count = use_state(|| 0);
        dbg!(count);
    }
}

fn main() {
    actuate::run(App);
}
