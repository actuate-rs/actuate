use actuate::Element;

fn main() {
    let mut elem = Element::default();
    elem.insert(0);

    dbg!(elem.query::<&i32>());
}