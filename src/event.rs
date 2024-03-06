use crate::attribute::{Font, Dimension};

#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Content(Content<'a>),
    BeginGroup,
    EndGroup,
    /// This type of event is used to signal a connection between the previous and next event.
    ///
    /// For example, a `Fraction` event signals that the next event is the denominator of a
    /// fraction where the previous event is the numerator.
    Infix(Infix),
    Visual(Visual),
    Space {
        width: Option<Dimension>,
        height: Option<Dimension>,
        depth: Option<Dimension>,
    },
}

/// Base events that produce `mathml` nodes
#[derive(Debug, PartialEq)]
pub enum Content<'a> {
    Text(&'a str),
    Number {
        content: &'a str,
        variant: Option<Font>,
    },
    Identifier(Identifier<'a>),
    Operator(Operator),
}

#[derive(Debug, Default, PartialEq)]
pub struct Operator {
    pub content: char,
    pub stretchy: Option<bool>,
    pub moveable_limits: Option<bool>,
    pub left_space: Option<Dimension>,
    pub right_space: Option<Dimension>,
    pub size: Option<Dimension>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'a> {
    Str(&'a str),
    Char {
        content: char,
        variant: Option<Font>,
    },
}

/// Events that affect the previous and next event
#[derive(Debug, PartialEq, Eq)]
pub enum Infix {
    Subscript,
    Superscript,
    Underscript,
    Overscript,
}

/// Event that affect the following 2 events visually
#[derive(Debug, PartialEq)]
pub enum Visual {
    Root,
    Fraction(Option<Dimension> /* TODO: style */),
}
