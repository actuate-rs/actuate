use actuate::{Compose, Composer, Scope};
use actuate_macros::Data;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!("app!");
    }
}

fn main() {
    let mut composer = Composer::new(App);
    composer.compose();
}
