use slotmap::DefaultKey;

use super::{AnyCompose, Node, Runtime};
use crate::{compose::Compose, data::Data, use_ref, Scope, ScopeData, Signal};
use core::{cell::RefCell, mem};
use std::rc::Rc;

/// Composable from an iterator, created with [`from_iter`].
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct FromIter<'a, I, Item, C> {
    iter: I,
    make_item: Box<dyn Fn(Signal<'a, Item>) -> C + 'a>,
}

unsafe impl<I, Item, C> Data for FromIter<'_, I, Item, C>
where
    I: Data,
    Item: Data,
    C: Data,
{
}

impl<I, Item, C> Compose for FromIter<'_, I, Item, C>
where
    I: IntoIterator<Item = Item> + Clone + Data,
    Item: Data,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let states: &RefCell<Vec<AnyItemState>> = use_ref(&cx, || RefCell::new(Vec::new()));
        let mut states = states.borrow_mut();

        let mut items: Vec<Option<_>> = cx.me().iter.clone().into_iter().map(Some).collect();

        let rt = Runtime::current();
        let mut nodes = rt.nodes.borrow_mut();

        if items.len() >= states.len() {
            for item in &mut items[states.len()..] {
                let item = item.take().unwrap();

                let state = ItemState { item, key: None };
                let boxed = Box::new(state);
                let boxed: Box<()> = unsafe { mem::transmute(boxed) };
                states.push(AnyItemState {
                    boxed: Some(boxed),
                    drop: |any_state| {
                        let state: Box<ItemState<Item>> =
                            unsafe { mem::transmute(any_state.boxed.take().unwrap()) };
                        drop(state);
                    },
                });
            }
        } else {
            states.truncate(items.len());
        }

        for state in &mut *states {
            let state: &mut ItemState<Item> =
                unsafe { mem::transmute(state.boxed.as_deref_mut().unwrap()) };

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
                    children: RefCell::new(Vec::new()),
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

            rt.pending.borrow_mut().push_back(state.key.unwrap());
        }
    }
}

/// Create a composable from an iterator.
///
/// `make_item` will be called for each item to produce a composable.
pub fn from_iter<'a, I, C>(
    iter: I,
    make_item: impl Fn(Signal<'a, I::Item>) -> C + 'a,
) -> FromIter<'a, I, I::Item, C>
where
    I: IntoIterator + Clone + Data,
    I::Item: Data,
    C: Compose,
{
    FromIter {
        iter,
        make_item: Box::new(make_item),
    }
}

struct ItemState<T> {
    item: T,
    key: Option<DefaultKey>,
}

struct AnyItemState {
    boxed: Option<Box<()>>,
    drop: fn(&mut Self),
}

impl Drop for AnyItemState {
    fn drop(&mut self) {
        (self.drop)(self)
    }
}
