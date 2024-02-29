use actuate::Diagram;

struct X(i32);

struct Y(i32);

fn a(X(x): &X) -> Y {
    dbg!(x);
    Y(1)
}

fn b(Y(y): &Y) -> X {
    dbg!(y);
    X(2)
}

fn main() {
    let mut builder = Diagram::builder();
    let a = builder.add_system(a);
    let b = builder.add_system(b);
}
