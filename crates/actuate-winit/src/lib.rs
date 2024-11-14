use actuate_core::{prelude::*, use_drop, Composer, MapCompose, Update, Updater};
use std::{cell::RefCell, collections::HashMap, mem, rc::Rc};
use winit::{
    application::ApplicationHandler,
    event::{Event, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window as RawWindow, WindowAttributes, WindowId},
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
    content: C,
    event_loop_cx: EventLoopContext,
}

unsafe impl<C: Data> Data for HandlerRoot<C> {
    type Id = HandlerRoot<C::Id>;
}

impl<C: Compose> Compose for HandlerRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || cx.me().event_loop_cx.clone());

        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
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
        tracing::trace!("Resumed");

        self.compose(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, mut event: Update) {
        #[cfg(feature = "tracing")]
        tracing::trace!("Update");

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
            .unwrap()(&Event::WindowEvent { window_id, event });

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
                content,
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
    handler_fns: HashMap<WindowId, Rc<dyn Fn(&Event<()>)>>,
    event_loop: Option<&'static ActiveEventLoop>,
}

#[derive(Clone, Default)]
pub struct EventLoopContext {
    inner: Rc<RefCell<Inner>>,
}

pub struct Window<'a, C> {
    window_attributes: WindowAttributes,
    on_event: Rc<dyn Fn(&RawWindow, &Event<()>) + 'a>,
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
            on_event: Rc::new(on_event),
            content,
        }
    }
}

// TODO
unsafe impl<C: Data> Data for Window<'_, C> {
    type Id = Window<'static, C::Id>;
}

impl<C: Compose> Compose for Window<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let event_loop_cx = use_context::<EventLoopContext>(&cx);
        let mut inner = event_loop_cx.inner.borrow_mut();

        let window = use_ref(&cx, || {
            inner
                .event_loop
                .as_ref()
                .unwrap()
                .create_window(cx.me().window_attributes.clone())
                .unwrap()
        });

        use_memo(&cx, &cx.me().window_attributes.title, || {
            window.set_title(&cx.me().window_attributes.title);
        });

        // TODO react to more attributes

        let drop_inner = event_loop_cx.inner.clone();
        let id = window.id();
        use_drop(&cx, move || {
            drop_inner.borrow_mut().handler_fns.remove(&id);
        });

        let on_event = cx.me().on_event.clone();
        let on_event: Rc<dyn Fn(&Event<()>)> = Rc::new(move |event| on_event(window, event));
        let on_event: Rc<dyn Fn(&Event<()>)> = unsafe { mem::transmute(on_event) };

        inner.handler_fns.insert(id, on_event);

        // Safety: The pointer to `me.content` is guranteed to remain constant.
        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
    }
}
