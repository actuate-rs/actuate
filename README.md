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
// Counter UI example.

use actuate::prelude::*;
use bevy::prelude::*;

// Counter composable.
#[derive(Data)]
struct Counter {
    start: i32,
}

impl Compose for Counter {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let count = use_mut(&cx, || cx.me().start);

        (
            spawn(Text::new(format!("High five count: {}", count))),
            spawn(Text::new("Up high"))
                .observe(move |_: Trigger<Pointer<Click>>| SignalMut::update(count, |x| *x += 1)),
            spawn(Text::new("Down low"))
                .observe(move |_: Trigger<Pointer<Click>>| SignalMut::update(count, |x| *x -= 1)),
            if *count == 0 {
                Some(spawn(Text::new("Gimme five!")))
            } else {
                None
            },
        )
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    // Spawn a composition with a `Counter`, adding it to the Actuate runtime.
    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Composition::new(Counter { start: 0 }),
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ActuatePlugin))
        .add_systems(Startup, setup)
        .run();
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
        spawn(Text::new(cx.me().name.to_string()))
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

## Inspiration
This crate is inspired by [Xilem](https://github.com/linebender/xilem) and uses a similar approach to type-safe reactivity. The main difference with this crate is the concept of scopes, components store their state in their own scope and updates to that scope re-render the component.

State management is inspired by React and [Dioxus](https://github.com/DioxusLabs/dioxus).

Previous implementations were in [Concoct](https://github.com/concoct-rs/concoct) but were never very compatible with lifetimes.
