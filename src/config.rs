//! Configuration options for the renderer.
//! 
//! The configuration of the `mathml` renderer is done through the [`RenderConfig`] struct.
use std::fmt::Display;

/// Configuration for the `mathml` renderer.
///
/// The default value is: [`RenderConfig::default`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderConfig<'a> {
    /// See [`DisplayMode`].
    pub display_mode: DisplayMode,
    /// If Some, the `mathml` generated includes an `<annotation>` element that contains the
    /// provided string. It is commonly used to include the LaTeX source code in the generated `mathml`.
    pub annotation: Option<&'a str>,
    /// A RGB color. This option determines the color in which errors and invalid LaTeX commands are rendered in.
    pub error_color: (u8, u8, u8),
    /// If true, a namespace `xml` will be written into the <math> element.
    /// That namespace is xmlns="http://www.w3.org/1998/Math/MathML".
    /// Such a namespace is unnecessary for modern browsers but can be helpful for other user agents,
    /// such as Microsoft Word.
    pub xml: bool,
    /// See [`MathStyle`].
    pub math_style: MathStyle,
}

impl<'a> RenderConfig<'a> {
    /// Create a new `RenderConfig` with the provided annotation, and default values for the rest of the fields.
    pub fn with_annotation(annotation: &'a str) -> Self {
        Self {
            annotation: Some(annotation),
            ..Self::default()
        }
    }
}

impl<'a> Default for RenderConfig<'a> {
    /// # Default Value
    /// ```rust
    /// # use pulldown_latex::{config::ParserConfig, DisplayMode};
    /// const DEFAULT: ParserConfig = ParserConfig {
    ///     display_mode: DisplayMode::Inline,
    ///     annotate: None,
    ///     error_color: (178, 34, 34),
    ///     xml: false,
    ///     math_style: MathStyle::TeX,
    /// };
    /// assert_eq!(ParserConfig::default(), DEFAULT);
    /// ```
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Inline,
            annotation: None,
            error_color: (178, 34, 34),
            xml: false,
            math_style: MathStyle::TeX,
        }
    }
}

/// The way in which math variables are displayed.
///
/// This is used to determine how single-letter variables are displayed. This affects lowercase and
/// uppercase latin letters (__a-z__ and __A-Z__), and the uppercase and lowercase greek letters
/// (__α-ω__ and __Α-Ω__). Here is a table of the different styles:
///
/// ## Math Styles
///
/// | Style     | Low. Latin | Upp. Latin | Low. Greek | Upp. Greek |
/// | -----     | ---------- | ---------- | ---------- | ---------- |
/// | `TeX`     | _italic_   | _italic_   | _italic_   | upright    |
/// | `ISO`     | _italic_   | _italic_   | _italic_   | _italic_    |
/// | `French`  | _italic_   | upright    | upright    | upright    |
/// | `Upright` | upright    | upright    | upright    | upright    |
///
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum MathStyle {
    /// The default style used in TeX.
    ///
    /// Makes everything italic except for uppercase greek letters.
    ///
    /// __This is the default value.__
    #[default]
    TeX,
    /// The style used in ISO 80000-2:2019.
    ///
    /// Makes everything italic.
    ISO,
    /// The style used in French typography.
    ///
    /// Makes everything upright except for lowercase latin letters.
    French,
    /// Makes everything upright.
    Upright,
}

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

impl Display for DisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayMode::Inline => f.write_str("inline"),
            DisplayMode::Block => f.write_str("block"),
        }
    }
}
