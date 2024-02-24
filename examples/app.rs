use actuate::{Diagram, Gain};

struct X(f64);

impl AsMut<f64> for X {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}


fn a(X(x): &X) {
    println!("A: {x}");

}
 


fn main() {
    let mut diagram = Diagram::builder()
        .add_input(X(1.))
        .add_system(a)
        .add_plugin(Gain::<X>::new(2.))
        .build();
    dbg!(&diagram);
    diagram.run();
}
