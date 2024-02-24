use actuate::{Diagram, Gain, Pid};

struct X(f64);

impl AsMut<f64> for X {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

struct Y(f64);

impl AsRef<f64> for Y {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

fn a(X(x): &X) {
    println!("A: {x}");
}

fn main() {
    let mut diagram = Diagram::builder()
        .add_input(X(1.))
        .add_input(Y(5.))
        .add_system(a)
        .add_plugin(Gain::<X>::new(1.1))
        .add_plugin(Pid::<X, Y>::default())
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
