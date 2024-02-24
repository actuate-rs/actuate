use core::{cell::UnsafeCell, marker::PhantomData, mem};

use crate::{Id, Query, World};

pub trait System<'a>: 'static {
    type Query: Query<'a>;

    fn run(&self, query: Self::Query);
}

pub struct FnSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<'a, F, Q> System<'a> for FnSystem<F, (Q,)>
where
    F: Fn(Q) + 'static,
    Q: Query<'a> + 'static,
{
    type Query = Q;

    fn run(&self, query: Self::Query) {
        (self.f)(query)
    }
}

impl<'a, F, Q1, Q2> System<'a> for FnSystem<F, (Q1, Q2)>
where
    F: Fn(Q1, Q2) + 'static,
    Q1: Query<'a> + 'static,
    Q2: Query<'a> + 'static,
{
    type Query = (Q1, Q2);

    fn run(&self, query: Self::Query) {
        (self.f)(query.0, query.1)
    }
}

impl<'a, F, Q1, Q2, Q3> System<'a> for FnSystem<F, (Q1, Q2, Q3)>
where
    F: Fn(Q1, Q2, Q3) + 'static,
    Q1: Query<'a> + 'static,
    Q2: Query<'a> + 'static,
    Q3: Query<'a> + 'static,
{
    type Query = (Q1, Q2, Q3);

    fn run(&self, query: Self::Query) {
        (self.f)(query.0, query.1, query.2)
    }
}

pub trait IntoSystem<'a, Marker> {
    type System: System<'a>;

    fn into_system(self) -> Self::System;
}

impl<'a, F, Q> IntoSystem<'a, (Q,)> for F
where
    F: Fn(Q) + 'static,
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

impl<'a, F, Q1, Q2> IntoSystem<'a, (Q1, Q2)> for F
where
    F: Fn(Q1, Q2) + 'static,
    Q1: Query<'a> + 'static,
    Q2: Query<'a> + 'static,
{
    type System = FnSystem<F, (Q1, Q2)>;

    fn into_system(self) -> Self::System {
        FnSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

impl<'a, F, Q1, Q2, Q3> IntoSystem<'a, (Q1, Q2, Q3)> for F
where
    F: Fn(Q1, Q2, Q3) + 'static,
    Q1: Query<'a> + 'static,
    Q2: Query<'a> + 'static,
    Q3: Query<'a> + 'static,
{
    type System = FnSystem<F, (Q1, Q2, Q3)>;

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
        let world = unsafe { mem::transmute(world) };
        let query = S::Query::query(world);
        self.run(query)
    }
}
