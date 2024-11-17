# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.3.0...actuate-core-v0.4.0) - 2024-11-17

### Other

- Reorganize `actuate` crate structure ([#42](https://github.com/actuate-rs/actuate/pull/42))
- Remove layout node on drop in Canvas and add list example
- Merge branch 'main' of https://github.com/actuate-rs/actuate into vec
- Fix `Compose` impl for `Option` ([#40](https://github.com/actuate-rs/actuate/pull/40))
- Mark Any_compose::any_compose as unsafe
- Make `ScopeState` invariant ([#37](https://github.com/actuate-rs/actuate/pull/37))
- Clean up tracing
- Make Update FnOnce ([#31](https://github.com/actuate-rs/actuate/pull/31))
- Make Mut only have one pointer ([#32](https://github.com/actuate-rs/actuate/pull/32))
- Impl Memoize for Ref
- Update Memo to use Memoized trait

## [0.3.0](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.2.2...actuate-core-v0.3.0) - 2024-11-16

### Other

- More efficient Canvas reactions
- Update example and docs
- Refactor macro for fns
- Create ContextError struct and return Result from use_context
- Replace Hash with new Memoize trait in use_memo
- Create `RefMap` enum ([#27](https://github.com/actuate-rs/actuate/pull/27))
- Fix DynCompose test case
- Test Memo composable
- Safely impl Compose for Map<C>
- Impl Compose for Option<C>
- Impl Data for fns
- Clean up
- Use Data macro in more places
- Refactor macro
- Add font size param
- Impl ptr-based change detection for Mut

## [0.2.2](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.2.1...actuate-core-v0.2.2) - 2024-11-14

### Other

- Fix parent to child reactions
- Refactor and impl better tracing
- Trigger paints
- Create unsafe MapCompose struct to fix Map soundness
- Fix reactivity for containers

## [0.2.1](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.2.0...actuate-core-v0.2.1) - 2024-11-14

### Other

- Better names in tracing
- Use shorter type name in tracing

## [0.2.0](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.1.1...actuate-core-v0.2.0) - 2024-11-13

### Other

- Create Window composable
- Create use_callback hook
- Create Handler struct
- Create actuate-winit crate and refactor

## [0.1.1](https://github.com/actuate-rs/actuate/compare/actuate-core-v0.1.0...actuate-core-v0.1.1) - 2024-11-12

### Other

- updated the following local packages: actuate-macros
