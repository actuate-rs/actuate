#[cfg(feature = "std")]
use actuate::{
    control::PidController,
    time::{Time, TimePlugin},
    Diagram,
};

#[cfg(feature = "std")]
struct State(f64);

#[cfg(feature = "std")]
struct TargetState(f64);

#[cfg(feature = "std")]
#[derive(Default)]
struct StatePidController(PidController);

#[cfg(feature = "std")]
fn state_pid_controller(
    State(state): &mut State,
    TargetState(target): &TargetState,
    Time(time): &Time,
    StatePidController(pid): &mut StatePidController,
) {
    pid.control(*time, state, target)
}

#[cfg(feature = "std")]
#[test]
fn main() {
    let mut diagram = Diagram::builder()
        .add_plugin(TimePlugin)
        .add_input(State(1.))
        .add_input(TargetState(5.))
        .add_state(StatePidController::default())
        .add_system(state_pid_controller)
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
