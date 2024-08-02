//! Contains the [`Parser`], which Transforms input `LaTeX` into a stream of `Result<Event, ParserError>`.
//!
//! The parser is used as an iterator, and the events it generates can be rendered by a renderer.
//! The `mahtml` renderer provided by this crate is available through [`push_mathml`] and [`write_mathml`].
//!
//! [`push_mathml`]: crate::mathml::push_mathml
//! [`write_mathml`]: crate::mathml::write_mathml
pub mod error;
mod lex;
pub mod macros;
mod primitives;
mod state;
pub mod storage;
mod tables;

use std::ops::Range;

use macros::MacroContext;

use crate::event::{Event, Grouping, ScriptPosition, ScriptType};

use self::{state::ParserState, storage::Storage};

pub(crate) use error::{ErrorKind, InnerResult, ParserError};

/// The parser completes the task of transforming the input `LaTeX` into a symbolic representation,
/// namely a stream of [`Event`]s.
///
/// Transforming the events into rendered math is a task for the
/// [`mahtml`](crate::mathml) renderer.
///
/// The algorithm of the [`Parser`] is driven by the [`Parser::next`] method.
/// This method is provided through the [`Iterator`] trait implementation, thus an end user should
/// only need to use the [`Parser`] as an iterator of `Result<Event, ParserError>`.
#[derive(Debug)]
pub struct Parser<'store> {
    /// The next thing that should be parsed or outputed.
    ///
    /// When this is a string/substring, we should parse it. Some commands output
    /// multiple events, so we need to keep track of them and ouput them in the next
    /// iteration before continuing parsing.
    ///
    /// Instructions are stored backward in this stack, in the sense that the next event to be popped
    /// is the next event to be outputed.
    instruction_stack: Vec<Instruction<'store>>,

    /// This buffer serves as a staging area when parsing a command.
    ///
    /// When a token is parsed, it is first pushed to this buffer, then scripts are checked
    /// (superscript, and subscript), and then the events are moved from the buffer to the instruction stack.
    buffer: Vec<Instruction<'store>>,

    /// Macro definitions.
    macro_context: MacroContext<'store>,

    /// Where Macros are expanded if ever needed.
    storage: &'store bumpalo::Bump,

    /// A stack that serves to provide context when an error occurs.
    span_stack: SpanStack<'store>,
}

impl<'store> Parser<'store> {
    pub fn new<'input>(input: &'input str, storage: &'store Storage) -> Self
    where
        'input: 'store,
    {
        let mut instruction_stack = Vec::with_capacity(32);
        instruction_stack.push(Instruction::SubGroup {
            content: input,
            allowed_alignment_count: None,
        });
        let buffer = Vec::with_capacity(16);
        Self {
            instruction_stack,
            buffer,
            macro_context: MacroContext::new(),
            storage: &storage.0,
            span_stack: SpanStack::from_input(input),
        }
    }
}

impl<'store> Iterator for Parser<'store> {
    type Item = Result<Event<'store>, ParserError>;

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
            Some(Instruction::SubGroup { content, .. }) if content.trim_start().is_empty() => {
                self.instruction_stack.pop();
                self.next()
            }
            Some(Instruction::SubGroup {
                content,
                allowed_alignment_count,
                ..
            }) => {
                let state = ParserState {
                    allowed_alignment_count: allowed_alignment_count.as_mut(),
                    ..Default::default()
                };

                let inner = InnerParser {
                    content,
                    buffer: &mut self.buffer,
                    state,
                    macro_context: &mut self.macro_context,
                    storage: self.storage,
                    span_stack: &mut self.span_stack,
                };

                let (desc, rest) = inner.parse_next();
                *content = rest;

                let script_event = match desc {
                    Err(e) => {
                        let content_str = *content;
                        return Some(Err(ParserError::new(
                            e,
                            content_str.as_ptr(),
                            &mut self.span_stack,
                        )));
                    }
                    Ok(Some((e, desc))) => {
                        if desc.subscript_start > desc.superscript_start {
                            let content = self.buffer.drain(desc.superscript_start..).rev();
                            let added_len = content.len();

                            self.instruction_stack.reserve(added_len);
                            let spare =
                                &mut self.instruction_stack.spare_capacity_mut()[..added_len];
                            let mut idx = desc.subscript_start - desc.superscript_start;

                            for e in content {
                                if idx == added_len {
                                    idx = 0;
                                }
                                spare[idx].write(e);
                                idx += 1;
                            }

                            // Safety: The new length is less than the vector's capacity because we
                            // reserved `added_len` previously. Every element in the vector up to
                            // that new length is also initialized by the loop.
                            unsafe {
                                self.instruction_stack
                                    .set_len(self.instruction_stack.len() + added_len)
                            };
                        } else {
                            self.instruction_stack
                                .extend(self.buffer.drain(desc.subscript_start..).rev());
                        }
                        Some(e)
                    }
                    Ok(None) => None,
                };

                self.instruction_stack.extend(self.buffer.drain(..).rev());
                if let Some(e) = script_event {
                    self.instruction_stack.push(Instruction::Event(e));
                }
                self.next()
            }
            None => None,
        }
    }
}

struct InnerParser<'b, 'store> {
    content: &'store str,
    buffer: &'b mut Vec<Instruction<'store>>,
    state: ParserState<'b>,
    macro_context: &'b mut MacroContext<'store>,
    storage: &'store bumpalo::Bump,
    span_stack: &'b mut SpanStack<'store>,
}

impl<'b, 'store> InnerParser<'b, 'store> {
    /// Parse an arugment and pushes the argument to the stack surrounded by a
    /// group: [..., EndGroup, Argument, BeginGroup], when the argument is a subgroup.
    /// Otherwise, it pushes the argument to the stack ungrouped.
    fn handle_argument(&mut self, argument: Argument<'store>) -> InnerResult<()> {
        match argument {
            Argument::Token(token) => {
                self.state.handling_argument = true;
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
                        allowed_alignment_count: None,
                    },
                    Instruction::Event(Event::End),
                ]);
            }
        };
        Ok(())
    }

    /// ## Script parsing
    ///
    /// The script parser first checks for directives about script placement, i.e. `\limits` and `\nolimits`,
    /// if the `allow_script_modifiers` flag is set on the parser state. If the flag is set, and if more than one directive is found,
    /// the last one takes effect, as per the [`amsmath docs`][amsdocs] (section 7.3). If the flag is not set, and a limit modifying
    /// directive is found, the parser emits an error.
    ///
    /// [amsdocs]: https://mirror.its.dal.ca/ctan/macros/latex/required/amsmath/amsldoc.pdf
    fn parse(&mut self) -> InnerResult<Option<(Event<'store>, ScriptDescriptor)>> {
        // 1. Parse the next token and output everything to the staging stack.
        let original_content = self.content.trim_start();
        let token = lex::token(&mut self.content)?;
        match token {
            Token::ControlSequence(cs) => {
                if let Some(result) =
                    self.macro_context
                        .try_expand_in(cs, self.content, self.storage)
                {
                    // TODO: Some ptr arithmetic with original_content, new_content, to figure out
                    // things for macro span.

                    let (new_content, arguments_consumed_length) = result?;
                    let call_site_length = cs.len() + arguments_consumed_length + 1;
                    self.span_stack
                        .add(new_content, original_content, call_site_length);

                    self.content = new_content;
                    return self.parse();
                }

                self.handle_primitive(cs)?
            }
            Token::Character(c) => self.handle_char_token(c)?,
        };

        // 2. Check for scripts, to complete the atom.
        if self.state.skip_scripts {
            return Ok(None);
        }

        if self.state.allow_script_modifiers {
            if let Some(limits) = lex::limit_modifiers(&mut self.content) {
                if limits {
                    self.state.script_position = ScriptPosition::AboveBelow;
                } else {
                    self.state.script_position = ScriptPosition::Right;
                }
            }
        }

        self.content = self.content.trim_start();
        let subscript_first = match self.content.chars().next() {
            Some('^') => false,
            Some('_') => true,
            _ => return Ok(None),
        };
        self.content = &self.content[1..];

        let first_script_start = self.buffer.len();
        let arg = lex::argument(&mut self.content)?;
        self.handle_argument(arg)?;
        let second_script_start = self.buffer.len();
        let next_char = self.content.chars().next();
        if (next_char == Some('_') && !subscript_first)
            || (next_char == Some('^') && subscript_first)
        {
            self.content = &self.content[1..];
            let arg = lex::argument(&mut self.content)?;
            self.handle_argument(arg)?;
        } else if next_char == Some('_') || next_char == Some('^') {
            return Err(if subscript_first {
                ErrorKind::DoubleSubscript
            } else {
                ErrorKind::DoubleSuperscript
            });
        }
        let second_script_end = self.buffer.len();

        Ok(Some(if second_script_start == second_script_end {
            if subscript_first {
                (
                    Event::Script {
                        ty: ScriptType::Subscript,
                        position: self.state.script_position,
                    },
                    ScriptDescriptor {
                        subscript_start: first_script_start,
                        superscript_start: second_script_start,
                    },
                )
            } else {
                (
                    Event::Script {
                        ty: ScriptType::Superscript,
                        position: self.state.script_position,
                    },
                    ScriptDescriptor {
                        subscript_start: second_script_start,
                        superscript_start: first_script_start,
                    },
                )
            }
        } else {
            (
                Event::Script {
                    ty: ScriptType::SubSuperscript,
                    position: self.state.script_position,
                },
                if subscript_first {
                    ScriptDescriptor {
                        subscript_start: first_script_start,
                        superscript_start: second_script_start,
                    }
                } else {
                    ScriptDescriptor {
                        subscript_start: second_script_start,
                        superscript_start: first_script_start,
                    }
                },
            )
        }))
    }

    fn parse_next(
        mut self,
    ) -> (
        InnerResult<Option<(Event<'store>, ScriptDescriptor)>>,
        &'store str,
    ) {
        (self.parse(), self.content)
    }
}

struct ScriptDescriptor {
    subscript_start: usize,
    superscript_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Token<'a> {
    ControlSequence(&'a str),
    Character(CharToken<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CharToken<'a> {
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

#[derive(Debug, Clone)]
enum Instruction<'a> {
    /// Send the event
    Event(Event<'a>),
    /// Parse the substring
    SubGroup {
        content: &'a str,
        allowed_alignment_count: Option<AlignmentCount>,
    },
}

#[derive(Debug, Clone)]
struct AlignmentCount {
    count: u16,
    max: u16,
}

impl AlignmentCount {
    fn new(max: u16) -> Self {
        Self { count: 0, max }
    }

    fn reset(&mut self) {
        self.count = 0;
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn can_increment(&self) -> bool {
        self.count < self.max
    }
}

/// For error reporting purposes.
///
/// Stores the context in which the parser is currently if an error were to arise.
#[derive(Debug, Clone)]
struct SpanStack<'store> {
    /// The original input given to the parser.
    input: &'store str,
    /// Expansions of macros.
    expansions: Vec<ExpansionSpan<'store>>,
}

impl<'store> SpanStack<'store> {
    fn from_input(input: &'store str) -> Self {
        Self {
            input,
            expansions: Vec::new(),
        }
    }

    fn add(&mut self, full_expansion: &'store str, call_site: &str, call_site_length: usize) {
        let call_site_start = self.reach_original_call_site(call_site.as_ptr());
        let expansion_length = (call_site_length as isize
            - (call_site.len() as isize - full_expansion.len() as isize))
            as usize;

        self.expansions.push(ExpansionSpan {
            full_expansion,
            expansion_length,
            call_site_in_origin: call_site_start..call_site_start + call_site_length,
        });
    }

    /// Navigate down the stack until we reach the original span for the given substring. Returns
    /// the index of the beginning of the call-site in the top-most span in the stack.
    fn reach_original_call_site(&mut self, substr_start: *const u8) -> usize {
        let mut ptr_val = substr_start as isize;

        while let Some(expansion) = self.expansions.last() {
            let expansion_ptr = expansion.full_expansion.as_ptr() as isize;

            if ptr_val >= expansion_ptr
                && ptr_val <= expansion_ptr + expansion.full_expansion.len() as isize
            {
                let index = if ptr_val <= expansion_ptr + expansion.expansion_length as isize {
                    (ptr_val - expansion_ptr) as usize
                } else {
                    let distance_from_effective_stop =
                        ptr_val - expansion_ptr - expansion.expansion_length as isize;
                    self.expansions.pop();
                    ptr_val = self
                        .expansions
                        .last()
                        .map(|exp| exp.full_expansion)
                        .unwrap_or(self.input)
                        .as_ptr() as isize
                        + distance_from_effective_stop;
                    continue;
                };
                return index;
            }
            self.expansions.pop();
        }
        let input_start = self.input.as_ptr() as isize;

        assert!(ptr_val > input_start && ptr_val <= input_start + self.input.len() as isize);
        (ptr_val - input_start) as usize
    }
}

/// A span of the input string. Used for error reporting.
/// ```text
///         full_expansion: [ -- Expanded --- | -- Rest -- ]
///                        /                   \ < effective_expansion_stop
///        [ -- Before -- | ---- Call Site ---- | -- Rest -- ]
///                       ^---------------------^
///                        declaration_in_origin
/// ```
#[derive(Debug, Clone)]
struct ExpansionSpan<'a> {
    /// The fully expaned string which is allocated in storage.
    ///
    /// This includes the expanded part and the included remaining.
    full_expansion: &'a str,
    /// The index where the expanded part ends and where the rest is equivalent to the rest of the
    /// original string.
    expansion_length: usize,
    /// What the expansion replaces in the original string (where the macro invocation is in the
    /// original string).
    ///
    /// The original string is the string coming before itself in the expansion stack.
    call_site_in_origin: Range<usize>,
}

#[cfg(test)]
mod tests {
    use crate::event::{Content, Visual};

    use super::*;

    #[test]
    fn substr_instructions() {
        let store = Storage::new();
        let parser = Parser::new("\\bar{y}", &store);

        let events = parser.collect::<Result<Vec<_>, ParserError>>().unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::Superscript,
                    position: ScriptPosition::AboveBelow
                },
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Ordinary {
                    content: 'y',
                    stretchy: false
                }),
                Event::End,
                Event::Content(Content::Ordinary {
                    content: '‾',
                    stretchy: false,
                }),
            ]
        );
    }

    #[test]
    fn subsuperscript() {
        let store = Storage::new();
        let parser = Parser::new(r"a^{1+3}_2", &store);
        let events = parser
            .inspect(|e| println!("{:?}", e))
            .collect::<Result<Vec<_>, ParserError>>()
            .unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::SubSuperscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Ordinary {
                    content: 'a',
                    stretchy: false,
                }),
                Event::Content(Content::Number("2")),
                Event::Begin(Grouping::Normal),
                Event::Content(Content::Number("1")),
                Event::Content(Content::BinaryOp {
                    content: '+',
                    small: false
                }),
                Event::Content(Content::Number("3")),
                Event::End,
            ]
        );
    }
    #[test]
    fn subscript_torture() {
        let store = Storage::new();
        let parser = Parser::new(r"a_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_5}}}}}}}}}}}", &store);
        let events = parser.collect::<Result<Vec<_>, ParserError>>().unwrap();

        assert_eq!(
            events,
            vec![
                Event::Script {
                    ty: ScriptType::Subscript,
                    position: ScriptPosition::Right
                },
                Event::Content(Content::Ordinary {
                    content: 'a',
                    stretchy: false,
                }),
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
        let store = Storage::new();
        let parser = Parser::new(r"\frac{1}{2}_2^4", &store);
        let events = parser.collect::<Result<Vec<_>, ParserError>>().unwrap();

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

    #[test]
    fn multidigit_number() {
        let store = Storage::new();
        let parser = Parser::new("123", &store);
        let events = parser.collect::<Result<Vec<_>, ParserError>>().unwrap();

        assert_eq!(events, vec![Event::Content(Content::Number("123"))]);
    }

    #[test]
    fn error() {
        let store = Storage::new();
        let parser = Parser::new(r"\def\blah#1#2{\fra#1#2} \def\abc#1{\blah{a}#1} \abc{b}", &store);
        let events = parser.collect::<Vec<_>>();

        assert!(events[0].is_err());
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
