use bevy::ecs::system::Commands;

mod lazy;
pub(crate) use lazy::LazySystem;
pub use lazy::{lazy, Lazy};

pub trait View {
    type State: Send + 'static;

    fn build(&mut self, commands: &mut Commands) -> Self::State;

    fn rebuild(&mut self, state: &mut Self::State, commands: &mut Commands);
}

impl View for () {
    type State = ();

    fn build(&mut self, _commands: &mut Commands) -> Self::State {}

    fn rebuild(&mut self, _state: &mut Self::State, _commands: &mut Commands) {}
}
