use actuate::prelude::*;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        Window::new((
            Text::new(format!("High five count: {}", *count))
                .font(GenericFamily::Cursive)
                .font_size(100.),
            (
                Text::new("Up high!")
                    .on_click(move || count.update(|x| *x += 1))
                    .background_color(Color::BLUE),
                Text::new("Down low!")
                    .on_click(move || count.update(|x| *x -= 1))
                    .background_color(Color::RED),
            )
                .font_size(50.),
        ))
    }
}

fn main() {
    actuate::run(App)
}
