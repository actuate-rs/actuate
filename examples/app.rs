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
