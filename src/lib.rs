mod scope;
pub use self::scope::Scope;

mod use_context;
pub use self::use_context::use_context;

mod use_provider;
pub use self::use_provider::use_provider;

mod use_state;
pub use self::use_state::{use_state, Setter};

mod vdom;
pub use self::vdom::VirtualDom;

pub mod view;
pub use self::view::View;

pub mod node;
pub use self::node::Node;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub trait WasmNotSend: Send {}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl<T: Send> WasmNotSend for T {}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub trait WasmNotSend {}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl<T> WasmNotSend for T {}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn channel<T>() -> (Tx<T>, Rx<T>) {
    tokio::sync::mpsc::unbounded_channel()
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub type Tx<T> = tokio::sync::mpsc::UnboundedSender<T>;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub type Rx<T> = tokio::sync::mpsc::UnboundedReceiver<T>;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web {
    use std::{
        cell::RefCell,
        rc::Rc,
        task::{Context, Poll, Waker},
    };

    struct Inner<T> {
        queue: Vec<T>,
        waker: Option<Waker>,
    }

    type Shared<T> = Rc<RefCell<Inner<T>>>;

    pub fn channel<T>() -> (Tx<T>, Rx<T>) {
        let shared = Rc::new(RefCell::new(Inner {
            queue: Vec::new(),
            waker: None,
        }));
        (
            Tx {
                shared: shared.clone(),
            },
            Rx { shared },
        )
    }

    pub struct Tx<T> {
        shared: Shared<T>,
    }

    impl<T> Clone for Tx<T> {
        fn clone(&self) -> Self {
            Self {
                shared: self.shared.clone(),
            }
        }
    }

    impl<T> Tx<T> {
        pub fn send(&self, value: T) -> Option<()> {
            let mut shared = self.shared.borrow_mut();
            shared.queue.push(value);

            if let Some(waker) = shared.waker.take() {
                waker.wake()
            }

            Some(())
        }
    }

    pub struct Rx<T> {
        shared: Shared<T>,
    }

    impl<T> Rx<T> {
        pub fn poll_recv(&mut self, cx: &mut Context) -> Poll<Option<T>> {
            let mut shared = self.shared.borrow_mut();
            if let Some(item) = shared.queue.pop() {
                Poll::Ready(Some(item))
            } else {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub fn run(view: impl crate::View) {
        let future = async move {
            let mut vdom = crate::VirtualDom::new(view.into_node());

            loop {
                vdom.run().await;
            }
        };

        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use self::web::run;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use self::web::{channel, Rx, Tx};
