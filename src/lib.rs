use std::{marker::PhantomData, mem};

use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        schedule::{IntoSystemConfigs, NodeConfigs, Schedule},
        system::{Commands, Local, ParamSet, System, SystemId, SystemParam, SystemParamFunction},
        world::World,
    },
    DefaultPlugins,
};

#[derive(Component)]
struct PendingScope {
    entity: Entity,
    effects: Vec<NodeConfigs<Box<dyn System<In = (), Out = ()>>>>,
}

fn run_effects(world: &mut World) {
    for (entity, scope) in world
        .query::<(Entity, &mut PendingScope)>()
        .iter_mut(world)
        .map(|(entity, mut scope)| {
            (
                entity,
                PendingScope {
                    entity: scope.entity,
                    effects: mem::take(&mut scope.effects),
                },
            )
        })
        .collect::<Vec<_>>()
    {
        let mut scope_handle = world.get_mut::<ScopeId>(scope.entity).unwrap();
        if let Some(mut schedule) = scope_handle.schedule.take() {
            schedule.run(world);

            let mut scope_handle = world.get_mut::<ScopeId>(scope.entity).unwrap();
            scope_handle.schedule = Some(schedule);
        } else {
            let mut schedule = Schedule::new(Update);
            for effect in scope.effects {
                schedule.add_systems(effect);
            }
            schedule.run(world);

            let mut scope_handle = world.get_mut::<ScopeId>(scope.entity).unwrap();
            scope_handle.schedule = Some(schedule);
        }

        world.despawn(entity);
    }
}

fn run_lazy(world: &mut World) {
    for (entity, mut system) in world
        .query::<(Entity, &mut LazySystem)>()
        .iter_mut(world)
        .map(|(entity, mut system)| (entity, system.add_system.take().unwrap()))
        .collect::<Vec<_>>()
    {
        let mut schedule = Schedule::new(Update);
        system(&mut schedule);
        schedule.run(world);

        world.despawn(entity);
    }
}

pub struct ActuatePlugin;

impl Plugin for ActuatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, ((run_effects, run_lazy).chain()));
    }
}

struct Effect {
    system: Option<NodeConfigs<Box<dyn System<In = (), Out = ()>>>>,
}

#[derive(Component)]
struct ScopeId {
    schedule: Option<Schedule>,
}

#[derive(Default)]
struct State {
    effects: Vec<Effect>,
    index: usize,
    entity: Option<Entity>,
}

#[derive(SystemParam)]
pub struct Scope<'w, 's> {
    commands: Commands<'w, 's>,
    state: Local<'s, State>,
}

impl Scope<'_, '_> {
    pub fn use_effect<Marker>(&mut self, system: impl IntoSystemConfigs<Marker>) {
        let idx = self.state.index;

        if let Some(effect) = self.state.effects.get_mut(idx) {
            effect.system = Some(system.into_configs());
        } else {
            self.state.effects.push(Effect {
                system: Some(system.into_configs()),
            })
        }

        self.state.index += 1;
    }
}

impl Drop for Scope<'_, '_> {
    fn drop(&mut self) {
        self.state.index = 0;

        let effects: Vec<_> = self
            .state
            .effects
            .iter_mut()
            .filter_map(|effect| effect.system.take())
            .collect();

        let entity = *self
            .state
            .entity
            .get_or_insert_with(|| self.commands.spawn(ScopeId { schedule: None }).id());

        self.commands.spawn(PendingScope { effects, entity });
    }
}

pub trait View {
    type State: Send + 'static;

    fn build(&mut self, commands: &mut Commands) -> Self::State;

    fn rebuild(&mut self, state: &mut Self::State, commands: &mut Commands);
}

impl View for () {
    type State = ();

    fn build(&mut self, commands: &mut Commands) -> Self::State {}

    fn rebuild(&mut self, state: &mut Self::State, commands: &mut Commands) {}
}

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
    add_system: Option<Box<dyn FnMut(&mut Schedule) + Send + Sync>>,
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
                schedule.add_systems(move |mut params: ParamSet<(F::Param,)>| {
                    f.run((), params.p0());
                });
            })),
        });
    }

    fn rebuild(&mut self, state: &mut Self::State, commands: &mut Commands) {}
}

pub fn run<F, V, Marker>(mut view_fn: F)
where
    F: SystemParamFunction<Marker, In = (), Out = V>,
    F::Param: 'static,
    V: View,
{
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(
            Update,
            move |mut commands: Commands,
                  mut state_cell: Local<Option<V::State>>,
                  mut params: ParamSet<(F::Param,)>| {
                let mut content = view_fn.run((), params.p0());

                if let Some(state) = &mut *state_cell {
                    content.rebuild(state, &mut commands);
                } else {
                    let state = content.build(&mut commands);
                    *state_cell = Some(state);
                }
            },
        )
        .run();
}
