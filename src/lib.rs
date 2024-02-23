pub mod event;
pub(crate) mod attribute;
pub mod config;
mod macros;
pub(crate) mod parse;

/// How the math is displayed.
///
/// Semantically, this affects the [`display`] attribute of the [`math`] in the mathml
/// output. The attribute will be set to `block` or `inline` depending on the value of this enum.
///
/// [`math`]: https://developer.mozilla.org/en-US/docs/Web/MathML/Element/math
/// [`display`]: https://developer.mozilla.org/en-US/docs/Web/MathML/Element/math#display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    /// The math is rendered inline.
    ///
    /// The equation is displayed in the middle of a paragraph, and elements such as
    /// `\int` and `\sum` are minimized to fit within the line.
    ///
    /// __This is the default value.__
    #[default]
    Inline,
    /// The math is rendered in display/block mode (`displaystyle` in LaTeX).
    ///
    /// The equation is centered on its own line
    /// and elements such as`\int` and `\sum` are displayed bigger.
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    ControlSequence(&'a str),
    Character(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Argument<'a> {
    Token(Token<'a>),
    Group(&'a str),
}
