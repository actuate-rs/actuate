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
Views combine together to form a statically-typed view tree that is stored on the stack,
giving this architecture its high performance.

```rust
use actuate::{use_state, view, View, VirtualDom};

fn counter(initial: i32) -> impl View {
    view::from_fn(move |cx| {
        let (count, set_count) = use_state(cx, || initial);

        set_count.set(count + 1);

        dbg!(count);
    })
}

fn app() -> impl View {
    (counter(0), counter(100))
}

#[tokio::main]
async fn main() {
    let mut vdom: VirtualDom<_, _, ()> = VirtualDom::new(app());

    tokio::spawn(async move {
        vdom.run().await;
        vdom.run().await;
    })
    .await
    .unwrap();
}
```