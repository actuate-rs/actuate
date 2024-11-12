use actuate::{
    native::{Flex, Text},
    use_mut, Compose, Data, Scope,
};
use actuate_macros::Data;

#[derive(Data)]
struct App;

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || 0);

        if *count == 0 {
            //count.update(|x| *x += 1);
        }

        Flex((
            Text(format!("High five count: {}", *count)),
            Text("Up high!"),
            Text("Down low!"),
        ))
    }
}

fn main() {
    actuate::native::run(App);
}
