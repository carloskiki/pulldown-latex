use std::collections::HashMap;

use crate::parser::{ErrorKind, InnerResult, Token};

use super::{lex, Argument};

#[derive(Debug)]
pub struct MacroContext<'input> {
    definitions: HashMap<&'input str, Definition<'input>>,
}

impl<'input> MacroContext<'input> {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
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
    pub(crate) fn define(
        &mut self,
        name: &'input str,
        mut parameter_text: &'input str,
        replacement_text: &'input str,
    ) -> InnerResult<()> {
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
                return Err(ErrorKind::BracesInParamText);
            };
            Some(prefix)
        };

        // Parse the arguments, making sure that they are in order and that the number of arguments
        // is less than 10.
        let parameters: Vec<_> = parameters
            .map(|(i, arg)| -> InnerResult<Parameter> {
                let mut chars = arg.chars();
                let param_index = chars
                    .next()
                    .and_then(|c| c.is_ascii_digit().then_some(c as u8 - b'0'))
                    .ok_or(ErrorKind::StandaloneHashSign)?;
                if param_index != i as u8 {
                    return Err(ErrorKind::IncorrectMacroParams(param_index, i as u8));
                };
                let suffix = chars.as_str();
                if suffix.is_empty() {
                    Ok(None)
                } else {
                    // The parameter text is already guaranteed to not contain '{'.
                    if suffix.find(|c| c == '}').is_some() {
                        return Err(ErrorKind::BracesInParamText);
                    };
                    Ok(Some(suffix))
                }
            })
            .collect::<InnerResult<Vec<_>>>()?;

        let replacement = parse_replacement_text(replacement_text, parameters.len() as u8)?;

        self.definitions.insert(
            name,
            Definition::Macro(MacroDef {
                prefix,
                last_param_brace_delimited,
                parameters,
                replacement,
            }),
        );
        Ok(())
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.definitions.contains_key(name)
    }

    /// Assign a new control sequence to a token.
    pub(crate) fn assign(&mut self, name: &'input str, alias_for: Token<'input>) {
        self.definitions.insert(name, Definition::Alias(alias_for));
    }

    /// The argument count must be less than 9 if the optional argument is None, and less than 8 if
    /// the optional argument is Some.
    pub(crate) fn insert_command(
        &mut self,
        name: &'input str,
        argument_count: u8,
        optional_argument: Option<&'input str>,
        replacement: &'input str,
    ) -> InnerResult<()> {
        let replacement = parse_replacement_text(replacement, argument_count)?;

        self.definitions.insert(
            name,
            Definition::Command(CommandDef {
                argument_count,
                optional_argument,
                replacement,
            }),
        );

        Ok(())
    }

    /// If a macro is successfully expanded, the rest of the input must be discarded, and the
    /// returned string, which will contain the rest of the input appended, must be used instead.
    pub(crate) fn try_expand_in(
        &self,
        name: &'input str,
        input_rest: &'input str,
        storage: &'input bumpalo::Bump,
    ) -> Option<InnerResult<&'input str>> {
        Some(self.expand_definition_in(self.definitions.get(name)?, input_rest, storage))
    }

    fn expand_definition_in(
        &self,
        definition: &Definition<'input>,
        mut input_rest: &'input str,
        storage: &'input bumpalo::Bump,
    ) -> InnerResult<&'input str> {
        Ok(match definition {
            Definition::Macro(MacroDef {
                prefix,
                parameters,
                last_param_brace_delimited,
                replacement,
            }) => {
                if let Some(prefix) = prefix {
                    input_rest = input_rest
                        .strip_prefix(prefix)
                        .ok_or(ErrorKind::IncorrectMacroPrefix)?;
                };

                let mut arguments: Vec<Result<Argument, &str>> =
                    Vec::with_capacity(parameters.len());
                for (index, param) in parameters.iter().enumerate() {
                    if index == parameters.len() - 1 && *last_param_brace_delimited {
                        if let Some(suffix) = param {
                            let full_suffix = format!("{}{{", suffix);
                            let (before, _) = input_rest
                                .split_once(&full_suffix)
                                .ok_or(ErrorKind::EndOfInput)?;
                            arguments.push(Err(before));
                            input_rest = &input_rest[before.len()..];
                        } else {
                            let (before, _) =
                                input_rest.split_once('{').ok_or(ErrorKind::EndOfInput)?;
                            arguments.push(Err(before));
                            input_rest = &input_rest[before.len()..];
                        }
                        break;
                    }
                    match param {
                        None => arguments.push(Ok(lex::argument(&mut input_rest)?)),
                        Some(suffix) => {
                            arguments.push(Err(lex::content_with_suffix(&mut input_rest, suffix)?));
                        }
                    }
                }

                expand_replacement(storage, replacement, &arguments, input_rest)
            }
            Definition::Alias(Token::Character(c)) => {
                let ch = char::from(*c);
                let mut string = bumpalo::collections::String::with_capacity_in(
                    ch.len_utf8() + input_rest.len(),
                    storage,
                );
                string.push(ch);
                string.push_str(input_rest);
                string.into_bump_str()
            }
            Definition::Alias(Token::ControlSequence(cs)) => {
                let mut string = bumpalo::collections::String::with_capacity_in(
                    cs.len() + input_rest.len() + 1,
                    storage,
                );
                string.push('\\');
                string.push_str(cs);
                string.push_str(input_rest);
                string.into_bump_str()
            }
            Definition::Command(CommandDef {
                argument_count,
                optional_argument,
                replacement,
            }) => {
                let mut arguments = Vec::with_capacity(
                    *argument_count as usize + optional_argument.is_some() as usize,
                );

                if let Some(default_argument) = optional_argument {
                    arguments.push(Err(lex::optional_argument(&mut input_rest)?.unwrap_or(default_argument)));
                }

                (0..*argument_count)
                    .try_for_each(|_| {
                        arguments.push(Ok(lex::argument(&mut input_rest)?));
                        Ok(())
                    })?;

                expand_replacement(storage, replacement, &arguments, input_rest)
            }
        })
    }
}

fn parse_replacement_text(
    replacement_text: &str,
    parameter_count: u8,
) -> InnerResult<Vec<ReplacementToken>> {
    let mut replacement_splits = replacement_text.split_inclusive('#').peekable();
    let mut replacement_tokens: Vec<ReplacementToken> = Vec::new();

    while let Some(split) = replacement_splits.next() {
        replacement_tokens.push(ReplacementToken::String(split));

        let next_split = match replacement_splits.peek_mut() {
            Some(next_split) => next_split,
            None if split.is_empty() => {
                replacement_tokens.pop();
                break;
            }
            None if *split
                .as_bytes()
                .last()
                .expect("checked for not none in previous branch")
                != b'#' =>
            {
                break;
            }
            None => {
                return Err(ErrorKind::StandaloneHashSign);
            }
        };
        let first_char = next_split
            .chars()
            .next()
            .expect("split inclusive always yields at least one char per element");
        if first_char == '#' {
            // skip the next split since it will contain the second '#'
            replacement_splits.next();
        } else if first_char.is_ascii_digit() {
            let param_index = first_char as u8 - b'0';
            if param_index > parameter_count || param_index == 0 {
                return Err(ErrorKind::IncorrectReplacementParams(
                    param_index,
                    parameter_count,
                ));
            };

            match replacement_tokens
                .last_mut()
                .expect("was pushed previously in the loop")
            {
                ReplacementToken::String(s) => {
                    if s.len() == 1 {
                        replacement_tokens.pop();
                    } else {
                        *s = &s[..s.len() - 1];
                    }
                }
                _ => unreachable!(),
            }

            replacement_tokens.push(ReplacementToken::Parameter(param_index));
            // Make it so that the next split wont begin with the digit.
            *next_split = &next_split[1..];
        } else {
            return Err(ErrorKind::StandaloneHashSign);
        }
    }

    replacement_tokens.shrink_to_fit();
    Ok(replacement_tokens)
}

fn expand_replacement<'store>(
    storage: &'store bumpalo::Bump,
    replacement: &[ReplacementToken],
    // If Ok, its a regular argument, if Err, its a raw string to be inserted.
    arguments: &[Result<Argument, &str>],
    input_rest: &str,
    ) -> &'store str {
    let mut replacement_string = bumpalo::collections::String::new_in(storage);

    for token in replacement {
        match token {
            ReplacementToken::Parameter(idx) => {
                match &arguments[*idx as usize] {
                    Ok(Argument::Token(Token::Character(ch))) => {
                        replacement_string.push(char::from(*ch));
                    }
                    Ok(Argument::Token(Token::ControlSequence(cs))) => {
                        replacement_string.push('\\');
                        replacement_string.push_str(cs);
                    }
                    Ok(Argument::Group(group)) => {
                        replacement_string.push('{');
                        replacement_string.push_str(group);
                        replacement_string.push('}');
                    }
                    Err(str) => {
                        replacement_string.push_str(str);
                    }
                }
            }
            ReplacementToken::String(str) => {
                replacement_string.push_str(str);
            }
        }
    }

    replacement_string.push_str(input_rest);
    replacement_string.shrink_to_fit();

    replacement_string.into_bump_str()
}

impl<'input> Default for MacroContext<'input> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct MacroDef<'a> {
    prefix: Option<&'a str>,
    parameters: Vec<Parameter<'a>>,
    last_param_brace_delimited: bool,
    replacement: Vec<ReplacementToken<'a>>,
}

#[derive(Debug)]
struct CommandDef<'a> {
    argument_count: u8,
    optional_argument: Option<&'a str>,
    replacement: Vec<ReplacementToken<'a>>,
}

/// Some if the argument has a suffix, None otherwise.
type Parameter<'a> = Option<&'a str>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacementToken<'a> {
    Parameter(u8),
    String(&'a str),
}

#[derive(Debug)]
enum Definition<'a> {
    Macro(MacroDef<'a>),
    Alias(Token<'a>),
    Command(CommandDef<'a>),
}

#[cfg(test)]
mod tests {
    use super::{MacroContext, ReplacementToken};

    #[test]
    fn no_params() {
        let mut ctx = MacroContext::new();
        ctx.define("foo", "", "\\this {} is a ## test")
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        dbg!(&ctx);

        let def = match ctx.definitions.get("foo").unwrap() {
            super::Definition::Macro(def) => def,
            _ => unreachable!(),
        };
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
        ctx.define("foo", "this#1test#2. should #", "\\this {} is a ## test#1")
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = match ctx.definitions.get("foo").unwrap() {
            super::Definition::Macro(def) => def,
            _ => unreachable!(),
        };
        assert_eq!(def.prefix, Some("this"));
        assert_eq!(def.parameters, vec![Some("test"), Some(". should ")]);
        assert!(def.last_param_brace_delimited);
        assert_eq!(
            def.replacement,
            vec![
                ReplacementToken::String("\\this {} is a #"),
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
        ctx.define("cs", r"AB#1#2C$#3\$ ", r"#3{ab#1}#1 c##\x #2")
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = match ctx.definitions.get("cs").unwrap() {
            super::Definition::Macro(def) => def,
            _ => unreachable!(),
        };
        assert_eq!(def.prefix, Some("AB"));
        assert_eq!(def.parameters, vec![None, Some("C$"), Some(r"\$ ")]);
        assert_eq!(
            def.replacement,
            vec![
                ReplacementToken::Parameter(3),
                ReplacementToken::String(r"{ab"),
                ReplacementToken::Parameter(1),
                ReplacementToken::String(r"}"),
                ReplacementToken::Parameter(1),
                ReplacementToken::String(r" c#"),
                ReplacementToken::String(r"\x "),
                ReplacementToken::Parameter(2),
            ]
        );
    }

    #[test]
    fn brace_delim_no_text() {
        let mut ctx = MacroContext::new();
        ctx.define("foo", "#", "2 + 2 = 4")
            .map_err(|e| eprintln!("{e}"))
            .unwrap();

        let def = match ctx.definitions.get("foo").unwrap() {
            super::Definition::Macro(def) => def,
            _ => unreachable!(),
        };
        assert_eq!(def.prefix, None);
        assert_eq!(def.parameters, vec![]);
        assert!(def.last_param_brace_delimited);
        assert_eq!(def.replacement, vec![ReplacementToken::String("2 + 2 = 4")]);
    }
}
