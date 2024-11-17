use crate::{
    draw::{BackgroundColor, Draw},
    prelude::*,
    ui::{
        canvas::CanvasContext,
        text::{IntoFontStack, TextContext},
    },
    WindowContext,
};
use parley::FontStack;
use std::{cell::Cell, mem, rc::Rc};
use vello::kurbo::Vec2;
use winit::event::{ElementState, MouseButton};

pub trait View: Compose {
    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> Modified<Clickable<'a>, Self> {
        Modified::new(Clickable::new(on_click), self)
    }

    fn with_state<T: Modify>(self, state: T) -> Modified<T, Self> {
        Modified::new(state, self)
    }

    fn font(self, font_stack: impl IntoFontStack<'static>) -> Modified<Font, Self> {
        self.with_state(Font {
            font_stack: font_stack.into_font_stack(),
        })
    }

    fn font_size(self, font_size: f32) -> Modified<FontSize, Self> {
        self.with_state(FontSize { font_size })
    }

    fn draw<D: Draw + 'static>(self, draw: D) -> Modified<DrawModifier<D>, Self> {
        self.with_state(DrawModifier::new(draw))
    }

    fn background_color(self, color: Color) -> Modified<DrawModifier<BackgroundColor>, Self> {
        self.draw(BackgroundColor { color })
    }
}

impl<C: Compose> View for C {}

pub trait Modify {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>);
}

pub struct Modified<T, C> {
    state: T,
    content: C,
}

impl<T, C> Modified<T, C> {
    pub fn new(state: T, content: C) -> Self {
        Self { state, content }
    }
}

unsafe impl<T: Data, C: Data> Data for Modified<T, C> {
    type Id = Modified<T::Id, C::Id>;
}

impl<T: Modify + Data, C: Compose> Compose for Modified<T, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        unsafe { cx.me().state.use_state(mem::transmute(&**cx)) }

        Ref::map(cx.me(), |me| &me.content)
    }
}

#[derive(Data)]
pub struct Clickable<'a> {
    on_click: Box<dyn Fn() + 'a>,
}

impl<'a> Clickable<'a> {
    pub fn new(on_click: impl Fn() + 'a) -> Self {
        Self {
            on_click: Box::new(on_click),
        }
    }
}

impl Modify for Clickable<'_> {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let renderer_cx = use_context::<WindowContext>(&cx).unwrap();

        use_ref(cx, || {
            let is_pressed = Cell::new(false);

            // Safety: `f` is removed from `canvas_update_fns` on drop.

            let f: Rc<dyn Fn(MouseButton, ElementState, Vec2)> =
                Rc::new(move |button, state, _| {
                    if button != MouseButton::Left {
                        return;
                    }

                    if state == ElementState::Pressed {
                        is_pressed.set(true)
                    } else if is_pressed.get() && state == ElementState::Released {
                        (self.on_click)()
                    }
                });
            let f: Rc<dyn Fn(MouseButton, ElementState, Vec2)> = unsafe { mem::transmute(f) };

            renderer_cx.pending_listeners.borrow_mut().push(f);
        });
    }
}

#[derive(Data)]
pub struct FontSize {
    pub font_size: f32,
}

impl Modify for FontSize {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let text_cx = use_context::<TextContext>(&cx).unwrap();

        use_provider(&cx, || TextContext {
            color: text_cx.color,
            font_size: self.font_size,
            font_stack: text_cx.font_stack.clone(),
        });
    }
}

#[derive(Data)]
pub struct Font {
    pub font_stack: FontStack<'static>,
}

impl Modify for Font {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let text_cx = use_context::<TextContext>(&cx).unwrap();

        use_provider(cx, || TextContext {
            color: text_cx.color,
            font_size: text_cx.font_size,
            font_stack: self.font_stack.clone(),
        });
    }
}

pub struct DrawModifier<T> {
    draw: Rc<T>,
}

impl<T> DrawModifier<T> {
    pub fn new(draw: T) -> Self {
        Self {
            draw: Rc::new(draw),
        }
    }
}

unsafe impl<T: Data> Data for DrawModifier<T> {
    type Id = DrawModifier<T::Id>;
}

impl<T: Draw + 'static> Modify for DrawModifier<T> {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let canvas_cx = use_context::<CanvasContext>(&cx).unwrap();

        let draw = self.draw.clone();
        use_provider(cx, move || {
            let canvas_cx = (*canvas_cx).clone();
            canvas_cx.draws.borrow_mut().push(draw.clone());
            canvas_cx
        });
    }
}
