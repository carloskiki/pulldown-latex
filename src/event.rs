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
    Visual(Visual),
    Space {
        width: Option<Dimension>,
        height: Option<Dimension>,
        depth: Option<Dimension>,
    },
    FontChange(Option<Font>),
}

/// Base events that produce `mathml` nodes
#[derive(Debug, PartialEq)]
pub enum Content<'a> {
    Text(&'a str),
    Number(&'a str),
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
    Char(char)
}

/// Event that affect the following 2 events visually
#[derive(Debug, PartialEq)]
pub enum Visual {
    /// The following event is the content of the root
    SquareRoot,
    /// The 2 following events are the numerator and denominator of a fraction
    Fraction(Option<Dimension>),
    /// The 2 following events are the base and and the subscript
    Subscript,
    /// The 2 following events are the base and and the superscript
    Superscript,
    /// The 3 following events are the base, subscript and superscript
    SubSuperscript,
    /// The 2 following events are the base and and the underscript
    Underscript,
    /// The 2 following events are the base and and the overscript
    Overscript,
    /// The 3 following events are the base, underscript and overscript
    UnderOverscript,
}
