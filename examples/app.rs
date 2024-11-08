use actuate::{Compose, Composer, Scope};

struct A<'a> {
    name: &'a str,
}

impl Compose for A<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!(cx.me.name);
    }
}

struct App {
    name: String,
}

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!("App");

        A { name: &cx.me.name }
    }
}

fn main() {
    Composer::new(App {
        name: String::from("Hello, World!"),
    })
    .compose();
}
