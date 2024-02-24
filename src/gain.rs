use crate::Plugin;
use std::marker::PhantomData;

pub struct Gain<T> {
    kp: f64,
    _marker: PhantomData<T>,
}

impl<T> Gain<T> {
    pub fn new(kp: f64) -> Self {
        Self {
            kp,
            _marker: PhantomData,
        }
    }
}

impl<T> Plugin for Gain<T>
where
    T: AsMut<f64> + 'static,
{
    fn build(self, diagram: &mut crate::diagram::Builder) {
        diagram.add_state(self).add_system(gain::<T>);
    }
}

pub fn gain<T: AsMut<f64>>(value: &mut T, state: &Gain<T>) {
    *value.as_mut() *= state.kp;
}
