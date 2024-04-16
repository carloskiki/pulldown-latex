//! Contains the [`Parser`], which Transforms input `LaTeX` into a stream of `Result<Event, ParserError>`.
//!
//! The parser can be used as an iterator, and the events can be rendered by a renderer.
//! The `mahtml` renderer provided by this crate is available through [`push_mathml`] and [`write_mathml`].
//!
//! [`push_mathml`]: crate::mathml::push_mathml
//! [`write_mathml`]: crate::mathml::write_mathml
mod lex;
mod macros;
mod primitives;
mod state;
pub mod tables;

use std::fmt::{Display, Write};

use thiserror::Error;

use crate::event::{Content, Event, ScriptPosition, ScriptType};

use self::state::ParserState;

/// The parser completes the task of transforming the input `LaTeX` into a symbolic representation,
/// namely a stream of [`Event`]s.
///
/// Transforming the events into rendered math is a task for the
/// `mahtml` renderer.
/// 
/// The algorithm of the [`Parser`] is driven by the [`Parser::next`] method on the [`Parser`].
/// This method is provided through the [`Iterator`] trait implementation, thus an end user should
/// only need to use the [`Parser`] as an iterator of `Result<Event, ParserError>`.
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

// TODO: make `trim_start` (removing whitespace) calls more systematic.
impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut instruction_stack = Vec::with_capacity(64);
        instruction_stack.push(Instruction::Substring(input));
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
    fn current_string(&mut self) -> Option<&mut &'a str> {
        match self.instruction_stack.last()? {
            Instruction::Event(_) => None,
            Instruction::Substring("") => {
                self.instruction_stack.pop();
                self.current_string()
            }
            Instruction::Substring(_) => self.instruction_stack.last_mut().map(|i| match i {
                Instruction::Substring(s) => s,
                _ => unreachable!(),
            }),
        }
    }

    /// Handles the superscript and/or subscript following what was parsed previously.
    ///
    /// Follows the design decisions described in [`design/suffixes.md`].
    fn check_suffixes(&mut self) -> InnerResult<Option<Event<'a>>> {
        if self.state.skip_suffixes {
            return Ok(None)
        }
        
        let mut script_position = if self.state.above_below_suffix_default {
            ScriptPosition::Movable
        } else {
            ScriptPosition::Right
        };

        if self.state.allow_suffix_modifiers {
            while let Some(str) = self.current_string() {
                *str = str.trim_start();
                if str.starts_with(r"\limits") {
                    *str = &str[7..];
                    script_position = ScriptPosition::AboveBelow;
                } else if str.starts_with(r"\nolimits") {
                    *str = &str[9..];
                    script_position = ScriptPosition::Right;
                } else {
                    break;
                }
            }
        }

        let mut subscript_first = false;
        let first_suffix_start = self.buffer.len();
        let Some(str) = self.current_string() else {
            return Ok(None);
        };
        *str = str.trim_start();

        let Some(next_char) = str.chars().next() else {
            return Ok(None);
        };
        match next_char {
            '^' => {}
            '_' => {
                subscript_first = true;
            }
            _ => return Ok(None),
        };
        *str = &str[1..];
        let str = self.current_string().ok_or(if subscript_first {
            ErrorKind::EmptySubscript
        } else {
            ErrorKind::EmptySuperscript
        })?;

        let arg = lex::argument(str)?;
        self.handle_argument(arg)?;
        let second_suffix_start = self.buffer.len();
        if let Some(str) = self.current_string() {
            let next_char = str.chars().next().expect("current_string is not empty");
            if (next_char == '_' && !subscript_first) || (next_char == '^' && subscript_first) {
                *str = &str[1..];
                let str = self.current_string().ok_or(if subscript_first {
                    ErrorKind::EmptySuperscript
                } else {
                    ErrorKind::EmptySubscript
                })?;
                let arg = lex::argument(str)?;
                self.handle_argument(arg)?;
            } else if next_char == '_' || next_char == '^' {
                return Err(if subscript_first {
                    ErrorKind::DoubleSubscript
                } else {
                    ErrorKind::DoubleSuperscript
                });
            }
        };
        let second_suffix_end = self.buffer.len();

        let script_type = if !subscript_first && second_suffix_start != second_suffix_end {
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
        };
        Ok(Some(Event::Script {
            ty: script_type,
            position: script_position,
        }))
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
                    Instruction::Event(Event::BeginGroup),
                    Instruction::Substring(group),
                    Instruction::Event(Event::EndGroup),
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
            Instruction::Substring(s) => Some(s.as_ptr()),
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
        match self.instruction_stack.last() {
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
            Some(Instruction::Substring(_)) => {
                let content = match self.current_string() {
                    Some(content) => content,
                    None => return self.next(),
                };

                // 1. Parse the next token and output everything to the staging stack.
                let maybe_err = match content.chars().next() {
                    Some('0'..='9') => {
                        let mut len = content
                            .chars()
                            .skip(1)
                            .take_while(|&c| matches!(c, '.' | ',' | '0'..='9'))
                            .count()
                            + 1;
                        if matches!(content.as_bytes()[len - 1], b'.' | b',') {
                            len -= 1;
                        }
                        let (number, rest) = content.split_at(len);
                        *content = rest;
                        self.buffer
                            .push(Instruction::Event(Event::Content(Content::Number(number))));
                        Ok(())
                    }
                    Some('%') => {
                        if let Some((_, rest)) = content.split_once('\n') {
                            *content = rest;
                        } else {
                            *content = &content[content.len()..];
                        }
                        return self.next();
                    }
                    // TODO: when expanding a user defined macro, we do not want to check for
                    // suffixes.
                    Some('\\') => {
                        *content = &content[1..];
                        match lex::rhs_control_sequence(content) {
                            Ok(cs) => self.handle_primitive(cs),
                            Err(err) => Err(err),
                        }
                    }
                    Some(c) => {
                        let (c, rest) = &content.split_at(c.len_utf8());
                        *content = rest;
                        self.handle_char_token(CharToken::from_str(c))
                    }
                    None => return self.next(),
                };
                if let Err(err) = maybe_err {
                    return Some(Err(self.error_with_context(err)));
                };

                // 2. Check for suffixes, to complete the atom.
                let suffix = match self.check_suffixes() {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Token<'a> {
    ControlSequence(&'a str),
    Character(CharToken<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CharToken<'a> {
    char: &'a str,
}

impl<'a> CharToken<'a> {
    fn from_str(s: &'a str) -> Self {
        assert!(
            s.chars().count() == 1,
            "CharToken must be a single character"
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
    Substring(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupType {
    /// The group was initiated by a `{` character.
    Brace,
    /// The group was initiated by a `\begingroup` command.
    BeginGroup,
    /// The group was initiated by a `\left` command.
    LeftRight,
}

impl Display for GroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupType::Brace => f.write_char('}'),
            GroupType::BeginGroup => f.write_str("\\endgroup"),
            GroupType::LeftRight => f.write_str("\\right"),
        }
    }
}

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
    #[error("unbalanced group found, expected {}", .0.map_or(String::from("no group closing"), |t| t.to_string()))]
    UnbalancedGroup(Option<GroupType>),
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
    #[error("expected a dimension specification")]
    Dimension,
    #[error("expected a dimension or glue specification")]
    Glue,
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
    #[error("trying to add a subscript with no content")]
    EmptySubscript,
    #[error("trying to add a superscript with no content")]
    EmptySuperscript,
    #[error("trying to add a subscript twice to the same element")]
    DoubleSubscript,
    #[error("trying to add a superscript twice to the same element")]
    DoubleSuperscript,
    #[error("trying to add a subscript as a token")]
    SubscriptAsToken,
    #[error("trying to add a superscript as a token")]
    SuperscriptAsToken,
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

#[cfg(test)]
mod tests {
    use crate::event::{Identifier, Operator, Visual};

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
                Event::BeginGroup,
                Event::Content(Content::Identifier(Identifier::Char('y'))),
                Event::EndGroup,
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
                Event::BeginGroup,
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
                Event::EndGroup,
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
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::BeginGroup,
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Number("5")),
                Event::Content(Content::Number("5")),
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
                Event::EndGroup,
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
                Event::BeginGroup,
                Event::Content(Content::Number("1")),
                Event::EndGroup,
                Event::BeginGroup,
                Event::Content(Content::Number("2")),
                Event::EndGroup,
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

        assert_eq!(
            events,
            vec![Event::Content(Content::Number("123"))]
        );
    }
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
