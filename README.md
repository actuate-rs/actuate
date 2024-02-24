# Actuate
A reactive runtime for embedded systems

```rust
use actuate::{Diagram, Gain, Pid};

struct State(f64);

impl AsMut<f64> for State {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

struct TargetState(f64);

impl AsRef<f64> for TargetState {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

fn a(State(x): &State) {
    println!("A: {x}");
}

fn main() {
    let mut diagram = Diagram::builder()
        .add_input(State(1.))
        .add_input(TargetState(5.))
        .add_system(a)
        .add_plugin(Gain::<State>::new(1.1))
        .add_plugin(Pid::<State, TargetState>::default())
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
```