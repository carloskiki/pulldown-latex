//! This crate provides a pull [`Parser`] for `LaTeX` math expressions, and a `MathML` renderer,
//! available through the [`mathml`] module, which closely follows the _MathML Core_
//! specification.

pub(crate) mod attribute;
pub mod config;
pub mod event;
pub mod mathml;
pub mod parser;

#[doc(inline)]
pub use parser::{Parser, ParserError};
#[doc(inline)]
pub use config::RenderConfig;
#[doc(inline)]
pub use mathml::{push_mathml, write_mathml};
#[doc(inline)]
pub use event::Event;
