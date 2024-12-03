# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.6...actuate-macros-v0.2.0) - 2024-12-3

### Breaking changes

- Add `#[actuate(path = "..")]` attribute to `Data` macro and use fully-qualified path to Actuate by default (b159478)

## [0.1.6](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.5...actuate-macros-v0.1.6) - 2024-11-25

### Other

- Add basic support for borrowed trait objects

## [0.1.5](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.4...actuate-macros-v0.1.5) - 2024-11-21

### Other

- Remove Data::Id in favor of typeid crate ([#65](https://github.com/actuate-rs/actuate/pull/65))

## [0.1.4](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.3...actuate-macros-v0.1.4) - 2024-11-18

### Other

- Move macros crate to root and refactor Mut to use NonNull

## [0.1.3](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.2...actuate-macros-v0.1.3) - 2024-11-16

### Other

- Test Memo composable
- Safely impl Compose for Map<C>
- Use Data macro in more places
- Refactor macro

## [0.1.2](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.1...actuate-macros-v0.1.2) - 2024-11-13

### Other

- Create Handler struct
- Create actuate-winit crate and refactor

## [0.1.1](https://github.com/actuate-rs/actuate/compare/actuate-macros-v0.1.0...actuate-macros-v0.1.1) - 2024-11-12

### Other

- Create core crate and refactor
