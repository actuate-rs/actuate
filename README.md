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
#[derive(Clone)]
struct App;

impl View for App {
    fn body(&self, cx: &actuate::Scope) -> impl View {
        let (count, set_count) = use_state(cx, || 0);

        (
            text(format!("High five count: {}", count)),
            div(text("Up high!")).on_click({
                clone!(count, set_count);
                move || set_count.set(count + 1)
            }),
            div(text("Down low!")).on_click({
                clone!(count);
                move || set_count.set(count - 1)
            }),
        )
    }
}
```

## Inspiration
This crate is inspired by [Xilem](https://github.com/linebender/xilem) and uses a similar approach to type-safe reactivity. The main difference with this crate is the concept of scopes, components store their state in their own scope and updates to that scope re-render the component.

State management is inspired by React and [Dioxus](https://github.com/DioxusLabs/dioxus).
