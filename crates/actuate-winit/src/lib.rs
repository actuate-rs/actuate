use actuate_core::{prelude::*, Composer, Executor, Update, Updater};
use std::{cell::RefCell, collections::HashMap, mem, rc::Rc, sync::mpsc, thread};
use winit::{
    application::ApplicationHandler,
    event::{Event, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window as RawWindow, WindowAttributes, WindowId},
};

struct UnsafeUpdate(Update);

unsafe impl Send for UnsafeUpdate {}

struct EventLoopUpdater {
    tx: mpsc::Sender<UnsafeUpdate>,
}

impl Updater for EventLoopUpdater {
    fn update(&self, update: Update) {
        if self.tx.send(UnsafeUpdate(update)).is_err() {
            panic!("Failed to send update to event loop.");
        }
    }
}

struct HandlerRoot<C> {
    content: C,
    event_loop_cx: EventLoopContext,
}

unsafe impl<C: Data> Data for HandlerRoot<C> {
    type Id = HandlerRoot<C::Id>;
}

impl<C: Compose> Compose for HandlerRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || cx.me().event_loop_cx.clone());

        Ref::map(cx.me(), |me| &me.content)
    }
}

struct Handler {
    composer: Composer,
    cx: EventLoopContext,
}

impl Handler {
    fn compose(&mut self, event_loop: &ActiveEventLoop) {
        // Safety: This reference to `event_loop` must not escape the context.
        let event_loop: &'static ActiveEventLoop = unsafe { mem::transmute(event_loop) };
        self.cx.inner.borrow_mut().event_loop = Some(event_loop);

        self.composer.compose();

        self.cx.inner.borrow_mut().event_loop = None;
    }
}

impl ApplicationHandler<Vec<UnsafeUpdate>> for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(feature = "tracing")]
        tracing::trace!("Resumed");

        self.compose(event_loop);

        for f in self.cx.inner.borrow_mut().handler_fns.values_mut() {
            f(&Event::Resumed)
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, events: Vec<UnsafeUpdate>) {
        #[cfg(feature = "tracing")]
        tracing::trace!("Update");

        for event in events {
            unsafe { event.0.apply() };
        }

        self.compose(event_loop);

        for f in self.cx.inner.borrow_mut().handler_fns.values_mut() {
            f(&Event::UserEvent(()))
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.compose(event_loop);

        self.cx
            .inner
            .borrow_mut()
            .handler_fns
            .get_mut(&window_id)
            .unwrap()(&Event::WindowEvent { window_id, event });
    }
}

pub fn run(content: impl Compose + 'static) {
    run_with_executor(content, tokio::runtime::Runtime::new().unwrap())
}

pub fn run_with_executor(content: impl Compose + 'static, executor: impl Executor + 'static) {
    let event_loop = EventLoop::with_user_event().build().unwrap();

    let proxy = event_loop.create_proxy();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(update) = rx.recv() {
            let mut updates = vec![update];
            while let Ok(next_update) = rx.try_recv() {
                updates.push(next_update);
            }

            if proxy.send_event(updates).is_err() {
                panic!("Failed to send update to event loop.");
            }
        }
    });

    let cx = EventLoopContext::default();

    let mut handler = Handler {
        composer: Composer::with_updater(
            HandlerRoot {
                content,
                event_loop_cx: cx.clone(),
            },
            EventLoopUpdater { tx },
            executor,
        ),
        cx,
    };

    event_loop.run_app(&mut handler).unwrap();
}

#[derive(Default)]
struct Inner {
    handler_fns: HashMap<WindowId, ListenerFn<'static>>,
    event_loop: Option<&'static ActiveEventLoop>,
}

#[derive(Clone, Default)]
pub struct EventLoopContext {
    inner: Rc<RefCell<Inner>>,
}

type ListenerFn<'a> = Rc<dyn Fn(&Event<()>) + 'a>;

type EventFn<'a> = Box<dyn Fn(&RawWindow, &Event<()>) + 'a>;

#[derive(Data)]
pub struct Window<'a, C> {
    window_attributes: WindowAttributes,
    on_event: EventFn<'a>,
    content: C,
}

impl<'a, C> Window<'a, C> {
    pub fn new(
        window_attributes: WindowAttributes,
        on_event: impl Fn(&RawWindow, &Event<()>) + 'a,
        content: C,
    ) -> Self {
        Self {
            window_attributes,
            on_event: Box::new(on_event),
            content,
        }
    }
}

impl<C: Compose> Compose for Window<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let event_loop_cx = use_context::<EventLoopContext>(&cx).unwrap();
        let mut inner = event_loop_cx.inner.borrow_mut();

        let window = use_ref(&cx, || {
            inner
                .event_loop
                .as_ref()
                .unwrap()
                .create_window(cx.me().window_attributes.clone())
                .unwrap()
        });

        use_memo(&cx, cx.me().window_attributes.title.clone(), || {
            window.set_title(&cx.me().window_attributes.title);
        });

        // TODO react to more attributes

        let drop_inner = event_loop_cx.inner.clone();
        let id = window.id();
        use_drop(&cx, move || {
            drop_inner.borrow_mut().handler_fns.remove(&id);
        });

        let on_event = &*cx.me().on_event;
        let on_event: ListenerFn = Rc::new(move |event| on_event(window, event));
        let on_event: ListenerFn = unsafe { mem::transmute(on_event) };

        inner.handler_fns.insert(id, on_event);

        // Safety: The pointer to `me.content` is guranteed to remain constant.
        Ref::map(cx.me(), |me| &me.content)
    }
}
