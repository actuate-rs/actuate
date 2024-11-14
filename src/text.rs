use crate::Canvas;
use actuate_core::prelude::*;
use masonry::{
    parley::{fontique::Weight, FontContext},
    text2::TextLayout,
    vello::peniko::Color,
    Point,
};
use std::fmt;
use taffy::{Size, Style};

pub struct Text<T>(pub T);

unsafe impl<T: Data> Data for Text<T> {
    type Id = Text<T::Id>;
}

impl<T> Compose for Text<T>
where
    T: Data + fmt::Display,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        Canvas::new(
            Style {
                size: Size::from_lengths(500., 200.),
                ..Default::default()
            },
            move |_layout, scene| {
                let mut font_cx = FontContext::default();
                font_cx
                    .collection
                    .register_fonts(include_bytes!("../assets/FiraMono-Medium.ttf").to_vec());

                let mut text_layout = TextLayout::new(format!("{}", cx.me().0), 50.);

                text_layout.set_font(masonry::parley::style::FontStack::Single(
                    masonry::parley::style::FontFamily::Named("Fira Mono"),
                ));
                text_layout.set_brush(Color::RED);
                text_layout.set_weight(Weight::MEDIUM);
                text_layout.rebuild(&mut font_cx);

                text_layout.draw(scene, Point::new(50., 50.));
            },
        )
    }
}
