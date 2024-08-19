//! The definition of the [`Event`] enum, which is used as a logical
//! representation of `LaTeX` content.
//!
//! A stream of `Result<Event, ParserError>`s is produced by the [`Parser`], which can then be typeset/rendered
//! by a renderer. This crate only provides a simple `mathml` renderer available through the
//! [`push_mathml`] and [`write_mathml`] functions.
//!
//! This module tries to be comprehensive in explaining the invariants that are be upheld by the [`Parser`].
//! If a user of this crate, or a renderer implementor finds a case where the invariants are not
//! satisfied, then it is a bug in the parser, and should be reported.
//!
//! [`Parser`]: crate::parser::Parser
//! [`push_mathml`]: crate::mathml::push_mathml
//! [`write_mathml`]: crate::mathml::write_mathml

use std::fmt::Display;

/// All events that can be produced by the parser.
///
/// # For Renderer Implementors
///
/// When an [`Event`] is referreing to an "_element_", it is referring to the next logical unit of
/// content in the stream. This can be a single [`Event::Content`] element, a group marked
/// by [`Event::Begin`] and [`Event::End`], an [`Event::Visual`] or an [`Event::Script`] element,
/// an [`Event::Space`], or an [`Event::StateChange`].
///
/// [`Event::Alignment`]s, and [`Event::NewLine`]s are not considered elements, and must never
/// occur when an element is expected.
///
/// ### Examples
///
/// The following examples all constitute a single element:
///
/// __Input__: `\text{Hello, world!}`
/// ```
/// # use pulldown_latex::event::{Event, Content};
/// [Event::Content(Content::Text("Hello, world!"))];
/// ```
///
/// __Input__: `x^2_{\text{max}}`
/// ```
/// # use pulldown_latex::event::{Event, Content, Grouping, ScriptType, ScriptPosition};
/// [
///     Event::Script { ty: ScriptType::SubSuperscript, position: ScriptPosition::Right },
///     Event::Begin(Grouping::Normal),
///     Event::Content(Content::Text("max")),
///     Event::End,
///     Event::Content(Content::Ordinary { content: 'x', stretchy: false }),
/// ];
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// The event is a [`Content`] element.
    Content(Content<'a>),
    /// The events following this one constitute a "group" which counts as a single _element_
    /// (i.e., a set of elements within `{}` in `LaTeX`), until the [`Event::End`] event
    /// is reached.
    Begin(Grouping),
    /// Marks the end of a group initiated with [`Event::Begin`].
    End,
    /// The `n` elements following this one constitute the content of the [`Visual`] element,
    /// where `n` is specified in the documentation of for each of the [`Visual`] variants.
    Visual(Visual),
    /// The `n` elements following this one constitute a base and its script(s), where `n` is
    /// specified in the documentation for each of the [`ScriptType`] variants.
    Script {
        /// The type of the script.
        ty: ScriptType,
        /// The position of the script.
        position: ScriptPosition,
    },
    /// This event specifes a custom spacing. This is produced by commands such as
    /// `\kern`, `\hspace`, etc.
    ///
    /// If any of the components are `None`, then the spacing is set to 0 for that component.
    Space {
        /// The amount of space to add before the element.
        width: Option<Dimension>,
        /// The amount of space to add after the element.
        height: Option<Dimension>,
    },
    /// This event specifies a state change in the renderer.
    ///
    /// This state change only applies to the current group nesting and deeper groups.
    StateChange(StateChange),
    /// This event specifies an alignment mark in a mathematical environment.
    ///
    /// This event is only emitted when inside a `Grouping` that allows it.
    Alignment,
    /// This event specifies a line break in a mathematical environment.
    ///
    /// This event is only emitted when inside a `Grouping` that allows it.
    NewLine {
        /// The amount of space to add after the line break.
        spacing: Option<Dimension>,
        /// The horizontal lines to draw after the line break.
        horizontal_lines: Box<[Line]>,
    },
}

/// Base events that produce `mathml` nodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Content<'a> {
    /// Text content that should be typeset following the rules of `LaTeX`'s `text` mode.
    Text(&'a str),
    /// A number, which can include decimal points and commas.
    Number(&'a str),
    /// A function identifier, such as `sin`, `lim`, or a custom function with
    /// `\operatorname{arccotan}`.
    Function(&'a str),
    /// A variable identifier, such as `x`, `\theta`, `\aleph`, etc., and other stuff that do not have
    /// any spacing around them. This includes things that normally go in under and overscripts
    /// which may be stretchy, e.g., `→`, `‾`, etc.
    Ordinary {
        /// The content character.
        content: char,
        /// Whether the character is stretchy.
        ///
        /// This applies to characters that are in under and overscripts, such as `→`.
        stretchy: bool,
    },
    /// A large operator, e.g., `\sum`, `\int`, `\prod`, etc.
    LargeOp {
        /// The content character.
        content: char,
        /// Whether the operator is a small variant, e.g., `\smallint`.
        small: bool,
    },
    /// A binary operator, e.g., `+`, `*`, `⊗`, `?`, etc.
    BinaryOp {
        /// The content character.
        content: char,
        /// Whether the operator is a small variant, e.g., `\smallsetminus`.
        small: bool,
    },
    /// A relation, e.g., `=`, `≠`, `≈`, etc.
    Relation {
        /// The content of the relation.
        content: RelationContent,
        /// Whether the relation is a small variant, e.g., `\shortparallel`.
        small: bool,
    },
    /// An opening, closing, or fence delimiter, e.g., `(`, `[`, `{`, `|`, `)`, `]`, `}`, etc.
    Delimiter {
        /// The delimiter character.
        content: char,
        /// The size of the delimiter, if any.
        size: Option<DelimiterSize>,
        /// The type of the delimiter.
        ty: DelimiterType,
    },
    /// A punctuation character, such as `,`, `.`, or `;`.
    Punctuation(char),
}

/// Modifies the visual representation of the following element(s)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Visual {
    /// The following element is the content of the root.
    SquareRoot,
    /// The 2 following elements are the radicand and the index of the root.
    Root,
    /// The 2 following elements are the numerator and denominator of the fraction.
    ///
    /// If the content of the variant is `None`, then the size of the middle line is set to the
    /// default size, otherwise the size is set to the specified size.
    Fraction(Option<Dimension>),
    /// The "negation" operator as in "not equal" (≠) or "does not exist" (∄). This applies to the
    /// next event in the stream.
    ///
    /// This event can occur before an arbitrary event, not just a `Content` event. It is left to
    /// the renderer to determine how to apply the negation. In `LaTeX`, the renderer usually
    /// generates an akward looking negation across the next element, when it does not correspond
    /// to a commonly negated element.
    Negation,
}

/// Logical type of the script. This is used to determine how to render the scripts.
///
/// Things like subscripts, underscripts, and movable scripts can be represented when using this
/// `enum` in conjunction with the [`ScriptPosition`] `enum`.
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
    /// Is set to `AboveBelow` by preference, but should be changed to `Right` when rendering in
    /// inline mode.
    ///
    /// This is used by the `lim` and `sum` (Σ) operators for example.
    Movable,
}

/// Represents a state change for the following content.
///
/// State changes take effect for the current group nesting and all deeper groups.
/// State changes are not maintained across `NewLine` and `Alignment` events, and are also reset
/// when entering a new group that is not a `Grouping::Normal`, or a `Grouping::LeftRight`. The
/// exception to the latter is `StateChange::Style`, which is maintained across all groups.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateChange {
    /// Changes the font of the content.
    ///
    /// If the font is `None`, then the default renderer font is used, otherwise the font is set to
    /// the specified font.
    Font(Option<Font>),
    /// Changes the color of the content.
    Color(ColorChange),
    /// Changes the style of the content (mostly affects the sizing of the content).
    ///
    /// __Important__: This state change does not affect scripts and root indices.
    Style(Style),
}

/// Available font styles from LaTeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// The bold and calligraphic font-face.
    BoldScript,
    /// The bold and italic font-face.
    BoldItalic,
    /// The bold font-face.
    Bold,
    /// The fraktur font-face.
    Fraktur,
    /// The calligraphic font-face.
    Script,
    /// The monospace font-face.
    Monospace,
    /// The sans-serif font-face.
    SansSerif,
    /// The double-struck font-face.
    DoubleStruck,
    /// The italic font-face.
    Italic,
    /// The bold fraktur font-face.
    BoldFraktur,
    /// The bold sans-serif font-face.
    SansSerifBoldItalic,
    /// The sans-serif italic font-face.
    SansSerifItalic,
    /// The sans-serif bold font-face.
    BoldSansSerif,
    /// The normal font-face.
    UpRight,
}

/// The style of the content.
///
/// This is analogous to the different "modes" in `LaTeX`, such as `display`, `text`, etc., which
/// are set by commands like `\displaystyle`, `\textstyle`, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Style {
    /// Set by the `\displaystyle` command.
    Display,
    /// Set by the `\textstyle` command.
    Text,
    /// Set by the `\scriptstyle` command.
    Script,
    /// Set by the `\scriptscriptstyle` command.
    ScriptScript,
}

/// Represents a color change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorChange {
    /// The color to change to.
    ///
    /// A string that represents the color to change to, either as a hex RGB color in the form #RRGGBB,
    /// or as one of the color names existing as part of CSS3 (e.g., "red").
    pub color: (u8, u8, u8),
    /// The target of the color change.
    ///
    /// Specifies which part of the content to change the color of.
    pub target: ColorTarget,
}

/// The target of the color change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorTarget {
    /// The text of the content.
    Text,
    /// The background of the content.
    Background,
    /// The border surrounding the content.
    Border,
}

/// Represents a grouping of elements, which is itself a single logical element.
///
/// This can be created by a lot of different `LaTeX` commands, such as `{}`, `\left`, `\right`,
/// `\begin{...}`, `\end{...}`, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum Grouping {
    /// A normal form of grouping, usually induced by `{}` or `\begingroup` and `\endgroup` in `LaTeX`.
    Normal,
    /// A grouping that is induced by `\left` and `\right` in `LaTeX`.
    LeftRight(Option<char>, Option<char>),
    /// The array environment of `LaTeX`.
    ///
    /// It's content is an array of columns, which represents the column specification in `LaTeX`.
    ///
    /// ## Example
    ///
    /// __Input__: `\begin{array}{lcr} ... \end{array}`
    /// __Generates__:
    /// ```
    /// # use pulldown_latex::event::{ArrayColumn, ColumnAlignment, Grouping};
    /// Grouping::Array(Box::new([
    ///     ArrayColumn::Column(ColumnAlignment::Left),
    ///     ArrayColumn::Column(ColumnAlignment::Center),
    ///     ArrayColumn::Column(ColumnAlignment::Right)
    ///]));
    /// ```
    /// ## Invariant
    ///
    /// The content of the `Array` variant is guaranteed to be non-empty, and contain at least one
    /// [`ArrayColumn::Column`].
    Array(Box<[ArrayColumn]>),
    /// The `matrix` environment of `LaTeX`.
    Matrix {
        /// The default alignment is `ColumnAlignment::Center`, but it can be specified by in `LaTeX`
        /// when using the `\begin{matrix*}[l] ... \end{matrix*}` syntax.
        alignment: ColumnAlignment,
    },
    /// The `cases` environment of `LaTeX`.
    Cases {
        /// `left` is true if the environment is `cases` and false if the environment is `rcases`.
        left: bool,
    },
    /// The `equation` environment of `LaTeX`.
    Equation {
        /// If `eq_numbers` is true, then equation numbers are displayed.
        eq_numbers: bool,
    },
    /// The `align` environment of `LaTeX`.
    Align {
        /// If `eq_numbers` is true, then equation numbers are displayed.
        eq_numbers: bool,
    },
    /// The `aligned` environment of `LaTeX`.
    Aligned,
    /// The `subarray` environment of `LaTeX`.
    SubArray {
        /// The alignment of the columns in the subarray.
        alignment: ColumnAlignment,
    },
    /// The `alignat` environment of `LaTeX`.
    Alignat {
        /// `pairs` specifies the number of left-right column pairs specified in the environment
        /// declaration.
        pairs: u16,
        /// If `eq_numbers` is true, then equation numbers are displayed.
        eq_numbers: bool,
    },
    /// The `alignedat` environment of `LaTeX`.
    Alignedat {
        /// `pairs` specifies the number of left-right column pairs specified in the environment
        pairs: u16,
    },
    /// The `gather` environment of `LaTeX`.
    Gather {
        /// If `eq_numbers` is true, then equation numbers are displayed.
        eq_numbers: bool,
    },
    /// The `gathered` environment of `LaTeX`.
    Gathered,
    /// The `multline` environment of `LaTeX`.
    Multline,
    /// The `split` environment of `LaTeX`.
    Split,
}

impl Grouping {
    pub(crate) fn is_math_env(&self) -> bool {
        !matches!(self, Self::Normal | Self::LeftRight(_, _))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum GroupingKind {
    Normal,
    OptionalArgument,
    BeginEnd,
    LeftRight,
    Array { display: bool },
    Matrix { ty: MatrixType, column_spec: bool },
    Cases { left: bool, display: bool },
    Equation { eq_numbers: bool },
    Align { eq_numbers: bool },
    Aligned,
    SubArray,
    Alignat { eq_numbers: bool },
    Alignedat,
    Gather { eq_numbers: bool },
    Gathered,
    Multline,
    Split,
}

impl GroupingKind {
    pub fn opening_str(&self) -> &'static str {
        match self {
            Self::Normal => "{",
            Self::OptionalArgument => "[",
            Self::BeginEnd => "\\begin",
            Self::LeftRight => "\\left",
            Self::Array { display: false } => "\\begin{array}",
            Self::Array { display: true } => "\\begin{darray}",
            Self::Matrix { ty, column_spec } => match (ty, column_spec) {
                (MatrixType::Normal, true) => "\\begin{matrix*}",
                (MatrixType::Normal, false) => "\\begin{matrix}",
                (MatrixType::Small, true) => "\\begin{smallmatrix*}",
                (MatrixType::Small, false) => "\\begin{smallmatrix}",
                (MatrixType::Parens, true) => "\\begin{pmatrix*}",
                (MatrixType::Parens, false) => "\\begin{pmatrix}",
                (MatrixType::Brackets, true) => "\\begin{bmatrix*}",
                (MatrixType::Brackets, false) => "\\begin{bmatrix}",
                (MatrixType::Braces, true) => "\\begin{Bmatrix*}",
                (MatrixType::Braces, false) => "\\begin{Bmatrix}",
                (MatrixType::Vertical, true) => "\\begin{vmatrix*}",
                (MatrixType::Vertical, false) => "\\begin{vmatrix}",
                (MatrixType::DoubleVertical, true) => "\\begin{Vmatrix*}",
                (MatrixType::DoubleVertical, false) => "\\begin{Vmatrix}",
            },
            Self::Cases { left, display } => match (left, display) {
                (true, false) => "\\begin{cases}",
                (true, true) => "\\begin{dcases}",
                (false, false) => "\\begin{rcases}",
                (false, true) => "\\begin{drcases}",
            },
            Self::Equation { eq_numbers: true } => "\\begin{equation}",
            Self::Equation { eq_numbers: false } => "\\begin{equation*}",
            Self::Align { eq_numbers: true } => "\\begin{align}",
            Self::Align { eq_numbers: false } => "\\begin{align*}",
            Self::Aligned => "\\begin{aligned}",
            Self::SubArray => "\\begin{subarray}",
            Self::Alignat { eq_numbers: true } => "\\begin{alignat}",
            Self::Alignat { eq_numbers: false } => "\\begin{alignat*}",
            Self::Alignedat => "\\begin{alignedat}",
            Self::Gather { eq_numbers: true } => "\\begin{gather}",
            Self::Gather { eq_numbers: false } => "\\begin{gather*}",
            Self::Gathered => "\\begin{gathered}",
            Self::Multline => "\\begin{multline}",
            Self::Split => "\\begin{split}",
        }
    }

    pub fn closing_str(&self) -> &'static str {
        match self {
            Self::Normal => "}",
            Self::OptionalArgument => "]",
            Self::BeginEnd => "\\end",
            Self::LeftRight => "\\right",
            Self::Array { display: false } => "\\end{array}",
            Self::Array { display: true } => "\\end{darray}",
            Self::Matrix { ty, column_spec } => match (ty, column_spec) {
                (MatrixType::Normal, true) => "\\end{matrix*}",
                (MatrixType::Normal, false) => "\\end{matrix}",
                (MatrixType::Small, true) => "\\end{smallmatrix*}",
                (MatrixType::Small, false) => "\\end{smallmatrix}",
                (MatrixType::Parens, true) => "\\end{pmatrix*}",
                (MatrixType::Parens, false) => "\\end{pmatrix}",
                (MatrixType::Brackets, true) => "\\end{bmatrix*}",
                (MatrixType::Brackets, false) => "\\end{bmatrix}",
                (MatrixType::Braces, true) => "\\end{Bmatrix*}",
                (MatrixType::Braces, false) => "\\end{Bmatrix}",
                (MatrixType::Vertical, true) => "\\end{vmatrix*}",
                (MatrixType::Vertical, false) => "\\end{vmatrix}",
                (MatrixType::DoubleVertical, true) => "\\end{Vmatrix*}",
                (MatrixType::DoubleVertical, false) => "\\end{Vmatrix}",
            },
            Self::Cases { left, display } => match (left, display) {
                (true, false) => "\\end{cases}",
                (true, true) => "\\end{dcases}",
                (false, false) => "\\end{rcases}",
                (false, true) => "\\end{drcases}",
            },
            Self::Equation { eq_numbers: true } => "\\end{equation}",
            Self::Equation { eq_numbers: false } => "\\end{equation*}",
            Self::Align { eq_numbers: true } => "\\end{align}",
            Self::Align { eq_numbers: false } => "\\end{align*}",
            Self::Aligned => "\\end{aligned}",
            Self::SubArray => "\\end{subarray}",
            Self::Alignat { eq_numbers: true } => "\\end{alignat}",
            Self::Alignat { eq_numbers: false } => "\\end{alignat*}",
            Self::Alignedat => "\\end{alignedat}",
            Self::Gather { eq_numbers: true } => "\\end{gather}",
            Self::Gather { eq_numbers: false } => "\\end{gather*}",
            Self::Gathered => "\\end{gathered}",
            Self::Multline => "\\end{multline}",
            Self::Split => "\\end{split}",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MatrixType {
    Normal,
    Small,
    Parens,
    Brackets,
    Braces,
    Vertical,
    DoubleVertical,
}

/// Represents a column in a matrix or array environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnAlignment {
    /// Content in the column is left-aligned.
    Left,
    /// Content in the column is center-aligned.
    Center,
    /// Content in the column is right-aligned.
    Right,
}

/// Represents a column in an array environment specification.
///
/// It can either be a column specification or a vertical separator specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrayColumn {
    /// A column specification.
    Column(ColumnAlignment),
    /// A vertical separator specification.
    Separator(Line),
}

/// Represents a delimiter size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DelimiterSize {
    /// Corresponds to `\bigl`, `\bigr`, etc.
    Big,
    /// Corresponds to `\Bigl`, `\Bigr`, etc.
    BIG,
    /// Corresponds to `\biggl`, `\biggr`, etc.
    Bigg,
    /// Corresponds to `\Biggl`, `\Biggr`, etc.
    BIGG,
}

impl DelimiterSize {
    pub(crate) fn to_em(self) -> f32 {
        match self {
            DelimiterSize::Big => 1.2,
            DelimiterSize::BIG => 1.8,
            DelimiterSize::Bigg => 2.4,
            DelimiterSize::BIGG => 3.,
        }
    }
}

/// Whether the delimiter is an opening, closing, or fence delimiter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DelimiterType {
    /// Corresponds to the left delimiter.
    Open,
    /// Corresponds to a delimiter that is introduced by the command `\middle`.
    Fence,
    /// Corresponds to the right delimiter.
    Close,
}

/// Represents a line in a `LaTeX` environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Line {
    /// A solid line.
    Solid,
    /// A dashed line.
    Dashed,
}

/// Sometimes mathematical relations can be made of more than one character, so we need a way to
/// represent them when one character is not enough.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelationContent {
    content: (char, Option<char>),
}

impl RelationContent {
    pub(crate) fn single_char(content: char) -> Self {
        Self {
            content: (content, None),
        }
    }

    pub(crate) fn double_char(first: char, second: char) -> Self {
        Self {
            content: (first, Some(second)),
        }
    }

    /// Write the content of the relation to a buffer, and output the filled slice of that
    /// buffer.
    ///
    /// To ensure a successful operation, the buffer must be at least 8 bytes long.
    pub fn encode_utf8_to_buf<'a>(&self, buf: &'a mut [u8]) -> &'a [u8] {
        let mut len = self.content.0.encode_utf8(buf).len();
        if let Some(second) = self.content.1 {
            len += second.encode_utf8(&mut buf[len..]).len();
        }
        &buf[..len]
    }
}

/// Represents a glue specification.
pub type Glue = (Dimension, Option<Dimension>, Option<Dimension>);

/// Represents a LaTeX dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimension {
    /// The value of the dimension.
    pub value: f32,
    /// The unit of the dimension.
    pub unit: DimensionUnit,
}

impl Dimension {
    /// Creates a new dimension.
    pub fn new(value: f32, unit: DimensionUnit) -> Self {
        Self { value, unit }
    }
}

/// Displays a LaTeX dimension into a CSS dimension string.
///
/// The conversion is done using the following relations between the TeX and CSS units:
/// | TeX unit         | CSS unit          |
/// | ---------------- | ----------------- |
/// | 1 em             | 1 em              |
/// | 18 mu            | 1 em              |
/// | 1 ex             | 1 ex              |
/// | 1 mm             | 1 mm              |
/// | 1 cm             | 1 cm              |
/// | 1 in             | 1 in              |
/// | 1 bp             | 1 pt              |
/// | 72.27 pt         | 72 pt             |
/// | 72.27 pc         | 72 pc             |
/// | 65536 * 72.27 sp | 1 pt              |
/// | 1157 * 72.27 dd  | 1238 * 72 pt      |
/// | 1157 * 72.27 cc  | 12 * 1238 * 72 pt |
impl Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const BP_PER_PT: f32 = 72. / 72.27;
        const BP_PER_SP: f32 = 1. / 65536. * BP_PER_PT;
        const BP_PER_DD: f32 = 1238. / 1157. * BP_PER_PT;
        const BP_PER_CC: f32 = 12. * BP_PER_DD;
        const EM_PER_MU: f32 = 1. / 18.;
        let mut value = self.value;
        let unit = match self.unit {
            DimensionUnit::Em => "em",
            DimensionUnit::Mu => {
                value *= EM_PER_MU;
                "mu"
            }
            DimensionUnit::Ex => "ex",
            DimensionUnit::Mm => "mm",
            DimensionUnit::Cm => "cm",
            DimensionUnit::In => "in",
            DimensionUnit::Bp => "pt",
            DimensionUnit::Pt => {
                value *= BP_PER_PT;
                "pt"
            }
            DimensionUnit::Pc => {
                value *= BP_PER_PT;
                "pc"
            }
            DimensionUnit::Sp => {
                value *= BP_PER_SP;
                "pt"
            }
            DimensionUnit::Dd => {
                value *= BP_PER_DD;
                "pt"
            }
            DimensionUnit::Cc => {
                value *= BP_PER_CC;
                "pt"
            }
        };
        write!(f, "{}{}", value, unit)
    }
}

// From the TeXbook, p. 57, 60, 167.
/// Represents a dimension unit in LaTeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionUnit {
    /// The `em` unit.
    Em,
    /// The "math" unit.
    Mu,
    /// The `ex` unit.
    Ex,
    /// The "point" unit.
    Pt,
    /// The "picas" unit.
    Pc,
    /// The "inch" unit.
    In,
    /// The "big point" unit.
    Bp,
    /// The "centimeter" unit.
    Cm,
    /// The "millimeter" unit.
    Mm,
    /// The "didot point" unit.
    Dd,
    /// The "cicero" unit.
    Cc,
    /// The "scaled point" unit.
    Sp,
}
