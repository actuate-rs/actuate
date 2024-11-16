# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
