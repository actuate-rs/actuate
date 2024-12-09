<div align="center">
  <h1>Actuate</h1>
  <a href="https://crates.io/crates/actuate">
    <img src="https://img.shields.io/crates/v/actuate?style=flat-square"
    alt="Crates.io version" />
  </a>
  <a href="https://docs.rs/actuate">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
   <a href="https://github.com/actuate-rs/actuate/actions">
    <img src="https://github.com/actuate-rs/actuate/actions/workflows/ci.yml/badge.svg"
      alt="CI status" />
  </a>
  <a href="https://discord.gg/AbyAdew3">
    <img src="https://img.shields.io/discord/1306713440873877576.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2" />
</div>

<div align="center">
 <a href="https://github.com/actuate-rs/actuate/tree/main/examples">Examples</a>
</div>

<br />

A high-performance and borrow-checker friendly framework for declarative programming in Rust.
This crate provides a generic library that lets you define reactive components (also known as composables).

## Features
- Declarative scenes and UI for [Bevy](https://github.com/bevyengine/bevy)
- Efficient and borrow-checker friendly state management: Manage state with components and hooks, all using zero-cost smart pointers
- Generic core for custom backends

```rust
use actuate::prelude::*;

#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        material_ui((
            text::headline(format!("High five count: {}", count)),
            button(text::label("Up high")).on_click(move || SignalMut::update(count, |x| *x += 1)),
            button(text::label("Down low")).on_click(move || SignalMut::update(count, |x| *x -= 1)),
            if *count == 0 {
                Some(text::label("Gimme five!"))
            } else {
                None
            },
        ))
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
    }
}
```

## Borrowing
Composables can borrow from their ancestors, as well as state.
```rs
use actuate::prelude::*;

#[derive(Data)]
struct User<'a> {
    // `actuate::Cow` allows for either a borrowed or owned value.
    name: Cow<'a, String>,
}

impl Compose for User<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        text::headline(cx.me().name.to_string())
    }
}

#[derive(Data)]
struct App {
    name: String
}

impl Compose for App {
    fn compose(cx: Scope<Self>) -> impl Compose {
        // Get a mapped reference to the app's `name` field.
        let name = Signal::map(cx.me(), |me| &me.name).into();

        User { name }
    }
}
```

## Installation
To add this crate to your project:
```
cargo add actuate --features full
```
For more feature flags, see the crate documentation for [features](https://docs.rs/actuate/latest/actuate/#features).

## Inspiration
This crate is inspired by [Xilem](https://github.com/linebender/xilem) and uses a similar approach to type-safe reactivity. The main difference with this crate is the concept of scopes, components store their state in their own scope and updates to that scope re-render the component.

State management is inspired by React and [Dioxus](https://github.com/DioxusLabs/dioxus).

Previous implementations were in [Concoct](https://github.com/concoct-rs/concoct) but were never very compatible with lifetimes.
