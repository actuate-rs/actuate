# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/actuate-rs/actuate/compare/actuate-winit-v0.1.3...actuate-winit-v0.1.4) - 2024-11-16

### Other

- Refactor macro for fns
- Create ContextError struct and return Result from use_context
- Replace Hash with new Memoize trait in use_memo
- Safely impl Compose for Map<C>

## [0.1.3](https://github.com/actuate-rs/actuate/compare/actuate-winit-v0.1.2...actuate-winit-v0.1.3) - 2024-11-14

### Other

- Fix initial layout pass
- Cache renderer
- Refactor and impl better tracing
- Create unsafe MapCompose struct to fix Map soundness

## [0.1.2](https://github.com/actuate-rs/actuate/compare/actuate-winit-v0.1.1...actuate-winit-v0.1.2) - 2024-11-14

### Other

- updated the following local packages: actuate-core

## [0.1.1](https://github.com/actuate-rs/actuate/compare/actuate-winit-v0.1.0...actuate-winit-v0.1.1) - 2024-11-13

### Other

- Create Canvas composable
- Create vello-backed Window
- Create Window composable
- Create use_callback hook
- Export use_window and refactor
- Create use_window hook
- Create Handler struct
