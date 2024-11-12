use crate::{Contexts, Mut, Scope, ScopeState};
use std::{
    any::Any,
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    rc::Rc,
};

#[doc(hidden)]
pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for &T {}

unsafe impl<T: 'static> StateField for Mut<'_, T> {}

#[doc(hidden)]
pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &&T {}

pub unsafe trait Data {}

unsafe impl Data for () {}

unsafe impl Data for &'static str {}

unsafe impl Data for String {}

unsafe impl<T: Data> Data for &T {}

unsafe impl Data for Box<dyn AnyCompose + '_> {}

unsafe impl Data for Rc<dyn AnyCompose + '_> {}

#[derive(Clone, Copy, Default)]
pub struct RebuildContext {
    pub(crate) is_changed: bool,
}

pub trait Node {
    type State: 'static;

    fn build(&self, contexts: &Contexts) -> Self::State;

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext);
}

pub struct ComposeNodeState {
    scope: Box<ScopeState>,
    node: Box<dyn AnyNode>,
    node_state: Box<dyn Any>,
}

pub struct ComposeNode<C> {
    compose: C,
}

impl<C: Compose> Node for ComposeNode<C> {
    type State = ComposeNodeState;

    fn build(&self, contexts: &Contexts) -> Self::State {
        let scope = Box::new(ScopeState {
            contexts: RefCell::new(contexts.clone()),
            ..Default::default()
        });

        let child = C::compose(Scope {
            me: &self.compose,
            state: unsafe { mem::transmute(&*scope) },
        });

        let node: Box<dyn AnyNode> = Box::new(child.into_node());
        let node_state = node.any_build(&*scope.contexts.borrow());

        let node = unsafe { mem::transmute(node) };

        ComposeNodeState {
            scope,
            node,
            node_state,
        }
    }

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let mut cx = *cx;
        if cx.is_changed || state.scope.is_changed.take() {
            cx.is_changed = true;

            state.scope.hook_idx.set(0);

            let child = C::compose(Scope {
                me: &self.compose,
                state: &state.scope,
            });

            let node: Box<dyn AnyNode> = Box::new(child.into_node());
            state.node = unsafe { mem::transmute(node) };
        }

        state.node.any_rebuild(&mut *state.node_state, &cx);
    }
}

pub trait AnyNode {
    fn any_build(&self, contexts: &Contexts) -> Box<dyn Any>;

    fn any_rebuild(&self, state: &mut dyn Any, cx: &RebuildContext);
}

impl<T: Node> AnyNode for T {
    fn any_build(&self, contexts: &Contexts) -> Box<dyn Any> {
        Box::new(self.build(contexts))
    }

    fn any_rebuild(&self, state: &mut dyn Any, cx: &RebuildContext) {
        self.rebuild(state.downcast_mut().unwrap(), cx)
    }
}

pub trait Compose: Data + Sized {
    fn compose(cx: Scope<Self>) -> impl Compose;

    fn into_node(self) -> impl Node {
        ComposeNode { compose: self }
    }
}

impl Compose for () {
    fn compose(_cx: Scope<Self>) -> impl Compose {}

    fn into_node(self) -> impl Node {}
}

impl Node for () {
    type State = ();

    fn build(&self, contexts: &Contexts) -> Self::State {
        let _ = contexts;
    }

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let _ = state;
        let _ = cx;
    }
}

impl<C: Compose> Compose for &C {
    fn compose(cx: Scope<Self>) -> impl Compose {
        C::compose(Scope {
            me: *cx.me,
            state: cx.state,
        })
    }
}

/* TODO
impl Compose for Box<dyn AnyCompose + '_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me).any_compose(cx.state)
    }
}

impl Compose for Rc<dyn AnyCompose + '_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me).any_compose(cx.state)
    }
}
*/

pub struct Memo<C> {
    compose: C,
}

impl<C: Compose + Hash> Memo<C> {
    pub fn new(compose: C) -> Self {
        Self { compose }
    }
}

unsafe impl<C: Data> Data for Memo<C> {}

impl<C: Compose + Hash> Compose for Memo<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let _ = cx;
    }

    fn into_node(self) -> impl Node {
        let mut hasher = DefaultHasher::new();
        self.compose.hash(&mut hasher);

        MemoNode {
            hash: hasher.finish(),
            node: self.compose.into_node(),
        }
    }
}
pub struct MemoNode<T> {
    hash: u64,
    node: T,
}

impl<T: Node> Node for MemoNode<T> {
    type State = (u64, T::State);

    fn build(&self, contexts: &Contexts) -> Self::State {
        (self.hash, self.node.build(contexts))
    }

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let _ = cx;

        let is_changed = if self.hash != state.0 {
            state.0 = self.hash;
            true
        } else {
            false
        };

        self.node
            .rebuild(&mut state.1, &RebuildContext { is_changed });
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {}

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
               let _ = cx;
            }

            fn into_node(self) -> impl Node {
                ($(self.$idx.into_node(),)*)
            }
        }

        impl<$($t: Node),*> Node for ($($t,)*) {
            type State = ($($t::State,)*);

            fn build(&self, contexts: &Contexts) -> Self::State {
                ($(self.$idx.build(contexts),)*)
            }

            fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
                $(self.$idx.rebuild(&mut state.$idx, cx);)*
            }
        }
    };
}

impl_tuples!(T1:0);
impl_tuples!(T1:0, T2:1);
impl_tuples!(T1:0, T2:1, T3:2);
impl_tuples!(T1:0, T2:1, T3:2, T4:3);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6, T8:7);

pub trait AnyCompose {
    fn as_ptr_mut(&mut self) -> *mut ();

    fn any_compose<'a>(
        &'a self,
        state: &'a ScopeState,
        content: &mut Option<Box<dyn AnyCompose + 'a>>,
    );
}

impl<C: Compose> AnyCompose for C {
    fn as_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }

    fn any_compose<'a>(
        &'a self,
        state: &'a ScopeState,
        content: &mut Option<Box<dyn AnyCompose + 'a>>,
    ) {
        let child = C::compose(Scope { me: self, state });
        unsafe {
            if let Some(ref mut content) = content {
                *(&mut *(content.as_ptr_mut() as *mut _)) = child
            } else {
                *content = Some(Box::new(child))
            }
        }
    }
}
