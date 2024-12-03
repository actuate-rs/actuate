# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.2](https://github.com/actuate-rs/actuate/compare/actuate-v0.14.1...actuate-v0.14.2) - 2024-12-03

### Features

- Optimize empty composables by skipping creation of ScopeData

### Fixes

- Enable Tokio dependency with animation and ecs features (5263fe4)

## [0.14.1](https://github.com/actuate-rs/actuate/compare/actuate-v0.14.0...actuate-v0.14.1) - 2024-12-03

### Fixes

- Remove unused tokio read lock guard (0ad962f)

## [0.14.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.13.0...actuate-v0.14.0) - 2024-12-03

### Breaking Changes

- Remove unsound `Compose` impl for `Map` and create `MapUnchecked` struct
  - The original `Compose` impl for `Map` would cause undefined behavior if multiple references to the same composable were used. The new unsafe `MapUnchecked` keeps this functionality for low-level components, where the documented safety contract can be checked. However, for most composables I now see `Compose + Clone` being a typical pattern (which I think is fine given some composables only copy references when cloned, and references to composables can still be passed around).

### Fixes

- Impl re-composition when the type has changed in `DynCompose` (7d41100)

### Documentation

- Update docs for `Spawn` composable (205b88a)
- Add example to showcase `DynCompose` (7d41100)

## [0.13.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.12.0...actuate-v0.13.0) - 2024-12-02

### Breaking Changes

- Use `PartialEq` in `use_memo` instead of the `Memoize` trait (6539c95)
  - This is to memoize tuples and other groups of data.
    To use pointer equality, you can still use `Signal::generation` or `Memoize::memoize` to get the current generation.
- Remove unused UseWorld struct (81615cd)

### Documentation

- Add more documentation to the `Catch` composable
  - Adds a quick explanation of using `Result` + `Catch`, and links to the `catch` constructor function for more info.
- Add explanation to `compose::from_iter` (dc6715d)

### Other

- Change release procedure and update CI (dd4be8d, fe23aad, 723fe6c)

## [0.12.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.11.0...actuate-v0.12.0) - 2024-12-02

### Other

- `#![no_std]` support ([#100](https://github.com/actuate-rs/actuate/pull/100))
- Clean up and add internal docs
- Remove Sized bound in Compose trait
- Create `Catch` composable and impl `Compose` for `Result` ([#99](https://github.com/actuate-rs/actuate/pull/99))
- Add getter and setter methods to ScopeData
- Update docs
- Remove is_empty from ScopeState in favor of checking for empty types
- Create README.md

## [0.11.0](https://github.com/actuate-rs/actuate/compare/actuate-v0.10.2...actuate-v0.11.0) - 2024-11-29

### Other

- Update to Bevy 0.15.0
- Disable observers after drop
- Add support for standard references in RefMap and Cow
- Fix formatting in README

## [0.10.2](https://github.com/actuate-rs/actuate/compare/actuate-v0.10.1...actuate-v0.10.2) - 2024-11-28

### Other

- Add specialized impl of SystemParamFunction for Triggers
- Export animation channel
- Impl Data for UseAnimated
- Impl Data for Pin
- Impl Data for Box<dyn Future<Output = ()>>
- Allow return values for Data fns
- Create `use_animated` hook ([#88](https://github.com/actuate-rs/actuate/pull/88))
- Fix tasks not running on the ecs

## [0.10.1](https://github.com/actuate-rs/actuate/compare/actuate-v0.10.0...actuate-v0.10.1) - 2024-11-26

### Other

- Apply system params in use_world_once
- Apply deferred system param updates
- Add SignalMut::set_if_neq and generation methods
