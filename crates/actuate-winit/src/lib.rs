use actuate_core::{prelude::*, Composer, Update, Updater};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::WindowId,
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

pub struct Handler {
    composer: Composer,
}

impl Handler {
    pub fn new(composer: Composer) -> Self {
        Self { composer }
    }
}

impl ApplicationHandler<Update> for Handler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(feature = "tracing")]
        tracing::info!("Resumed");

        self.composer.compose();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: Update) {
        #[cfg(feature = "tracing")]
        tracing::info!("Update");

        unsafe { event.apply() };

        self.composer.compose();
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }
}

pub fn run(content: impl Compose + 'static) {
    let event_loop = EventLoop::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();

    let composer = Composer::with_updater(content, EventLoopUpdater { proxy });
    let mut handler = Handler::new(composer);

    event_loop.run_app(&mut handler).unwrap();
}
