use actuate::{use_state, web::Div, Scope, View, VirtualDom};

struct Counter {
    start: i32,
}

impl View for Counter {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || self.start);

        //set_count.set(count + 1);

        tracing::info!("{}", count);

        Div::new()
    }
}

struct App;

impl View for App {
    fn body(&self, _cx: &Scope) -> impl View {
        (
            Counter { start: 0 },
            Counter { start: 100 },
            Counter { start: 200 },
        )
    }
}

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    actuate::run(App)
}
