use actuate::{
    clone, use_state,
    web::{div, text},
    Scope, View,
};

struct Counter {
    initial: i32,
}

impl View for Counter {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || self.initial);

        (
            text(format!("High five count: {}", count)),
            div(text("Up high!")).on_click({
                clone!(count, set_count);
                move || set_count.set(count + 1)
            }),
            div(text("Down low!")).on_click({
                clone!(count);
                move || set_count.set(count - 1)
            }),
        )
    }
}

#[derive(Clone)]
struct App;

impl View for App {
    fn body(&self, _cx: &Scope) -> impl View {
        (Counter { initial: 0 }, Counter { initial: 100 })
    }
}

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    actuate::mount(
        App,
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .into(),
    )
}
