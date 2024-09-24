use crate::{Entity, UnsafeWorldCell};
use std::marker::PhantomData;

pub trait QueryData {
    type Data<'w>;

    unsafe fn query_data<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Self::Data<'w>;
}

pub struct Query<'w, D> {
    pub(crate) world: UnsafeWorldCell<'w>,
    pub(crate) _marker: PhantomData<D>,
}

impl<'w, D> Query<'w, D> {
    pub fn get(&self, entity: Entity) -> D::Data<'w>
    where
        D: QueryData,
    {
        unsafe { D::query_data(self.world, entity) }
    }
}
