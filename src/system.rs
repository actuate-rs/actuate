use crate::{Id, Query, World};
use alloc::vec::Vec;
use core::{any, cell::UnsafeCell, marker::PhantomData, mem};

pub trait System<'a>: 'static {
    type Input<'w>;
    type Query<'w>: Query<'a, Output<'w> = Self::Input<'w>>;

    fn run(&self, input: Self::Input<'_>);

    fn name(&self) -> &'static str;
}

pub struct FnSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<'a, F, Q> System<'a> for FnSystem<F, (Q,)>
where
    F: 'static,
    &'a F: for<'w> Fn(Q) + for<'w> Fn(Q::Output<'w>),
    Q: Query<'a> + 'static,
{
    type Input<'w> = Q::Output<'w>;
    type Query<'w> = Q;

    fn run<'w>(&self, input: Self::Input<'w>) {
        fn call_inner<QParam>(f: impl Fn(QParam), a0: QParam) {
            f(a0)
        }
        let f: &'a F = unsafe { mem::transmute(&self.f) };
        call_inner(f, input)
    }

    fn name(&self) -> &'static str {
        any::type_name::<F>()
    }
}

macro_rules! impl_system_for_fn_system {
    ($($t:tt: $idx:tt),*) => {
        impl<'a, F, $($t: Query<'a> + 'static),*> System<'a> for FnSystem<F, ($($t),*)>
        where
            F: 'static,
            &'a F: for<'w> Fn($($t),*) + for<'w> Fn($($t::Output<'w>),*),
        {
            type Input<'w> = ($($t::Output<'w>),*);
            type Query<'w> = ($($t),*);

            fn run<'w>(&self, input: Self::Input<'w>) {
                fn call_inner<'a, $($t),*>(
                    f: impl Fn($($t),*),
                    input: ($($t),*)
               ){
                   f($(input.$idx),*)
               }
               let f: &'a F = unsafe { mem::transmute(&self.f)};
               call_inner(f, input)
            }

            fn name(&self) -> &'static str {
                any::type_name::<F>()
            }
        }

        impl<'a, F, $($t: Query<'a> + 'static),*> IntoSystem<'a, ($($t),*)> for F
        where
            F: 'static,
            &'a F: for<'w> Fn($($t),*) + for<'w> Fn($($t::Output<'w>),*),
        {
            type System = FnSystem<F, ($($t),*)>;

            fn into_system(self) -> Self::System {
                FnSystem {
                    f: self,
                    _marker: PhantomData,
                }
            }
        }
    };
}

impl_system_for_fn_system!(Q1: 0, Q2: 1);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2, Q4: 3);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2, Q4: 3, Q5: 4);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2, Q4: 3, Q5: 4, Q6: 5);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2, Q4: 3, Q5: 4, Q6: 5, Q7: 6);
impl_system_for_fn_system!(Q1: 0, Q2: 1, Q3: 2, Q4: 3, Q5: 4, Q6: 5, Q7: 6, Q9: 7);

pub trait IntoSystem<'a, Marker> {
    type System: System<'a>;

    fn into_system(self) -> Self::System;
}

impl<'a, F, Q> IntoSystem<'a, (Q,)> for F
where
    F: 'static,
    &'a F: for<'w> Fn(Q) + for<'w> Fn(Q::Output<'w>),
    Q: Query<'a> + 'static,
{
    type System = FnSystem<F, (Q,)>;

    fn into_system(self) -> Self::System {
        FnSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

pub(crate) trait AnySystem {
    fn reads_any(&self, ids: &mut Vec<Id>);

    fn writes_any(&self, ids: &mut Vec<Id>);

    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>);
}

impl<'a, S: System<'a>> AnySystem for S {
    fn reads_any(&self, ids: &mut Vec<Id>) {
        S::Query::reads(ids)
    }

    fn writes_any(&self, ids: &mut Vec<Id>) {
        S::Query::writes(ids)
    }

    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>) {
        let world_cell = mem::transmute(world);
        let query = S::Query::query(world_cell);
        self.run(query)
    }
}
