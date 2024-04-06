use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
};

#[derive(Default)]
pub struct Element {
    attributes: HashMap<TypeId, Box<dyn Any>>,
}

impl Element {
    pub fn insert(&mut self, attr: impl Any) -> &mut Self {
        self.attributes.insert(attr.type_id(), Box::new(attr));
        self
    }

    pub fn query<Q: Query>(&mut self) -> Option<Q::Output<'_>> {
        Q::query(&UnsafeCell::new(self))
    }
}

pub trait Query {
    type Output<'e>;

    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>>;
}

impl<T: 'static> Query for &T {
    type Output<'e> = &'e T;

    // TODO super unsafe
    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>> {
        let elem = unsafe { &*element.get() };
        elem.attributes
            .get(&TypeId::of::<T>())
            .and_then(|attr| attr.downcast_ref())
    }
}

impl<T: 'static> Query for &mut T {
    type Output<'e> = &'e mut T;

    // TODO super unsafe
    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>> {
        let elem = unsafe { &mut *element.get() };
        elem.attributes
            .get_mut(&TypeId::of::<T>())
            .and_then(|attr| attr.downcast_mut())
    }
}
