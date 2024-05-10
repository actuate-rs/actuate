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
</div>

<div align="center">
 <a href="https://github.com/actuate-rs/actuate/tree/main/examples">Examples</a>
</div>

<br />

A high-performance reactive user-interface framework for Rust.
This crate provides a generic library that lets you define UI using declarative, type-safe syntax.
Views combine together to form a statically-typed view tree that can be stored on the stack,
giving this architecture its high performance.

```rust
use actuate::{use_state, Scope, View, VirtualDom};

struct Counter {
    start: i32,
}

impl View for Counter {
    fn body(&self, cx: &Scope) -> impl View {
        let (count, set_count) = use_state(cx, || self.start);

        set_count.set(count + 1);

        dbg!(count);
    }
}

struct App;

impl View for App {
    fn body(&self, _cx: &Scope) -> impl View {
        (Counter { start: 0 }, Counter { start: 100 })
    }
}

#[tokio::main]
async fn main() {
    let mut vdom = VirtualDom::new(App.into_node());

    tokio::spawn(async move {
        vdom.run().await;
        vdom.run().await;
    })
    .await
    .unwrap();
}
```

## Inspiration
This crate is inspired by [Xilem](https://github.com/linebender/xilem) and uses a similar approach to type-safe reactivity. The main difference with this crate is the concept of scopes, components store their state in their own scope and updates to that scope re-render the component.

State management is inspired by React and [Dioxus](https://github.com/DioxusLabs/dioxus), but this project aims to be higher performance by taking advantage of multi-threading.
