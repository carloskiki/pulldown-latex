//! This crate provides a pull [`Parser`] for `LaTeX` math notation, and a `MathML` renderer,
//! available through the [`mathml`] module. This renderer closely follows the _MathML Core_
//! specification.

#![doc = include_str!("../docs/usage.md")]

pub(crate) mod attribute;
pub mod config;
pub mod event;
pub mod mathml;
pub mod parser;

#[doc(inline)]
pub use config::RenderConfig;
#[doc(inline)]
pub use event::Event;
#[doc(inline)]
pub use mathml::{push_mathml, write_mathml};
#[doc(inline)]
pub use parser::{error::ParserError, storage::Storage, Parser};
