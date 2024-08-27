//! Error type returned by the parser upon failure.
//!
//! This error type is used to provide context to an error which occurs during the parsing stage.
use std::{error::Error, fmt::Display};

use super::SpanStack;
use crate::event::GroupingKind;

/// Anything that could possibly go wrong while parsing.
///
/// This error type is used to provide context to an error which occurs during the parsing stage.
///
/// The [`Parser`](crate::Parser) implements the [`Iterator`] trait, which returns a stream of `Result<Event, ParserError>`.
#[derive(Debug)]
pub struct ParserError {
    inner: Box<Inner>,
}

#[derive(Debug)]
struct Inner {
    error: ErrorKind,
    context: Box<str>,
}

// TODO: add arrows: ^^^ to show where in the context the error occurred.
impl ParserError {
    pub(super) fn new(error: ErrorKind, place: *const u8, span_stack: &mut SpanStack) -> Self {
        const CONTEXT_SIZE: usize = 12;
        const CONTEXT_PREFIX: &str = "╭─► context:\n";
        const EXPANSION_PREFIX: &str = "─► which was expanded from:\n";

        let index = span_stack.reach_original_call_site(place);
        let mut context = String::from(CONTEXT_PREFIX);

        let first_string = span_stack
            .expansions
            .last()
            .map(|exp| exp.full_expansion)
            .unwrap_or(span_stack.input);

        let (mut lower_bound, mut upper_bound) = (
            floor_char_boundary(first_string, index.saturating_sub(CONTEXT_SIZE)),
            floor_char_boundary(first_string, index + CONTEXT_SIZE),
        );

        for (index, expansion) in span_stack.expansions.iter().rev().enumerate() {
            let next_string = (span_stack.expansions.len() - 1)
                .checked_sub(index + 1)
                .map(|index| span_stack.expansions[index].full_expansion)
                .unwrap_or(span_stack.input);

            if lower_bound > expansion.expansion_length {
                lower_bound += expansion.call_site_in_origin.start;
                upper_bound =
                    (expansion.call_site_in_origin.end + upper_bound).min(next_string.len());

                continue;
            }

            let context_str = &expansion.full_expansion[lower_bound..upper_bound];
            write_context_str(context_str, &mut context, false, lower_bound > 0);
            context.push_str(EXPANSION_PREFIX);

            lower_bound = floor_char_boundary(
                next_string,
                expansion
                    .call_site_in_origin
                    .start
                    .saturating_sub(CONTEXT_SIZE),
            );
            upper_bound = floor_char_boundary(
                next_string,
                expansion.call_site_in_origin.end + CONTEXT_SIZE,
            );
        }
        write_context_str(
            &span_stack.input[lower_bound..upper_bound],
            &mut context,
            true,
            lower_bound > 0,
        );
        context.shrink_to_fit();

        Self {
            inner: Box::new(Inner {
                error,
                context: context.into_boxed_str(),
            }),
        }
    }
}

fn write_context_str(context: &str, out: &mut String, last: bool, has_previous_content: bool) {
    out.push_str("│\n");
    let mut lines = context.lines();
    if let Some(line) = lines.next() {
        out.push('│');
        if has_previous_content {
            out.push('…');
        } else {
            out.push(' ');
        }
        out.push_str(line);
        out.push('\n');
    }

    lines.for_each(|line| {
        out.push_str("│ ");
        out.push_str(line);
        out.push('\n');
    });
    let last_line_len = context.lines().last().unwrap_or_default().len();
    out.push_str("│ ");
    (0..last_line_len).for_each(|_| out.push('^'));
    out.push('\n');
    if last {
        out.push_str("╰─");
        (0..last_line_len).for_each(|_| out.push('─'));
    } else {
        out.push('├');
    }
}

// fn reach_original_call_site(&mut self, substr_start: *const u8) -> usize {
//     let mut ptr_val = substr_start as isize;
//
//     dbg!(&self, ptr_val);
//
//     while let Some(expansion) = self.expansions.last() {
//         let expansion_ptr = expansion.full_expansion.as_ptr() as isize;
//
//         if ptr_val >= expansion_ptr
//             && ptr_val <= expansion_ptr + expansion.full_expansion.len() as isize
//         {
//             let index = if ptr_val <= expansion_ptr + expansion.expansion_length as isize {
//                 (ptr_val - expansion_ptr) as usize
//             } else {
//                 dbg!("we are here");
//                 let distance_from_effective_stop =
//                     ptr_val - expansion_ptr - expansion.expansion_length as isize;
//                 self.expansions.pop();
//                 ptr_val = self
//                     .expansions
//                     .last()
//                     .map(|exp| exp.full_expansion)
//                     .unwrap_or(self.input)
//                     .as_ptr() as isize
//                     + distance_from_effective_stop;
//                 continue;
//             };
//             return index;
//         }
//         self.expansions.pop();
//     }
//     let input_start = self.input.as_ptr() as isize;
//
//     dbg!(&self, ptr_val, input_start, self.input, self.input.len());
//
//     assert!(ptr_val > input_start && ptr_val <= input_start + self.input.len() as isize);
//     (ptr_val - input_start) as usize
// }

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner.error)
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("parsing error: ")?;
        self.inner.error.fmt(f)?;
        f.write_str("\n")?;
        f.write_str(&self.inner.context)?;
        Ok(())
    }
}

pub(crate) type InnerResult<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug)]
pub(crate) enum ErrorKind {
    UnbalancedGroup(Option<GroupingKind>),
    Environment,
    MathShift,
    HashSign,
    EndOfInput,
    DimensionArgument,
    DimensionUnit,
    MathUnit,
    Delimiter,
    ControlSequence,
    Number,
    CharacterNumber,
    Argument,
    GroupArgument,
    DoubleSubscript,
    DoubleSuperscript,
    UnknownPrimitive,
    ControlSequenceAsArgument,
    ScriptAsArgument,
    EmptyControlSequence,
    UnknownColor,
    InvalidCharNumber,
    Relax,
    BracesInParamText,
    CommentInParamText,
    IncorrectMacroParams(u8, u8),
    IncorrectReplacementParams(u8, u8),
    TooManyParams,
    StandaloneHashSign,
    IncorrectMacroPrefix,
    MacroAlreadyDefined,
    MacroNotDefined,
    Alignment,
    NewLine,
    ArrayNoColumns,
    MissingExpansion,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::UnbalancedGroup(Some(missing)) => {
                write!(f, "unbalanced group found, expected it to be closed with `{}`", missing.closing_str())
            },
            ErrorKind::UnbalancedGroup(None) => f.write_str("unbalanced group found, unexpected group closing found"),
            ErrorKind::Environment => f.write_str("unkown mathematical environment found"),
            ErrorKind::MathShift => f.write_str(
                "unexpected math `$` (math shift) character - this character cannot be used inside math mode"),
            ErrorKind::HashSign => f.write_str(
                "unexpected hash sign `#` character - this character can only be used in macro definitions"
            ),
            ErrorKind::EndOfInput => f.write_str("unexpected end of input"),
            ErrorKind::MathUnit => f.write_str("expected mathematical units (mu) in dimension specification"),
            ErrorKind::Delimiter => f.write_str("expected a delimiter token"),
            ErrorKind::ControlSequence => f.write_str("expected a control sequence"),
            ErrorKind::Number => f.write_str("expected a number"),
            ErrorKind::CharacterNumber => f.write_str("expected a character representing a number after '`'. found a non ascii character"),
            ErrorKind::Argument => f.write_str("expected an argument"),
            ErrorKind::GroupArgument => f.write_str("expected an argument delimited by `{{}}`"),
            ErrorKind::DoubleSubscript => f.write_str("trying to add a subscript twice to the same element"),
            ErrorKind::DoubleSuperscript => f.write_str("trying to add a superscript twice to the same element"),
            ErrorKind::UnknownPrimitive => f.write_str("unknown primitive command found"),
            ErrorKind::ControlSequenceAsArgument => f.write_str("control sequence found as argument to a command that does not support them"),
            ErrorKind::ScriptAsArgument => f.write_str("subscript and/or superscript found as argument to a command"),
            ErrorKind::EmptyControlSequence => f.write_str("empty control sequence"),
            ErrorKind::UnknownColor => f.write_str("unkown color. colors must either be predefined or in the form `#RRGGBB`"),
            ErrorKind::InvalidCharNumber => f.write_str("expected a number in the range 0..=255 for it to be translated into a character"),
            ErrorKind::Relax => f.write_str("cannot use the `\\relax` command in this context"),
            ErrorKind::BracesInParamText => f.write_str("macro definition of parameters contains '{{' or '}}'"),
            ErrorKind::CommentInParamText => f.write_str("macro definition of parameters contains a (`%`) comment"),
            ErrorKind::IncorrectMacroParams(found, expected) => {
                write!(f, "macro definition found parameter #{} but expected #{}", found, expected)
            }
            ErrorKind::IncorrectReplacementParams(found, expected) => {
                write!(f, "macro definition found parameter #{} but expected a parameter in the range [1, {}]", found, expected)
            }
            ErrorKind::TooManyParams => f.write_str("macro definition contains too many parameters, the maximum is 9"),
            ErrorKind::StandaloneHashSign => f.write_str("macro definition contains a standalone '#'"),
            ErrorKind::IncorrectMacroPrefix => f.write_str("macro use does not match its definition, expected it to begin with a prefix string as specified in the definition"),
            ErrorKind::MacroAlreadyDefined => f.write_str("macro already defined"),
            ErrorKind::MacroNotDefined => f.write_str("macro not defined"),
            ErrorKind::DimensionArgument => f.write_str("expected a dimension or glue argument"),
            ErrorKind::DimensionUnit => f.write_str("expected a dimensional unit"),
            ErrorKind::Alignment => f.write_str("alignment not allowed in current environment"),
            ErrorKind::NewLine => f.write_str("new line command not allowed in current environment"),
            ErrorKind::ArrayNoColumns => f.write_str("array must have at least one column of the type `c`, `l` or `r`"),
            ErrorKind::MissingExpansion => f.write_str("The macro definition is missing an expansion"),
        }
    }
}

impl Error for ErrorKind {}

fn floor_char_boundary(str: &str, index: usize) -> usize {
    if index >= str.len() {
        str.len()
    } else {
        let lower_bound = index.saturating_sub(3);
        let new_index = str.as_bytes()[lower_bound..=index].iter().rposition(|b| {
            // This is bit magic equivalent to: b < 128 || b >= 192
            (*b as i8) >= -0x40
        });

        // SAFETY: we know that the character boundary will be within four bytes
        unsafe { lower_bound + new_index.unwrap_unchecked() }
    }
}
