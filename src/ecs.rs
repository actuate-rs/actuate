use crate::{
    composer::{Composer, Update, Updater},
    prelude::*,
    use_callback,
};
use bevy::{
    app::Plugin,
    ecs::{
        component::{ComponentHooks, StorageType},
        system::{SystemParam, SystemParamItem, SystemState},
        world::CommandQueue,
    },
    prelude::{
        App, BuildChildren, Bundle, Command, Component, Entity, EntityWorldMut, Event, In,
        ParamSet, Trigger, World,
    },
    utils::HashMap,
};
use slotmap::{DefaultKey, SlotMap};
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    mem, ptr,
    rc::Rc,
    sync::{mpsc, Arc},
};
use tokio::sync::RwLockWriteGuard;

/// Actuate plugin to run [`Composition`]s.
pub struct ActuatePlugin;

impl Plugin for ActuatePlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel();
        let rt = Runtime {
            composers: RefCell::new(HashMap::new()),
            lock: None,
            tx,
            rx,
        };

        app.insert_non_send_resource(rt)
            .add_systems(bevy::prelude::Update, compose);
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

struct RuntimeUpdater {
    queue: mpsc::Sender<Update>,
}

impl Updater for RuntimeUpdater {
    fn update(&self, update: Update) {
        self.queue.send(update).unwrap();
    }
}

unsafe impl Send for RuntimeUpdater {}

unsafe impl Sync for RuntimeUpdater {}

struct RuntimeComposer {
    composer: Composer,
    guard: Option<RwLockWriteGuard<'static, ()>>,
}

struct Runtime {
    composers: RefCell<HashMap<Entity, RuntimeComposer>>,
    lock: Option<RwLockWriteGuard<'static, ()>>,
    tx: mpsc::Sender<Update>,
    rx: mpsc::Receiver<Update>,
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

        Ref::map(cx.me(), |me| &me.content)
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

                let tx = world.non_send_resource::<Runtime>().tx.clone();

                let rt = world.non_send_resource_mut::<Runtime>();
                rt.composers.borrow_mut().insert(
                    entity,
                    RuntimeComposer {
                        composer: Composer::with_updater(
                            CompositionContent { content, target },
                            RuntimeUpdater { queue: tx },
                        ),
                        guard: None,
                    },
                );
            });
        });
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

    let rt = world.non_send_resource_mut::<Runtime>();
    let mut composers = rt.composers.borrow_mut();
    for rt_composer in composers.values_mut() {
        rt_composer.guard = None;
        rt_composer.composer.compose();
    }
    drop(composers);

    while let Ok(update) = rt.rx.try_recv() {
        unsafe { update.apply() }
    }

    {
        world.increment_change_tick();
        let rt_cx = RuntimeContext::current();
        let mut rt = rt_cx.inner.borrow_mut();
        for f in &mut rt.updates {
            f(world);
        }

        rt.updates.clear();

        rt.commands.borrow_mut().apply(world);
    }

    let rt = &mut *world.non_send_resource_mut::<Runtime>();
    let mut composers = rt.composers.borrow_mut();
    for rt_composer in composers.values_mut() {
        let guard = rt_composer.composer.lock();
        let guard: RwLockWriteGuard<'static, ()> = unsafe { mem::transmute(guard) };
        rt_composer.guard = Some(guard);
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
    };
}

impl_system_param_fn!();
impl_system_param_fn!(T1);
impl_system_param_fn!(T1, T2);
impl_system_param_fn!(T1, T2, T3);
impl_system_param_fn!(T1, T2, T3, T4);
impl_system_param_fn!(T1, T2, T3, T4, T5);
impl_system_param_fn!(T1, T2, T3, T4, T5, T6);
impl_system_param_fn!(T1, T2, T3, T4, T5, T6, T7);
impl_system_param_fn!(T1, T2, T3, T4, T5, T6, T7, T8);

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
        let query = system_state.get_mut(world);
        with_world.run((), query)
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

impl_system_param_fn_once!(T1);
impl_system_param_fn_once!(T1, T2);
impl_system_param_fn_once!(T1, T2, T3);
impl_system_param_fn_once!(T1, T2, T3, T4);
impl_system_param_fn_once!(T1, T2, T3, T4, T5);
impl_system_param_fn_once!(T1, T2, T3, T4, T5, T6);
impl_system_param_fn_once!(T1, T2, T3, T4, T5, T6, T7);
impl_system_param_fn_once!(T1, T2, T3, T4, T5, T6, T7, T8);

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
        with_world.run(item)
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
    spawn_with(bundle, ())
}

/// Create a [`Spawn`] composable that spawns the provided `bundle` when composed, with some content as its children.
///
/// On re-composition, the spawned entity is updated to the latest provided value.
pub fn spawn_with<'a, B, C>(bundle: B, content: C) -> Spawn<'a, C>
where
    B: Bundle + Clone,
    C: Compose,
{
    Spawn {
        spawn_fn: Arc::new(move |world, cell| {
            if let Some(entity) = cell {
                world.entity_mut(*entity).insert(bundle.clone());
            } else {
                *cell = Some(world.spawn(bundle.clone()).id())
            }
        }),
        content,
        target: None,
        observer_fns: Vec::new(),
    }
}

type ObserverFn<'a> = Box<dyn Fn(&mut EntityWorldMut) + 'a>;

/// Spawn composable with content.
///
/// See [`spawn`] and [`spawn_with`] for more information.
#[must_use = "Composables do nothing unless composed with `actuate::run` or returned from other composables"]
pub struct Spawn<'a, C> {
    spawn_fn: SpawnFn,
    content: C,
    target: Option<Entity>,
    observer_fns: Vec<ObserverFn<'a>>,
}

impl<'a, C> Spawn<'a, C> {
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

    /// Add an observer to the spawned entity.
    pub fn observe<F, E, B, Marker>(mut self, observer: F) -> Self
    where
        F: SystemParamFunction<Marker, In = Trigger<'static, E, B>, Out = ()> + Send + Sync + 'a,
        E: Event,
        B: Bundle,
    {
        let cell = Cell::new(Some(observer));
        self.observer_fns.push(Box::new(move |entity| {
            let mut observer = cell.take().unwrap();

            type SpawnObserveFn<'a, F, E, B, Marker> = Box<
                dyn FnMut(
                        Trigger<'_, E, B>,
                        ParamSet<'_, '_, (<F as SystemParamFunction<Marker>>::Param,)>,
                    ) + Send
                    + Sync
                    + 'a,
            >;

            let f: SpawnObserveFn<'a, F, E, B, Marker> = Box::new(move |trigger, mut params| {
                let trigger: Trigger<'static, E, B> = unsafe { mem::transmute(trigger) };
                observer.run(trigger, params.p0())
            });
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

        Ref::map(cx.me(), |me| &me.content)
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
