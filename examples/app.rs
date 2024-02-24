use actuate::Builder;

struct X(i32);

struct Y(i32);

fn a(X(x): &X, Y(y): &mut Y) {
    dbg!(x);
    *y += 1;
}

fn b(Y(y): &Y) {
    dbg!(y);
}

fn main() {
    let diagram = Builder::default()
        .add_input(X(0))
        .add_system(a)
        .add_system(b)
        .build();
    dbg!(diagram);
}
