use std::collections::VecDeque;

mod primitives;

use thiserror::Error;

use crate::{
    ast::{Content, Event, Grouping, Identifier, Infix},
    attribute::{DimensionUnit, Font},
    Argument, Token,
};

pub type Dimension = (f32, DimensionUnit);
type Glue = (Dimension, Option<Dimension>, Option<Dimension>);

pub type Result<T> = std::result::Result<T, ParseError>;

// TODO: change invalid char in favor of more expressive errors.
//      - We do not need to know the character, since we know the byte offset.
//      - We need to know _why_ the character is invalid.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid character found in input: {0}")]
    InvalidChar(char),
    #[error("unexpected end of input")]
    EndOfInput,
}

// The algorithm:
// - We want to be able to stack events that should be outputed on following cycles.
// - We should be able to stack the parsing of a substring.
// - This parsing of the substring should itself be able to stack events that should occur before
// continuing parsing of the substring.

pub enum Instruction<'a> {
    /// Push the event
    Event(Event<'a>),
    /// Parse the substring
    Substring(&'a str),
}

pub struct Parser<'a> {
    /// The input to parse.
    input: &'a str,
    /// The initial byte pointer of the input.
    initial_byte_ptr: *const u8,
    /// Whether the parsing is done. We need this variable because when the last event parsed
    /// is outputed, we still need to output the `EndGroup` event.
    done: bool,
    /// Some commands output multiple events, so we need to keep track of them.
    pub(crate) instruction_stack: Vec<Instruction<'a>>,
    pub(crate) font_state: Vec<Option<Font>>,
    pub(crate) input_stack: Vec<&'a str>,
}

// TODO: make `trim_start` (removing whitespace) calls more systematic.
impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            done: false,
            initial_byte_ptr: input.as_ptr(),
            instruction_stack: Vec::from([Instruction::Event(Event::Begin(Grouping::Group))]),
            font_state: Vec::from([Some(Font::Italic)]),
            input_stack: Vec::new(),
        }
    }

    /// Parse the right-hand side of a definition (TeXBook p. 271).
    ///
    /// In this case, a definition is any of `def`, `edef`, `gdef`, or `xdef`.
    ///
    /// Returns the control sequence, the parameter text, and the replacement text.
    fn definition(&mut self) -> Result<(&'a str, &'a str, &'a str)> {
        let control_sequence = self.control_sequence()?;
        let (parameter_text, rest) = self.input.split_once('{').ok_or(ParseError::EndOfInput)?;
        self.input = rest;
        let replacement_text = self.group_content()?;

        Ok((control_sequence, parameter_text, replacement_text))
    }

    pub fn argument(&mut self) -> Result<Argument<'a>> {
        self.input = self.input.trim_start();

        if self.input.starts_with('{') {
            self.input = &self.input[1..];
            let content = self.group_content()?;
            Ok(Argument::Group(content))
        } else {
            Ok(Argument::Token(self.token()?))
        }
    }

    /// Parses the inside of a group, when the first `{` is already parsed.
    pub fn group_content(&mut self) -> Result<&'a str> {
        let mut escaped = false;
        // In this case `Err` is the desired result.
        let end_index = self
            .input
            .char_indices()
            .try_fold(0usize, |balance, (index, c)| match c {
                '{' if !escaped => Ok(balance + 1),
                '}' if !escaped => {
                    if balance == 0 {
                        Err(index)
                    } else {
                        Ok(balance - 1)
                    }
                }
                '\\' => {
                    // Makes it so that two backslashes in a row don't escape the next character.
                    escaped = !escaped;
                    Ok(balance)
                }
                _ => {
                    escaped = false;
                    Ok(balance)
                }
            });

        if let Err(end_index) = end_index {
            let (argument, rest) = self.input.split_at(end_index);
            self.input = &rest[1..];
            Ok(argument)
        } else {
            Err(ParseError::EndOfInput)
        }
    }

    /// Converts a control sequence or character into its corresponding delimiter unicode
    /// character.
    ///
    /// Current delimiters supported are listed in TeXBook p. 146.
    pub fn delimiter(&mut self) -> Result<char> {
        self.input = self.input.trim_start();
        let maybe_delim = self.token()?;
        match maybe_delim {
            Token::Character('(') => Ok('('),
            Token::Character(')') => Ok(')'),
            Token::Character('[') | Token::ControlSequence("lbrack") => Ok('['),
            Token::Character(']') | Token::ControlSequence("rbrack") => Ok(']'),
            Token::ControlSequence("{") | Token::ControlSequence("lbrace") => Ok('{'),
            Token::ControlSequence("}") | Token::ControlSequence("rbrace") => Ok('}'),
            Token::ControlSequence("lfloor") => Ok('⌊'),
            Token::ControlSequence("rfloor") => Ok('⌋'),
            Token::ControlSequence("lceil") => Ok('⌈'),
            Token::ControlSequence("rceil") => Ok('⌉'),
            Token::ControlSequence("langle") => Ok('⟨'),
            Token::ControlSequence("rangle") => Ok('⟩'),
            Token::Character('/') => Ok('/'),
            Token::ControlSequence("backslash") => Ok('\\'),
            Token::Character('|') | Token::ControlSequence("vert") => Ok('|'),
            Token::ControlSequence("|") | Token::ControlSequence("Vert") => Ok('‖'),
            Token::ControlSequence("uparrow") => Ok('↑'),
            Token::ControlSequence("Uparrow") => Ok('⇑'),
            Token::ControlSequence("downarrow") => Ok('↓'),
            Token::ControlSequence("Downarrow") => Ok('⇓'),
            Token::ControlSequence("updownarrow") => Ok('↕'),
            Token::ControlSequence("Updownarrow") => Ok('⇕'),
            Token::Character(c) => Err(ParseError::InvalidChar(c)),
            Token::ControlSequence(cs) => Err(cs
                .chars()
                .next()
                .map_or(ParseError::EndOfInput, |c| ParseError::InvalidChar(c))),
        }
    }

    /// Parse the right-hand side of a `futurelet` assignment (TeXBook p. 273).
    ///
    /// Returns the control sequence and both following tokens.
    fn futurelet_assignment(&mut self) -> Result<(&'a str, Token, Token)> {
        let control_sequence = self.control_sequence()?;

        let token1 = self.token()?;
        let token2 = self.token()?;
        Ok((control_sequence, token1, token2))
    }

    /// Parse the right-hand side of a `let` assignment (TeXBook p. 273).
    ///
    /// Returns the control sequence and the value it is assigned to.
    fn let_assignment(&mut self) -> Result<(&'a str, Token)> {
        let control_sequence = self.control_sequence()?;

        self.input = self.input.trim_start();
        if let Some(s) = self.input.strip_prefix('=') {
            self.input = s;
            self.one_optional_space();
        }

        let token = self.token()?;
        Ok((control_sequence, token))
    }

    /// Parse a control_sequence, including the leading `\`.
    fn control_sequence(&mut self) -> Result<&'a str> {
        if self.input.starts_with('\\') {
            self.input = &self.input[1..];
            Ok(self.rhs_control_sequence())
        } else {
            self.input
                .chars()
                .next()
                .map_or(Err(ParseError::EndOfInput), |c| {
                    Err(ParseError::InvalidChar(c))
                })
        }
    }

    /// Parse the right side of a control sequence (`\` already being parsed).
    ///
    /// A control sequence can be of the form `\controlsequence`, or `\#` (control symbol).
    pub fn rhs_control_sequence(&mut self) -> &'a str {
        if self.input.is_empty() {
            return self.input;
        };

        let len = self
            .input
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .count()
            .max(1);

        let (control_sequence, rest) = self.input.split_at(len);
        self.input = rest.trim_start();
        control_sequence
    }

    /// Parse a glue (TeXBook p. 267).
    pub fn glue(&mut self) -> Result<Glue> {
        let mut dimension = (self.dimension()?, None, None);
        if let Some(s) = self.input.trim_start().strip_prefix("plus") {
            self.input = s;
            dimension.1 = Some(self.dimension()?);
        }
        if let Some(s) = self.input.trim_start().strip_prefix("minus") {
            self.input = s;
            dimension.2 = Some(self.dimension()?);
        }
        Ok(dimension)
    }

    /// Parse a dimension (TeXBook p. 266).
    pub fn dimension(&mut self) -> Result<Dimension> {
        let number = self.floating_point()?;
        let unit = self.dimension_unit()?;
        Ok((number, unit))
    }

    /// Parse a dimension unit (TeXBook p. 266).
    fn dimension_unit(&mut self) -> Result<DimensionUnit> {
        self.input = self.input.trim_start();
        if self.input.len() < 2 {
            return Err(ParseError::EndOfInput);
        }

        let unit = self.input.get(0..2).ok_or_else(|| {
            let first_non_ascii = self
                .input
                .chars()
                .find(|c| !c.is_ascii())
                .expect("there is a known non-ascii character");
            ParseError::InvalidChar(first_non_ascii)
        })?;
        let unit = match unit {
            "em" => DimensionUnit::Em,
            "ex" => DimensionUnit::Ex,
            "pt" => DimensionUnit::Pt,
            "pc" => DimensionUnit::Pc,
            "in" => DimensionUnit::In,
            "bp" => DimensionUnit::Bp,
            "cm" => DimensionUnit::Cm,
            "mm" => DimensionUnit::Mm,
            "dd" => DimensionUnit::Dd,
            "cc" => DimensionUnit::Cc,
            "sp" => DimensionUnit::Sp,
            "mu" => DimensionUnit::Mu,
            _ => {
                if matches!(
                    unit.as_bytes()[0],
                    b'e' | b'p' | b'i' | b'b' | b'c' | b'm' | b'd' | b's'
                ) {
                    return Err(ParseError::InvalidChar(unit.chars().nth(1).unwrap()));
                } else {
                    return Err(ParseError::InvalidChar(unit.chars().next().unwrap()));
                }
            }
        };

        self.input = &self.input[2..];
        self.one_optional_space();

        Ok(unit)
    }

    /// Parse an integer that may be positive or negative (TeXBook p. 265).
    fn integer(&mut self) -> Result<isize> {
        // TODO: support for internal values
        let signum = self.signs()?;

        // The following character must be ascii.
        let next_char = self.input.chars().next().ok_or(ParseError::EndOfInput)?;
        if !next_char.is_ascii() {
            return Err(ParseError::InvalidChar(next_char));
        }

        if next_char.is_ascii_digit() {
            return self.decimal().map(|x| x as isize * signum);
        }
        self.input = &self.input[1..];
        let unsigned_int = match next_char as u8 {
            b'`' => {
                let mut next_byte = *self
                    .input
                    .as_bytes()
                    .first()
                    .ok_or(ParseError::EndOfInput)?;
                if next_byte == b'\\' {
                    self.input = &self.input[1..];
                    next_byte = *self
                        .input
                        .as_bytes()
                        .first()
                        .ok_or(ParseError::EndOfInput)?;
                }
                if next_byte.is_ascii() {
                    self.input = &self.input[1..];
                    Ok(next_byte as usize)
                } else {
                    Err(ParseError::InvalidChar(
                        self.input.chars().next().expect("the input is not empty"),
                    ))
                }
            }
            b'\'' => self.octal(),
            b'"' => self.hexadecimal(),
            x => return Err(ParseError::InvalidChar(x as char)),
        }?;

        Ok(unsigned_int as isize * signum)
    }

    /// Parse the signs in front of a number, returning the signum.
    fn signs(&mut self) -> Result<isize> {
        let signs = self.input.trim_start();
        let mut minus_count = 0;
        self.input = signs
            .trim_start_matches(|c: char| {
                if c == '-' {
                    minus_count += 1;
                    true
                } else {
                    c == '+' || c.is_whitespace()
                }
            })
            .trim_start();
        Ok(if minus_count % 2 == 0 { 1 } else { -1 })
    }

    /// Parse a base 16 unsigned number.
    fn hexadecimal(&mut self) -> Result<usize> {
        let mut number = 0;
        self.input = self.input.trim_start_matches(|c: char| {
            if c.is_ascii_alphanumeric() && c < 'G' {
                number = number * 16
                    + c.to_digit(16).expect("the character is a valid hex digit") as usize;
                true
            } else {
                false
            }
        });
        self.one_optional_space();

        Ok(number)
    }

    /// Parse a floating point number (named `factor` in TeXBook p. 266).
    fn floating_point(&mut self) -> Result<f32> {
        let signum = self.signs()?;

        let mut number = 0.;
        self.input = self.input.trim_start_matches(|c: char| {
            if c.is_ascii_digit() {
                number = number * 10. + (c as u8 - b'0') as f32;
                true
            } else {
                false
            }
        });

        if let Some(stripped_decimal_point) = self.input.strip_prefix(|c| c == '.' || c == ',') {
            let mut decimal = 0.;
            let mut decimal_divisor = 1.;
            self.input = stripped_decimal_point.trim_start_matches(|c: char| {
                if c.is_ascii_digit() {
                    decimal = decimal * 10. + (c as u8 - b'0') as f32;
                    decimal_divisor *= 10.;
                    true
                } else {
                    false
                }
            });
            number += decimal / decimal_divisor;
        };

        Ok(signum as f32 * number)
    }

    /// Parse a base 10 unsigned number.
    fn decimal(&mut self) -> Result<usize> {
        let mut number = 0;
        self.input = self.input.trim_start_matches(|c: char| {
            if c.is_ascii_digit() {
                number = number * 10 + (c as u8 - b'0') as usize;
                true
            } else {
                false
            }
        });
        self.one_optional_space();

        Ok(number)
    }

    /// Parse a base 8 unsigned number.
    fn octal(&mut self) -> Result<usize> {
        let mut number = 0;
        self.input = self.input.trim_start_matches(|c: char| {
            if c.is_ascii_digit() {
                number = number * 8 + (c as u8 - b'0') as usize;
                true
            } else {
                false
            }
        });
        self.one_optional_space();

        Ok(number)
    }

    fn one_optional_space(&mut self) -> bool {
        let mut chars = self.input.chars();
        if chars.next().is_some_and(|c| c.is_whitespace()) {
            self.input = &self.input[1..];
            true
        } else {
            false
        }
    }

    fn token(&mut self) -> Result<Token<'a>> {
        match self.control_sequence() {
            Ok(cs) => Ok(Token::ControlSequence(cs)),
            Err(e) => match e {
                ParseError::InvalidChar(c) => Ok(Token::Character(c)),
                ParseError::EndOfInput => Err(ParseError::EndOfInput),
            },
        }
    }

    /// Parse the next event in the input.
    ///
    /// This is different from `handle_token` in the sense that it parses numbers completely.
    ///
    /// ### Panic
    /// This function must be called with a non-empty input.
    fn next_event(&mut self) -> Result<Event<'a>> {
        let next_char = self.input.chars().next().expect("the input is not empty");
        
        match next_char {
            '.' | '0'..='9' => {
                let len = self.input
                    .chars()
                    .skip(1)
                    .take_while(|&c| c.is_ascii_digit() || c == '.')
                    .count()
                    + 1;
                let (number, rest) = self.input.split_at(len);
                self.input = rest;
                Ok(Event::Content(Content::Number(number)))
            }
            '\\' => {
                self.input = &self.input[1..];
                let cs = self.rhs_control_sequence();
                self.handle_primitive(cs)
            },
            c => {
                // TODO: Advance parser by one character (one codepoint) instead of one byte.
                self.input = &self.input[1..];
                self.handle_char_token(c)
            }
        }
    }

    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    fn handle_char_token(&mut self, token: char) -> Result<Event<'a>> {
        Ok(match token {
            '{' => Event::Begin(Grouping::Group),
            '}' => Event::EndGroup,
            '_' => Event::Infix(Infix::Subscript),
            '^' => Event::Infix(Infix::Superscript),
            // TODO: handle every character correctly.
            c => Event::Content(Content::Identifier(Identifier::Char {
                content: c,
                is_normal: false,
            })),
        })
    }

    /// Return the byte index of the current position in the input.
    fn get_byte_index(&self) -> usize {
        // Safety:
        // * Both `self` and `origin` must be either in bounds or one
        //   byte past the end of the same [allocated object].
        //   => this is true, as self never changes the allocation of the `input`.
        //
        // * Both pointers must be *derived from* a pointer to the same object.
        //   (See below for an example.)
        //   => this is true, as `initial_byte_ptr` is derived from `input.as_ptr()`.
        // * The distance between the pointers, in bytes, must be an exact multiple
        //   of the size of `T`.
        //   => this is true, as both pointers are `u8` pointers.
        // * The distance between the pointers, **in bytes**, cannot overflow an `isize`.
        //   => this is true, as the distance is always positive.
        // * The distance being in bounds cannot rely on "wrapping around" the address space.
        //   => this is true, as the distance is always positive.
        unsafe { self.input.as_ptr().offset_from(self.initial_byte_ptr) as usize }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Event<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        
        let event = match self.instruction_stack.pop() {
            Some(Instruction::Event(event)) =>  Ok(event),
            substr_or_none => {
                if let Some(Instruction::Substring(substr)) = substr_or_none {
                    self.input_stack.push(self.input);
                    self.input = substr;
                }
                
                self.input = self.input.trim_start();
                while self.input.is_empty() {
                    if let Some(substr) = self.input_stack.pop() {
                        self.input = substr;
                    } else {
                        self.done = true;
                        return Some(Ok(Event::EndGroup));
                    }
                }
                
                self.next_event()
            }
        };

        // Apply the following rules based on the current state:
        // - If the token is a character identifier, apply the font state.
        // let event = match event {
        //     Event::Content(Content::Identifier(Identifier::Char {
        //         content,
        //         is_normal: false,
        //     })) => {
        //         if let Some(current_font) = self.font_state.last().expect("there is a font state") {
        //             Event::Content(Content::Identifier(Identifier::Char {
        //                 content: current_font.map_char(content),
        //                 is_normal: false,
        //             }))
        //         } else {
        //             Event::Content(Content::Identifier(Identifier::Char {
        //                 content,
        //                 is_normal: true,
        //             }))
        //         }
        //     }
        //     _ => event,
        // };
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_byte_index() {
        let input = "+ --+-   \\test";
        let mut parser = Parser::new(input);
        assert_eq!(parser.get_byte_index(), 0);

        parser.signs().unwrap();
        assert_eq!(parser.get_byte_index(), 9);
    }

    #[test]
    fn signs() {
        let input = "  +    +-   \\test";
        let mut parser = Parser::new(input);
        assert_eq!(parser.signs().unwrap(), -1);
    }

    #[test]
    fn no_signs() {
        let input = "\\mycommand";
        let mut parser = Parser::new(input);
        assert_eq!(parser.signs().unwrap(), 1);
    }

    // A complex exanple from problem 20.7 in TeXBook (p. 205):
    // \def\cs AB#1#2C$#3\$ {#3{ab#1}#1 c##\x #2}
    #[test]
    fn definition_texbook() {
        let mut parser = Parser::new("\\cs AB#1#2C$#3\\$ {#3{ab#1}#1 c##\\x #2}");

        let (cs, param, repl) = parser.definition().unwrap();
        assert_eq!(cs, "cs");
        assert_eq!(param, "AB#1#2C$#3\\$ ");
        assert_eq!(repl, "#3{ab#1}#1 c##\\x #2");
    }

    #[test]
    fn complex_definition() {
        let mut parser = Parser::new(r"\foo #1\test#2#{##\####2#2 \{{\}} \{\{\{}");
        let (cs, param, repl) = parser.definition().unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(param, r"#1\test#2#");
        assert_eq!(repl, r"##\####2#2 \{{\}} \{\{\{");
    }

    #[test]
    fn let_assignment() {
        let mut parser = Parser::new(r"\foo = \bar");
        let (cs, token) = parser.let_assignment().unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(token, Token::ControlSequence("bar".into()));
    }

    #[test]
    fn futurelet_assignment() {
        let mut parser = Parser::new(r"\foo\bar\baz");
        let (cs, token1, token2) = parser.futurelet_assignment().unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(token1, Token::ControlSequence("bar".into()));
        assert_eq!(token2, Token::ControlSequence("baz".into()));
    }

    #[test]
    fn dimension() {
        let mut parser = Parser::new("1.2pt ");
        let dim = parser.dimension().unwrap();

        assert_eq!(dim, (1.2, DimensionUnit::Pt));
    }

    #[test]
    fn complex_glue() {
        let mut parser = Parser::new(r"1.2 pt plus 3.4pt minus 5.6pt");
        let glue = parser.glue().unwrap();

        assert_eq!(
            glue,
            (
                (1.2, DimensionUnit::Pt),
                Some((3.4, DimensionUnit::Pt)),
                Some((5.6, DimensionUnit::Pt))
            )
        );
    }

    #[test]
    fn numbers() {
        let mut parser = Parser::new("123 -\"AEF24 --'3475 `\\a -.47");
        assert_eq!(parser.integer().unwrap(), 123);
        assert_eq!(parser.integer().unwrap(), -716580);
        assert_eq!(parser.integer().unwrap(), 1853);
        assert_eq!(parser.integer().unwrap(), 97);
        assert_eq!(parser.floating_point().unwrap(), -0.47);
    }

    // Tests for event generation.
    #[test]
    fn substr_instructions() {
        let parser = Parser::new("\\bar{y}");
        let events = parser.collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(
            events,
            vec![
                Event::Begin(Grouping::Group),
                Event::Begin(Grouping::Group),
                Event::Content(Content::Identifier(Identifier::Char {
                    content: 'y',
                    is_normal: false,
                })),
                Event::EndGroup,
                Event::Content(
                    Content::Operator { content: '‾', stretchy: None, moveable_limits: None, left_space: None, right_space: None }
                    ),
                Event::EndGroup
            ]
        );
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
//
