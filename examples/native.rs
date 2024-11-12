use actuate::{native::Text, use_mut, Compose, Data, Scope};

struct App;

unsafe impl Data for App {}

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let x = use_mut(&cx, || 0);

        dbg!(*x);

        if *x == 0 {
            x.update(|x| *x += 1);
        }

        Text(format!("{}", *x))
    }
}

fn main() {
    actuate::native::run(App);
}
