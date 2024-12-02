use crate::{
    composer::Composer,
    prelude::{Signal, *},
};
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::{Component, ComponentHooks, StorageType},
    entity::Entity,
    prelude::*,
    system::{SystemParam, SystemParamItem, SystemState},
    world::{CommandQueue, World},
};
use bevy_hierarchy::BuildChildren;
use bevy_utils::HashMap;
use slotmap::{DefaultKey, SlotMap};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem, ptr,
    rc::Rc,
    sync::{Arc, Mutex},
};
use tokio::sync::RwLockWriteGuard;

macro_rules! impl_trait_for_tuples {
    ($t:tt) => {
        $t!();
        $t!(T1);
        $t!(T1, T2);
        $t!(T1, T2, T3);
        $t!(T1, T2, T3, T4);
        $t!(T1, T2, T3, T4, T5);
        $t!(T1, T2, T3, T4, T5, T6);
        $t!(T1, T2, T3, T4, T5, T6, T7);
        $t!(T1, T2, T3, T4, T5, T6, T7, T8);
    };
}

/// Actuate plugin to run [`Composition`]s.
pub struct ActuatePlugin;

impl Plugin for ActuatePlugin {
    fn build(&self, app: &mut App) {
        let rt = Runtime {
            composers: RefCell::new(HashMap::new()),
            lock: None,
        };

        app.insert_non_send_resource(rt)
            .add_systems(bevy_app::prelude::Update, compose);
    }
}

type UpdateFn = Box<dyn FnMut(&mut World)>;

type WorldListenerFn = Rc<dyn Fn(&mut World)>;

struct Inner {
    world_ptr: *mut World,
    listeners: SlotMap<DefaultKey, WorldListenerFn>,
    updates: Vec<UpdateFn>,
    commands: Rc<RefCell<CommandQueue>>,
}

#[derive(Clone)]
struct RuntimeContext {
    inner: Rc<RefCell<Inner>>,
}

impl RuntimeContext {
    fn current() -> Self {
        RUNTIME_CONTEXT.with(|cell| {
            let cell_ref = cell.borrow();
            let Some(rt) = cell_ref.as_ref() else {
                panic!("Must be called from within a composable.")
            };
            rt.clone()
        })
    }

    unsafe fn world_mut(&self) -> &'static mut World {
        &mut *self.inner.borrow().world_ptr
    }
}

thread_local! {
    static RUNTIME_CONTEXT: RefCell<Option<RuntimeContext>> = const { RefCell::new(None) };
}

struct RuntimeComposer {
    composer: Composer,
}

struct Runtime {
    composers: RefCell<HashMap<Entity, RuntimeComposer>>,
    lock: Option<RwLockWriteGuard<'static, ()>>,
}

/// Composition of some composable content.
pub struct Composition<C> {
    content: Option<C>,
    target: Option<Entity>,
}

impl<C> Composition<C>
where
    C: Compose + Send + Sync + 'static,
{
    /// Create a new composition from its content.
    pub fn new(content: C) -> Self {
        Self {
            content: Some(content),
            target: None,
        }
    }

    /// Get the target entity to spawn the composition into.
    ///
    /// If `None`, this will use the composition's parent (if any).
    pub fn target(&self) -> Option<Entity> {
        self.target
    }

    /// Set the target entity to spawn the composition into.
    ///
    /// If `None`, this will use the composition's parent (if any).
    pub fn set_target(&mut self, target: Option<Entity>) {
        self.target = target;
    }

    /// Set the target entity to spawn the composition into.
    ///
    /// If `None`, this will use the composition's parent (if any).
    pub fn with_target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
    }
}

impl<C> Component for Composition<C>
where
    C: Compose + Send + Sync + 'static,
{
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, entity, _| {
            world.commands().queue(move |world: &mut World| {
                let mut composition = world.get_mut::<Composition<C>>(entity).unwrap();

                let content = composition.content.take().unwrap();
                let target = composition.target.unwrap_or(entity);

                let rt = world.non_send_resource_mut::<Runtime>();

                rt.composers.borrow_mut().insert(
                    entity,
                    RuntimeComposer {
                        composer: Composer::new(CompositionContent { content, target }),
                    },
                );
            });
        });
    }
}

#[derive(Data)]
struct CompositionContent<C> {
    content: C,
    target: Entity,
}

impl<C: Compose> Compose for CompositionContent<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || SpawnContext {
            parent_entity: cx.me().target,
        });

        Signal::map(cx.me(), |me| &me.content)
    }
}

fn compose(world: &mut World) {
    let mut rt = world.non_send_resource_mut::<Runtime>();
    rt.lock = None;

    RUNTIME_CONTEXT.with(|runtime_cx| {
        let mut cell = runtime_cx.borrow_mut();
        let runtime_cx = cell.get_or_insert_with(|| RuntimeContext {
            inner: Rc::new(RefCell::new(Inner {
                world_ptr: ptr::null_mut(),
                listeners: SlotMap::new(),
                updates: Vec::new(),
                commands: Rc::new(RefCell::new(CommandQueue::default())),
            })),
        });

        runtime_cx.inner.borrow_mut().world_ptr = world as *mut World;

        for f in runtime_cx.inner.borrow().listeners.values() {
            f(world)
        }
    });

    world.increment_change_tick();
    let rt_cx = RuntimeContext::current();
    let mut rt = rt_cx.inner.borrow_mut();

    for f in &mut rt.updates {
        f(world);
    }
    rt.updates.clear();

    rt.commands.borrow_mut().apply(world);
    drop(rt);

    let rt = &mut *world.non_send_resource_mut::<Runtime>();
    let mut composers = rt.composers.borrow_mut();
    for rt_composer in composers.values_mut() {
        // TODO handle composition error.
        let _ = rt_composer.composer.try_compose();
    }
}

/// Hook for [`use_world`].
pub struct UseWorld<'a> {
    _marker: PhantomData<ScopeState<'a>>,
}

/// A function that takes a [`SystemParam`] as input.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system",
    label = "invalid system"
)]
pub trait SystemParamFunction<Marker> {
    /// The input type to this system. See [`System::In`].
    type In;

    /// The return type of this system. See [`System::Out`].
    type Out;

    /// The [`SystemParam`].
    type Param: SystemParam + 'static;

    /// Run the function with the provided [`SystemParam`]'s item.
    fn run(&mut self, input: Self::In, param_value: SystemParamItem<Self::Param>) -> Self::Out;
}

#[doc(hidden)]
pub struct Wrap<T>(T);

macro_rules! impl_system_param_fn {
    ($($t:tt),*) => {
        impl<Out, Func, $($t: SystemParam + 'static),*> SystemParamFunction<Wrap<fn($($t,)*) -> Out>> for Func
        where
        for <'a> &'a mut Func:
                FnMut($($t),*) -> Out +
                FnMut($(SystemParamItem<$t>),*) -> Out, Out: 'static
        {
            type In = ();
            type Out = Out;
            type Param = ($($t,)*);

            #[inline]
            #[allow(non_snake_case)]
            fn run(&mut self, _input: (), param_value: SystemParamItem< ($($t,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Out, $($t,)*>(mut f: impl FnMut($($t,)*) -> Out, $($t: $t,)*)->Out{
                    f($($t,)*)
                }
                let ($($t,)*) = param_value;
                call_inner(self, $($t),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($t: SystemParam + 'static),*> SystemParamFunction<fn(In<Input>, $($t,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(In<Input>, $($t),*) -> Out +
                FnMut(In<Input>, $(SystemParamItem<$t>),*) -> Out, Out: 'static
        {
            type In = Input;
            type Out = Out;
            type Param = ($($t,)*);
            #[inline]
            fn run(&mut self, input: Input, param_value: SystemParamItem< ($($t,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<Input, Out, $($t,)*>(
                    mut f: impl FnMut(In<Input>, $($t,)*)->Out,
                    input: In<Input>,
                    $($t: $t,)*
                )->Out{
                    f(input, $($t,)*)
                }
                let ($($t,)*) = param_value;
                call_inner(self, In(input), $($t),*)
            }
        }

        #[allow(non_snake_case)]
        impl<E: Event, B: Bundle, Out, Func, $($t: SystemParam + 'static),*> SystemParamFunction<fn(Trigger<E, B>, $($t,)*) -> Out> for Func
        where
        for <'a> &'a mut Func:
                FnMut(Trigger<E, B>, $($t),*) -> Out +
                FnMut(Trigger<E, B>, $(SystemParamItem<$t>),*) -> Out, Out: 'static
        {
            type In = Trigger<'static, E, B>;
            type Out = Out;
            type Param = ($($t,)*);
            #[inline]
            fn run(&mut self, input: Trigger<E, B>, param_value: SystemParamItem< ($($t,)*)>) -> Out {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<E: Event, B: Bundle, Out, $($t,)*>(
                    mut f: impl FnMut(Trigger<E, B>, $($t,)*)->Out,
                    input: Trigger<E, B>,
                    $($t: $t,)*
                )->Out{
                    f(input, $($t,)*)
                }
                let ($($t,)*) = param_value;
                call_inner(self, input, $($t),*)
            }
        }
    };
}

impl_trait_for_tuples!(impl_system_param_fn);

/// Use a [`SystemParam`] from the ECS world.
///
/// `with_world` will be called on every frame with the latest query.
///
/// Change detection is implemented as a traditional system parameter.
pub fn use_world<'a, Marker, F>(cx: ScopeState<'a>, mut with_world: F)
where
    F: SystemParamFunction<Marker, In = (), Out = ()> + 'a,
{
    let system_state_cell = use_ref(cx, || RefCell::new(None));

    let f: Rc<dyn Fn(&'static mut World)> = use_callback(cx, move |world: &'static mut World| {
        let mut system_state_cell = system_state_cell.borrow_mut();
        let system_state =
            system_state_cell.get_or_insert_with(|| SystemState::<F::Param>::new(world));

        let params = system_state.get_mut(world);
        with_world.run((), params);

        system_state.apply(world);
    })
    .clone();

    let key = *use_ref(cx, || {
        let f: Rc<dyn Fn(&mut World)> = unsafe { mem::transmute(f) };

        RuntimeContext::current()
            .inner
            .borrow_mut()
            .listeners
            .insert(f)
    });

    use_drop(cx, move || {
        RuntimeContext::current()
            .inner
            .borrow_mut()
            .listeners
            .remove(key);
    });
}

/// A function that takes a [`SystemParam`] as input.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid system",
    label = "invalid system"
)]
pub trait SystemParamFunctionOnce<Marker> {
    /// The [`SystemParam`].
    type Param: SystemParam + 'static;

    /// The return type of this function.
    type Output: 'static;

    /// Run the function with the provided [`SystemParam`]'s item.
    fn run(self, param: <Self::Param as SystemParam>::Item<'_, '_>) -> Self::Output;
}

macro_rules! impl_system_param_fn_once {
    ($($t:tt),*) => {
        impl<$($t: SystemParam + 'static,)* R: 'static, F: FnOnce($($t),*) -> R + FnOnce($($t::Item<'_, '_>),*) -> R> SystemParamFunctionOnce<fn($($t),*)> for F {
            type Param = ($($t,)*);

            type Output = R;

            fn run(self, param: <Self::Param as SystemParam>::Item<'_, '_>) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($t,)*) = param;
                self($($t,)*)
            }
        }
    };
}

impl_trait_for_tuples!(impl_system_param_fn_once);

/// Use a [`SystemParam`] from the ECS world.
///
/// `with_world` will be called once during the first composition.
pub fn use_world_once<Marker, F>(cx: ScopeState, with_world: F) -> &F::Output
where
    F: SystemParamFunctionOnce<Marker>,
{
    use_ref(cx, || {
        let world = unsafe { RuntimeContext::current().world_mut() };
        let mut param = SystemState::<F::Param>::new(world);
        let item = param.get_mut(world);

        let output = with_world.run(item);
        param.apply(world);
        output
    })
}

/// Hook for [`use_commands`].
pub struct UseCommands {
    commands: Rc<RefCell<CommandQueue>>,
}

impl UseCommands {
    /// Push a [`Command`] to the command queue.
    pub fn push<C>(&mut self, command: C)
    where
        C: Command,
    {
        self.commands.borrow_mut().push(command);
    }
}

/// Use access to the current [`Command`] queue.
pub fn use_commands(cx: ScopeState) -> &UseCommands {
    use_ref(cx, || {
        let commands = RuntimeContext::current().inner.borrow().commands.clone();
        UseCommands { commands }
    })
}

struct SpawnContext {
    parent_entity: Entity,
}

/// Use a spawned bundle.
///
/// `make_bundle` is called once to create the bundle.
pub fn use_bundle<B: Bundle>(cx: ScopeState, make_bundle: impl FnOnce() -> B) -> Entity {
    use_bundle_inner(cx, |world, cell| {
        let bundle = make_bundle();
        if let Some(entity) = cell {
            world.entity_mut(*entity).insert(bundle);
        } else {
            *cell = Some(world.spawn(bundle).id());
        }
    })
}

type SpawnFn = Arc<dyn Fn(&mut World, &mut Option<Entity>)>;

/// Create a [`Spawn`] composable that spawns the provided `bundle` when composed.
///
/// On re-composition, the spawned entity is updated to the latest provided value.
pub fn spawn<'a, B>(bundle: B) -> Spawn<'a, ()>
where
    B: Bundle + Clone,
{
    Spawn {
        spawn_fn: Arc::new(move |world, cell| {
            if let Some(entity) = cell {
                world.entity_mut(*entity).insert(bundle.clone());
            } else {
                *cell = Some(world.spawn(bundle.clone()).id())
            }
        }),
        content: (),
        target: None,
        observer_fns: Vec::new(),
        observer_guard: Arc::new(Mutex::new(true)),
        on_add: Cell::new(None),
    }
}

type ObserverFn<'a> = Box<dyn Fn(&mut EntityWorldMut) + 'a>;

type OnAddFn<'a> = Box<dyn FnOnce(EntityWorldMut) + 'a>;

/// Spawn composable with content.
///
/// See [`spawn`] and [`spawn_with`] for more information.
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct Spawn<'a, C> {
    spawn_fn: SpawnFn,
    content: C,
    target: Option<Entity>,
    observer_fns: Vec<ObserverFn<'a>>,
    on_add: Cell<Option<OnAddFn<'a>>>,
    observer_guard: Arc<Mutex<bool>>,
}

impl<'a, C> Spawn<'a, C> {
    /// Set the target entity to spawn the composition into.
    ///
    /// If `None`, this will use the composition's parent (if any).
    pub fn target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the child content.
    pub fn content<C2>(self, content: C2) -> Spawn<'a, C2> {
        Spawn {
            spawn_fn: self.spawn_fn,
            content,
            target: self.target,
            observer_fns: self.observer_fns,
            on_add: self.on_add,
            observer_guard: Arc::new(Mutex::new(false)),
        }
    }

    /// Set a function to be called when this entity is spawned.
    pub fn on_spawn<F>(self, f: F) -> Self
    where
        F: FnOnce(EntityWorldMut) + 'a,
    {
        self.on_add.set(Some(Box::new(f)));
        self
    }

    /// Add an observer to the spawned entity.
    pub fn observe<F, E, B, Marker>(mut self, observer: F) -> Self
    where
        F: SystemParamFunction<Marker, In = Trigger<'static, E, B>, Out = ()> + Send + Sync + 'a,
        E: Event,
        B: Bundle,
    {
        let cell = Cell::new(Some(observer));
        let guard = self.observer_guard.clone();

        self.observer_fns.push(Box::new(move |entity| {
            let mut observer = cell.take().unwrap();
            let guard = guard.clone();

            type SpawnObserveFn<'a, F, E, B, Marker> = Box<
                dyn FnMut(
                        Trigger<'_, E, B>,
                        ParamSet<'_, '_, (<F as SystemParamFunction<Marker>>::Param,)>,
                    ) + Send
                    + Sync
                    + 'a,
            >;

            let f: SpawnObserveFn<'a, F, E, B, Marker> = Box::new(move |trigger, mut params| {
                let guard = guard.lock().unwrap();
                if !*guard {
                    panic!("Actuate observer called after its scope was dropped.")
                }

                // Safety: The event will be accessed under a shortened lifetime.
                let trigger: Trigger<'static, E, B> = unsafe { mem::transmute(trigger) };
                observer.run(trigger, params.p0())
            });

            // Safety: The observer will be disabled after this scope is dropped.
            let f: SpawnObserveFn<'static, F, E, B, Marker> = unsafe { mem::transmute(f) };

            entity.observe(f);
        }));
        self
    }
}

unsafe impl<C: Data> Data for Spawn<'_, C> {}

impl<C: Compose> Compose for Spawn<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let spawn_cx = use_context::<SpawnContext>(&cx);

        let is_initial = use_ref(&cx, || Cell::new(true));
        let entity = use_bundle_inner(&cx, |world, entity| {
            if let Some(target) = cx.me().target {
                *entity = Some(target);
            }

            (cx.me().spawn_fn)(world, entity);

            if is_initial.get() {
                let mut entity_mut = world.entity_mut(entity.unwrap());
                for f in &cx.me().observer_fns {
                    f(&mut entity_mut);
                }

                if let Some(f) = cx.me().on_add.take() {
                    f(entity_mut);
                }

                is_initial.set(false);
            }
        });

        use_provider(&cx, || {
            if cx.me().target.is_none() {
                if let Ok(parent_entity) = spawn_cx.map(|cx| cx.parent_entity) {
                    let world = unsafe { RuntimeContext::current().world_mut() };
                    world.entity_mut(parent_entity).add_child(entity);
                }
            }

            SpawnContext {
                parent_entity: entity,
            }
        });

        // Use the initial guard.
        let guard = use_ref(&cx, || cx.me().observer_guard.clone());
        use_drop(&cx, move || {
            *guard.lock().unwrap() = false;
        });

        Signal::map(cx.me(), |me| &me.content)
    }
}

fn use_bundle_inner(cx: ScopeState, spawn: impl FnOnce(&mut World, &mut Option<Entity>)) -> Entity {
    let mut f_cell = Some(spawn);
    let entity = *use_ref(cx, || {
        let world = unsafe { RuntimeContext::current().world_mut() };

        let mut cell = None;
        f_cell.take().unwrap()(world, &mut cell);
        cell.unwrap()
    });

    if let Some(f) = f_cell {
        let world = unsafe { RuntimeContext::current().world_mut() };
        f(world, &mut Some(entity));
    }

    use_drop(cx, move || {
        let world = unsafe { RuntimeContext::current().world_mut() };
        world.try_despawn(entity);
    });

    entity
}
