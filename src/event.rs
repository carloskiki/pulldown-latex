//! The definition of the [`Event`] enum, which is used as a logical
//! representation of `LaTeX` content.
//!
//! A stream of `Result<Event, ParserError>`s is produced by the [`Parser`], which can then be typeset/rendered
//! by a renderer. This crate only provides a simple `mathml` renderer available through the
//! [`push_mathml`] and [`write_mathml`] functions.
//! 
//! [`Parser`]: crate::parser::Parser
//! [`push_mathml`]: crate::mathml::push_mathml
//! [`write_mathml`]: crate::mathml::write_mathml

use crate::attribute::{Dimension, Font};

/// All events that can be produced by the parser.
/// 
/// When an [`Event`] is referreing to an "_element_", it is referring to the next logical unit of
/// content in the stream. This can be a single content element, a group, a visual element, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<'a> {
    /// The event is a [`Content`] Element.
    Content(Content<'a>),
    /// The events following this one constitute a "group" of elements (i.e., a set of elements
    /// within `{}` in `LaTeX`), until the [`Event::EndGroup`] event is reached.
    BeginGroup,
    /// Marks the end of a group of elements.
    EndGroup,
    /// The `n` events following this one constitute the content of the [`Visual`] element. `n` is
    /// defined in the documentation of for the [`Visual`] variant.
    Visual(Visual),
    /// The `n` events following this one constitute a base and its script(s). `n` is defined in
    /// the documentation for the associated [`ScriptType`] variant.
    Script {
        ty: ScriptType,
        position: ScriptPosition,
    },
    /// This events specifes a "custom" spacing element. This is produced by commands such as
    /// `\kern`, `\hspace`, etc.
    ///
    /// If any of the components are `None`, then the spacing is set to 0 for that component.
    Space {
        width: Option<Dimension>,
        height: Option<Dimension>,
        depth: Option<Dimension>,
    },
    /// This event specifies a state change in the renderer.
    ///
    /// This state change only applies to the current group nesting and deeper groups.
    StateChange(StateChange<'a>),
}

/// Base events that produce `mathml` nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Content<'a> {
    /// Text content that should be typeset following the rules of `LaTeX`'s `text` mode.
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
    /// > In MathML the list of things that should "render as an operator" includes a number of
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
    /// If this is set to true, the unicode character VS1 (U+20D2) is added to the operator. This
    /// is used to allow for special negation operators, such as `\varsupsetneqq` (⫌︀).
    pub unicode_variant: bool,
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
    /// The following element is the content of the root.
    SquareRoot,
    /// The 2 following elements are the radicand and the index of the root.
    Root,
    /// The 2 following elements are the numerator and denominator of the fraction.
    Fraction(Option<Dimension>),
    /// The "negation" operator as in "not equal" (≠) or "does not exist" (∄). This applies to the
    /// next event in the stream.
    /// 
    /// This event can occur before an arbitrary event, not just a `Content` event. It is left to
    /// the renderer to determine how to apply the negation.
    Negation,
}

/// Logical type of the script. This is used to determine how to render the scripts.
///
/// Things like subscripts, underscripts, and movable scripts can be represented when using this in
/// conjunction with the [`ScriptPosition`] enum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    /// The 2 following elements are the base and and the subscript
    Subscript,
    /// The 2 following elements are the base and and the superscript
    Superscript,
    /// The 3 following elements are the base, subscript and superscript
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
    /// compact (a.k.a. inline) mode.
    ///
    /// This is used by the `lim` operator and `sum`s (Σ) for example.
    Movable,
}

/// Represents a state change in the renderer.
///
/// State changes take effect for the current group nesting and all deeper groups.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateChange<'a> {
    /// Changes the font of the content.
    ///
    /// If the font is `None`, then the default renderer font is used, otherwise the font is set to
    /// the specified font.
    Font(Option<Font>),
    /// Changes the color of the content.
    Color(ColorChange<'a>),
    /// Changes the style of the content (mostly affects the sizing of the content).
    Style(Style),
}

/// The style of the content.
///
/// This is analogous to the different "modes" in `LaTeX`, such as `display`, `text`, etc., which
/// are set by commands like `\displaystyle`, `\textstyle`, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// Represents a color change in the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorChange<'a> {
    /// The color to change to.
    ///
    /// A string that represents the color to change to, either as a hex code in the form #RRGGBB,
    /// or as one of the color names existing as part of CSS3 (e.g., "red").
    pub color: &'a str,
    /// The target of the color change.
    ///
    /// Specifies which part of the content to change the color of.
    pub target: ColorTarget,
}

/// The target of the color change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorTarget {
    Text,
    Background,
    Border,
}
