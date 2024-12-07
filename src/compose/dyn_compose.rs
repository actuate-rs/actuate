use super::{drop_node, AnyCompose, Node, Pending, Runtime};
use crate::{compose::Compose, use_ref, Scope, ScopeData};
use core::{any::TypeId, cell::UnsafeCell, mem};
use slotmap::DefaultKey;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Create a new dynamically-typed composable.
///
/// # Examples
///
/// ```
/// use actuate::prelude::*;
///
/// #[derive(Data)]
/// struct A;
///
/// impl Compose for A {
///     fn compose(_cx: Scope<Self>) -> impl Compose {
///         dbg!("A");
///     }
/// }
///
/// #[derive(Data)]
/// struct B;
///
/// impl Compose for B {
///     fn compose(_cx: Scope<Self>) -> impl Compose {
///         dbg!("B");
///     }
/// }
///
/// #[derive(Data)]
/// struct App;
///
/// impl Compose for App {
///     fn compose(cx: Scope<Self>) -> impl Compose {
///         let count = use_mut(&cx, || 0);
///
///         SignalMut::update(count, |x| *x += 1);
///
///         if *count == 0 {
///             dyn_compose(A)
///         } else {
///             dyn_compose(B)
///         }
///     }
/// }
/// ```
pub fn dyn_compose<'a>(content: impl Compose + 'a) -> DynCompose<'a> {
    DynCompose {
        compose: UnsafeCell::new(Some(Box::new(content))),
    }
}

/// Dynamically-typed composable.
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct DynCompose<'a> {
    compose: UnsafeCell<Option<Box<dyn AnyCompose + 'a>>>,
}

#[derive(Clone, Copy)]
struct DynComposeState {
    key: DefaultKey,
    data_id: TypeId,
}

impl Compose for DynCompose<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let state: &Cell<Option<DynComposeState>> = use_ref(&cx, || Cell::new(None));

        let rt = Runtime::current();
        let mut nodes = rt.nodes.borrow_mut();

        if let Some(state) = state.get() {
            let compose: &mut dyn AnyCompose = unsafe { &mut *cx.me().compose.get() }
                .as_deref_mut()
                .unwrap();
            let mut compose: Box<dyn AnyCompose> = unsafe { mem::transmute(compose) };
            let data_id = compose.data_id();

            if data_id == state.data_id {
                let mut last = nodes[state.key].compose.borrow_mut();
                unsafe { compose.reborrow(last.as_ptr_mut()) };

                let key = state.key;
                let node = nodes[key].clone();
                let mut indices = Vec::new();
                let mut parent = node.parent;
                while let Some(key) = parent {
                    indices.push(nodes.get(key).unwrap().child_idx);
                    parent = nodes.get(key).unwrap().parent;
                }
                indices.push(node.child_idx);

                rt.pending.borrow_mut().insert(Pending { key, indices });

                return;
            } else {
                drop_node(&mut nodes, state.key);
            }
        }

        let Some(compose) = unsafe { &mut *cx.me().compose.get() }.take() else {
            if let Some(state) = state.get() {
                let key = state.key;
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

            return;
        };
        let compose: Box<dyn AnyCompose> = unsafe { mem::transmute(compose) };
        let data_id = compose.data_id();

        let level = nodes.get(rt.current_key.get()).unwrap().level + 1;
        let key = nodes.insert(Rc::new(Node {
            compose: RefCell::new(crate::composer::ComposePtr::Boxed(compose)),
            scope: ScopeData::default(),
            parent: Some(rt.current_key.get()),
            children: RefCell::new(Vec::new()),
            level,
            child_idx: 0,
        }));
        state.set(Some(DynComposeState { key, data_id }));

        nodes
            .get(rt.current_key.get())
            .unwrap()
            .children
            .borrow_mut()
            .push(key);

        let child_state = &nodes[key].scope;

        *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
        child_state
            .contexts
            .borrow_mut()
            .values
            .extend(cx.child_contexts.borrow().values.clone());

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
