use actuate::{AnyCompose, Compose, Composer, Scope};

struct A<'a> {
    name: &'a str,
    child: Box<dyn AnyCompose + 'a>,
}

impl Compose for A<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!(cx.me.name);

        &cx.me.child
    }
}

struct B;

impl Compose for B {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!("B");
    }
}

struct App {
    name: String,
}

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        dbg!(&cx.me.name);

        let name_mut = cx.use_mut(|| String::from("bar"));

        name_mut.update(|name| name.push('a'));

        let name = cx.use_ref(|| (*name_mut).clone());

        A {
            name,
            child: Box::new(B),
        }
    }
}

fn main() {
    Composer::new(App {
        name: String::from("foo"),
    })
    .compose();
}
