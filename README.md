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

A reactive diagram for robotics and control systems.
Actuate leverages Rust's type system to create an efficient diagram that connects your application's systems. It can then render that diagram to a mermaid graph.

This crate provides a library that
can run on embedded systems with `#![no_std]`.

## Example

```mermaid
graph TD
  Input[Input] --> |"app::TargetState"| A
  Input[Input] --> |"app::State"| A

  Input[Input] --> |"Time"| A
  Input[Input] --> |"Time"| C
  
  A["app::state_pid_controller"]
  A --> |"app::State"| B
  A --> |"app::State"| C
  B["app::debugger"]
  C["app::pendulum_output"]
  C --> |"app::State"| A
```

```rust
use actuate::{
    control::PidController,
    plant::PendulumPlant,
    time::{Time, TimePlugin},
    Diagram,
};

struct State(f64);

struct TargetState(f64);

#[derive(Default)]
struct StatePidController(PidController);

#[derive(Default)]
struct ExamplePendulumPlant(PendulumPlant);

fn state_pid_controller(
    State(state): &mut State,
    TargetState(target): &TargetState,
    Time(time): &Time,
    StatePidController(pid): &mut StatePidController,
) {
    pid.control(*time, state, target)
}

fn pendulum_plant(
    Time(time): &Time,
    State(state): &State,
    ExamplePendulumPlant(pendulum): &mut ExamplePendulumPlant,
) {
    pendulum.update(*time, *state)
}

fn debugger(State(state): &State) {
    dbg!(state);
}

fn main() {
    let diagram = Diagram::builder()
        .add_plugin(TimePlugin)
        .add_input(State(1.))
        .add_input(TargetState(5.))
        .add_state(StatePidController::default())
        .add_state(ExamplePendulumPlant::default())
        .add_system(state_pid_controller)
        .add_system(pendulum_plant)
        .add_system(debugger)
        .build();

    println!("{}", diagram.visualize());
}
```

## How it works
Actuate connects systems together by their inputs and outputs.
A system taking `&T` as a parameter will be linked to another system taking `&mut T`.

Output: `&mut T` -> Input: `&T`

## Installation
On a device with `std` support:
```
cargo add actuate
```

In a `#![no_std]` enviornment:
```
cargo add actuate --no-default-features
```

## Inspiration
This crate is inspired by [Drake](https://drake.mit.edu) and aims to provide a similar model of
connecting systems together to form a complete diagram of your program.
Similar to [Bevy](https://docs.rs/bevy/latest/bevy/), Actuate uses function parameter types to connect systems.
In contrast to the ECS pattern, however, this crate requires each type be unique per `Diagram`.
