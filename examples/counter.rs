use actuate::prelude::*;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        dbg!(*count);

        Window::new((
            Text::new(format!("High five count: {}", *count)),
            Text::new("Up high!").on_click(move || count.update(|x| *x += 1)),
            Text::new("Down low!").on_click(move || count.update(|x| *x -= 1)),
        ))
    }
}

fn main() {
    actuate::run(App);
}
