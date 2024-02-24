use actuate::{control::PidController, Diagram, Time, TimePlugin};

struct State(f64);

struct TargetState(f64);

#[derive(Default)]
struct StatePidController(PidController);

fn state_pid_controller(
    StatePidController(pid): &mut StatePidController,
    State(state): &mut State,
    Time(time): &Time,
    TargetState(target): &TargetState,
) {
    pid.control(*time, state, target)
}

fn debugger(State(state): &State) {
    dbg!(state);
}

fn main() {
    let mut diagram = Diagram::builder()
        .add_plugin(TimePlugin)
        .add_input(State(1.))
        .add_input(TargetState(5.))
        .add_state(StatePidController::default())
        .add_system(state_pid_controller)
        .add_system(debugger)
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
