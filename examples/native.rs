use actuate::{
    native::{Button, Flex, Text},
    use_mut, Compose, Scope,
};
use actuate_macros::Data;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        if *count == 0 {
            count.update(|x| *x += 1);
        }

        Flex::column((
            Text::new(format!("High five count: {}", *count)),
            Button::new("Up high!").on_press(move || {
                count.update(|x| *x += 1);
            }),
            Button::new("Down low!").on_press(move || {
                count.update(|x| *x -= 1);
            }),
        ))
    }
}

fn main() {
    actuate::native::run(App);
}
