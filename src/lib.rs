use actuate_core::{
    use_context, use_memo, use_provider, use_ref, Compose, Composer, Data, Ref, Scope, ScopeState,
    Update, Updater,
};
use masonry::event_loop_runner::{MasonryState, MasonryUserEvent};
use masonry::widget::{Button as ButtonWidget, Flex as FlexWidget, Label, RootWidget, WidgetMut};
use masonry::{Action, AppDriver, Color, DriverCtx, WidgetId, WidgetPod};
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

pub use actuate_core as core;

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::{Button, Flex, Text};
}

pub fn run(compose: impl Compose + 'static) {
    let main_widget = FlexWidget::column();

    let event_loop = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    masonry::event_loop_runner::run_with(
        event_loop,
        Window::default_attributes(),
        RootWidget::new(main_widget),
        Driver::new(compose, proxy),
        Color::BLACK,
    )
    .unwrap();
}

struct Inner {
    child_idx: usize,
    widget: Option<WidgetMut<'static, FlexWidget>>,
    listeners: HashMap<WidgetId, Vec<Box<dyn FnMut(&Action)>>>,
}

#[derive(Clone)]
pub struct TreeContext {
    proxy: EventLoopProxy<MasonryUserEvent>,
    inner: Rc<RefCell<Inner>>,
}

pub struct Tree<C> {
    content: C,
    tree_cx: TreeContext,
}

unsafe impl<C: Data> Data for Tree<C> {
    type Id = Tree<C::Id>;
}

impl<C: Compose> Compose for Tree<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || cx.me().tree_cx.clone());

        unsafe { &cx.me_as_ref().content }
    }
}

struct DriverUpdate(Update);

unsafe impl Send for DriverUpdate {}

struct DriverUpdater {
    proxy: EventLoopProxy<MasonryUserEvent>,
}

impl Updater for DriverUpdater {
    fn update(&self, update: crate::Update) {
        self.proxy
            .send_event(MasonryUserEvent::Action(
                Action::Other(Box::new(DriverUpdate(update))),
                WidgetId::reserved(u16::MAX),
            ))
            .unwrap();
    }
}

pub struct Driver {
    composer: Composer,
    tree_cx: TreeContext,
}

impl Driver {
    pub fn new(content: impl Compose + 'static, proxy: EventLoopProxy<MasonryUserEvent>) -> Self {
        let tree_cx = TreeContext {
            proxy: proxy.clone(),
            inner: Rc::new(RefCell::new(Inner {
                widget: None,
                child_idx: 0,
                listeners: HashMap::new(),
            })),
        };

        let updater = DriverUpdater { proxy };

        Self {
            composer: Composer::new(
                Tree {
                    content,
                    tree_cx: tree_cx.clone(),
                },
                updater,
            ),
            tree_cx,
        }
    }
}

impl AppDriver for Driver {
    fn on_action(&mut self, masonry_ctx: &mut DriverCtx, widget_id: WidgetId, action: Action) {
        let mut root = masonry_ctx.get_root::<RootWidget<FlexWidget>>();
        let flex = RootWidget::child_mut(&mut root);
        let widget: WidgetMut<'static, FlexWidget> = unsafe { mem::transmute(flex) };

        if let Action::Other(mut action) = action {
            if let Some(update) = action.downcast_mut::<DriverUpdate>() {
                unsafe { update.0.apply() }
            }

            let mut tree_cx = self.tree_cx.inner.borrow_mut();
            tree_cx.widget = Some(widget);
            tree_cx.child_idx = 0;
            drop(tree_cx);

            self.composer.compose();

            self.tree_cx.inner.borrow_mut().widget = None;
        } else {
            let mut tree_cx = self.tree_cx.inner.borrow_mut();
            if let Some(listeners) = tree_cx.listeners.get_mut(&widget_id) {
                for f in listeners {
                    f(&action);
                }
            }
        }
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        state.get_root().edit_root_widget(|mut root| {
            let mut root = root.downcast::<RootWidget<FlexWidget>>();
            let flex = RootWidget::child_mut(&mut root);
            let widget: WidgetMut<'static, FlexWidget> = unsafe { mem::transmute(flex) };

            self.tree_cx.inner.borrow_mut().widget = Some(widget);
            self.tree_cx.inner.borrow_mut().child_idx = 0;

            self.composer.compose();

            self.tree_cx.inner.borrow_mut().widget = None;
        });
    }
}

pub fn use_listener<'a>(cx: &'a ScopeState, id: WidgetId, on_action: impl Fn(&Action) + 'a) {
    let tree_cx = use_context::<TreeContext>(cx);

    let f: Box<dyn FnMut(&Action)> = Box::new(on_action);
    let f: Box<dyn FnMut(&Action)> = unsafe { mem::transmute(f) };

    use_ref(cx, || {
        tree_cx.inner.borrow_mut().listeners.insert(id, vec![f]);
    });
}

#[derive(Clone)]
pub struct Text<T>(pub T);

impl<T> Text<T> {
    pub fn new(content: T) -> Self {
        Self(content)
    }
}

unsafe impl<T: Data> Data for Text<T> {
    type Id = Text<T::Id>;
}

impl<T> Compose for Text<T>
where
    T: Data + Deref<Target = str>,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);

        let mut tree_inner = tree_cx.inner.borrow_mut();

        let child_idx = tree_inner.child_idx;
        tree_inner.child_idx += 1;

        let widget_cell = &mut tree_inner.widget;

        let mut is_build = false;
        let id = use_ref(&cx, || {
            let mut widget = widget_cell.as_mut().unwrap();

            let label = Label::new(cx.me().0.to_string());
            let pod = WidgetPod::new(label).boxed();
            let id = pod.id();
            FlexWidget::insert_child_pod(&mut widget, child_idx, pod);

            is_build = true;
            id
        });

        // TODO don't clone
        use_memo(&cx, cx.me().0.to_string(), || {
            if !is_build {
                let mut widget = widget_cell.as_mut().unwrap();

                let mut child = FlexWidget::child_mut(&mut widget, child_idx).unwrap();
                let mut label = child.downcast::<Label>();

                Label::set_text(&mut label, cx.me().0.to_string());
            }
        });

        drop(tree_inner);
    }
}

#[derive(Clone)]
pub struct Button<'a, T> {
    content: T,
    on_press: RefCell<Option<Rc<dyn Fn() + 'a>>>,
}

impl<'a, T> Button<'a, T> {
    pub fn new(content: T) -> Self {
        Self {
            content,
            on_press: RefCell::new(None),
        }
    }

    pub fn on_press(mut self, on_press: impl Fn() + 'a) -> Self {
        self.on_press = RefCell::new(Some(Rc::new(on_press)));
        self
    }
}

unsafe impl<T: Data> Data for Button<'_, T> {
    type Id = Text<T::Id>;
}

impl<T> Compose for Button<'_, T>
where
    T: Data + Deref<Target = str>,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);

        let mut tree_inner = tree_cx.inner.borrow_mut();

        let child_idx = tree_inner.child_idx;
        tree_inner.child_idx += 1;

        let widget_cell = &mut tree_inner.widget;

        let mut is_build = false;
        let id = use_ref(&cx, || {
            let mut widget = widget_cell.as_mut().unwrap();

            let label = ButtonWidget::new(cx.me().content.to_string());
            let pod = WidgetPod::new(label).boxed();
            let id = pod.id();
            FlexWidget::insert_child_pod(&mut widget, child_idx, pod);

            is_build = true;
            id
        });

        // TODO don't clone
        use_memo(&cx, cx.me().content.to_string(), || {
            if !is_build {
                let mut widget = widget_cell.as_mut().unwrap();

                let mut child = FlexWidget::child_mut(&mut widget, child_idx).unwrap();
                let mut label = child.downcast::<ButtonWidget>();

                ButtonWidget::set_text(&mut label, cx.me().content.to_string());
            }
        });

        drop(tree_inner);

        let f = cx.me().on_press.take();

        use_listener(&cx, *id, move |action| {
            if let Some(f) = &f {
                f();
            }
        });
    }
}

pub struct Flex<C>(pub C);

impl<C> Flex<C> {
    pub fn column(content: C) -> Self {
        Self(content)
    }
}

unsafe impl<C: Data> Data for Flex<C> {
    type Id = Flex<C::Id>;
}

impl<C> Compose for Flex<C>
where
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);
        let mut tree_inner = tree_cx.inner.borrow_mut();
        tree_inner.child_idx = 0;

        Ref::map(cx.me(), |me| &me.0)
    }
}
