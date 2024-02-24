use actuate::Builder;

fn a(_x: &i32) {}

fn main() {
    let diagram = Builder::default().add_input(0).add_system(a).build();
    dbg!(diagram);
}
