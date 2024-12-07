use super::{use_node, AnyCompose, Pending, Runtime};
use crate::{compose::Compose, composer::ComposePtr, data::Data, use_ref, Scope};
use alloc::borrow::Cow;
use core::cell::RefCell;
use std::mem;

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
        let ptr: *const dyn AnyCompose =
            unsafe { mem::transmute(&cx.me().content as *const dyn AnyCompose) };
        let (key, node) = use_node(&cx, ComposePtr::Ptr(ptr), 0);

        let rt = Runtime::current();
        let nodes = rt.nodes.borrow();

        let last = use_ref(&cx, RefCell::default);
        let mut last = last.borrow_mut();
        if let Some(last) = &mut *last {
            if cx.me().dependency != *last {
                *last = cx.me().dependency.clone();

                let mut indices = Vec::new();
                let mut parent = node.parent;
                while let Some(key) = parent {
                    indices.push(nodes.get(key).unwrap().child_idx);
                    parent = nodes.get(key).unwrap().parent;
                }
                indices.push(node.child_idx);

                rt.pending.borrow_mut().insert(Pending { key, indices });
            }
        } else {
            *last = Some(cx.me().dependency.clone());
            let node = nodes[key].clone();
            let mut indices = Vec::new();
            let mut parent = node.parent;
            while let Some(key) = parent {
                indices.push(nodes.get(key).unwrap().child_idx);
                parent = nodes.get(key).unwrap().parent;
            }
            indices.push(node.child_idx);

            rt.pending.borrow_mut().insert(Pending { key, indices });
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
