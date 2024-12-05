use super::{AnyCompose, Node, Runtime};
use crate::{compose::Compose, data::Data, use_ref, Scope, ScopeData};
use alloc::borrow::Cow;
use core::cell::RefCell;
use std::{mem, rc::Rc};

/// Create a new memoized composable.
///
/// The content of the memoized composable is only re-composed when the dependency changes.
///
/// Children of this `Memo` may still be re-composed if their state has changed.
pub fn memo<D, C>(dependency: D, content: C) -> Memo<D, C>
where
    D: Data + Clone + PartialEq + 'static,
    C: Compose,
{
    Memo {
        dependency,
        content,
    }
}

/// Memoized composable.
///
/// See [`memo`] for more.
#[derive(Data)]
#[actuate(path = "crate")]
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct Memo<T, C> {
    dependency: T,
    content: C,
}

impl<T, C> Compose for Memo<T, C>
where
    T: Clone + Data + PartialEq + 'static,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        /*
        unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) }
        */

        let rt = Runtime::current();

        let mut is_init = false;

        let child_key = use_ref(&cx, || {
            is_init = true;

            let mut nodes = rt.nodes.borrow_mut();

            let ptr: *const dyn AnyCompose =
                unsafe { mem::transmute(&cx.me().content as *const dyn AnyCompose) };
            let child_key = nodes.insert(Rc::new(Node {
                compose: RefCell::new(crate::composer::ComposePtr::Ptr(ptr)),
                scope: ScopeData::default(),
                children: RefCell::new(Vec::new()),
            }));

            nodes
                .get(rt.current_key.get())
                .unwrap()
                .children
                .borrow_mut()
                .push(child_key);

            let child_state = &nodes[child_key].scope;

            *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
            child_state
                .contexts
                .borrow_mut()
                .values
                .extend(cx.child_contexts.borrow().values.clone());

            child_state.is_parent_changed.set(true);

            child_key
        });

        if !is_init {
            let last = rt.nodes.borrow().get(*child_key).unwrap().clone();
            let ptr: *const dyn AnyCompose =
                unsafe { mem::transmute(&cx.me().content as *const dyn AnyCompose) };
            *last.compose.borrow_mut() = crate::composer::ComposePtr::Ptr(ptr);
        }

        let last = use_ref(&cx, RefCell::default);
        let mut last = last.borrow_mut();
        if let Some(last) = &mut *last {
            if cx.me().dependency != *last {
                *last = cx.me().dependency.clone();
                rt.pending.borrow_mut().push_back(*child_key);
            }
        } else {
            *last = Some(cx.me().dependency.clone());
            rt.pending.borrow_mut().push_back(*child_key);
        }
    }

    fn name() -> Option<Cow<'static, str>> {
        Some(
            C::name()
                .map(|name| format!("Memo<{}>", name).into())
                .unwrap_or("Memo".into()),
        )
    }
}
