use actuate::{Diagram, PidController};

struct State(f64);

struct TargetState(f64);

struct StatePidController(PidController);

fn pid_controller(
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
        .add_system(pid_controller)
        .add_system(debugger)
        .build();

    for _ in 0..100 {
        diagram.run();
    }
}
