//! # Actuate
//! Actuate is a native, declarative, and friendly user-interface (UI) framework.
//!
//! ## Hooks
//! Functions that begin with `use_` are called `hooks` in Actuate.
//! Hooks are used to manage state and side effects in composables.
//!
//! Hooks must be used in the same order for every re-compose.
//! Don’t use hooks inside loops, conditions, nested functions, or match blocks.
//! Instead, always use hooks at the top level of your composable, before any early returns.
#![cfg_attr(docsrs, feature(doc_cfg))]

use actuate_core::{prelude::*, Executor};
use ui::RenderRoot;

pub use actuate_core as core;

pub mod ui;

#[cfg(feature = "event-loop")]
#[cfg_attr(docsrs, doc(cfg(feature = "event-loop")))]
/// System event loop for windowing.
pub mod event_loop;

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::ui::{
        view::{use_font, Canvas, Flex, Text, View, Window},
        Draw,
    };

    pub use parley::GenericFamily;

    pub use taffy::prelude::*;

    pub use vello::peniko::Color;

    pub use winit::window::WindowAttributes;
}

pub fn run(content: impl Compose + 'static) {
    event_loop::run(RenderRoot { content });
}

pub fn run_with_executor(content: impl Compose + 'static, executor: impl Executor + 'static) {
    event_loop::run_with_executor(RenderRoot { content }, executor);
}
