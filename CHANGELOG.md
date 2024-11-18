# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [0.7.1] - 2024-11-18

## Added

- Added support for `equation` and `equation*` environments.

# [0.7.0] - 2024-10-10

## Fixed

- Fix comments parsing inside of environments and groups.
- `\hline` and `\hdashline` before any content in a math environment.

## Changed

- __Breaking Change__: `Event::Alignment` and `Event::NewLine` were moved to
    `Event::EnvironmentFlow(EnvironmentFlow::Alignment)` and
    `Event::EnvironmentFlow(EnvironmentFlow::NewLine)` respectively.

## Added

- Added the `Event::EnvironmentFlow(EnvironmentFlow::StartLines)` variant, for when the first thing in the environment
    is a `\hline`/`\hdashline`.

# [0.6.3] - 2024-09-04

## Added

- The `Token` and `MacroSuffixNotFound` error variants.

## Fixed

- Fix comments parsing.

## Removed

- Removed the `ErrorKind::EndOfInput` variant in favor of more descriptive ones.

# [0.6.2] - 2024-09-02

No notable changes.

# [0.6.1] - 2024-08-31

## Fix

- Fix the `mathml` output when `annotation` is set.
- Fix the error display having asymmetric lines.

# [0.6.0] - 2024-08-27

## Added

- Robust CI setup.
- Miscellaneous documentation improvements.
- Errors are now pretty :)

## Changed

- Use criterion for benchmarks.
- Set MSRV to 1.74.1. (__Breaking Change__)
- The Dimension `type` is now a `newtype`, and is more ergonomic. (__Breaking Change__)
- The `ColorChange` event changed to be smaller in memory. (__Breaking Change__)

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

## Changed

- Added a full error trace to the errors returned by the `Parser`.
- Updated `fantoccini` from `0.19.0` to `0.21.0` in test suite.
