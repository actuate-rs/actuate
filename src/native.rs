use std::cell::{RefCell, UnsafeCell};
use std::mem;
use std::ops::Deref;
use std::rc::Rc;

use crate::{use_context, use_provider, Compose, Composer, Data, Scope};
use masonry::event_loop_runner::{MasonryState, MasonryUserEvent};
use masonry::widget::{Flex, RootWidget, Textbox, WidgetMut};
use masonry::{Action, AppDriver, Color, Widget, WidgetId};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

pub fn run(compose: impl Compose + 'static) {
    let main_widget = Flex::column();

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

#[derive(Clone)]
pub struct TreeContext {
    proxy: EventLoopProxy<MasonryUserEvent>,
    widget: Rc<RefCell<Option<WidgetMut<'static, RootWidget<Flex>>>>>,
}

pub struct Tree<C> {
    content: C,
    tree_cx: TreeContext,
}

unsafe impl<C: Data> Data for Tree<C> {}

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
            widget: Rc::new(RefCell::new(None)),
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
    fn on_action(
        &mut self,
        masonry_ctx: &mut masonry::DriverCtx<'_>,
        widget_id: masonry::WidgetId,
        action: masonry::Action,
    ) {
        let widget: WidgetMut<'_, RootWidget<Flex>> = masonry_ctx.get_root::<RootWidget<Flex>>();
        let widget: WidgetMut<'static, RootWidget<Flex>> = unsafe { mem::transmute(widget) };

        *self.tree_cx.widget.borrow_mut() = Some(widget);

        self.composer.rebuild();

        *self.tree_cx.widget.borrow_mut() = None;

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
        }
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        state.get_root().edit_root_widget(|mut root| {
            let widget: WidgetMut<'_, RootWidget<Flex>> = root.downcast();
            let widget: WidgetMut<'static, RootWidget<Flex>> = unsafe { mem::transmute(widget) };

            *self.tree_cx.widget.borrow_mut() = Some(widget);

            self.composer.build();

            *self.tree_cx.widget.borrow_mut() = None;
        });

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
        }
    }
}

pub struct Text<T>(pub T);

unsafe impl<T: Data> Data for Text<T> {}

impl<T> Compose for Text<T>
where
    T: Data + Deref<Target = str>,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);
        tree_cx
            .proxy
            .send_event(MasonryUserEvent::Action(
                Action::Other(Box::new(())),
                WidgetId::reserved(u16::MAX),
            ))
            .unwrap();

        let mut widget_cell = tree_cx.widget.borrow_mut();
        if let Some(widget) = &mut *widget_cell {
            let mut flex = RootWidget::child_mut(widget);

            let child = masonry::widget::Label::new(cx.me().0.to_string());
            Flex::add_child(&mut flex, child);
            dbg!("WAT");
        }
    }
}
