use crate::event::{DelimiterType, Dimension, DimensionUnit, Glue, GroupingKind, Line};

use super::{
    tables::{primitive_color, token_to_delim},
    Argument, CharToken, ErrorKind, InnerResult, Token,
};

/// Parse the right-hand side of a definition (TeXBook p. 271).
///
/// In this case, a definition is any of `def`, `edef`, `gdef`, or `xdef`.
///
/// Returns the control sequence, the parameter text, and the replacement text.
pub fn definition<'a>(input: &mut &'a str) -> InnerResult<(&'a str, &'a str, &'a str)> {
    let control_sequence = control_sequence(input)?;
    let (parameter_text, rest) = input.split_once('{').ok_or(ErrorKind::MissingExpansion)?;

    if let Some(idx) = parameter_text.find(|c: char| c == '%' || c == '}') {
        return Err(if parameter_text.as_bytes()[idx] == b'%' {
            ErrorKind::CommentInParamText
        } else {
            ErrorKind::BracesInParamText
        });
    }

    *input = rest;
    let replacement_text = group_content(input, GroupingKind::Normal)?;

    Ok((control_sequence, parameter_text, replacement_text))
}

/// Parse an argument to a control sequence, and return it.
pub fn argument<'a>(input: &mut &'a str) -> InnerResult<Argument<'a>> {
    if let Some(rest) = input.trim_start().strip_prefix('{') {
        *input = rest;
        let content = group_content(input, GroupingKind::Normal)?;
        Ok(Argument::Group(content))
    } else {
        Ok(Argument::Token(token(input)?))
    }
}

pub fn optional_argument<'a>(input: &mut &'a str) -> Option<&'a str> {
    if let Some(rest) = input.trim_start().strip_prefix('[') {
        *input = rest;
        let content = group_content(input, GroupingKind::OptionalArgument).ok()?;
        Some(content)
    } else {
        None
    }
}

pub fn brace_argument<'a>(input: &mut &'a str) -> InnerResult<&'a str> {
    if let Some(rest) = input.trim_start().strip_prefix('{') {
        *input = rest;
        group_content(input, GroupingKind::Normal)
    } else {
        Err(ErrorKind::GroupArgument)
    }
}

/// Parses the inside of a group, when the first opening tag is already parsed.
///
/// The output is the content within the group without the surrounding `start` and `end`.
/// This content is guaranteed to be balanced.
pub fn group_content<'a>(input: &mut &'a str, grouping_kind: GroupingKind) -> InnerResult<&'a str> {
    let start = grouping_kind.opening_str();
    let end = grouping_kind.closing_str();
    let mut escaped = false;
    let mut index = 0;
    let mut depth = 0u32;
    let bytes = input.as_bytes();
    while escaped || depth > 0 || !bytes[index..].starts_with(end.as_bytes()) {
        if index + end.len() > input.len() {
            *input = &input[input.len()..];
            return Err(ErrorKind::UnbalancedGroup(Some(grouping_kind)));
        }
        if !escaped && bytes[index..].starts_with(start.as_bytes()) {
            depth += 1;
            index += start.len();
            continue;
        }
        if !escaped && bytes[index..].starts_with(end.as_bytes()) {
            if depth.checked_sub(1).is_none() {
                break;
            }
            depth -= 1;
            index += end.len();
            continue;
        }
        match bytes[index] {
            b'\\' => escaped = !escaped,
            b'%' if !escaped => {
                let Some(rest_pos) = bytes[index..].iter().position(|&c| c == b'\n') else {
                    return Err(ErrorKind::UnbalancedGroup(Some(grouping_kind)));
                };
                index += rest_pos;
            }
            _ => escaped = false,
        }
        index += 1;
    }
    let (argument, rest) = input.split_at(index);
    *input = &rest[end.len()..];
    Ok(argument)
}

/// Finds the shortest prefix of `input` that contains balanced groups and ends with the given `suffix`.
pub fn content_with_suffix<'a>(input: &mut &'a str, suffix: &str) -> InnerResult<&'a str> {
    let mut escaped = false;
    let mut index = 0;
    let bytes = input.as_bytes();
    while escaped || !bytes[index..].starts_with(suffix.as_bytes()) {
        if index + suffix.len() > input.len() {
            *input = &input[input.len()..];
            return Err(ErrorKind::MacroSuffixNotFound);
        }
        match bytes[index] {
            b'\\' => escaped = !escaped,
            b'%' if !escaped => {
                let rest_pos = bytes[index..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .unwrap_or(bytes.len());
                index += rest_pos;
            }
            b'{' if !escaped => {
                let content = group_content(&mut &input[index + 1..], GroupingKind::Normal)?;
                index += content.len() + 1;
            }
            _ => escaped = false,
        }
        index += 1;
    }
    let (argument, rest) = input.split_at(index);
    *input = &rest[suffix.len()..];
    Ok(argument)
}

/// Converts a control sequence or character into its corresponding delimiter unicode
/// character, and whether or not the delimiter is an opening.
///
/// Current delimiters supported are listed in TeXBook p. 146, and on https://temml.org/docs/en/supported ("delimiter" section).
pub fn delimiter(input: &mut &str) -> InnerResult<(char, DelimiterType)> {
    let maybe_delim = token(input)?;
    token_to_delim(maybe_delim).ok_or(ErrorKind::Delimiter)
}

/// Parse the right-hand side of a `futurelet` assignment (TeXBook p. 273).
///
/// Returns the control sequence, the token it should be assigned to, and the rest of the input
/// with both tokens not consumed.
pub fn futurelet_assignment<'a>(input: &mut &'a str) -> InnerResult<(&'a str, Token<'a>, &'a str)> {
    let control_sequence = control_sequence(input)?;

    let input_with_tokens = *input;

    let _ = token(input)?;
    let token = token(input)?;
    Ok((control_sequence, token, input_with_tokens))
}

/// Parse the right-hand side of a `let` assignment (TeXBook p. 273).
///
/// Returns the control sequence and the value it is assigned to.
pub fn let_assignment<'a>(input: &mut &'a str) -> InnerResult<(&'a str, Token<'a>)> {
    let control_sequence = control_sequence(input)?;
    if let Some(s) = input.trim_start().strip_prefix('=') {
        *input = s;
    }
    let token = token(input)?;
    Ok((control_sequence, token))
}

/// Parse a control_sequence, including the leading `\`.
pub fn control_sequence<'a>(input: &mut &'a str) -> InnerResult<&'a str> {
    if let Some(rest) = input.strip_prefix('\\') {
        *input = rest;
        rhs_control_sequence(input)
    } else {
        input
            .chars()
            .next()
            .map_or(Err(ErrorKind::EmptyControlSequence), |_| {
                Err(ErrorKind::ControlSequence)
            })
    }
}

pub fn limit_modifiers(input: &mut &str) -> Option<bool> {
    let mut output = None;
    while let Some((rest, limits)) = input
        .trim_start()
        .strip_prefix(r"\limits")
        .map(|rest| (rest, true))
        .or_else(|| {
            input
                .trim_start()
                .strip_prefix(r"\nolimits")
                .map(|rest| (rest, false))
        })
    {
        *input = rest;
        output = Some(limits);
    }
    output
}

/// Parse the right side of a control sequence (`\` already being parsed).
///
/// A control sequence can be of the form `\controlsequence`, or `\#` (control symbol).
pub fn rhs_control_sequence<'a>(input: &mut &'a str) -> InnerResult<&'a str> {
    let first_char_byte_count = input
        .chars()
        .next()
        .ok_or(ErrorKind::EmptyControlSequence)?
        .len_utf8();

    let len = input
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .count()
        .max(first_char_byte_count);

    let (control_sequence, rest) = input.split_at(len);
    *input = rest.trim_start();
    Ok(control_sequence)
}

/// Parse a glue (TeXBook p. 267).
pub fn glue(input: &mut &str) -> InnerResult<Glue> {
    let mut dimen = (dimension(input)?, None, None);
    if let Some(s) = input.trim_start().strip_prefix("plus") {
        *input = s;
        dimen.1 = Some(dimension(input)?);
    }
    if let Some(s) = input.trim_start().strip_prefix("minus") {
        *input = s;
        dimen.2 = Some(dimension(input)?);
    }
    Ok(dimen)
}

/// Parse a dimension (TeXBook p. 266).
pub fn dimension(input: &mut &str) -> InnerResult<Dimension> {
    let number = floating_point(input)?;
    let unit = dimension_unit(input)?;
    Ok(Dimension::new(number, unit))
}

/// Parse a dimension unit (TeXBook p. 266).
pub fn dimension_unit(input: &mut &str) -> InnerResult<DimensionUnit> {
    *input = input.trim_start();
    let unit = input.get(0..2).ok_or(ErrorKind::DimensionUnit)?;
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
        _ => return Err(ErrorKind::DimensionUnit),
    };

    *input = &input[2..];
    one_optional_space(input);

    Ok(unit)
}

/// Parse an integer that may be positive or negative and may be represented as octal, decimal,
/// hexadecimal, or a character code (TeXBook p. 265).
// For future use maybe.
#[allow(dead_code)]
pub fn integer(input: &mut &str) -> InnerResult<isize> {
    let signum = signs(input)?;

    let unsigned_int = unsigned_integer(input)?;

    Ok(unsigned_int as isize * signum)
}

pub fn unsigned_integer(input: &mut &str) -> InnerResult<usize> {
    // The following character must be ascii.
    let next_char = input.chars().next().ok_or(ErrorKind::Number)?;
    if next_char.is_ascii_digit() {
        return Ok(decimal(input));
    }
    *input = &input[1..];
    match next_char {
        '`' => {
            let mut next_byte = *input.as_bytes().first().ok_or(ErrorKind::Number)?;
            if next_byte == b'\\' {
                *input = &input[1..];
                next_byte = *input.as_bytes().first().ok_or(ErrorKind::Number)?;
            }
            if next_byte.is_ascii() {
                *input = &input[1..];
                Ok(next_byte as usize)
            } else {
                Err(ErrorKind::CharacterNumber)
            }
        }
        '\'' => Ok(octal(input)),
        '"' => Ok(hexadecimal(input)),
        _ => Err(ErrorKind::Number),
    }
}

/// Parse the signs in front of a number, returning the signum.
pub fn signs(input: &mut &str) -> InnerResult<isize> {
    let mut minus_count = 0;
    *input = input
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
pub fn hexadecimal(input: &mut &str) -> usize {
    let mut number = 0;
    *input = input.trim_start_matches(|c: char| {
        if c.is_ascii_alphanumeric() && c < 'G' {
            number =
                number * 16 + c.to_digit(16).expect("the character is a valid hex digit") as usize;
            true
        } else {
            false
        }
    });
    one_optional_space(input);

    number
}

/// Parse a floating point number (named `factor` in TeXBook p. 266).
pub fn floating_point(input: &mut &str) -> InnerResult<f32> {
    let signum = signs(input)?;

    let mut number = 0.;
    *input = input.trim_start_matches(|c: char| {
        if c.is_ascii_digit() {
            number = number * 10. + (c as u8 - b'0') as f32;
            true
        } else {
            false
        }
    });

    if let Some(stripped_decimal_point) = input.strip_prefix(|c| c == '.' || c == ',') {
        let mut decimal = 0.;
        let mut decimal_divisor = 1.;
        *input = stripped_decimal_point.trim_start_matches(|c: char| {
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
pub fn decimal(input: &mut &str) -> usize {
    let mut number = 0;
    *input = input.trim_start_matches(|c: char| {
        if c.is_ascii_digit() {
            number = number * 10 + (c as u8 - b'0') as usize;
            true
        } else {
            false
        }
    });
    one_optional_space(input);

    number
}

/// Parse a base 8 unsigned number.
pub fn octal(input: &mut &str) -> usize {
    let mut number = 0;
    *input = input.trim_start_matches(|c: char| {
        if c.is_ascii_digit() {
            number = number * 8 + (c as u8 - b'0') as usize;
            true
        } else {
            false
        }
    });
    one_optional_space(input);

    number
}

/// Parse an optional space.
pub fn one_optional_space(input: &mut &str) -> bool {
    let mut chars = input.chars();
    if chars.next().is_some_and(|c| c.is_whitespace()) {
        *input = &input[1..];
        true
    } else {
        false
    }
}

/// Return the next token in the input.
///
/// A token will never be whitespace, and will never be inside of a comment.
pub fn token<'a>(input: &mut &'a str) -> InnerResult<Token<'a>> {
    *input = input.trim_start();
    match input.chars().next() {
        Some('\\') => {
            *input = &input[1..];
            Ok(Token::ControlSequence(rhs_control_sequence(input)?))
        }
        Some('%') => {
            let (_, rest) = input
                .split_once('\n')
                .unwrap_or(("", &input[input.len()..]));
            *input = rest;
            token(input)
        }
        Some(c) => {
            let context = *input;
            *input = input.split_at(c.len_utf8()).1;
            Ok(Token::Character(CharToken::from_str(context)))
        }
        None => Err(ErrorKind::Token),
    }
}

pub fn color(color: &str) -> Option<(u8, u8, u8)> {
    match color.strip_prefix('#') {
        Some(color) if color.len() == 6 => {
            let r = u8::from_str_radix(&color[..2], 16).ok()?;
            let g = u8::from_str_radix(&color[2..4], 16).ok()?;
            let b = u8::from_str_radix(&color[4..], 16).ok()?;
            Some((r, g, b))
        }
        None => primitive_color(color),
        _ => None,
    }
}

pub fn horizontal_lines(content: &mut &str) -> Box<[Line]> {
    let mut horizontal_lines = Vec::new();
    while let Some((rest, line)) = content
        .trim_start()
        .strip_prefix("\\hline")
        .map(|rest| (rest, Line::Solid))
        .or_else(|| {
            content
                .trim_start()
                .strip_prefix("\\hdashline")
                .map(|rest| (rest, Line::Dashed))
        })
    {
        horizontal_lines.push(line);
        *content = rest;
    }

    horizontal_lines.into()
}

#[cfg(test)]
mod tests {
    use crate::{
        event::{Dimension, DimensionUnit, GroupingKind},
        parser::{lex, Token},
    };

    #[test]
    fn signs() {
        let mut input = "  +    +-   \\test";
        assert_eq!(lex::signs(&mut input).unwrap(), -1);
        assert_eq!(input, "\\test");
    }

    #[test]
    fn no_signs() {
        let mut input = "\\mycommand";
        assert_eq!(lex::signs(&mut input).unwrap(), 1);
        assert_eq!(input, "\\mycommand");
    }

    // A complex exanple from problem 20.7 in TeXBook (p. 205):
    // \def\cs AB#1#2C$#3\$ {#3{ab#1}#1 c##\x #2}
    #[test]
    fn definition_texbook() {
        let mut input = "\\cs AB#1#2C$#3\\$ {#3{ab#1}#1 c##\\x #2}";

        let (cs, param, repl) = lex::definition(&mut input).unwrap();
        assert_eq!(cs, "cs");
        assert_eq!(param, "AB#1#2C$#3\\$ ");
        assert_eq!(repl, "#3{ab#1}#1 c##\\x #2");
        assert_eq!(input, "");
    }

    #[test]
    fn complex_definition() {
        let mut input = r"\foo #1\test#2#{##\####2#2 \{{\}} \{\{\{} 5 + 5 = 10";
        let (cs, param, repl) = lex::definition(&mut input).unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(param, r"#1\test#2#");
        assert_eq!(repl, r"##\####2#2 \{{\}} \{\{\{");
        assert_eq!(input, " 5 + 5 = 10");
    }

    #[test]
    fn let_assignment() {
        let mut input = r"\foo = \bar";
        let (cs, token) = lex::let_assignment(&mut input).unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(token, Token::ControlSequence("bar"));
        assert_eq!(input, "");
    }

    #[test]
    fn futurelet_assignment() {
        let mut input = r"\foo\bar\baz blah";
        let (cs, token, rest) = lex::futurelet_assignment(&mut input).unwrap();

        assert_eq!(cs, "foo");
        assert_eq!(token, Token::ControlSequence("baz"));
        assert_eq!(rest, r"\bar\baz blah");
    }

    #[test]
    fn dimension() {
        let mut input = "1.2pt";
        let dim = lex::dimension(&mut input).unwrap();

        assert_eq!(dim, Dimension::new(1.2, DimensionUnit::Pt));
        assert_eq!(input, "");
    }

    #[test]
    fn complex_glue() {
        let mut input = "1.2 pt plus 3.4pt minus 5.6pt nope";
        let glue = lex::glue(&mut input).unwrap();

        assert_eq!(
            glue,
            (
                Dimension::new(1.2, DimensionUnit::Pt),
                Some(Dimension::new(3.4, DimensionUnit::Pt)),
                Some(Dimension::new(5.6, DimensionUnit::Pt))
            )
        );
        assert_eq!(input, "nope");
    }

    #[test]
    fn numbers() {
        let mut input = "123 -\"AEF24 --'3475 `\\a -.47";
        assert_eq!(lex::integer(&mut input).unwrap(), 123);
        assert_eq!(lex::integer(&mut input).unwrap(), -716580);
        assert_eq!(lex::integer(&mut input).unwrap(), 1853);
        assert_eq!(lex::integer(&mut input).unwrap(), 97);
        assert_eq!(lex::floating_point(&mut input).unwrap(), -0.47);
        assert_eq!(input, "");
    }

    #[test]
    fn group_content() {
        let mut input =
            "this { { is a test } to see if { the content parsing { of this } } } works }";
        let content = lex::group_content(&mut input, GroupingKind::Normal).unwrap();
        assert_eq!(
            content,
            "this { { is a test } to see if { the content parsing { of this } } } works "
        );
    }
}
