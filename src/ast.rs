use crate::parse::Dimension;

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
    Visuals(Visual)
}

/// Base events that produce `mathml` nodes
#[derive(Debug, PartialEq)]
pub enum Content<'a> {
    Text(&'a str),
    Number(&'a str), // done
    Identifier(Identifier<'a>),
    Operator {
        content: char,
        stretchy: Option<bool>,
        moveable_limits: Option<bool>,
        left_space: Option<Dimension>,
        right_space: Option<Dimension>,
    },
    StringLiteral(&'a str),
    Error(String),
    Space,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'a> {
    Str(&'a str),
    Char{ 
        content: char,
        is_normal: bool,
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
    Fraction(Option<Dimension>, /* TODO: style */),
}
