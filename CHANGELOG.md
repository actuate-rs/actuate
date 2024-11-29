# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
