pub mod ast;
pub(crate) mod attribute;
pub mod config;
mod macros;
pub(crate) mod parse;

/// display style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    #[default]
    Inline,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    ControlSequence(&'a str),
    Character(char),
}

pub enum Argument<'a> {
    Token(Token<'a>),
    Group(&'a str),
}
