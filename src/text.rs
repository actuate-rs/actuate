use crate::Canvas;
use actuate_core::{prelude::*, ScopeState};
use parley::{
    Alignment, FontFamily, FontStack, GenericFamily, LayoutContext, PositionedLayoutItem,
    StyleProperty,
};
use std::{cell::RefCell, fmt};
use taffy::{Size, Style};
use vello::{
    self,
    kurbo::Affine,
    peniko::{Color, Fill},
    Glyph,
};

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
        FontStack::Single(FontFamily::Named(self.into()))
    }
}

impl<'a> IntoFontStack<'a> for GenericFamily {
    fn into_font_stack(self) -> FontStack<'a> {
        FontStack::Single(FontFamily::Generic(self))
    }
}

#[derive(Clone, PartialEq)]
pub struct TextContext {
    pub color: Color,
    pub font_size: f32,
    pub font_stack: FontStack<'static>,
}

impl Default for TextContext {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            font_size: 18.,
            font_stack: FontStack::Single(FontFamily::Generic(
                parley::style::GenericFamily::SansSerif,
            )),
        }
    }
}

#[derive(Data)]
pub struct Text<T> {
    content: T,
}

impl<T> Text<T> {
    pub fn new(content: T) -> Self {
        Self { content }
    }
}

impl<T> Compose for Text<T>
where
    T: Data + fmt::Display,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let font_cx = use_context::<FontContext>(&cx);
        let text_cx = use_context::<TextContext>(&cx);
        let content = format!("{}", cx.me().content);

        let text_layout = use_memo(&cx, (content.clone(), text_cx.clone()), || {
            let mut font_cx = font_cx.inner.borrow_mut();

            let mut layout_cx = LayoutContext::<Color>::new();
            let mut text_layout = layout_cx.ranged_builder(&mut font_cx, &content, 1.);
            text_layout.push_default(StyleProperty::Brush(text_cx.color));
            text_layout.push_default(StyleProperty::FontSize(text_cx.font_size));
            text_layout.push_default(text_cx.font_stack.clone());

            let mut layout = text_layout.build(&content);
            layout.break_all_lines(None);
            layout.align(None, Alignment::Start);
            layout
        });

        Memo::new(
            content.clone(),
            Canvas::new(
                Style {
                    size: Size::from_lengths(text_layout.full_width(), text_layout.height()),
                    ..Default::default()
                },
                move |_layout, scene| {
                    for line in text_layout.lines() {
                        for item in line.items() {
                            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                                continue;
                            };
                            let mut x = glyph_run.offset();
                            let y = glyph_run.baseline();
                            let run = glyph_run.run();
                            let font = run.font();
                            let font_size = run.font_size();
                            let synthesis = run.synthesis();
                            let glyph_xform = synthesis
                                .skew()
                                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                            let coords = run
                                .normalized_coords()
                                .iter()
                                .map(|coord| {
                                    vello::skrifa::instance::NormalizedCoord::from_bits(*coord)
                                })
                                .collect::<Vec<_>>();
                            scene
                                .draw_glyphs(font)
                                .brush(Color::BLACK)
                                .hint(true)
                                .glyph_transform(glyph_xform)
                                .font_size(font_size)
                                .normalized_coords(&coords)
                                .draw(
                                    Fill::NonZero,
                                    glyph_run.glyphs().map(|glyph| {
                                        let gx = x + glyph.x;
                                        let gy = y - glyph.y;
                                        x += glyph.advance;
                                        Glyph {
                                            id: glyph.id as _,
                                            x: gx,
                                            y: gy,
                                        }
                                    }),
                                );
                        }
                    }
                },
            ),
        )
    }
}
