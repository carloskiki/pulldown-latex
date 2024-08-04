//! Error type returned by the parser upon failure.
//!
//! This error type is used to provide context to an error which occurs during the parsing stage.
use std::{error::Error, fmt::Display};
use thiserror::Error;

use super::SpanStack;
use crate::event::Grouping;

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

impl ParserError {
    pub(super) fn new(error: ErrorKind, place: *const u8, span_stack: &mut SpanStack) -> Self {
        const CONTEXT_SIZE: usize = 12;
        const CONTEXT_PREFIX: &str = "context: ";
        const EXPANSION_PREFIX: &str = "    which was expanded from: ";

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
            context.push_str(context_str);
            context.push('\n');
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
        context.push_str(&span_stack.input[lower_bound..upper_bound]);
        context.shrink_to_fit();

        Self {
            inner: Box::new(Inner {
                error,
                context: context.into_boxed_str(),
            }),
        }
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

#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    // TODO: this error is very misleading. Rework it.
    #[error("unbalanced group found, expected {:?}", .0)]
    UnbalancedGroup(Option<Grouping>),
    #[error("unkown mathematical environment found")]
    Environment,
    #[error(
        "unexpected math `$` (math shift) character - this character cannot be used inside math mode"
    )]
    MathShift,
    #[error(
        "unexpected hash sign `#` character - this character can only be used in macro definitions"
    )]
    HashSign,
    #[error("unexpected end of input")]
    EndOfInput,
    #[error("expected a dimension or glue argument")]
    DimensionArgument,
    #[error("expected a dimensional unit")]
    DimensionUnit,
    #[error("expected mathematical units (mu) in dimension specification")]
    MathUnit,
    #[error("expected a delimiter token")]
    Delimiter,
    #[error("expected a control sequence")]
    ControlSequence,
    #[error("expected a number")]
    Number,
    #[error("expected a character representing a number after '`'. found a non ascii character")]
    CharacterNumber,
    #[error("expected an argument")]
    Argument,
    #[error("expected an argument delimited by `{{}}`")]
    GroupArgument,
    #[error("trying to add a subscript twice to the same element")]
    DoubleSubscript,
    #[error("trying to add a superscript twice to the same element")]
    DoubleSuperscript,
    #[error("unknown primitive command found")]
    UnknownPrimitive,
    #[error("control sequence found as argument to a command that does not support them")]
    ControlSequenceAsArgument,
    #[error("subscript and/or superscript found as argument to a command")]
    ScriptAsArgument,
    #[error("empty control sequence")]
    EmptyControlSequence,
    #[error("unkown color. colors must either be predefined or in the form `#RRGGBB`")]
    UnknownColor,
    #[error("expected a number in the range 0..=255 for it to be translated into a character")]
    InvalidCharNumber,
    #[error("cannot use the `\\relax` command in this context")]
    Relax,
    #[error("macro definition of parameters contains '{{' or '}}'")]
    BracesInParamText,
    #[error("macro definition of parameters contains a (`%`) comment")]
    CommentInParamText,
    #[error("macro definition found parameter #{0} but expected #{1}")]
    IncorrectMacroParams(u8, u8),
    #[error(
        "macro definition found parameter #{0} but expected a parameter in the range [#1, #{1}]"
    )]
    IncorrectReplacementParams(u8, u8),
    #[error("macro definition contains too many parameters, the maximum is 9")]
    TooManyParams,
    #[error("macro definition contains a standalone '#'")]
    StandaloneHashSign,
    // TODO: should specify what the macro expects the prefix string to be.
    #[error("macro use does not match its definition, expected it to begin with a prefix string as specified in the definition")]
    IncorrectMacroPrefix,
    #[error("macro already defined")]
    MacroAlreadyDefined,
    #[error("macro not defined")]
    MacroNotDefined,
}

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
