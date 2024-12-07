use crate::{
    compose::Compose,
    composer::{Composer, Pending},
    data::Data,
    use_callback, use_drop, use_provider, use_ref, Cow, Scope, ScopeState, Signal,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::{Component, ComponentHooks, StorageType},
    entity::Entity,
    prelude::*,
    system::{SystemParam, SystemParamItem, SystemState},
    world::{CommandQueue, World},
};
use bevy_utils::HashMap;
use bevy_winit::{EventLoopProxy, EventLoopProxyWrapper, WakeUp};
use core::fmt;
use slotmap::{DefaultKey, SlotMap};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    mem, ptr,
    rc::Rc,
    sync::Arc,
    task::{Context, Wake, Waker},
};

#[cfg(feature = "picking")]
use bevy_picking::prelude::*;

mod spawn;
pub use self::spawn::{spawn, Spawn};

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
#[actuate(path = "crate")]
struct CompositionContent<C> {
    content: C,
    target: Entity,
}

impl<C: Compose> Compose for CompositionContent<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || SpawnContext {
            parent_entity: cx.me().target,
            keys: RefCell::new(BTreeSet::new()),
        });

        unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) }
    }
}

struct RuntimeWaker {
    proxy: EventLoopProxy<WakeUp>,
}

impl Wake for RuntimeWaker {
    fn wake(self: Arc<Self>) {
        self.proxy.send_event(WakeUp).unwrap();
    }
}

fn compose(world: &mut World) {
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

    let proxy = (*world
        .get_resource::<EventLoopProxyWrapper<WakeUp>>()
        .unwrap())
    .clone();
    let rt = &mut *world.non_send_resource_mut::<Runtime>();
    let mut composers = rt.composers.borrow_mut();
    for rt_composer in composers.values_mut() {
        let waker = Waker::from(Arc::new(RuntimeWaker {
            proxy: proxy.clone(),
        }));
        let mut cx = Context::from_waker(&waker);

        // TODO handle composition error.
        let _ = rt_composer.composer.poll_compose(&mut cx);
    }
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

/// Use one or more [`SystemParam`]s from the ECS world.
///
/// `with_world` will be called on every frame with the latest query.
///
/// Change detection is implemented as a traditional system parameter.
///
/// # Examples
///
/// ```no_run
/// use actuate::prelude::*;
/// use bevy::prelude::*;
///
/// // Timer composable.
/// #[derive(Data)]
/// struct Timer;
///
/// impl Compose for Timer {
///     fn compose(cx: Scope<Self>) -> impl Compose {
///         let current_time = use_mut(&cx, Time::default);
///
///         // Use the `Time` resource from the ECS world, updating the `current_time`.
///         use_world(&cx, move |time: Res<Time>| {
///             SignalMut::set(current_time, *time)
///         });
///
///         // Spawn a `Text` component, updating it when this scope is re-composed.
///         spawn(Text::new(format!("Elapsed: {:?}", current_time.elapsed())))
///     }
/// }
/// ```
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

/// Use one or more [`SystemParam`]s from the ECS world.
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
    keys: RefCell<BTreeSet<Pending>>,
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

/// ECS bundle modifier.
#[derive(Clone, Default)]
pub struct Modifier<'a> {
    fns: Vec<Rc<dyn Fn(Spawn<'a>) -> Spawn<'a> + 'a>>,
}

impl<'a> Modifier<'a> {
    /// Apply this modifier.
    pub fn apply(&self, spawn: Spawn<'a>) -> Spawn<'a> {
        self.fns
            .iter()
            .fold(spawn, |spawn, modifier| modifier(spawn))
    }

    /// Append another stack of modifiers to this modifier.
    pub fn append(&mut self, modifier: Cow<'a, Modifier>) {
        let modifier: Modifier<'_> = modifier.into_owned();
        let modifier: Modifier<'a> = unsafe { mem::transmute(modifier) };
        self.fns.extend(modifier.fns);
    }
}

impl fmt::Debug for Modifier<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Modifier").finish()
    }
}

unsafe impl Data for Modifier<'_> {}

/// Modifiable composable.
pub trait Modify<'a> {
    /// Get a mutable reference to the modifier of this button.
    fn modifier(&mut self) -> &mut Modifier<'a>;

    /// Append a modifier to this composable.
    fn append(mut self, modifier: Cow<'a, Modifier>) -> Self
    where
        Self: Sized,
    {
        self.modifier().append(modifier);
        self
    }

    /// Add a function to run when this composable's bundle is spawned.
    fn on_insert<F>(mut self, f: F) -> Self
    where
        Self: Sized,
        F: Fn(EntityWorldMut) + 'a,
    {
        let f = Rc::new(f);
        self.modifier().fns.push(Rc::new(move |spawn| {
            let f = f.clone();
            spawn.on_insert(move |e| f(e))
        }));
        self
    }

    /// Add an observer to the container of this button.
    fn observe<F, E, B, Marker>(mut self, observer: F) -> Self
    where
        Self: Sized,
        F: SystemParamFunction<Marker, In = Trigger<'static, E, B>, Out = ()> + Send + Sync + 'a,
        E: Event,
        B: Bundle,
    {
        let observer_cell = Cell::new(Some(observer));
        let f: Rc<dyn Fn(Spawn) -> Spawn> = Rc::new(move |spawn| {
            let observer = observer_cell.take().unwrap();
            let spawn: Spawn<'a> = unsafe { mem::transmute(spawn) };
            let spawn = spawn.observe(observer);
            let spawn: Spawn = unsafe { mem::transmute(spawn) };
            spawn
        });
        let f: Rc<dyn Fn(Spawn) -> Spawn> = unsafe { mem::transmute(f) };
        self.modifier().fns.push(f);
        self
    }

    #[cfg(feature = "picking")]
    #[cfg_attr(docsrs, doc(cfg(feature = "picking")))]
    /// Add an click observer to the container of this button.
    fn on_click(self, f: impl Fn() + Send + Sync + 'a) -> Self
    where
        Self: Sized,
    {
        self.observe(move |_: Trigger<Pointer<Click>>| f())
    }
}
