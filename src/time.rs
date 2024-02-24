#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time(pub u64);

#[cfg(feature = "std")]
mod time_std {
    use super::Time;
    use crate::{diagram, Plugin};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct TimePlugin;

    impl Plugin for TimePlugin {
        fn build(self, diagram: &mut diagram::Builder) {
            diagram.add_state(Time(0)).add_system(time_system);
        }
    }

    pub fn time_system(Time(time): &mut Time) {
        *time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as _;
    }
}

#[cfg(feature = "std")]
pub use self::time_std::{time_system, TimePlugin};
