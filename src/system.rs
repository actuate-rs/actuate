use std::marker::PhantomData;

use crate::{Query, QueryData, UnsafeWorldCell};

pub trait SystemParam {
    type Item<'w>;

    fn system_param<'w>(world: UnsafeWorldCell<'w>) -> Self::Item<'w>;
}

impl<'a, D: QueryData> SystemParam for Query<'a, D> {
    type Item<'w> = Query<'w, D>;

    fn system_param<'w>(world: UnsafeWorldCell<'w>) -> Self::Item<'w> {
        Query {
            world,
            _marker: PhantomData,
        }
    }
}

pub trait System: 'static {
    fn run<'w>(&mut self, world: UnsafeWorldCell<'w>);
}

pub trait IntoSystem<Marker> {
    type System: System;

    fn into_system(self) -> Self::System;
}

impl<Marker: 'static, F: SystemParamFunction<Marker> + 'static> IntoSystem<Marker> for F {
    type System = FunctionSystem<F, Marker>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

pub struct FunctionSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<Marker: 'static, F: SystemParamFunction<Marker> + 'static> System
    for FunctionSystem<F, Marker>
{
    fn run<'w>(&mut self, world: UnsafeWorldCell<'w>) {
        self.f.run(F::Param::system_param(world));
    }
}

pub trait SystemParamFunction<Marker> {
    type Param: SystemParam;

    fn run(&mut self, param: <Self::Param as SystemParam>::Item<'_>);
}

impl<P, F> SystemParamFunction<fn(P)> for F
where
    P: SystemParam,
    F: FnMut(P) + FnMut(P::Item<'_>),
{
    type Param = P;

    fn run(&mut self, param: <Self::Param as SystemParam>::Item<'_>) {
        self(param)
    }
}
