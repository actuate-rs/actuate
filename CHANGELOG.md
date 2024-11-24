# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.7.0...actuate-v0.8.0) - 2024-11-24

### Other

- Pass clippy
- Move TaskFuture to composer module
- Create `executor` feature flag ([#73](https://github.com/actuate-rs/actuate/pull/73))
- Fix FromIter
- Impl Executor for unsized types
- Make Executor dyn-safe to replace AnyExecutor
- Export AnyExecutor
- Impl Executor for Rc and Arc
- Update example
- Add Mut::set and refactor params
- Fix lifetime escape in Fn

## [0.7.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.6.1...actuate-v0.7.0) - 2024-11-21

### Other

- Pass lints
- Return reference from use_provider (to match use_context)
- Reborrow composables in FromIter
- Make Data for FnMut more strict as Fn
- Add docs to Compose trait
- Pass Ref in FromIter and add docs
- Add must_use lints
- Add docs and ExecutorContext::spawn_boxed
- Remove Data::Id in favor of typeid crate ([#65](https://github.com/actuate-rs/actuate/pull/65))

## [0.6.1](https://github.com/actuate-rs/actuate/compare/actuate-v0.6.0...actuate-v0.6.1) - 2024-11-19

### Other

- Hide inner data traits
- Update docs
- Update docs
- Update README.md

## [0.6.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.5.0...actuate-v0.6.0) - 2024-11-19

### Other

- Drop tasks before scope
- Impl Data for HashMap
- Quick fix for executor context
- Add Composer::lock
- Fix docs for use_task
- Update examples
- Update examples
- Update CI
- Use additive features by default
- Impl more std traits for Cow and RefMap and add borrowing example
- Update tracing
- Clean up
- Update docs for use_context
- Merge with main
- Export text module
- Require Sync for pointers to impl Send

## [0.5.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.4.0...actuate-v0.5.0) - 2024-11-18

### Other

- Add deny_missing_docs
- Move macros crate to root and refactor Mut to use NonNull
- Merge workspace into main crate ([#55](https://github.com/actuate-rs/actuate/pull/55))
- Allow alternate task executors with new Executor trait ([#54](https://github.com/actuate-rs/actuate/pull/54))
- Update FUNDING.yml
- Create `use_layout` hook and refactor `Canvas` ([#50](https://github.com/actuate-rs/actuate/pull/50))
- Add clippy lints ([#48](https://github.com/actuate-rs/actuate/pull/48))
- Cache FromIter and export task hooks from prelude
- Make compose::from_iter more general
- Add crate-level docs
- Create `use task` and `use_local_task` hooks ([#44](https://github.com/actuate-rs/actuate/pull/44))
- Rename modify module to view and refactor
- Refactor View trait
- Create Handler trait and Event enum
- Restructure core
- Create ui module
- Create draw and modify modules in actuate crate

## [0.4.0-alpha.7](https://github.com/actuate-rs/actuate/compare/actuate-v0.4.0-alpha.6...actuate-v0.4.0-alpha.7) - 2024-11-16

### Other

- More efficient Canvas reactions
- Update example and docs
- Refactor macro for fns
- Inherit TextContext in Window and update example
- Create ContextError struct and return Result from use_context
- Replace Hash with new Memoize trait in use_memo
- Make font size contextual and update example
- Render text directly with Vello and Parley
- Update CI
- Make the View trait more composable
- Simplify re-render logic for Canvas
- Update FUNDING.yml
- Update README.md
- Safely impl Compose for Map<C>
- Update README.md
- Add FUNDING.yml
- Impl Data for fns
- Clean up
- Use Data macro in more places
- Create Draw trait
- Add background color modifier
- Refactor
- Add font size param
- Clean up

## [0.4.0-alpha.6](https://github.com/actuate-rs/actuate/compare/actuate-v0.4.0-alpha.5...actuate-v0.4.0-alpha.6) - 2024-11-14

### Other

- Update README.md
- Update README.md
- Change logo
- Better click handling
- Fix initial layout pass
- Cache renderer
- Refactor and impl better tracing
- Clean up
- Trigger paints
- Create unsafe MapCompose struct to fix Map soundness
- Better names in tracing
- Impl basic click handling and fix text sizing
- Refactor Window
- Update README.md
- Update README.md
- Add logo
- Add Flex::row and Flex::column constructors
- Create Flex composable
- Setup default fonts
- Clean up
- Render basic text
- Refactor Canvas and add Layout to fn args

## [0.4.0-alpha.5](https://github.com/actuate-rs/actuate/compare/actuate-v0.4.0-alpha.4...actuate-v0.4.0-alpha.5) - 2024-11-13

### Other

- Update canvas example
- Temp fix for CI
- Redraw from reactions
- Create Canvas composable
- Create vello-backed Window
- Create Window composable
- Export use_window and refactor
- Create Handler struct
- Create actuate-winit crate and refactor

## [0.4.0-alpha.4](https://github.com/actuate-rs/actuate/compare/actuate-v0.4.0-alpha.3...actuate-v0.4.0-alpha.4) - 2024-11-12

### Other

- Add release CI
- Create core crate and refactor
- Masonry backend ([#14](https://github.com/actuate-rs/actuate/pull/14))
- Reborrow data
- Remove `Node` trait ([#15](https://github.com/actuate-rs/actuate/pull/15))
- Add use_context hook
- Split up code
