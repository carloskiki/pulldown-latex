use super::AlignmentCount;
use crate::event::ScriptPosition;

/// State belonging to the parser that is reset every call to the `next` method of the parser.
#[derive(Debug)]
pub struct ParserState<'a> {
    /// Whether the parser is currently parsing an operator that allows for its suffixes to be
    /// modifies by the commands `\nolimits`, `\limits`, and `\displaylimits`.
    pub allow_suffix_modifiers: bool,
    /// What type of suffix should be rendered by default for the current operator.
    pub suffix_position: ScriptPosition,
    /// Whether the parser should skip suffix parsing for the current event.
    pub skip_suffixes: bool,
    /// Whether we are currently handling an arument to a control sequence.
    ///
    /// This affects things like whether we can parse the `\relax` command and
    /// subscripts/superscripts.
    pub handling_argument: bool,
    /// Number of `&` characters allowed in the current line of the current group.
    ///
    /// If `None`, then we are in a group where both `\\` (newlines) and `&` (alignments) are disallowed.
    /// Otherwise, this is the number of `&` characters allowed in the current line.
    pub allowed_alignment_count: Option<&'a mut AlignmentCount>,
}

impl<'a> Default for ParserState<'a> {
    fn default() -> Self {
        Self {
            allow_suffix_modifiers: false,
            suffix_position: ScriptPosition::Right,
            skip_suffixes: false,
            handling_argument: false,
            allowed_alignment_count: None,
        }
    }
}
