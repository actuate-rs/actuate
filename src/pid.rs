use crate::Plugin;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

pub struct Pid<State, TargetState> {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub total_error: f64,
    pub last_error: f64,
    pub last_instant: Option<Instant>,
    _marker: PhantomData<(State, TargetState)>,
}

impl<T, U> Default for Pid<T, U> {
    fn default() -> Self {
        Self {
            kp: 0.5,
            ki: 0.1,
            kd: 0.2,
            total_error: 0.,
            last_error: 0.,
            last_instant: None,
            _marker: PhantomData,
        }
    }
}

impl<T, U> Plugin for Pid<T, U>
where
    T: AsMut<f64> + 'static,
    U: AsRef<f64> + 'static,
{
    fn build(self, diagram: &mut crate::diagram::Builder) {
        diagram.add_state(self).add_system(pid::<T, U>);
    }
}

pub fn pid<T, U>(value: &mut T, target: &U, state: &mut Pid<T, U>)
where
    T: AsMut<f64>,
    U: AsRef<f64>,
{
    let now = Instant::now();
    let elapsed = match state.last_instant {
        Some(last_time) => now.duration_since(last_time),
        None => Duration::from_millis(1),
    };
    let elapsed_ms = (elapsed.as_millis() as f64).max(1.0);

    let error = *target.as_ref() - *value.as_mut();
    let error_delta = (error - state.last_error) / elapsed_ms;
    state.total_error += error * elapsed_ms;
    state.last_error = error;
    state.last_instant = Some(now);

    let p = state.kp * error;
    let i = state.ki * state.total_error;
    let d = state.kd * error_delta;
    *value.as_mut() = p + i + d;
}
