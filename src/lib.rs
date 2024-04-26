use std::mem;

use bevy::{
    app::{Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        schedule::{IntoSystemConfigs, NodeConfigs, Schedule},
        system::{Commands, Local, System, SystemId, SystemParam},
        world::World,
    },
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

pub struct ActuatePlugin;

impl Plugin for ActuatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, run_effects);
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
