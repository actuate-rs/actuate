use crate::Canvas;
use actuate_core::{prelude::*, ScopeState};
use masonry::{
    parley::{
        self,
        style::{FontFamily, FontStack},
    },
    text2::TextLayout,
    vello::peniko::Color,
    Point,
};
use std::{cell::RefCell, fmt};
use taffy::{Size, Style};

#[derive(Default)]
pub struct FontContext {
    inner: RefCell<parley::FontContext>,
}

pub fn use_font<R>(cx: &ScopeState, make_font: impl FnOnce() -> R)
where
    R: Into<Vec<u8>>,
{
    let font_cx = use_context::<FontContext>(cx);

    use_ref(cx, || {
        font_cx
            .inner
            .borrow_mut()
            .collection
            .register_fonts(make_font().into());
    });
}

pub trait IntoFontStack<'a> {
    fn into_font_stack(self) -> FontStack<'a>;
}

impl<'a> IntoFontStack<'a> for FontStack<'a> {
    fn into_font_stack(self) -> FontStack<'a> {
        self
    }
}

impl<'a> IntoFontStack<'a> for &'a str {
    fn into_font_stack(self) -> FontStack<'a> {
        FontStack::Single(FontFamily::Named(self))
    }
}

pub struct Text<T> {
    content: T,
    font_stack: FontStack<'static>,
}

impl<T> Text<T> {
    pub fn new(content: T) -> Self {
        Self {
            content,
            font_stack: FontStack::Single(FontFamily::Generic(
                parley::style::GenericFamily::SansSerif,
            )),
        }
    }

    pub fn with_font(mut self, font_stack: impl IntoFontStack<'static>) -> Self {
        self.font_stack = font_stack.into_font_stack();
        self
    }
}

unsafe impl<T: Data> Data for Text<T> {
    type Id = Text<T::Id>;
}

impl<T> Compose for Text<T>
where
    T: Data + fmt::Display,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let font_cx = use_context::<FontContext>(&cx);

        let text_layout = use_ref(&cx, || {
            let mut text_layout = TextLayout::new(format!("{}", cx.me().content), 50.);
            text_layout.rebuild(&mut font_cx.inner.borrow_mut());

            RefCell::new(text_layout)
        });

        Canvas::new(
            Style {
                size: Size::from_lengths(
                    text_layout.borrow().full_size().width as _,
                    text_layout.borrow().full_size().height as _,
                ),
                ..Default::default()
            },
            move |_layout, scene| {
                let mut text_layout = text_layout.borrow_mut();

                text_layout.set_font(cx.me().font_stack);
                text_layout.set_brush(Color::WHITE);

                text_layout.rebuild(&mut font_cx.inner.borrow_mut());
                text_layout.draw(scene, Point::default());
            },
        )
    }
}
