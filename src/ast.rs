use crate::parse::Dimension;

#[derive(Debug, PartialEq)]
pub enum Event<'a> {
    Content(Content<'a>),
    Begin(Grouping),
    EndGroup,
    /// This type of event is used to signal a connection between the previous and next event.
    ///
    /// For example, a `Fraction` event signals that the next event is the denominator of a
    /// fraction where the previous event is the numerator.
    Infix(Infix),
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

/// Grouping events
#[derive(Debug, PartialEq, Eq)]
pub enum Grouping {
    Group, // `mrow`
    SquareRoot,
    Padded,
}

/// Events that affect the previous and next event
#[derive(Debug, PartialEq)]
pub enum Infix {
    Root,
    Fraction(Option<Dimension>),
    Subscript,
    Superscript,
    Underscript,
    Overscript,
}
