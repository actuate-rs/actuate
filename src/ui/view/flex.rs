use crate::{
    prelude::*,
    ui::{LayoutContext, WindowContext},
};
use taffy::{FlexDirection, Style};

#[derive(Data)]
pub struct Flex<C> {
    style: Style,
    content: C,
}

impl<C> Flex<C> {
    pub fn new(style: Style, content: C) -> Self {
        Self { style, content }
    }

    pub fn column(content: C) -> Self {
        Self::new(
            Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            content,
        )
    }

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
