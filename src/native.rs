use crate::{use_context, use_provider, Compose, Composer, Data, Scope};
use masonry::event_loop_runner::{MasonryState, MasonryUserEvent};
use masonry::widget::{RootWidget, Textbox};
use masonry::{Action, AppDriver, Color, WidgetId};
use winit::event_loop::{EventLoop, EventLoopProxy};
use winit::window::Window;

pub fn run(compose: impl Compose + 'static) {
    let main_widget = Textbox::new("");

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
}

impl Driver {
    pub fn new(content: impl Compose + 'static, proxy: EventLoopProxy<MasonryUserEvent>) -> Self {
        Self {
            composer: Composer::new(Tree {
                content,
                tree_cx: TreeContext { proxy },
            }),
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
        dbg!("action");
        
        self.composer.rebuild();

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
        }
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        self.composer.build();

        while let Ok(mut update) = self.composer.rx.try_recv() {
            (update.f)();
        }
    }
}

pub struct Text {}

unsafe impl Data for Text {}

impl Compose for Text {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let tree_cx = use_context::<TreeContext>(&cx);
        tree_cx
            .proxy
            .send_event(MasonryUserEvent::Action(
                Action::Other(Box::new(())),
                WidgetId::reserved(u16::MAX),
            ))
            .unwrap();
    }
}
