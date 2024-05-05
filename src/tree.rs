use crate::{Context, Inner, Scope, View};
use slotmap::DefaultKey;
use std::{cell::UnsafeCell, mem};

pub trait Tree {
    type State: 'static;

    fn build(&mut self, cx: &mut Context, children: &mut Vec<DefaultKey>) -> Self::State;

    fn rebuild(
        &mut self,
        cx: &mut Context,
        state: &mut Self::State,
        children: &mut Vec<DefaultKey>,
    );

    fn remove(state: &mut Self::State, children: &mut Vec<DefaultKey>);
}

impl Tree for () {
    type State = ();

    fn build(&mut self, cx: &mut Context, children: &mut Vec<DefaultKey>) -> Self::State {
        let _ = children;
        let _ = cx;
    }

    fn rebuild(
        &mut self,
        cx: &mut Context,
        state: &mut Self::State,
        children: &mut Vec<DefaultKey>,
    ) {
        let _ = children;
        let _ = state;
        let _ = cx;
    }

    fn remove(state: &mut Self::State, children: &mut Vec<DefaultKey>) {
        let _ = children;
        let _ = state;
    }
}

impl<T: Tree> Tree for Option<T> {
    type State = Option<T::State>;

    fn build(&mut self, cx: &mut Context, children: &mut Vec<DefaultKey>) -> Self::State {
        if let Some(tree) = self {
            Some(tree.build(cx, children))
        } else {
            None
        }
    }

    fn rebuild(
        &mut self,
        cx: &mut Context,
        state: &mut Self::State,
        children: &mut Vec<DefaultKey>,
    ) {
        if let Some(tree) = self {
            if let Some(state) = state {
                tree.rebuild(cx, state, children);
            } else {
                *state = Some(tree.build(cx, children));
            }
        } else if let Some(state) = state {
            T::remove(state, children)
        }
    }

    fn remove(state: &mut Self::State, children: &mut Vec<DefaultKey>) {
        if let Some(state) = state {
            T::remove(state, children)
        }
    }
}

impl<T1: Tree, T2: Tree> Tree for (T1, T2) {
    type State = (T1::State, T2::State);

    fn build(&mut self, cx: &mut Context, children: &mut Vec<DefaultKey>) -> Self::State {
        (self.0.build(cx, children), self.1.build(cx, children))
    }

    fn rebuild(
        &mut self,
        cx: &mut Context,
        state: &mut Self::State,
        children: &mut Vec<DefaultKey>,
    ) {
        self.0.rebuild(cx, &mut state.0, children);
        self.1.rebuild(cx, &mut state.1, children);
    }

    fn remove(state: &mut Self::State, children: &mut Vec<DefaultKey>) {
        T1::remove(&mut state.0, children);
        T2::remove(&mut state.1, children);
    }
}

pub struct ViewTree<V, B, F> {
    pub(crate) view: V,
    pub(crate) body: Option<B>,
    pub(crate) f: F,
}

impl<V, B, F> Tree for ViewTree<V, B, F>
where
    V: View,
    B: Tree + 'static,
    F: Fn(&'static V, &'static Scope) -> B,
{
    type State = (DefaultKey, Box<Scope>, B::State);

    fn build(&mut self, cx: &mut Context, children: &mut Vec<DefaultKey>) -> Self::State {
        let key = cx.nodes.insert(crate::Node {
            view: &self.view as *const V,
            children: Vec::new(),
        });
        children.push(key);

        let scope = Box::new(Scope {
            key,
            inner: UnsafeCell::new(Inner {
                hooks: Vec::new(),
                idx: 0,
            }),
            tx: cx.tx.clone(),
        });

        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let scope_ref: &'static Scope = unsafe { mem::transmute(&*scope) };
        let body = (self.f)(view_ref, scope_ref);
        self.body = Some(body);

        let mut inner_children = Vec::new();
        let body_state = self.body.as_mut().unwrap().build(cx, &mut inner_children);
        cx.nodes.get_mut(key).unwrap().children = inner_children;

        (key, scope, body_state)
    }

    fn rebuild(
        &mut self,
        cx: &mut Context,
        state: &mut Self::State,
        _children: &mut Vec<DefaultKey>,
    ) {
        {
            let scope = unsafe { &mut *state.1.inner.get() };
            scope.idx = 0;

            if let Some(updates) = cx.pending_updates.get_mut(state.0) {
                for update in mem::take(updates) {
                    scope.hooks[update.idx] = update.value;
                }
            }
        }

        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let scope_ref: &'static Scope = unsafe { mem::transmute(&*state.1) };
        let body = (self.f)(view_ref, scope_ref);
        self.body = Some(body);

        let mut inner_children = Vec::new();
        self.body
            .as_mut()
            .unwrap()
            .rebuild(cx, &mut state.2, &mut inner_children);
        cx.nodes.get_mut(state.0).unwrap().children = inner_children;

        let node = cx.nodes.get_mut(state.0).unwrap();
        node.view = &self.view as _;
    }

    fn remove(state: &mut Self::State, children: &mut Vec<DefaultKey>) {
        if let Some(idx) = children.iter().position(|key| *key == state.0) {
            children.remove(idx);
        }
    }
}
