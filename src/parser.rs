//! Contains the [`Parser`], which Transforms input `LaTeX` into a stream of `Result<Event, ParserError>`.
//!
//! The parser is used as an iterator, and the events it generates can be rendered by a renderer.
//! The `mahtml` renderer provided by this crate is available through [`push_mathml`] and [`write_mathml`].
//!
//! [`push_mathml`]: crate::mathml::push_mathml
//! [`write_mathml`]: crate::mathml::write_mathml
mod lex;
mod macros;
mod primitives;
mod state;
mod tables;

use std::fmt::Display;

use thiserror::Error;

use crate::event::{Event, Grouping, ScriptPosition, ScriptType};

use self::state::ParserState;

/// The parser completes the task of transforming the input `LaTeX` into a symbolic representation,
/// namely a stream of [`Event`]s.
///
/// Transforming the events into rendered math is a task for the
/// [`mahtml`](crate::mathml) renderer.
///
/// The algorithm of the [`Parser`] is driven by the [`Parser::next`] method on the [`Parser`].
/// This method is provided through the [`Iterator`] trait implementation, thus an end user should
/// only need to use the [`Parser`] as an iterator of `Result<Event, ParserError>`.
// TODO: Change the parser structure so that we have a state where we know we are parsing a string
// and thus we do not have to match on the last element of the stack since we know it to be a
// subgroup.
#[derive(Debug)]
pub struct Parser<'a> {
    /// What the initial input is.
    ///
    /// This is required for error reporting to find and display the context of the error.
    input: &'a str,
    /// The next thing that should be parsed or outputed.
    ///
    /// When this is a string/substring, we should parse it. Some commands output
    /// multiple events, so we need to keep track of them and ouput them in the next
    /// iteration before continuing parsing.
    ///
    /// Instructions are stored backward in this stack, in the sense that the next event to be popped
    /// is the next event to be outputed.
    instruction_stack: Vec<Instruction<'a>>,

    /// This buffer serves as a staging area when parsing a command.
    ///
    /// When a token is parsed, it is first pushed to this stack, then suffixes are checked
    /// (superscript, and subscript), and then the event is moved from the buffer to the instruction stack.
    buffer: Vec<Instruction<'a>>,

    /// The current state of the parser
    state: ParserState,
}

// TODO: When using macros, one should truly just prepend the extended macro to the start of the
// current string.
// We should thus never call `current_string` repeatedly, the string
// outputed by current string is always fully formed.
impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut instruction_stack = Vec::with_capacity(64);
        instruction_stack.push(Instruction::SubGroup {
            content: input,
            allows_alignment: false,
        });
        let buffer = Vec::with_capacity(16);
        Self {
            input,
            instruction_stack,
            buffer,
            state: ParserState::default(),
        }
    }

    /// Get the current string we are parsing.
    ///
    /// This function guarantees that the string returned is not empty.
    ///
    /// # Panics
    ///
    /// This function panics if the top instruction of the stack is not a [`SubGroup`].
    fn current_string(&mut self) -> &mut &'a str {
        match self
            .instruction_stack
            .last_mut()
            .expect("there is something in the stack")
        {
            Instruction::SubGroup { content: s, .. } => s,
            _ => unreachable!(""),
        }
    }

    fn allows_alignment(&self) -> bool {
        match self
            .instruction_stack
            .last()
            .expect("there is something in the stack")
        {
            Instruction::SubGroup {
                allows_alignment, ..
            } => *allows_alignment,
            _ => unreachable!(),
        }
    }

    /// Handles the superscript and/or subscript following what was parsed previously.
    ///
    /// Follows the design decisions described in [`design/suffixes.md`].
    fn handle_suffixes(&mut self) -> InnerResult<Option<Event<'a>>> {
        if self.state.skip_suffixes {
            return Ok(None);
        }

        let mut script_position = if self.state.above_below_suffix_default {
            ScriptPosition::Movable
        } else {
            ScriptPosition::Right
        };

        if self.state.allow_suffix_modifiers {
            if let Some(limits) = lex::limit_modifiers(self.current_string()) {
                if limits {
                    script_position = ScriptPosition::AboveBelow;
                } else {
                    script_position = ScriptPosition::Right;
                }
            }
        }
        let str = self.current_string();
        *str = str.trim_start();

        let Some(next_char) = str.chars().next() else {
            return Ok(None);
        };
        let subscript_first = match next_char {
            '^' => false,
            '_' => true,
            _ => return Ok(None),
        };
        *str = &str[1..];

        let ty = self.rhs_suffixes(subscript_first)?;
        Ok(Some(Event::Script {
            ty,
            position: script_position,
        }))
    }

    fn rhs_suffixes(&mut self, subscript_first: bool) -> InnerResult<ScriptType> {
        let first_suffix_start = self.buffer.len();
        let arg = lex::argument(self.current_string())?;
        self.handle_argument(arg)?;
        let second_suffix_start = self.buffer.len();
        let str = self.current_string();
        let next_char = str.chars().next();
        if (next_char == Some('_') && !subscript_first)
            || (next_char == Some('^') && subscript_first)
        {
            *str = &str[1..];
            let arg = lex::argument(str)?;
            self.handle_argument(arg)?;
        } else if next_char == Some('_') || next_char == Some('^') {
            return Err(if subscript_first {
                ErrorKind::DoubleSubscript
            } else {
                ErrorKind::DoubleSuperscript
            });
        }
        let second_suffix_end = self.buffer.len();

        Ok(
            if !subscript_first && second_suffix_start != second_suffix_end {
                self.instruction_stack.extend(
                    self.buffer[first_suffix_start..second_suffix_start]
                        .iter()
                        .rev(),
                );
                self.instruction_stack.extend(
                    self.buffer
                        .drain(first_suffix_start..)
                        .skip(second_suffix_start - first_suffix_start)
                        .rev(),
                );
                ScriptType::SubSuperscript
            } else {
                let suffixes = self.buffer.drain(first_suffix_start..);
                self.instruction_stack.extend(suffixes.rev());
                if second_suffix_start != second_suffix_end {
                    ScriptType::SubSuperscript
                } else if subscript_first {
                    ScriptType::Subscript
                } else {
                    ScriptType::Superscript
                }
            },
        )
    }

    /// Parse an arugment and pushes the argument to the stack surrounded by a
    /// group: [..., EndGroup, Argument, BeginGroup], when the argument is a subgroup.
    /// Otherwise, it pushes the argument to the stack ungrouped.
    fn handle_argument(&mut self, argument: Argument<'a>) -> InnerResult<()> {
        match argument {
            Argument::Token(token) => {
                self.state.invalidate_relax = true;
                match token {
                    Token::ControlSequence(cs) => self.handle_primitive(cs)?,
                    Token::Character(c) => self.handle_char_token(c)?,
                };
            }
            Argument::Group(group) => {
                self.buffer.extend([
                    Instruction::Event(Event::Begin(Grouping::Normal)),
                    Instruction::SubGroup {
                        content: group,
                        allows_alignment: false,
                    },
                    Instruction::Event(Event::End),
                ]);
            }
        };
        Ok(())
    }

    /// Return the context surrounding the error reported.
    fn error_with_context(&mut self, kind: ErrorKind) -> ParserError<'a> {
        let Some(curr_ptr) = self.instruction_stack.last().and_then(|i| match i {
            Instruction::Event(_) => None,
            // TODO: Here we should check whether the pointer is currently inside a macro definition or inside
            // of the inputed string, when macros are supported.
            Instruction::SubGroup { content: s, .. } => Some(s.as_ptr()),
        }) else {
            return ParserError {
                context: None,
                error: kind,
            };
        };
        let initial_byte_ptr = self.input.as_ptr();
        // Safety:
        // * Both `self` and `origin` must be either in bounds or one
        //   byte past the end of the same [allocated object].
        //   => this is true, as self never changes the allocation of the `input`.
        //
        // * Both pointers must be *derived from* a pointer to the same object.
        //   (See below for an example.)
        //   => this is true, as `initial_byte_ptr` is derived from `input.as_ptr()`, and
        //   `curr_ptr` is derived from `s.as_ptr()`, which points to `input`.
        // * The distance between the pointers, in bytes, must be an exact multiple
        //   of the size of `T`.
        //   => this is true, as both pointers are `u8` pointers.
        // * The distance between the pointers, **in bytes**, cannot overflow an `isize`.
        //   => this is obvious as the size of a string should not overflow an `isize`.
        // * The distance being in bounds cannot rely on "wrapping around" the address space.
        //   => this is true, a `str` does not rely on this behavior either.
        let distance = unsafe { curr_ptr.offset_from(initial_byte_ptr) } as usize;
        let start = floor_char_boundary(self.input, distance.saturating_sub(15));
        let end = floor_char_boundary(self.input, distance + 15);

        ParserError {
            context: Some((&self.input[start..end], distance - start)),
            error: kind,
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Event<'a>, ParserError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.instruction_stack.last_mut() {
            Some(Instruction::Event(_)) => {
                let event = self
                    .instruction_stack
                    .pop()
                    .expect("there is something in the stack");
                Some(Ok(match event {
                    Instruction::Event(event) => event,
                    _ => unreachable!(),
                }))
            }
            Some(Instruction::SubGroup {
                content,
                ..
            }) if content.trim_start().is_empty() => {
                self.instruction_stack.pop();
                self.next()
            }
            Some(Instruction::SubGroup {
                content,
                ..
            }) => {
                // 1. Parse the next token and output everything to the staging stack.
                // TODO: when try blocks hit stable, we should use them. (Please be stable soon!)
                let token = match lex::token(content) {
                    Ok(token) => token,
                    Err(err) => return Some(Err(self.error_with_context(err))),
                };
                let maybe_err = match token {
                    // TODO: when expanding a user defined macro, we do not want to check for
                    // suffixes.
                    Token::ControlSequence(cs) => self.handle_primitive(cs),
                    Token::Character(c) => self.handle_char_token(c),
                };
                if let Err(err) = maybe_err {
                    return Some(Err(self.error_with_context(err)));
                }

                // 2. Check for suffixes, to complete the atom.
                let suffix = match self.handle_suffixes() {
                    Err(err) => return Some(Err(self.error_with_context(err))),
                    Ok(suffix) => suffix,
                };

                // 3. Drain the staging stack to the instruction stack.
                self.instruction_stack.extend(self.buffer.drain(..).rev());
                if let Some(suffix) = suffix {
                    self.instruction_stack.push(Instruction::Event(suffix));
                }

                self.state = ParserState::default();
                self.next()
            }
            None => None,
        }
    }
}

struct ScriptDescriptor {
    subscript_start: usize,
    superscript_start: usize,
    end: usize,
}

pub struct InnerParser<'a> {
    content: &'a str,
    buffer: &'a mut Vec<Instruction<'a>>,
    state: ParserState,
}

impl<'a> InnerParser<'a> {
    fn new(content: &'a str, buffer: &'a mut Vec<Instruction<'a>>, state: ParserState) -> Self {
        Self {
            content,
            buffer,
            state,
        }
    }

    fn parse(self) -> ScriptDescriptor {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Token<'a> {
    ControlSequence(&'a str),
    Character(CharToken<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CharToken<'a> {
    char: &'a str,
}

/// A verified character that retains the string context.
impl<'a> CharToken<'a> {
    fn from_str(s: &'a str) -> Self {
        debug_assert!(
            s.chars().next().is_some(),
            "CharToken must be constructed from a non-empty string"
        );
        Self { char: s }
    }

    fn as_str(&self) -> &'a str {
        self.char
    }
}

impl From<CharToken<'_>> for char {
    fn from(token: CharToken) -> char {
        token.char.chars().next().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Argument<'a> {
    Token(Token<'a>),
    Group(&'a str),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Instruction<'a> {
    /// Send the event
    Event(Event<'a>),
    /// Parse the substring
    SubGroup {
        content: &'a str,
        allows_alignment: bool,
    },
}

/// Anything that could possibly go wrong while parsing.
///
/// This error type is used to provide context to an error which occurs during the parsing stage.
///
/// The [`Parser`] implements the [`Iterator`] trait, which returns a stream of `Result<Event, ParserError>`.
#[derive(Debug, Error)]
pub struct ParserError<'a> {
    context: Option<(&'a str, usize)>,
    #[source]
    error: ErrorKind,
}

impl Display for ParserError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error while parsing: ")?;
        self.error.fmt(f)?;
        if let Some((context, char_position)) = self.context {
            let context = context.replace(['\n', '\t'], " ");
            f.write_str("\n --> Context: ")?;
            const PREFIX_LEN: usize = 14;
            f.write_str(&context)?;
            f.write_str("\n")?;
            f.write_fmt(format_args!("{:>1$}", "^", char_position + PREFIX_LEN))?;
        }
        Ok(())
    }
}

pub(crate) type InnerResult<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug, Error)]
pub(crate) enum ErrorKind {
    #[error("unbalanced group found, expected {:?}", .0)]
    UnbalancedGroup(Option<Grouping>),
    #[error("unkown mathematical environment found")]
    Environment,
    #[error(
        "unexpected math `$` (math shift) character - this character is currently unsupported"
    )]
    MathShift,
    #[error(
        "unexpected hash sign `#` character - this character can only be used in macro definitions"
    )]
    HashSign,
    #[error("unexpected alignment character `&` - this character can only be used in tabular environments (not yet supported)")]
    AlignmentChar,
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
    #[error("trying to add a subscript twice to the same element")]
    DoubleSubscript,
    #[error("trying to add a superscript twice to the same element")]
    DoubleSuperscript,
    #[error("unknown primitive command found")]
    UnknownPrimitive,
    #[error("control sequence found as argument to a command that does not support them")]
    ControlSequenceAsArgument,
    #[error("enpty control sequence")]
    EmptyControlSequence,
    #[error("unkown color. colors must either be predefined or in the form `#RRGGBB`")]
    UnknownColor,
    #[error("expected a number in the range 0..=255 for it to be translated into a character")]
    InvalidCharNumber,
    #[error("cannot use the `\\relax` command in this context")]
    Relax,
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

#[cfg(test)]
mod tests {
    use crate::event::{Content, Identifier, Operator, Visual};

    use super::*;

    #[test]
    fn substr_instructions() {
        let parser = Parser::new("\\bar{y}");
        let events = parser
            .collect::<Result<Vec<_>, ParserError<'static>>>()
            .unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::Superscript,
                    position: ScriptPosition::AboveBelow
                },
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Identifier(Identifier::Char('y'))),
                Event::End,
                Event::Content(Content::Operator(Operator {
                    content: '‾',
                    stretchy: None,
                    deny_movable_limits: false,
                    unicode_variant: false,
                    left_space: None,
                    right_space: None,
                    size: None,
                })),
            ]
        );
    }

    #[test]
    fn subsuperscript() {
        let parser = Parser::new(r"a^{1+3}_2");
        let events = parser
            .inspect(|e| println!("{:?}", e))
            .collect::<Result<Vec<_>, ParserError<'static>>>()
            .unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::SubSuperscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Identifier(Identifier::Char('a'))),
                Event::Content(Content::Number("2")),
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Number("1")),
                Event::Content(Content::Operator(Operator {
                    content: '+',
                    stretchy: None,
                    deny_movable_limits: false,
                    unicode_variant: false,
                    left_space: None,
                    right_space: None,
                    size: None,
                })),
                Event::Content(Content::Number("3")),
                Event::End,
            ]
        );
    }
    #[test]
    fn subscript_torture() {
        let parser = Parser::new(r"a_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_5}}}}}}}}}}}");
        let events = parser
            .collect::<Result<Vec<_>, ParserError<'static>>>()
            .unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Identifier(Identifier::Char('a'))),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Begin(Grouping::Normal),
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Content(Content::Number("5")),
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
                Event::End,
            ]
        )
    }

    #[test]
    fn fraction() {
        let parser = Parser::new(r"\frac{1}{2}_2^4");
        let events = parser
            .collect::<Result<Vec<_>, ParserError<'static>>>()
            .unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::SubSuperscript,
                    position: ScriptPosition::Right
                },
                Event::Visual(Visual::Fraction(None)),
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Number("1")),
                Event::End,
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Number("2")),
                Event::End,
                Event::Content(Content::Number("2")),
                Event::Content(Content::Number("4")),
            ]
        );
    }

    // For mir
    #[test]
    fn multidigit_number() {
        let parser = Parser::new("123");
        let events = parser
            .collect::<Result<Vec<_>, ParserError<'static>>>()
            .unwrap();

        assert_eq!(events, vec![Event::Content(Content::Number("123"))]);
    }
}

// Token parsing procedure, as per TeXbook p. 46-47.
//
// This is roughly what the lexer implementation will look like for text mode.
//
// 1. Trim any trailing whitespace from a line.
//
// 2. If '\' (escape character) is encountered, parse the next token.
//  '\n' => _The name is empty_???
//  'is_ascii_alphabetic' => parse until an non ASCII alphabetic, and the name is the token
//  'otherwise' => parse next character, and the name is the symbol.
//
//  Go to SkipBlanks mode if the token is a word or a space symbol.
//  Otherwise, go to MidLine mode.
//
// 3. If `^^` is found:
//  - If the following are two characters of type ASCII lowercase letter or digit,
//  then `^^__` is converted to the correspoding ascii value.
//  - If the following is a single ASCII character, then `^^_` is converted to the corresponding ASCII
//  value with the formula: if `c` is the character, then `c + 64` if `c` if the character has code
//  between 0 and 63, and `c - 64` if the character has code between 64 and 127.
//
//  __Note__: This rule takes precedence over escape character parsing. If such a sequence is found
//  in an escape sequence, it is converted to the corresponding ASCII value.
//
// 4. If the token is a single character, go to MidLine mode.
//
// 5. If the token is an end of line, go to the next line. If nothing was on the line (were in NewLine state), then the
//  `par` token is emitted, meaning that a new paragraph should be started.
//  If the state was MidLine, then the newline is transformed into a space.
//  If the state was SkipBlanks, then the newline is ignored.
//
// 6. Ignore characters from the `Ignore` category.
//
// 7. If the token is a space and the mode is MidLine, the space is transformed into a space token.
//
// 8. If the token is a comment, ignore the rest of the line, and go to the next line.
//
// 9. Go to newlines on the next line.
