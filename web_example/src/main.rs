use actuate::{
    web::{div, text, Div},
    View,
};

#[derive(Clone)]
struct App;

impl View for App {
    fn body(&self, cx: &actuate::Scope) -> impl View {
        div(text("Hello World!"))
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
