use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    marker::PhantomData,
    mem,
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

    pub fn query<Q: Queryable>(&mut self) -> Option<Q::Output<'_>> {
        Q::query(&UnsafeCell::new(self))
    }
}

pub trait Queryable {
    type Output<'e>;

    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>>;
}

impl<T: 'static> Queryable for &T {
    type Output<'e> = &'e T;

    // TODO super unsafe
    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>> {
        let elem = unsafe { &*element.get() };
        elem.attributes
            .get(&TypeId::of::<T>())
            .and_then(|attr| attr.downcast_ref())
    }
}

impl<T: 'static> Queryable for &mut T {
    type Output<'e> = &'e mut T;

    // TODO super unsafe
    fn query<'e>(element: &UnsafeCell<&'e mut Element>) -> Option<Self::Output<'e>> {
        let elem = unsafe { &mut *element.get() };
        elem.attributes
            .get_mut(&TypeId::of::<T>())
            .and_then(|attr| attr.downcast_mut())
    }
}

#[derive(Default)]
pub struct World {
    elements: SlotMap<DefaultKey, Element>,
}

impl World {
    pub fn add_element(&mut self, element: Element) -> ElementHandle {
        let key = self.elements.insert(element);
        ElementHandle { key }
    }

    pub fn spawn<'w, Marker>(&'w mut self, task: impl IntoTask<'w, Marker>) {
        let mut task = task.into_task();
        let input = task.input(&UnsafeCell::new(self));
        task.run(input)
    }
}

#[derive(Clone, Copy)]
pub struct ElementHandle {
    key: DefaultKey,
}

pub trait FromWorld<'w> {
    fn from_world(world: &UnsafeCell<&'w mut World>) -> Self;
}

pub struct Query<'w, Q> {
    world: &'w UnsafeCell<&'w mut World>,
    queryable: PhantomData<Q>,
}

impl<'w, Q> Query<'w, Q> {
    pub fn get(&self, element: ElementHandle) -> Option<Q::Output<'_>>
    where
        Q: Queryable,
    {
        let world = unsafe { &mut *self.world.get() };
        let element = UnsafeCell::new(&mut world.elements[element.key]);
        Q::query(&element)
    }
}

impl<'w, Q: Queryable> FromWorld<'w> for Query<'w, Q> {
    fn from_world(world: &UnsafeCell<&mut World>) -> Self {
        // TODO Probs really unsafe
        let world = unsafe { mem::transmute(world) };
        Self {
            world,
            queryable: PhantomData,
        }
    }
}

pub trait Task<'w> {
    type Input: FromWorld<'w>;

    fn input(&self, world: &UnsafeCell<&'w mut World>) -> Self::Input {
        Self::Input::from_world(world)
    }

    fn run(&mut self, input: Self::Input);
}

pub struct FnTask<'w, F, T> {
    f: F,
    _marker: PhantomData<&'w T>,
}

impl<'w, I, F> Task<'w> for FnTask<'w, F, I>
where
    I: FromWorld<'w>,
    F: FnMut(I),
{
    type Input = I;

    fn run(&mut self, input: Self::Input) {
        (self.f)(input)
    }
}

pub trait IntoTask<'w, Marker> {
    type Task: Task<'w>;

    fn into_task(self) -> Self::Task;
}

impl<'w, I, F> IntoTask<'w, FnTask<'w, F, I>> for F
where
    I: FromWorld<'w>,
    F: FnMut(I),
{
    type Task = FnTask<'w, F, I>;

    fn into_task(self) -> Self::Task {
        FnTask {
            f: self,
            _marker: PhantomData,
        }
    }
}
