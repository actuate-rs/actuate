use super::{draw::BackgroundColor, Event};
use crate::{
    prelude::*,
    ui::view::{
        canvas::CanvasContext,
        text::{IntoFontStack, TextContext},
    },
};
use parley::FontStack;
use std::{borrow::Cow, cell::RefCell, mem, rc::Rc};
use winit::event::{ElementState, MouseButton};

pub(crate) mod canvas;
pub use self::canvas::Canvas;

mod flex;
pub use self::flex::Flex;

/// Text composable.
pub mod text;
pub use self::text::Text;

mod window;
pub use self::window::Window;

/// Composable view modifiers.
pub trait View: Compose {
    /// Modify this view with the provided modifier.
    fn modify<T: Modify>(self, modify: T) -> Modified<T, Self> {
        Modified::new(modify, self)
    }

    /// Add an event handler to this view.
    fn on_event<H: Handler>(self, on_event: H) -> Modified<OnEvent<H>, Self> {
        self.modify(OnEvent::new(on_event))
    }

    /// Add an event handler for mouse-in events to this view.
    fn on_mouse_in<'a>(
        self,
        on_mouse_in: impl Fn() + 'a,
    ) -> Modified<OnEvent<OnMouseIn<'a>>, Self> {
        self.on_event(OnMouseIn::new(on_mouse_in))
    }

    /// Add an event handler for mouse-out events to this view.
    fn on_mouse_out<'a>(
        self,
        on_mouse_out: impl Fn() + 'a,
    ) -> Modified<OnEvent<OnMouseOut<'a>>, Self> {
        self.on_event(OnMouseOut::new(on_mouse_out))
    }

    /// Add an event handler for click events to this view.
    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> Modified<OnEvent<Clickable<'a>>, Self> {
        self.on_event(Clickable::new(on_click))
    }

    /// Set the font for this view.
    fn font(self, font_stack: impl IntoFontStack<'static>) -> Modified<Font, Self> {
        self.modify(Font {
            font_stack: font_stack.into_font_stack(),
        })
    }

    /// Set the text color for this view.
    fn color(self, color: Color) -> Modified<FontColor, Self> {
        self.modify(FontColor { color })
    }

    /// Set the font size for this view.
    fn font_size(self, font_size: f32) -> Modified<FontSize, Self> {
        self.modify(FontSize { font_size })
    }

    /// Add a drawable modifier to this view.
    fn draw<D: Draw + 'static>(self, draw: D) -> Modified<DrawModifier<D>, Self> {
        self.modify(DrawModifier::new(draw))
    }

    /// Set the background color for this view.
    fn background_color(self, color: Color) -> Modified<DrawModifier<BackgroundColor>, Self> {
        self.draw(BackgroundColor { color })
    }
}

impl<C: Compose> View for C {}

/// Modifier.
pub trait Modify {
    /// Use the state of this modifier.
    fn use_state<'a>(&'a self, cx: ScopeState<'a>);
}

/// Modified view.
#[derive(Data)]
pub struct Modified<T, C> {
    modify: T,
    content: C,
}

impl<T, C> Modified<T, C> {
    /// Create a new modified view from the given modifier and `content`.
    pub fn new(modify: T, content: C) -> Self {
        Self { modify, content }
    }
}

impl<T, C> Compose for Modified<T, C>
where
    T: Modify + Data,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        // Safety: `state` is guranteed to live as long as `cx`.
        let state_cx: ScopeState = unsafe { mem::transmute(&**cx) };

        cx.me().modify.use_state(state_cx);

        Ref::map(cx.me(), |me| &me.content)
    }

    fn name() -> Option<Cow<'static, str>> {
        None
    }
}

/// Event handler.
pub trait Handler {
    /// State of this handler.
    type State: 'static;

    /// Build the initial state.
    fn build(&self) -> Self::State;

    /// Handle an event with the current state.
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

/// Event handler modifier.
pub struct OnEvent<H> {
    on_event: RefCell<H>,
}

impl<H> OnEvent<H> {
    /// Create a new event handler modifier from the given event handler.
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
        let canvas_cx = use_context::<CanvasContext>(cx).unwrap();

        let state = use_ref(cx, || RefCell::new(self.on_event.borrow_mut().build()));
        use_provider(cx, || {
            // Safety: `f` is removed from `canvas_update_fns` on drop.
            let f: Rc<dyn Fn(Event)> = Rc::new(move |msg| {
                self.on_event
                    .borrow_mut()
                    .handle(&mut state.borrow_mut(), msg)
            });
            let f: Rc<dyn Fn(Event)> = unsafe { mem::transmute(f) };

            let mut pending_listeners = canvas_cx.pending_listeners.borrow().clone();
            pending_listeners.push(f);

            CanvasContext {
                draws: canvas_cx.draws.clone(),
                pending_listeners: Rc::new(RefCell::new(pending_listeners)),
            }
        });
    }
}

/// Mouse-in event handler.
#[derive(Data)]
pub struct OnMouseIn<'a> {
    on_mouse_in: Box<dyn Fn() + 'a>,
}

impl<'a> OnMouseIn<'a> {
    /// Create a new mouse-in event handler from the provided function.
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

/// Mouse-out event handler.
#[derive(Data)]
pub struct OnMouseOut<'a> {
    on_mouse_out: Box<dyn Fn() + 'a>,
}

impl<'a> OnMouseOut<'a> {
    /// Create a new mouse-out event handler from the provided function.
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

/// Click event handler.
#[derive(Data)]
pub struct Clickable<'a> {
    on_click: Box<dyn Fn() + 'a>,
}

impl<'a> Clickable<'a> {
    /// Create a new click event handler from the provided function.
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

/// Font color modifier.
#[derive(Data)]
pub struct FontColor {
    /// Font color.
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

/// Font size modifier.
#[derive(Data)]
pub struct FontSize {
    /// Font size.
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

/// Font modifier.
#[derive(Data)]
pub struct Font {
    /// Font stack.
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

/// Drawable modifier.
pub struct DrawModifier<T> {
    draw: Rc<T>,
}

impl<T> DrawModifier<T> {
    /// Create a new drawable modifier from the provided drawable.
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
