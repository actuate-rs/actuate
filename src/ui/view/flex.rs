use crate::{
    prelude::*,
    ui::{LayoutContext, WindowContext},
};
use taffy::{FlexDirection, Style};

/// Flex composable.
#[derive(Data)]
pub struct Flex<C> {
    style: Style,
    content: C,
}

impl<C> Flex<C> {
    /// Create a new flex from its style and the given `content`.
    pub fn new(style: Style, content: C) -> Self {
        Self { style, content }
    }

    /// Create a new flex column from the given `content`.
    pub fn column(content: C) -> Self {
        Self::new(
            Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            content,
        )
    }

    /// Create a new flex row from the given `content`.
    pub fn row(content: C) -> Self {
        Self::new(
            Style {
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            content,
        )
    }
}

impl<C: Compose> Compose for Flex<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let layout_cx = use_context::<LayoutContext>(&cx).unwrap();
        let renderer_cx = use_context::<WindowContext>(&cx).unwrap();

        use_provider(&cx, || {
            let id = renderer_cx
                .taffy
                .borrow_mut()
                .new_leaf(cx.me().style.clone())
                .unwrap();
            renderer_cx
                .taffy
                .borrow_mut()
                .add_child(layout_cx.parent_id, id)
                .unwrap();
            LayoutContext { parent_id: id }
        });

        Ref::map(cx.me(), |me| &me.content)
    }
}
