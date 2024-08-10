# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

## Added

- Robust CI setup.
- Miscellaneous documentation improvements.

## Changed

- Use criterion for benchmarks.
- Set MSRV to 1.74.1. (__Breaking Change__)
- The Dimension `type` is now a `newtype`, and is more ergonomic. (__Breaking Change__)

## Fixed

- Array rendering with custom line spacing.
- Expansion spans being to eagerly popped.
- Benchmark errors and doc-tests not compiling.

## Removed

- Dependency on `thiserror`.

# [0.5.1] - 2024-08-02

## Changed

- Made the demo site look somewhat good.

## Fixed

- Fix `\phi` and `\varphi` being inverted.
- Fix spacing in mathematical environments rows.

# [0.5.0] - 2024-08-02

## Changed

- Small documentation improvements.

## Added

- A usage section in the crate documentation.

## Removed

- Made `InnerParser` private. (__Breaking Change__)
- Made `MacroContext` private.  (__Breaking Change__)

# [0.4.0] - 2024-08-01

### Changed

- Added a full error trace to the errors returned by the `Parser`.
- Updated `fantoccini` from `0.19.0` to `0.21.0` in test suite.
