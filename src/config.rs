use crate::DisplayMode;

pub struct ParserConfig {
    /// If [`DisplayMode::Block`], the math will be rendered in display/block mode,
    /// which will put the math in display style (so \int and \sum are large, for example),
    /// and will center the math on the page on its own line.
    /// If false the math will be rendered in inline mode. (default: [`DisplayMode::Inline`])
    pub display_mode: DisplayMode,
    /// If true, Temml will include an <annotation> element that contains the input TeX string. (default: false)
    pub annotate: bool,
    /// A RGB color. This option determines the color that unsupported commands and invalid LaTeX are rendered in.
    /// (default: (178, 34, 34))
    pub error_color: (u8, u8, u8),
    /// If true, Temml will write a namespace into the <math> element.
    /// That namespace is xmlns="http://www.w3.org/1998/Math/MathML".
    /// Such a namespace is unnecessary for modern browsers but can be helpful for other user agents,
    /// such as Microsoft Word. (default: false)
    pub xml: bool,
    /// If false (similar to MathJax), allow features that make writing LaTeX convenient
    /// but are not actually supported by LaTeX. If true (LaTeX faithfulness mode), throws
    /// an error for any such transgressions. (default: false)
    pub strict: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::Inline,
            annotate: false,
            error_color: (178, 34, 34),
            xml: false,
            strict: false,
        }
    }
}
