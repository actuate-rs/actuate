use crate::{prelude::*, LayoutContext, WindowContext};

pub(crate) mod canvas;
pub use self::canvas::Canvas;

mod flex;
pub use self::flex::Flex;

pub(crate) mod text;
pub use self::text::{use_font, Text};

mod window;
pub use self::window::Window;

/// Use a new layout node.
pub fn use_layout(cx: ScopeState, style: Style) -> (NodeId, Layout) {
    let layout_cx = use_context::<LayoutContext>(&cx).unwrap();
    let renderer_cx = use_context::<WindowContext>(&cx).unwrap();

    let parent_key = layout_cx.parent_id;
    let key = *use_ref(&cx, || {
        let key = renderer_cx
            .taffy
            .borrow_mut()
            .new_leaf(style.clone())
            .unwrap();
        renderer_cx
            .taffy
            .borrow_mut()
            .add_child(parent_key, key)
            .unwrap();

        renderer_cx.is_layout_changed.set(true);

        key
    });

    let last_style = use_ref(&cx, || style.clone());
    if style != *last_style {
        renderer_cx.is_layout_changed.set(true);
        renderer_cx
            .taffy
            .borrow_mut()
            .set_style(key, style.clone())
            .unwrap();
    }

    use_drop(&cx, move || {
        renderer_cx.taffy.borrow_mut().remove(key).unwrap();
        renderer_cx.listeners.borrow_mut().remove(&key);
    });

    let layout = *renderer_cx.taffy.borrow().layout(key).unwrap();
    (key, layout)
}
