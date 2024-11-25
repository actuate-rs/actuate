// Example showing borrowed trait objects.

use actuate::prelude::*;

#[data]
trait MyTrait: Data {
    fn run(&self);
}

#[derive(Data)]
struct A<'a> {
    my_trait: Box<dyn MyTrait + 'a>,
}

impl Compose for A<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.me().my_trait.run();
    }
}

#[derive(Data)]
struct X;

impl MyTrait for X {
    fn run(&self) {
        dbg!("X");
    }
}

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        A {
            my_trait: Box::new(X),
        }
    }
}

fn main() {}
