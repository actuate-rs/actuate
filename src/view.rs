use crate::{
    draw::{BackgroundColor, Draw},
    prelude::*,
    ui::{
        canvas::CanvasContext,
        text::{IntoFontStack, TextContext},
    },
    Event, WindowContext,
};
use parley::FontStack;
use std::{cell::RefCell, mem, rc::Rc};
use winit::event::{ElementState, MouseButton};

/// Composable view modifiers.
pub trait View: Compose {
    fn modify<T: Modify>(self, state: T) -> Modified<T, Self> {
        Modified::new(state, self)
    }

    fn on_event<H: Handler>(self, on_event: H) -> Modified<OnEvent<H>, Self> {
        self.modify(OnEvent::new(on_event))
    }

    fn on_mouse_in<'a>(
        self,
        on_mouse_in: impl Fn() + 'a,
    ) -> Modified<OnEvent<OnMouseIn<'a>>, Self> {
        self.on_event(OnMouseIn::new(on_mouse_in))
    }

    fn on_mouse_out<'a>(
        self,
        on_mouse_out: impl Fn() + 'a,
    ) -> Modified<OnEvent<OnMouseOut<'a>>, Self> {
        self.on_event(OnMouseOut::new(on_mouse_out))
    }

    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> Modified<OnEvent<Clickable<'a>>, Self> {
        self.on_event(Clickable::new(on_click))
    }

    fn font(self, font_stack: impl IntoFontStack<'static>) -> Modified<Font, Self> {
        self.modify(Font {
            font_stack: font_stack.into_font_stack(),
        })
    }

    fn color(self, color: Color) -> Modified<FontColor, Self> {
        self.modify(FontColor { color })
    }

    fn font_size(self, font_size: f32) -> Modified<FontSize, Self> {
        self.modify(FontSize { font_size })
    }

    fn draw<D: Draw + 'static>(self, draw: D) -> Modified<DrawModifier<D>, Self> {
        self.modify(DrawModifier::new(draw))
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
        // Safety: `state` is guranteed to live as long as `cx`.
        let state_cx: ScopeState = unsafe { mem::transmute(&**cx) };

        cx.me().state.use_state(state_cx);

        Ref::map(cx.me(), |me| &me.content)
    }

    fn name() -> std::borrow::Cow<'static, str> {
        C::name()
    }
}

pub trait Handler {
    type State: 'static;

    fn build(&self) -> Self::State;

    fn handle(&self, state: &mut Self::State, event: Event);
}

impl<F: Fn(Event)> Handler for F {
    type State = ();

    fn build(&self) -> Self::State {}

    fn handle(&self, state: &mut Self::State, event: Event) {
        let _ = state;

        self(event)
    }
}

pub struct OnEvent<H> {
    on_event: RefCell<H>,
}

impl<H> OnEvent<H> {
    pub fn new(on_event: H) -> Self {
        Self {
            on_event: RefCell::new(on_event),
        }
    }
}

unsafe impl<H: Data> Data for OnEvent<H> {
    type Id = OnEvent<H::Id>;
}

impl<H: Handler> Modify for OnEvent<H> {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let renderer_cx = use_context::<WindowContext>(cx).unwrap();

        let state = use_ref(cx, || RefCell::new(self.on_event.borrow_mut().build()));
        use_ref(cx, || {
            // Safety: `f` is removed from `canvas_update_fns` on drop.
            let f: Rc<dyn Fn(Event)> = Rc::new(move |msg| {
                self.on_event
                    .borrow_mut()
                    .handle(&mut state.borrow_mut(), msg)
            });
            let f: Rc<dyn Fn(Event)> = unsafe { mem::transmute(f) };

            renderer_cx.pending_listeners.borrow_mut().push(f);
        });
    }
}

#[derive(Data)]
pub struct OnMouseIn<'a> {
    on_mouse_in: Box<dyn Fn() + 'a>,
}

impl<'a> OnMouseIn<'a> {
    pub fn new(on_mouse_in: impl Fn() + 'a) -> Self {
        Self {
            on_mouse_in: Box::new(on_mouse_in),
        }
    }
}

impl Handler for OnMouseIn<'_> {
    type State = ();

    fn build(&self) -> Self::State {}

    fn handle(&self, state: &mut Self::State, event: Event) {
        let _ = state;

        if let Event::MouseIn = event {
            (self.on_mouse_in)()
        }
    }
}

#[derive(Data)]
pub struct OnMouseOut<'a> {
    on_mouse_out: Box<dyn Fn() + 'a>,
}

impl<'a> OnMouseOut<'a> {
    pub fn new(on_mouse_out: impl Fn() + 'a) -> Self {
        Self {
            on_mouse_out: Box::new(on_mouse_out),
        }
    }
}

impl Handler for OnMouseOut<'_> {
    type State = ();

    fn build(&self) -> Self::State {}

    fn handle(&self, state: &mut Self::State, event: Event) {
        let _ = state;

        if let Event::MouseOut = event {
            (self.on_mouse_out)()
        }
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

impl Handler for Clickable<'_> {
    type State = bool;

    fn build(&self) -> Self::State {
        false
    }

    fn handle(&self, state: &mut Self::State, event: Event) {
        if let Event::MouseInput {
            button,
            state: button_state,
            ..
        } = event
        {
            if button != MouseButton::Left {
                return;
            }

            if button_state == ElementState::Pressed {
                *state = true
            } else if *state && button_state == ElementState::Released {
                (self.on_click)()
            }
        }
    }
}

#[derive(Data)]
pub struct FontColor {
    pub color: Color,
}

impl Modify for FontColor {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let text_cx = use_context::<TextContext>(cx).unwrap();

        use_provider(cx, || TextContext {
            color: self.color,
            font_size: text_cx.font_size,
            font_stack: text_cx.font_stack.clone(),
        });
    }
}

#[derive(Data)]
pub struct FontSize {
    pub font_size: f32,
}

impl Modify for FontSize {
    fn use_state<'a>(&'a self, cx: ScopeState<'a>) {
        let text_cx = use_context::<TextContext>(cx).unwrap();

        use_provider(cx, || TextContext {
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
        let text_cx = use_context::<TextContext>(cx).unwrap();

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
        let canvas_cx = use_context::<CanvasContext>(cx).unwrap();

        let draw = self.draw.clone();
        use_provider(cx, move || {
            let canvas_cx = (*canvas_cx).clone();
            canvas_cx.draws.borrow_mut().push(draw.clone());
            canvas_cx
        });
    }
}
