use crate::attribute::{Dimension, Font};

#[derive(Debug, Clone, Copy, PartialEq)]
/// All events that can be produced by the parser.
pub enum Event<'a> {
    Content(Content<'a>),
    BeginGroup,
    EndGroup,
    /// The events following this one constitute a base and its script(s).
    Visual(Visual),
    Script {
        ty: ScriptType,
        position: ScriptPosition,
    },
    Space {
        width: Option<Dimension>,
        height: Option<Dimension>,
        depth: Option<Dimension>,
    },
    FontChange(Option<Font>),
}

/// Base events that produce `mathml` nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Content<'a> {
    /// Text content that should be typeset following the rules of `LaTeX`'s `text` mode
    Text(&'a str),
    /// A number, which can include decimal points and commas.
    Number(&'a str),
    /// A mathematical identifier, such as a variable or a function name.
    ///
    /// If the identifier is a single character, then the character follows the typesetting rules
    /// of single character variables. If the identifier is a string, even if that string is a
    /// single character, it is typeset as a function name.
    Identifier(Identifier<'a>),
    /// A mathematical operator.
    ///
    /// This variant ecompasses many different types of operators, such as binary operators,
    /// relation, large operators, delimiters, etc. Specifically, it represents an operator
    /// according to the [`mathml` specification](https://w3c.github.io/mathml-core/#operator-fence-separator-or-accent-mo).
    /// 
    /// > in MathML the list of things that should "render as an operator" includes a number of
    /// > notations that are not mathematical operators in the ordinary sense. Besides ordinary
    /// > operators with infix, prefix, or postfix forms, these include fence characters such as
    /// > braces, parentheses, and "absolute value" bars; separators such as comma and semicolon; and
    /// > mathematical accents such as a bar or tilde over a symbol.
    Operator(Operator),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Operator {
    pub content: char,
    pub stretchy: Option<bool>,
    pub deny_movable_limits: bool,
    pub left_space: Option<Dimension>,
    pub right_space: Option<Dimension>,
    pub size: Option<Dimension>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An identifier can either be a single character, or a string (e.g., a command such as `sin`,
/// `lim`, etc.).
pub enum Identifier<'a> {
    Str(&'a str),
    Char(char),
}

/// Modifies the visual representation of the following event(s)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visual {
    /// The following event is the content of the root
    SquareRoot,
    /// The 2 following events are the numerator and denominator of a fraction
    Fraction(Option<Dimension>),
}

/// Logical type of the script. This is used to determine how to render the scripts.
///
/// Things like subscripts, underscripts, and movable scripts can be represented when using this in
/// conjunction with the `ScriptPosition` enum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    /// The 2 following events are the base and and the subscript
    Subscript,
    /// The 2 following events are the base and and the superscript
    Superscript,
    /// The 3 following events are the base, subscript and superscript
    SubSuperscript,
}

/// Position of the script. This is used to determine how to render the scripts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptPosition {
    /// The scripts are rendered to the (bottom and top) right of the operator.
    Right,
    /// The scripts are rendered above and below the operator instead of to the right.
    AboveBelow,
    /// Is set to `AboveBelow` by preference, but can be changed to `Normal` when rendering in
    /// compact mode.
    ///
    /// This is used by the `lim` operator for example.
    Movable,
}
