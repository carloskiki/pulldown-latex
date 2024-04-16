/// State belonging to the parser that is reset every call to the `next` method of the parser.
#[derive(Debug, Clone, Copy)]
pub struct ParserState {
    /// Whether the parser is currently parsing an operator that allows for its suffixes to be
    /// modifies by the commands `\nolimits`, `\limits`, and `\displaylimits`.
    pub allow_suffix_modifiers: bool,
    /// Whether the suffixes of the operator are set above and below the operator by default.
    pub above_below_suffix_default: bool,
    /// Whether the parser should skip suffix parsing for the current event.
    pub skip_suffixes: bool
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            allow_suffix_modifiers: false,
            above_below_suffix_default: false,
            skip_suffixes: false
        }
    }
}
