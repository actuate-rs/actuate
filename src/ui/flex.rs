use crate::{prelude::*, LayoutContext, WindowContext};
use taffy::{FlexDirection, Style};

use super::use_layout;

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
        let (id, _layout) = use_layout(&cx, cx.me().style.clone());

        use_provider(&cx, || LayoutContext { parent_id: id });

        Ref::map(cx.me(), |me| &me.content)
    }
}
