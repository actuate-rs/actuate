use crate::{
    prelude::*,
    ui::{use_layout, LayoutContext},
};
use taffy::{FlexDirection, Style};

/// Flex composable.
#[derive(Data)]
#[must_use = "Composables do nothing unless composed with `actuate::run` or returned from other composables"]
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
        let (id, _layout) = use_layout(&cx, cx.me().style.clone());

        use_provider(&cx, || LayoutContext { parent_id: id });

        Ref::map(cx.me(), |me| &me.content)
    }
}
