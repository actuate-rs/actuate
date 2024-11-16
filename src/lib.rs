use actuate_core::prelude::*;
use canvas::CanvasContext;
use parley::{FontStack, Rect};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem,
    rc::Rc,
};
use taffy::{Layout, NodeId, TaffyTree};
use text::{FontContext, IntoFontStack, TextContext};
use vello::{
    kurbo::{Affine, Vec2},
    peniko::{Color, Fill},
    Scene,
};
use winit::event::{ElementState, MouseButton};

pub use actuate_core as core;

mod canvas;
pub use self::canvas::Canvas;

mod flex;
pub use self::flex::Flex;

mod text;
pub use self::text::{use_font, Text};

mod window;
pub use self::window::Window;

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::{use_font, Canvas, Flex, Text, View, Window};

    pub use parley::GenericFamily;

    pub use taffy::prelude::*;

    pub use vello::peniko::Color;

    pub use winit::window::WindowAttributes;
}

pub struct WindowContext {
    scene: RefCell<Scene>,
    taffy: RefCell<TaffyTree>,
    parent_key: RefCell<NodeId>,
    is_changed: Cell<bool>,
    is_layout_changed: Cell<bool>,
    canvas_update_fns: RefCell<HashMap<NodeId, Box<dyn Fn()>>>,
    listeners: Rc<RefCell<HashMap<NodeId, Vec<Rc<dyn Fn(MouseButton, ElementState, Vec2)>>>>>,
    pending_listeners: Rc<RefCell<Vec<Rc<dyn Fn(MouseButton, ElementState, Vec2)>>>>,
    base_color: Cell<Color>,
}

struct RenderRoot<C> {
    content: C,
}

unsafe impl<C: Data> Data for RenderRoot<C> {
    type Id = RenderRoot<C::Id>;
}

impl<C: Compose> Compose for RenderRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, CanvasContext::default);

        use_provider(&cx, FontContext::default);

        let text_context = use_context::<TextContext>(&cx).map(|rc| (*rc).clone());
        use_provider(&cx, || text_context.unwrap_or_default());

        Ref::map(cx.me(), |me| &me.content)
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}

pub trait View: Compose {
    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> WithState<Clickable<'a>, Self> {
        WithState {
            state: Clickable::new(on_click),
            content: self,
        }
    }

    fn with_state<T: State>(self, state: T) -> WithState<T, Self> {
        WithState::new(state, self)
    }

    fn font(self, font_stack: impl IntoFontStack<'static>) -> WithState<FontStackState, Self> {
        self.with_state(FontStackState {
            font_stack: font_stack.into_font_stack(),
        })
    }

    fn font_size(self, font_size: f32) -> WithState<FontSize, Self> {
        self.with_state(FontSize { font_size })
    }

    fn draw<D: Draw + 'static>(self, draw: D) -> WithState<DrawState<D>, Self> {
        self.with_state(DrawState::new(draw))
    }

    fn background_color(self, color: Color) -> WithState<DrawState<BackgroundColor>, Self> {
        self.draw(BackgroundColor { color })
    }
}

impl<C: Compose> View for C {}

pub trait State {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>);
}

pub struct WithState<T, C> {
    state: T,
    content: C,
}

impl<T, C> WithState<T, C> {
    pub fn new(state: T, content: C) -> Self {
        Self { state, content }
    }
}

unsafe impl<T: Data, C: Data> Data for WithState<T, C> {
    type Id = WithState<T::Id, C::Id>;
}

impl<T: State + Data, C: Compose> Compose for WithState<T, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        unsafe { cx.me().state.use_state(mem::transmute(&**cx)) }

        Ref::map(cx.me(), |me| &me.content)
    }
}

pub struct Clickable<'a> {
    on_click: Rc<dyn Fn() + 'a>,
}

impl<'a> Clickable<'a> {
    pub fn new(on_click: impl Fn() + 'a) -> Self {
        Self {
            on_click: Rc::new(on_click),
        }
    }
}

unsafe impl Data for Clickable<'_> {
    type Id = Clickable<'static>;
}

impl State for Clickable<'_> {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let renderer_cx = use_context::<WindowContext>(&cx).unwrap();

        use_ref(cx, || {
            let is_pressed = Cell::new(false);

            // Safety: `f` is removed from `canvas_update_fns` on drop.
            let f: Rc<dyn Fn() + 'static> = unsafe { mem::transmute(self.on_click.clone()) };
            let f = Rc::new(move |button, state, _| {
                if button != MouseButton::Left {
                    return;
                }

                if state == ElementState::Pressed {
                    is_pressed.set(true)
                } else if is_pressed.get() && state == ElementState::Released {
                    f()
                }
            });

            renderer_cx.pending_listeners.borrow_mut().push(f);
        });
    }
}

#[derive(Data)]
pub struct FontSize {
    pub font_size: f32,
}

impl State for FontSize {
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
pub struct FontStackState {
    pub font_stack: FontStack<'static>,
}

impl State for FontStackState {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let text_cx = use_context::<TextContext>(&cx).unwrap();

        use_provider(cx, || TextContext {
            color: text_cx.color,
            font_size: text_cx.font_size,
            font_stack: self.font_stack.clone(),
        });
    }
}

pub trait Draw {
    fn pre_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }

    fn post_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }
}

pub struct DrawState<T> {
    draw: Rc<T>,
}

impl<T> DrawState<T> {
    pub fn new(draw: T) -> Self {
        Self {
            draw: Rc::new(draw),
        }
    }
}

unsafe impl<T: Data> Data for DrawState<T> {
    type Id = DrawState<T::Id>;
}

impl<T: Draw + 'static> State for DrawState<T> {
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

#[derive(Data)]
pub struct BackgroundColor {
    pub color: Color,
}

impl Draw for BackgroundColor {
    fn pre_process(&self, layout: &Layout, scene: &mut Scene) {
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            self.color,
            None,
            &Rect::new(0., 0., layout.size.width as _, layout.size.height as _),
        );
    }
}
