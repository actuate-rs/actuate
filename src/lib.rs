use core::fmt;
use std::{
    fmt::Debug,
    future::{self, Future},
    mem,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

pub trait View: Send + Sized + 'static {
    fn body(&self) -> impl View;

    fn into_node(self) -> impl TreeNode {
        Node {
            view: self,
            body_fn: |me: &'static Self| me.body().into_node(),
            body: None,
            is_view_ready: false,
            is_body_ready: false,
            body_waker: None,
        }
    }
}

impl View for () {
    fn body(&self) -> impl View {}

    fn into_node(self) -> impl TreeNode {}
}

impl<V1: View, V2: View> View for (V1, V2) {
    fn body(&self) -> impl View {}

    fn into_node(self) -> impl TreeNode {
        (self.0.into_node(), self.1.into_node())
    }
}

pub trait TreeNode: Debug + Send + 'static {
    fn poll_ready(&mut self, cx: &mut Context) -> Poll<()>;

    fn view(&mut self) -> impl Future<Output = ()> + Send;
}

impl TreeNode for () {
    fn poll_ready(&mut self, cx: &mut Context) -> Poll<()> {
        todo!()
    }

    async fn view(&mut self) {}
}

impl<T1: TreeNode, T2: TreeNode> TreeNode for (T1, T2) {
    fn poll_ready(&mut self, cx: &mut Context) -> Poll<()> {
        todo!()
    }

    async fn view(&mut self) {
        todo!()
    }
}

pub struct Node<V, F, B> {
    view: V,
    body_fn: F,
    body: Option<B>,
    is_view_ready: bool,
    is_body_ready: bool,
    body_waker: Option<Arc<NodeWaker>>,
}

impl<V, F, B: fmt::Debug> fmt::Debug for Node<V, F, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut tuple = f.debug_tuple(&std::any::type_name::<V>());

        if let Some(ref body) = self.body {
            tuple.field(body);
        }

        tuple.finish()
    }
}

impl<V, F, B> TreeNode for Node<V, F, B>
where
    V: View,
    F: Fn(&'static V) -> B + 'static + Send,
    B: TreeNode,
{
    fn poll_ready(&mut self, cx: &mut Context) -> Poll<()> {
        if let Some(ref mut body) = self.body {
            if let Some(ref waker) = self.body_waker {
                if *waker.is_ready.lock().unwrap() {
                    self.is_body_ready = true;
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            } else {
                let node_waker = Arc::new(NodeWaker {
                    is_ready: Mutex::new(false),
                    waker: cx.waker().clone(),
                });
                self.body_waker = Some(node_waker.clone());

                let waker = Waker::from(node_waker);
                let mut body_cx = Context::from_waker(&waker);

                if body.poll_ready(&mut body_cx).is_ready() {
                    self.is_body_ready = true;
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        } else {
            // Ready to build the initial view.
            self.is_view_ready = true;
            Poll::Ready(())
        }
    }

    async fn view(&mut self) {
        if mem::take(&mut self.is_view_ready) {
            let view = unsafe { mem::transmute(&self.view) };
            let body = (self.body_fn)(view);
            self.body = Some(body);
        }

        if mem::take(&mut self.is_body_ready) {
            self.body.as_mut().unwrap().view().await;
        }
    }
}

struct NodeWaker {
    is_ready: Mutex<bool>,
    waker: Waker,
}

impl Wake for NodeWaker {
    fn wake(self: Arc<Self>) {
        *self.is_ready.lock().unwrap() = true;
        self.waker.wake_by_ref();
    }
}

pub async fn run(view: impl View) {
    let mut node = view.into_node();
    future::poll_fn(|cx| node.poll_ready(cx)).await;
    node.view().await;
    dbg!(node);
}
