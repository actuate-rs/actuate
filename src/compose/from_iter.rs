use super::{AnyCompose, Node, Runtime};
use crate::{compose::Compose, data::Data, use_ref, Scope, ScopeData, Signal};
use alloc::rc::Rc;
use core::{cell::RefCell, mem};
use slotmap::DefaultKey;

/// Create a composable from an iterator.
///
/// `make_item` will be called for each item to produce a composable.
///
/// # Examples
///
/// ```
/// use actuate::prelude::*;
///
/// #[derive(Data)]
/// struct User {
///     id: i32,
/// }
///
/// impl Compose for User {
///     fn compose(cx: Scope<Self>) -> impl Compose {}
/// }
///
/// #[derive(Data)]
/// struct App;
///
/// impl Compose for App {
///     fn compose(cx: Scope<Self>) -> impl Compose {
///         compose::from_iter(0..10, |id| {
///             User { id: *id }
///         })
///     }
/// }
/// ```
pub fn from_iter<'a, I, C>(
    iter: I,
    make_item: impl Fn(Signal<'a, I::Item>) -> C + 'a,
) -> FromIter<'a, I, I::Item, C>
where
    I: IntoIterator + Clone + Data,
    I::Item: 'static,
    C: Compose,
{
    FromIter {
        iter,
        make_item: Box::new(make_item),
    }
}

/// Composable from an iterator.
///
/// For more see [`from_iter`].
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct FromIter<'a, I, Item, C> {
    iter: I,
    make_item: Box<dyn Fn(Signal<'a, Item>) -> C + 'a>,
}

unsafe impl<I, Item, C> Data for FromIter<'_, I, Item, C>
where
    I: Data,
    Item: 'static,
    C: Data,
{
}

impl<I, Item, C> Compose for FromIter<'_, I, Item, C>
where
    I: IntoIterator<Item = Item> + Clone + Data,
    Item: 'static,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let states: &RefCell<Vec<ItemState<Item>>> = use_ref(&cx, || RefCell::new(Vec::new()));
        let mut states = states.borrow_mut();

        let mut items: Vec<Option<_>> = cx.me().iter.clone().into_iter().map(Some).collect();

        let rt = Runtime::current();

        if items.len() >= states.len() {
            for item in &mut items[states.len()..] {
                let item = item.take().unwrap();

                let state = ItemState { item, key: None };
                states.push(state);
            }
        } else {
            states.truncate(items.len());
        }

        for (idx, state) in states.iter_mut().enumerate() {
            let mut nodes = rt.nodes.borrow_mut();

            if state.key.is_none() {
                let item_ref: &Item = &state.item;
                let item_ref: &Item = unsafe { mem::transmute(item_ref) };
                let compose = (cx.me().make_item)(Signal {
                    value: item_ref,
                    generation: &cx.generation as _,
                });
                let any_compose: Box<dyn AnyCompose> = Box::new(compose);
                let any_compose: Box<dyn AnyCompose> = unsafe { mem::transmute(any_compose) };

                let key = nodes.insert(Rc::new(Node {
                    compose: RefCell::new(crate::composer::ComposePtr::Boxed(any_compose)),
                    scope: ScopeData::default(),
                    parent: Some(rt.current_key.get()),
                    children: RefCell::new(Vec::new()),
                    child_idx: idx,
                }));
                nodes
                    .get(rt.current_key.get())
                    .unwrap()
                    .children
                    .borrow_mut()
                    .push(key);

                state.key = Some(key);
            }

            let node = nodes.get(state.key.unwrap()).unwrap().clone();

            *node.scope.contexts.borrow_mut() = cx.contexts.borrow().clone();
            node.scope
                .contexts
                .borrow_mut()
                .values
                .extend(cx.child_contexts.borrow().values.clone());

            drop(nodes);

            rt.queue(state.key.unwrap());
        }
    }
}

struct ItemState<T> {
    item: T,
    key: Option<DefaultKey>,
}
