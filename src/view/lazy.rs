use super::View;
use bevy::{
    app::Plugin,
    ecs::{
        component::Component,
        schedule::Schedule,
        system::{Commands, Local, ParamSet, System, SystemParamFunction},
    },
};
use std::marker::PhantomData;

pub fn lazy<F, V, Marker>(f: F) -> Lazy<F, V, Marker>
where
    F: SystemParamFunction<Marker, In = (), Out = V>,
    V: View,
{
    Lazy {
        f: Some(f),
        _marker: PhantomData,
    }
}

#[derive(Component)]
pub struct LazySystem {
    pub(crate) add_system: Option<Box<dyn FnMut(&mut Schedule) + Send + Sync>>,
}

pub struct Lazy<F, V, Marker> {
    f: Option<F>,
    _marker: PhantomData<(V, Marker)>,
}

impl<F, V, Marker> View for Lazy<F, V, Marker>
where
    F: SystemParamFunction<Marker, In = (), Out = V>,
    F::Param: 'static,
    V: View,
{
    type State = ();

    fn build(&mut self, commands: &mut Commands) -> Self::State {
        let mut f = self.f.take();
        commands.spawn(LazySystem {
            add_system: Some(Box::new(move |schedule: &mut Schedule| {
                let mut f = f.take().unwrap();
                schedule.add_systems(
                    move |mut commands: Commands,
                          mut params: ParamSet<(F::Param,)>,
                          mut state_cell: Local<Option<V::State>>| {
                        let mut content = f.run((), params.p0());

                        if let Some(state) = &mut *state_cell {
                            content.rebuild(state, &mut commands);
                        } else {
                            let state = content.build(&mut commands);
                            *state_cell = Some(state);
                        }
                    },
                );
            })),
        });
    }

    fn rebuild(&mut self, _state: &mut Self::State, _commands: &mut Commands) {}
}
