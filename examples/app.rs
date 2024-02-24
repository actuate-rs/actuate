use actuate::Diagram;

struct X(i32);

struct Y(i32);

fn a(X(x): &X, Y(y): &mut Y) {
    println!("A: {x}");
    *y += 1;
}

fn b(Y(y): &Y) {
    println!("B: {y}")
}

fn c(X(x): &X, Y(y): &Y) {
    println!("C: {x} {y}")
}

fn main() {
    let mut diagram = Diagram::builder()
        .add_input(X(0))
        .add_state(Y(0))
        .add_system(a)
        .add_system(b)
        .add_system(c)
        .build();
    diagram.run();
    diagram.run();
}
