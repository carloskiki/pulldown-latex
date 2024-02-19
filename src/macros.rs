use std::collections::HashMap;

use thiserror::Error;

use crate::Token;

const MAX_EXPANSION: usize = 32;

struct MacroContext<'a> {
    definitions: HashMap<&'a str, MacroDef<'a>>,
    assignments: HashMap<&'a str, Token<'a>>,
}

// QUESTIONS:
// If parameter text is unicode but user inputs tokens that represent the unicode?
// > In that case we say that the text has to match exactly.
// What does "fully expandable" mean?
// > https://tex.stackexchange.com/a/66168
// Difference between \def and \let?
// > \def creates a new table entry for the macro, while let points to an existing entry
//  (for us however, it needs to copy the entry).
// What does \relax do?
// > https://tex.stackexchange.com/questions/86385/what-is-the-difference-between-relax-and
//
// TODO: Make sure that we do not own an assignment and a definition at the same time.
impl<'a> MacroContext<'a> {
    fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            assignments: HashMap::new(),
        }
    }

    // Problem 20.7 shows a convoluted example
    //
    // To check:
    // - Strip braces of arguments

    /// Define a new macro, from its name, parameter text, and replacement text.
    ///
    /// - The replacement text must be properly balanced.
    /// - The parameter text must not contain '{' or '}'.
    fn define(
        &mut self,
        name: &'a str,
        mut parameter_text: &'a str,
        replacement_text: &'a str,
        expand_replacement: bool,
    ) -> Result<()> {
        // Check for the '#{' rule of the last parameter (TeXBook p. 204).
        let last_param_brace_delimited = parameter_text.ends_with('#');
        if last_param_brace_delimited {
            // We know the parameter text is at least 1 character long, and the character in
            // question is ASCII so we are fine slicing.
            parameter_text = &parameter_text[..parameter_text.len() - 1];
        };

        let mut parameters = parameter_text.split('#').enumerate();

        let prefix = parameters
            .next()
            .expect("split always yields at least one element")
            .1;
        let prefix = if prefix.is_empty() {
            None
        } else {
            // The parameter text is already guaranteed to not contain '{'.
            if prefix.find(|c| c == '}').is_some() {
                return Err(MacroContextError::BracesInParamText);
            };
            Some(prefix)
        };

        // Parse the arguments, making sure that they are in order and that the number of arguments
        // is less than 10.
        let parameters: Vec<_> = parameters
            .map(|(i, arg)| -> Result<Parameter> {
                let mut chars = arg.chars();
                let param_index = chars
                    .next()
                    .and_then(|c| c.is_ascii_digit().then_some(c as u8 - b'0'))
                    .ok_or(MacroContextError::StandaloneHashSign)?;
                if param_index != i as u8 {
                    return Err(MacroContextError::IncorrectMacroParams(
                        param_index,
                        i as u8,
                    ));
                };
                let suffix = chars.as_str();
                if suffix.is_empty() {
                    Ok(None)
                } else {
                    // The parameter text is already guaranteed to not contain '{'.
                    if suffix.find(|c| c == '}').is_some() {
                        return Err(MacroContextError::BracesInParamText);
                    };
                    Ok(Some(suffix))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        // Parse the replacement text, making sure that it is properly balanced.
        // TODO: expand replacement text when necessary
        let mut replacement_splits = replacement_text
            .split_inclusive(|c| matches!(c, '#' | '\\'))
            .peekable();
        let mut replacement_tokens: Vec<ReplacementToken> = Vec::new();

        while let Some(split) = replacement_splits.next() {
            let split_char = split
                .chars()
                .last()
                .expect("split inclusive always yields at least one char per element");
            replacement_tokens.push(ReplacementToken::String(&split));

            match split_char {
                '#' => {
                    let next_split = replacement_splits
                        .peek_mut()
                        .ok_or(MacroContextError::StandaloneHashSign)?;
                    let first_char = next_split
                        .chars()
                        .next()
                        .expect("split inclusive always yields at least one char per element");
                    if first_char == '#' {
                        // skip the next split since it will contain the second '#'
                        replacement_splits.next();
                    } else if first_char.is_ascii_digit() {
                        let param_index = first_char as u8 - b'0';
                        if param_index > parameters.len() as u8 || param_index == 0 {
                            return Err(MacroContextError::IncorrectReplacementParams(
                                param_index,
                                parameters.len() as u8,
                            ));
                        };

                        replacement_tokens.last_mut().map(|t| match t {
                            ReplacementToken::String(s) => {
                                *s = &s[..s.len() - 1];
                            }
                            _ => unreachable!(),
                        });
                        if replacement_tokens
                            .last()
                            .is_some_and(|t| matches!(t, ReplacementToken::String("")))
                        {
                            replacement_tokens.pop();
                        }
                        replacement_tokens.push(ReplacementToken::Parameter(param_index));
                        // Make it so that the next split wont begin with a digit.
                        *next_split = &next_split[1..];
                        // We know we are done if the next split becomes empty when removing a
                        // digit.
                        if next_split.is_empty() {
                            break;
                        }
                    } else {
                        return Err(MacroContextError::StandaloneHashSign);
                    }
                }
                '\\' => {
                    // TODO: this should be changed when allowing for expansion.
                    let next_split = replacement_splits
                        .peek()
                        .expect("the last character of the replacement text cannot be a backslash");
                    // The next split can only be 1 byte long if it only contains a splitting character.
                    if next_split.len() == 1 {
                        replacement_tokens.push(ReplacementToken::String(next_split));
                        replacement_splits.next();
                    }

                    if expand_replacement {
                        todo!("potentially expand");
                    }
                }
                _ => {}
            }
        }

        self.definitions.insert(
            name,
            MacroDef {
                prefix,
                last_param_brace_delimited,
                parameters,
                replacement: replacement_tokens,
            },
        );
        Ok(())
    }

    /// Assign a new control sequence to a token.
    fn assign(&mut self, name: &'a str, alias_for: Token<'a>) -> Result<()> {
        self.assignments.insert(name, alias_for);
        Ok(())
    }
}

#[derive(Clone)]
struct MacroDef<'a> {
    prefix: Option<&'a str>,
    parameters: Vec<Parameter<'a>>,
    last_param_brace_delimited: bool,
    replacement: Vec<ReplacementToken<'a>>,
}

/// Some if the argument has a suffix, None otherwise.
type Parameter<'a> = Option<&'a str>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacementToken<'a> {
    Parameter(u8),
    String(&'a str),
}

#[derive(Debug, Error)]
enum MacroContextError {
    #[error("macro definition of parameters contains '{{' or '}}'")]
    BracesInParamText,
    #[error("macro definition found parameter #{0} but expected #{1}")]
    IncorrectMacroParams(u8, u8),
    #[error("macro definition found parameter #{0} but expected a parameter in the range [#1, #{1}]")]
    IncorrectReplacementParams(u8, u8),
    #[error("macro definition contains a standalone '#'")]
    StandaloneHashSign,
}

type Result<T> = std::result::Result<T, MacroContextError>;

#[cfg(test)]
mod tests {
    use crate::macros::ReplacementToken;

    use super::MacroContext;

    #[test]
    fn no_params() {
        let mut ctx = MacroContext::new();
        ctx.define("foo", "", "\\this {} is a ## test", false)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = ctx.definitions.get("foo").unwrap();
        assert_eq!(def.prefix, None);
        assert!(def.parameters.is_empty());
        assert_eq!(
            &def.replacement
                .iter()
                .filter_map(|t| match t {
                    ReplacementToken::String(s) => Some(*s),
                    _ => None,
                })
                .collect::<String>(),
            "\\this {} is a # test"
        );
    }

    #[test]
    fn with_params() {
        let mut ctx = MacroContext::new();
        ctx.define(
            "foo",
            "this#1test#2. should #",
            "\\this {} is a ## test#1",
            false,
        )
        .map_err(|e| eprintln!("{e}"))
        .unwrap();

        let def = ctx.definitions.get("foo").unwrap();
        assert_eq!(def.prefix, Some("this"));
        assert_eq!(def.parameters, vec![Some("test".into()), Some(". should ")]);
        assert!(def.last_param_brace_delimited);
        assert_eq!(
            def.replacement,
            vec![
                ReplacementToken::String("\\"),
                ReplacementToken::String("this {} is a #"),
                ReplacementToken::String(" test"),
                ReplacementToken::Parameter(1)
            ]
        );
    }

    // A complex exanple from p.20.7 in TeXBook:
    // \def\cs AB#1#2C$#3\$ {#3{ab#1}#1 c##\x #2}
    #[test]
    fn texbook() {
        let mut ctx = MacroContext::new();
        ctx.define("cs", r"AB#1#2C$#3\$ ", r"#3{ab#1}#1 c##\x #2", false)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = ctx.definitions.get("cs").unwrap();
        assert_eq!(def.prefix, Some("AB".into()));
        assert_eq!(
            def.parameters,
            vec![None, Some("C$".into()), Some(r"\$ ".into())]
        );
        assert_eq!(
            def.replacement,
            vec![
                ReplacementToken::Parameter(3),
                ReplacementToken::String(r"{ab"),
                ReplacementToken::Parameter(1),
                ReplacementToken::String(r"}"),
                ReplacementToken::Parameter(1),
                ReplacementToken::String(r" c#"), 
                ReplacementToken::String(r"\"),
                ReplacementToken::String("x "),
                ReplacementToken::Parameter(2),
            ]
        );
    }

    #[test]
    fn brace_delim_no_text() {
        let mut ctx = MacroContext::new();
        ctx.define("foo", "#", "2 + 2 = 4", false)
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = ctx.definitions.get("foo").unwrap();
        assert_eq!(def.prefix, None);
        assert_eq!(def.parameters, vec![]);
        assert!(def.last_param_brace_delimited);
        assert_eq!(def.replacement, vec![ReplacementToken::String("2 + 2 = 4")]);
    }
}
