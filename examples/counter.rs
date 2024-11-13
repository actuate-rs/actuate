use actuate::prelude::*;

#[derive(Data)]
struct Counter;

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        Flex::column((
            Text::new(format!("High five count: {}", *count)),
            Button::new("Up high!").on_press(move || count.update(|x| *x += 1)),
            Button::new("Down low!").on_press(move || count.update(|x| *x -= 1)),
        ))
    }
}

fn main() {
    actuate::run(Counter);
}
