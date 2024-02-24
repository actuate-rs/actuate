<div align="center">
  <h1>Actuate</h1>

 <a href="https://crates.io/crates/actuate">
    <img src="https://img.shields.io/crates/v/actuate?style=flat-square"
    alt="Crates.io version" />
  </a>
  <a href="https://docs.rs/actuate">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
   <a href="https://github.com/actuate-rs/actuate/actions">
    <img src="https://github.com/actuate-rs/actuate/actions/workflows/ci.yml/badge.svg"
      alt="CI status" />
  </a>
</div>

<div align="center">
 <a href="https://github.com/actuate-rs/actuate/tree/main/examples">Examples</a>
</div>

<br />
A reactive runtime for embedded systems

```rust
use actuate::{Diagram, PidController};

struct State(f64);

struct TargetState(f64);

struct StatePidController(PidController);

fn state_pid_controller(
    StatePidController(pid): &mut StatePidController,
    State(state): &mut State,
    TargetState(target): &TargetState,
) {
    pid.control(state, target)
}

fn debugger(State(state): &State) {
    dbg!(state);
}

fn main() {
    let mut diagram = Diagram::builder()
        .add_input(State(1.))
        .add_input(TargetState(5.))
        .add_state(StatePidController(PidController::default()))
        .add_system(state_pid_controller)
        .add_system(debugger)
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
```