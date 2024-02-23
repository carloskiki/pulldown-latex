use crate::DisplayMode;

/// Configuration for the parser.
///
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParserConfig {
    /// See [`DisplayMode`].
    pub display_mode: DisplayMode,
    /// If true, the `mathml` generated includes an `<annotation>` element that contains the input
    /// TeX string.
    pub annotate: bool,
    /// A RGB color. This option determines the color that unsupported commands and invalid LaTeX are rendered in.
    pub error_color: (u8, u8, u8),
    /// If true, a namespace will be written into the <math> element.
    /// That namespace is xmlns="http://www.w3.org/1998/Math/MathML".
    /// Such a namespace is unnecessary for modern browsers but can be helpful for other user agents,
    /// such as Microsoft Word.
    pub xml: bool,
    /// See [`MathStyle`].
    pub math_style: MathStyle,
}

impl Default for ParserConfig {
    /// # Default Value
    /// ```rust
    /// # use pulldown_latexmml::{config::ParserConfig, DisplayMode};
    /// const DEFAULT: ParserConfig = ParserConfig {
    ///     display_mode: DisplayMode::Inline,
    ///     annotate: false,
    ///     error_color: (178, 34, 34),
    ///     xml: false,
    ///     math_style: MathStyle::TeX,
    /// };
    /// assert_eq!(ParserConfig::default(), DEFAULT);
    /// ```
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Inline,
            annotate: false,
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
