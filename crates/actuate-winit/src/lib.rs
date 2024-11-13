use std::{cell::RefCell, collections::HashMap, mem, rc::Rc};

use actuate_core::{prelude::*, use_drop, Composer, ScopeState, Update, Updater};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{WindowAttributes, WindowId},
};

struct EventLoopUpdater {
    proxy: EventLoopProxy<Update>,
}

impl Updater for EventLoopUpdater {
    fn update(&self, update: Update) {
        if self.proxy.send_event(update).is_err() {
            panic!("Failed to send update to event loop.");
        }
    }
}

struct HandlerRoot<C> {
    compose: C,
    event_loop_cx: EventLoopContext,
}

unsafe impl<C: Data> Data for HandlerRoot<C> {
    type Id = HandlerRoot<C::Id>;
}

impl<C: Compose> Compose for HandlerRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || cx.me().event_loop_cx.clone());

        cx.me().map(|me| &me.compose)
    }
}

struct Handler {
    composer: Composer,
    cx: EventLoopContext,
}

impl Handler {
    fn compose(&mut self, event_loop: &ActiveEventLoop) {
        self.cx.inner.borrow_mut().event_loop = Some(unsafe { mem::transmute(event_loop) });

        self.composer.compose();

        self.cx.inner.borrow_mut().event_loop = None;
    }
}

impl ApplicationHandler<Update> for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(feature = "tracing")]
        tracing::info!("Resumed");

        self.compose(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, mut event: Update) {
        #[cfg(feature = "tracing")]
        tracing::info!("Update");

        unsafe { event.apply() };

        self.compose(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.cx
            .inner
            .borrow_mut()
            .handler_fns
            .get_mut(&window_id)
            .unwrap()(event);

        self.compose(event_loop);
    }
}

pub fn run(content: impl Compose + 'static) {
    let event_loop = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    let cx = EventLoopContext::default();

    let mut handler = Handler {
        composer: Composer::with_updater(
            HandlerRoot {
                compose: content,
                event_loop_cx: cx.clone(),
            },
            EventLoopUpdater { proxy },
        ),
        cx,
    };

    event_loop.run_app(&mut handler).unwrap();
}

#[derive(Default)]
struct Inner {
    handler_fns: HashMap<WindowId, Box<dyn FnMut(WindowEvent)>>,
    event_loop: Option<&'static ActiveEventLoop>,
}

#[derive(Clone, Default)]
pub struct EventLoopContext {
    inner: Rc<RefCell<Inner>>,
}

pub fn use_window<'a>(
    cx: &'a ScopeState,
    window_attributes: WindowAttributes,
    f: impl FnMut(WindowEvent) + 'a,
) {
    let event_loop_cx = use_context::<EventLoopContext>(cx);
    let mut inner = event_loop_cx.inner.borrow_mut();

    let window = use_ref(cx, || {
        inner
            .event_loop
            .as_ref()
            .unwrap()
            .create_window(window_attributes)
            .unwrap()
    });

    use_drop(cx, || {
        inner.handler_fns.remove(&window.id());
    });

    let f: Box<dyn FnMut(WindowEvent)> = Box::new(f);
    let f = unsafe { mem::transmute(f) };
    inner.handler_fns.insert(window.id(), f);
}
