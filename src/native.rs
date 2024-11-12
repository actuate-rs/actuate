use crate::{use_context, use_memo, use_provider, use_ref, Compose, Composer, Data, Ref, Scope};
use masonry::event_loop_runner::{MasonryState, MasonryUserEvent};
use masonry::widget::{Flex as FlexWidget, Label, RootWidget, WidgetMut};
use masonry::{Action, AppDriver, Color, DriverCtx, WidgetId};
use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

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

        &cx.me.content
    }
}

pub struct Driver {
    composer: Composer,
    tree_cx: TreeContext,
}

impl Driver {
    pub fn new(content: impl Compose + 'static, proxy: EventLoopProxy<MasonryUserEvent>) -> Self {
        let tree_cx = TreeContext {
            proxy,
            inner: Rc::new(RefCell::new(Inner {
                widget: None,
                child_idx: 0,
            })),
        };

        Self {
            composer: Composer::new(Tree {
                content,
                tree_cx: tree_cx.clone(),
            }),
            tree_cx,
        }
    }
}

impl AppDriver for Driver {
    fn on_action(&mut self, masonry_ctx: &mut DriverCtx, _widget_id: WidgetId, _action: Action) {
        let mut root = masonry_ctx.get_root::<RootWidget<FlexWidget>>();
        let flex = RootWidget::child_mut(&mut root);
        let widget: WidgetMut<'static, FlexWidget> = unsafe { mem::transmute(flex) };

        self.tree_cx.inner.borrow_mut().widget = Some(widget);
        self.tree_cx.inner.borrow_mut().child_idx = 0;

        self.composer.compose();

        self.tree_cx.inner.borrow_mut().widget = None;

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
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

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
        }
    }
}

#[derive(Clone)]
pub struct Text<T>(pub T);

unsafe impl<T: Data> Data for Text<T> {
    type Id = Text<T::Id>;
}

impl<T> Compose for Text<T>
where
    T: Data + Deref<Target = str>,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);
        /* tree_cx
        .proxy
        .send_event(MasonryUserEvent::Action(
            Action::Other(Box::new(())),
            WidgetId::reserved(u16::MAX),
        ))
        .unwrap(); */

        let mut tree_inner = tree_cx.inner.borrow_mut();

        let child_idx = tree_inner.child_idx;
        tree_inner.child_idx += 1;

        let widget_cell = &mut tree_inner.widget;

        let mut is_build = false;
        use_ref(&cx, || {
            let mut widget = widget_cell.as_mut().unwrap();

            let label = Label::new(cx.me().0.to_string());
            FlexWidget::add_child(&mut widget, label);

            is_build = true;
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
    }
}

pub struct Flex<C>(pub C);

unsafe impl<C: Data> Data for Flex<C> {
    type Id = Flex<C::Id>;
}

impl<C> Compose for Flex<C>
where
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        Ref::map(cx.me(), |me| &me.0)
    }
}
