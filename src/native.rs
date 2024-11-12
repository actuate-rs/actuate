use crate::{Compose, Composer};
use masonry::event_loop_runner::MasonryState;
use masonry::widget::{RootWidget, Textbox};
use masonry::AppDriver;
use winit::window::Window;

pub fn run(compose: impl Compose + 'static) {
    let main_widget = Textbox::new("");

    masonry::event_loop_runner::run(
        masonry::event_loop_runner::EventLoop::with_user_event(),
        Window::default_attributes(),
        RootWidget::new(main_widget),
        Driver::new(compose),
    )
    .unwrap();
}

pub struct Driver {
    composer: Composer,
}

impl Driver {
    pub fn new(compose: impl Compose + 'static) -> Self {
        Self {
            composer: Composer::new(compose),
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
    }

    fn on_start(&mut self, state: &mut MasonryState) {
        self.composer.build();
    }
}
