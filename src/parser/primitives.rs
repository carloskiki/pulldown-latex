//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use crate::{
    attribute::{DimensionUnit, Font},
    event::{Content, Event, Identifier, Infix, Operator},
};

use super::{
    lex,
    operator_table::{is_delimiter, is_operator},
    Argument, GroupNesting, GroupType, Instruction, Parser, ParserError, Result, Token,
};

/// Return a `Content::Identifier` event with the given content and font variant.
///
/// If self is not provided, the font variant is set to `None`.
macro_rules! ident {
    ($content:expr) => {
        Content::Identifier(Identifier::Char {
            content: $content,
            variant: None,
        })
    };
    ($content:expr, $self_:ident) => {
        Content::Identifier(Identifier::Char {
            content: $content,
            variant: $self_.current_group().font_state,
        })
    };
}

/// Return an `Operator` event with the given content and default modifiers.
macro_rules! op {
    ($content:expr) => {
        Content::Operator(Operator {
            content: $content,
            ..Default::default()
        })
    };
    ($content:expr, {$($field:ident: $value:expr),*}) => {
        Content::Operator(Operator {
            content: $content,
            $($field: $value,)*
            ..Default::default()
        })
    };
}


impl<'a> Parser<'a> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` character is given
    pub fn handle_char_token(&mut self, token: char) -> Result<Event<'a>> {
        Ok(match token {
            '\\' => panic!("this function does not handle control sequences"),
            // TODO: Check how we want to handle comments actually.
            '%' => {
                let Some(content) = self.current_string() else {
                    return self.next_unwrap();
                };
                if let Some((_, rest)) = content.split_once('\n') {
                    *content = rest;
                } else {
                    *content = &content[content.len()..];
                };
                return self.next_unwrap();
            }
            '{' => {
                self.group_stack.push(GroupNesting {
                    font_state: self.current_group().font_state,
                    group_type: GroupType::Brace,
                });
                Event::BeginGroup
            }
            '}' => {
                let group = self.group_stack.pop();
                assert!(matches!(
                    group,
                    Some(GroupNesting {
                        group_type: GroupType::Brace,
                        ..
                    })
                ));
                Event::EndGroup
            }
            '_' => Event::Infix(Infix::Subscript),
            '^' => Event::Infix(Infix::Superscript),
            '$' => return Err(ParserError::MathShift),
            '#' => return Err(ParserError::HashSign),
            '&' => return Err(ParserError::AlignmentChar),
            // TODO: check for double and triple primes
            '\'' => Event::Content(op!('′')),

            c if is_delimiter(c) => Event::Content(op!(c, {stretchy: Some(false)})),
            c if is_operator(c) => Event::Content(op!(c)),
            // TODO: handle every character correctly.
            c => Event::Content(ident!(c)),
        })
    }

    /// Handle a control sequence, returning a corresponding event.
    ///
    /// 1. If the control sequence is user defined, apply the corresponding definition.
    /// 2. If the event is a primitive, apply the corresponding primitive.
    /// 3. If the control sequence is not defined, return an error.
    // TODO: change assert! to ensure!
    pub fn handle_primitive(&mut self, control_sequence: &'a str) -> Result<Event<'a>> {
        Ok(match control_sequence {
            "arccos" | "cos" | "csc" | "exp" | "ker" | "sinh" | "arcsin" | "cosh" | "deg"
            | "lg" | "ln" | "arctan" | "cot" | "det" | "hom" | "log" | "sec" | "tan" | "arg"
            | "coth" | "dim" | "sin" | "tanh" => {
                Event::Content(Content::Identifier(Identifier::Str(control_sequence)))
            }
            // TODO: The following have `under` subscripts in display math: Pr sup liminf max inf gcd limsup min

            /////////////////////////
            // Non-Latin Alphabets //
            /////////////////////////
            // Lowercase Greek letters
            "alpha" => Event::Content(ident!('α', self)),
            "beta" => Event::Content(ident!('β', self)),
            "gamma" => Event::Content(ident!('γ', self)),
            "delta" => Event::Content(ident!('δ', self)),
            "epsilon" => Event::Content(ident!('ϵ', self)),
            "varepsilon" => Event::Content(ident!('ε', self)),
            "zeta" => Event::Content(ident!('ζ', self)),
            "eta" => Event::Content(ident!('η', self)),
            "theta" => Event::Content(ident!('θ', self)),
            "vartheta" => Event::Content(ident!('ϑ', self)),
            "iota" => Event::Content(ident!('ι', self)),
            "kappa" => Event::Content(ident!('κ', self)),
            "lambda" => Event::Content(ident!('λ', self)),
            "mu" => Event::Content(ident!('µ', self)),
            "nu" => Event::Content(ident!('ν', self)),
            "xi" => Event::Content(ident!('ξ', self)),
            "pi" => Event::Content(ident!('π', self)),
            "varpi" => Event::Content(ident!('ϖ', self)),
            "rho" => Event::Content(ident!('ρ', self)),
            "varrho" => Event::Content(ident!('ϱ', self)),
            "sigma" => Event::Content(ident!('σ', self)),
            "varsigma" => Event::Content(ident!('ς', self)),
            "tau" => Event::Content(ident!('τ', self)),
            "upsilon" => Event::Content(ident!('υ', self)),
            "phi" => Event::Content(ident!('φ', self)),
            "varphi" => Event::Content(ident!('ϕ', self)),
            "chi" => Event::Content(ident!('χ', self)),
            "psi" => Event::Content(ident!('ψ', self)),
            "omega" => Event::Content(ident!('ω', self)),
            // Uppercase Greek letters
            "Alpha" => Event::Content(ident!('Α', self)),
            "Beta" => Event::Content(ident!('Β', self)),
            "Gamma" => Event::Content(ident!('Γ', self)),
            "Delta" => Event::Content(ident!('Δ', self)),
            "Epsilon" => Event::Content(ident!('Ε', self)),
            "Zeta" => Event::Content(ident!('Ζ', self)),
            "Eta" => Event::Content(ident!('Η', self)),
            "Theta" => Event::Content(ident!('Θ', self)),
            "Iota" => Event::Content(ident!('Ι', self)),
            "Kappa" => Event::Content(ident!('Κ', self)),
            "Lambda" => Event::Content(ident!('Λ', self)),
            "Mu" => Event::Content(ident!('Μ', self)),
            "Nu" => Event::Content(ident!('Ν', self)),
            "Xi" => Event::Content(ident!('Ξ', self)),
            "Pi" => Event::Content(ident!('Π', self)),
            "Rho" => Event::Content(ident!('Ρ', self)),
            "Sigma" => Event::Content(ident!('Σ', self)),
            "Tau" => Event::Content(ident!('Τ', self)),
            "Upsilon" => Event::Content(ident!('Υ', self)),
            "Phi" => Event::Content(ident!('Φ', self)),
            "Chi" => Event::Content(ident!('Χ', self)),
            "Psi" => Event::Content(ident!('Ψ', self)),
            "Omega" => Event::Content(ident!('Ω', self)),
            // Hebrew letters
            "aleph" => Event::Content(ident!('ℵ')),
            "beth" => Event::Content(ident!('ℶ')),
            "gimel" => Event::Content(ident!('ℷ')),
            "daleth" => Event::Content(ident!('ℸ')),
            // Other symbols
            "eth" => Event::Content(ident!('ð')),
            "ell" => Event::Content(ident!('ℓ')),
            "nabla" => Event::Content(ident!('∇')),
            "partial" => Event::Content(ident!('⅁')),
            "Finv" => Event::Content(ident!('Ⅎ')),
            "Game" => Event::Content(ident!('ℷ')),
            "hbar" | "hslash" => Event::Content(ident!('ℏ')),
            "imath" => Event::Content(ident!('ı')),
            "jmath" => Event::Content(ident!('ȷ')),
            "Im" => Event::Content(ident!('ℑ')),
            "Re" => Event::Content(ident!('ℜ')),
            "wp" => Event::Content(ident!('℘')),
            "Bbbk" => Event::Content(ident!('𝕜')),
            "Angstrom" => Event::Content(ident!('Å')),
            "backepsilon" => Event::Content(ident!('϶')),

            ////////////////////////
            // Font state changes //
            ////////////////////////
            // LaTeX native absolute font changes (old behavior a.k.a NFSS 1)
            "bf" => self.font_override(Font::Bold)?,
            "cal" => self.font_override(Font::Script)?,
            "it" => self.font_override(Font::Italic)?,
            "rm" => self.font_override(Font::UpRight)?,
            "sf" => self.font_override(Font::SansSerif)?,
            "tt" => self.font_override(Font::Monospace)?,
            // amsfonts font changes (old behavior a.k.a NFSS 1)
            // unicode-math font changes (old behavior a.k.a NFSS 1)
            // TODO: Make it so that there is a different between `\sym_` and `\math_` font
            // changes, as described in https://mirror.csclub.uwaterloo.ca/CTAN/macros/unicodetex/latex/unicode-math/unicode-math.pdf
            // (section. 3.1)
            "mathbf" | "symbf" | "mathbfup" | "symbfup" => self.font_group(Some(Font::Bold))?,
            "mathcal" | "symcal" | "mathup" | "symup" => self.font_group(Some(Font::Script))?,
            "mathit" | "symit" => self.font_group(Some(Font::Italic))?,
            "mathrm" | "symrm" => self.font_group(Some(Font::UpRight))?,
            "mathsf" | "symsf" | "mathsfup" | "symsfup" => self.font_group(Some(Font::SansSerif))?,
            "mathtt" | "symtt" => self.font_group(Some(Font::Monospace))?,
            "mathbb" | "symbb" => self.font_group(Some(Font::DoubleStruck))?,
            "mathfrak" | "symfrak" => self.font_group(Some(Font::Fraktur))?,
            "mathbfcal" | "symbfcal" => self.font_group(Some(Font::BoldScript))?,
            "mathsfit" | "symsfit" => self.font_group(Some(Font::SansSerifItalic))?,
            "mathbfit" | "symbfit" => self.font_group(Some(Font::BoldItalic))?,
            "mathbffrak" | "symbffrak" => self.font_group(Some(Font::BoldFraktur))?,
            "mathbfsfup" | "symbfsfup" => self.font_group(Some(Font::BoldSansSerif))?,
            "mathbfsfit" | "symbfsfit" => self.font_group(Some(Font::SansSerifBoldItalic))?,
            "mathnormal" | "symnormal" => self.font_group(None)?,

            //////////////////
            // Miscellanous //
            //////////////////
            "#" | "%" | "&" | "$" | "_" => Event::Content(Content::Identifier(Identifier::Char {
                content: control_sequence.chars().next().unwrap(),
                variant: None,
            })),
            "|" => Event::Content(op!('∥', {stretchy: Some(false)})),

            //////////////////////////////
            // Delimiter size modifiers //
            //////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => self.em_sized_delim(1.2)?,
            "Big" | "Bigl" | "Bigr" | "Bigm" => self.em_sized_delim(1.8)?,
            "bigg" | "biggl" | "biggr" | "biggm" => self.em_sized_delim(2.4)?,
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => self.em_sized_delim(3.0)?,

            // TODO: maybe use something else than an internal group for this?
            "left" => {
                let curr_str = self.current_string().ok_or(ParserError::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string().ok_or(ParserError::Delimiter)?)?;
                    self.instruction_stack
                        .push(Instruction::Event(Event::Content(op!(delimiter))));
                }
                self.group_stack.push(GroupNesting {
                    font_state: self.current_group().font_state,
                    group_type: GroupType::Internal,
                });
                Event::BeginGroup
            }
            "middle" => {
                let delimiter =
                    lex::delimiter(self.current_string().ok_or(ParserError::Delimiter)?)?;
                Event::Content(op!(delimiter))
            }
            "right" => {
                let group = self.group_stack.pop();
                assert!(matches!(
                    group,
                    Some(GroupNesting {
                        group_type: GroupType::Internal,
                        ..
                    })
                ));

                let curr_str = self.current_string().ok_or(ParserError::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    Event::EndGroup
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string().ok_or(ParserError::Delimiter)?)?;
                    self.instruction_stack
                        .push(Instruction::Event(Event::EndGroup));
                    Event::Content(op!(delimiter))
                }
            }

            ///////////////////
            // Big Operators //
            ///////////////////
            "sum" => Event::Content(op!('∑')),
            "prod" => Event::Content(op!('∏')),
            "coprod" => Event::Content(op!('∐')),
            "int" => Event::Content(op!('∫')),
            "iint" => Event::Content(op!('∬')),
            "intop" => Event::Content(op!('∫')),
            "iiint" => Event::Content(op!('∭')),
            "smallint" => Event::Content(op!('∫')),
            "iiiint" => Event::Content(op!('⨌')),
            "intcap" => Event::Content(op!('⨙')),
            "intcup" => Event::Content(op!('⨚')),
            "oint" => Event::Content(op!('∮')),
            "varointclockwise" => Event::Content(op!('∲')),
            "intclockwise" => Event::Content(op!('∱')),
            "oiint" => Event::Content(op!('∯')),
            "pointint" => Event::Content(op!('⨕')),
            "rppolint" => Event::Content(op!('⨒')),
            "scpolint" => Event::Content(op!('⨓')),
            "oiiint" => Event::Content(op!('∰')),
            "intlarhk" => Event::Content(op!('⨗')),
            "sqint" => Event::Content(op!('⨖')),
            "intx" => Event::Content(op!('⨘')),
            "intbar" => Event::Content(op!('⨍')),
            "intBar" => Event::Content(op!('⨎')),
            "fint" => Event::Content(op!('⨏')),
            "bigoplus" => Event::Content(op!('⨁')),
            "bigotimes" => Event::Content(op!('⨂')),
            "bigvee" => Event::Content(op!('⋁')),
            "bigwedge" => Event::Content(op!('⋀')),
            "bigodot" => Event::Content(op!('⨀')),
            "bigcap" => Event::Content(op!('⋂')),
            "biguplus" => Event::Content(op!('⨄')),
            "bigcup" => Event::Content(op!('⋃')),
            "bigsqcup" => Event::Content(op!('⨆')),
            "bigsqcap" => Event::Content(op!('⨅')),
            "bigtimes" => Event::Content(op!('⨉')),

            /////////////
            // Accents //
            /////////////
            "acute" => self.accent(op!('´'))?,
            "bar" | "overline" => self.accent(op!('‾'))?,
            "underbar" | "underline" => self.underscript(op!('_'))?,
            "breve" => self.accent(op!('˘'))?,
            "check" => self.accent(op!('ˇ', {stretchy: Some(false)}))?,
            "dot" => self.accent(op!('˙'))?,
            "ddot" => self.accent(op!('¨'))?,
            "grave" => self.accent(op!('`'))?,
            "hat" => self.accent(op!('^', {stretchy: Some(false)}))?,
            "tilde" => self.accent(op!('~', {stretchy: Some(false)}))?,
            "vec" => self.accent(op!('→', {stretchy: Some(false)}))?,
            "mathring" => self.accent(op!('˚'))?,

            // Arrows
            "overleftarrow" => self.accent(op!('←'))?,
            "underleftarrow" => self.underscript(op!('←'))?,
            "overrightarrow" => self.accent(op!('→'))?,
            "Overrightarrow" => self.accent(op!('⇒'))?,
            "underrightarrow" => self.underscript(op!('→'))?,
            "overleftrightarrow" => self.accent(op!('↔'))?,
            "underleftrightarrow" => self.underscript(op!('↔'))?,
            "overleftharpoon" => self.accent(op!('↼'))?,
            "overrightharpoon" => self.accent(op!('⇀'))?,

            // Wide ops
            "widecheck" => self.accent(op!('ˇ'))?,
            "widehat" => self.accent(op!('^'))?,
            "widetilde" => self.accent(op!('~'))?,
            "wideparen" | "overparen" => self.accent(op!('⏜'))?,

            // Groups
            "overgroup" => self.accent(op!('⏠'))?,
            "undergroup" => self.underscript(op!('⏡'))?,
            "overbrace" => self.accent(op!('⏞'))?,
            "underbrace" => self.underscript(op!('⏟'))?,
            "underparen" => self.underscript(op!('⏝'))?,

            // Primes
            "prime" => Event::Content(op!('′')),
            "dprime" => Event::Content(op!('″')),
            "trprime" => Event::Content(op!('‴')),
            "qprime" => Event::Content(op!('⁗')),
            "backprime" => Event::Content(op!('‵')),
            "backdprime" => Event::Content(op!('‶')),
            "backtrprime" => Event::Content(op!('‷')),

            /////////////
            // Spacing //
            /////////////
            "," | "thinspace" => Event::Space {
                width: Some((3. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            ">" | ":" | "medspace" => Event::Space {
                width: Some((4. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            ";" | "thickspace" => Event::Space {
                width: Some((5. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "enspace" => Event::Space {
                width: Some((0.5, DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "quad" => Event::Space {
                width: Some((1., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "qquad" => Event::Space {
                width: Some((2., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "~" | "nobreakspace" => Event::Content(Content::Text("&nbsp;")),
            c if c.trim_start().is_empty() => Event::Content(Content::Text("&nbsp;")),
            // Variable spacing
            "kern" => {
                let dimension =
                    lex::dimension(self.current_string().ok_or(ParserError::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "hskip" => {
                let glue = lex::glue(self.current_string().ok_or(ParserError::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "mkern" => {
                let dimension =
                    lex::math_dimension(self.current_string().ok_or(ParserError::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "mskip" => {
                let glue = lex::math_glue(self.current_string().ok_or(ParserError::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "hspace" => {
                let Argument::Group(mut argument) =
                    lex::argument(self.current_string().ok_or(ParserError::Argument)?)?
                else {
                    return Err(ParserError::DimensionArgument);
                };
                let glue = lex::glue(&mut argument)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            // Negative spacing
            "!" | "negthinspace" => Event::Space {
                width: Some((-3. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "negmedspace" => Event::Space {
                width: Some((-4. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "negthickspace" => Event::Space {
                width: Some((-5. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },

            ////////////////////////
            // Logic & Set Theory //
            ////////////////////////
            "forall" => Event::Content(op!('∀')),
            "complement" => Event::Content(op!('∁')),
            "therefore" => Event::Content(op!('∴')),
            "emptyset" => Event::Content(op!('∅')),
            "exists" => Event::Content(op!('∃')),
            "subset" => Event::Content(op!('⊂')),
            "because" => Event::Content(op!('∵')),
            "varnothing" => Event::Content(op!('⌀')),
            "nexists" => Event::Content(op!('∄')),
            "supset" => Event::Content(op!('⊃')),
            "mapsto" => Event::Content(op!('↦')),
            "implies" => Event::Content(op!('⟹')),
            "in" => Event::Content(op!('∈')),
            "mid" => Event::Content(op!('∣')),
            "to" => Event::Content(op!('→')),
            "impliedby" => Event::Content(op!('⟸')),
            "ni" => Event::Content(op!('∋')),
            "land" => Event::Content(op!('∧')),
            "gets" => Event::Content(op!('←')),
            "iff" => Event::Content(op!('⟺')),
            "notni" => Event::Content(op!('∌')),
            "neg" | "lnot" => Event::Content(op!('¬')),
            "strictif" => Event::Content(op!('⥽')),
            "strictfi" => Event::Content(op!('⥼')),

            //////////////////////
            // Binary Operators //
            //////////////////////
            "ldotp" => Event::Content(op!('.')),
            "cdotp" => Event::Content(op!('·')),
            "cdot" => Event::Content(op!('⋅')),
            "centerdot" => Event::Content(op!('·')),
            "circ" => Event::Content(op!('∘')),
            "circledast" => Event::Content(op!('⊛')),
            "circledcirc" => Event::Content(op!('⊚')),
            "circleddash" => Event::Content(op!('⊝')),
            "bigcirc" => Event::Content(op!('◯')),
            "leftthreetimes" => Event::Content(op!('⋋')),
            "rhd" => Event::Content(op!('⊳')),
            "lhd" => Event::Content(op!('⊲')),
            "leftouterjoin" => Event::Content(op!('⟕')),
            "rightouterjoin" => Event::Content(op!('⟖')),
            "rightthreetimes" => Event::Content(op!('⋌')),
            "rtimes" => Event::Content(op!('⋊')),
            "ltimes" => Event::Content(op!('⋉')),
            "leftmodels" => Event::Content(op!('⊨')),
            "amalg" => Event::Content(op!('⨿')),
            "ast" => Event::Content(op!('*')),
            "asymp" => Event::Content(op!('≍')),
            "And" => Event::Content(op!('&')),
            "lor" => Event::Content(op!('∨')),
            "setminus" => Event::Content(op!('∖')),
            "Cup" => Event::Content(op!('⋓')),
            "cup" => Event::Content(op!('∪')),
            "sqcup" => Event::Content(op!('⊔')),
            "sqcap" => Event::Content(op!('⊓')),
            "lessdot" => Event::Content(op!('⋖')),
            "smallsetminus" => Event::Content(op!('∖', {size: Some((0.7, DimensionUnit::Em))})),
            "barwedge" => Event::Content(op!('⌅')),
            "curlyvee" => Event::Content(op!('⋎')),
            "curlywedge" => Event::Content(op!('⋏')),
            "sslash" => Event::Content(op!('⫽')),
            "bowtie" | "Join" => Event::Content(op!('⋈')),
            "div" => Event::Content(op!('÷')),
            "mp" => Event::Content(op!('∓')),
            "times" => Event::Content(op!('×')),
            "boxdot" => Event::Content(op!('⊡')),
            "divideontimes" => Event::Content(op!('⋇')),
            "odot" => Event::Content(op!('⊙')),
            "unlhd" => Event::Content(op!('⊴')),
            "boxminus" => Event::Content(op!('⊟')),
            "dotplus" => Event::Content(op!('∔')),
            "ominus" => Event::Content(op!('⊖')),
            "unrhd" => Event::Content(op!('⊵')),
            "boxplus" => Event::Content(op!('⊞')),
            "doublebarwedge" => Event::Content(op!('⩞')),
            "oplus" => Event::Content(op!('⊕')),
            "uplus" => Event::Content(op!('⊎')),
            "boxtimes" => Event::Content(op!('⊠')),
            "doublecap" => Event::Content(op!('⋒')),
            "otimes" => Event::Content(op!('⊗')),
            "vee" => Event::Content(op!('∨')),
            "veebar" => Event::Content(op!('⊻')),
            "Cap" => Event::Content(op!('⋒')),
            "fullouterjoin" => Event::Content(op!('⟗')),
            "parr" => Event::Content(op!('⅋')),
            "wedge" => Event::Content(op!('∧')),
            "cap" => Event::Content(op!('∩')),
            "gtrdot" => Event::Content(op!('⋗')),
            "pm" => Event::Content(op!('±')),
            "with" => Event::Content(op!('&')),
            "intercal" => Event::Content(op!('⊺')),
            "wr" => Event::Content(op!('≀')),
            ///////////////
            // Fractions //
            ///////////////
            "frac" => {
                // TODO: This does not handle the case where both arguments are separated across different
                // instructions.
                let [numerator, denominator] =
                    lex::arguments(self.current_string().ok_or(ParserError::Argument)?)?;
                self.instruction_stack
                    .push(Instruction::Event(Event::EndGroup));
                let denom_instruction = match denominator {
                    Argument::Token(Token::Character(c)) => {
                        Instruction::Event(self.handle_char_token(c)?)
                    }
                    Argument::Token(Token::ControlSequence(cs)) => {
                        Instruction::Event(self.handle_primitive(cs)?)
                    }
                    Argument::Group(content) => Instruction::Substring {
                        content,
                        pop_internal_group: false,
                    },
                };
                self.instruction_stack.extend([
                    denom_instruction,
                    Instruction::Event(Event::BeginGroup),
                    Instruction::Event(Event::EndGroup),
                ]);

                let num_instruction = match numerator {
                    Argument::Token(Token::Character(c)) => {
                        Instruction::Event(self.handle_char_token(c)?)
                    }
                    Argument::Token(Token::ControlSequence(cs)) => {
                        Instruction::Event(self.handle_primitive(cs)?)
                    }
                    Argument::Group(content) => Instruction::Substring {
                        content,
                        pop_internal_group: false,
                    },
                };
                self.instruction_stack
                    .extend([num_instruction, Instruction::Event(Event::BeginGroup)]);
                Event::Visual(crate::event::Visual::Fraction(None))
            }

            "angle" => Event::Content(ident!('∠')),
            "approx" => Event::Content(op!('≈')),
            "approxeq" => Event::Content(op!('≊')),
            "approxcolon" => {
                self.instruction_stack.push(Instruction::Event(Event::Content(op! {
                    ':',
                    {left_space: Some((0., DimensionUnit::Em))}
                })));
                Event::Content(op! {
                    '≈',
                    {right_space: Some((0., DimensionUnit::Em))}
                })
            }
            "approxcoloncolon" => {
                self.instruction_stack.push(Instruction::Event(
                    Event::Content(op! {':', {left_space: Some((0., DimensionUnit::Em))}}),
                ));
                self.instruction_stack.push(Instruction::Event(Event::Content(op! {
                    ':',
                    {
                        left_space: Some((0., DimensionUnit::Em)),
                        right_space: Some((0., DimensionUnit::Em))
                    }
                })));
                Event::Content(op! {
                    '≈',
                    {right_space: Some((0., DimensionUnit::Em))}
                })
            }
            "backsim" => Event::Content(op!('∽')),
            "backsimeq" => Event::Content(op!('⋍')),
            "backslash" => Event::Content(ident!('\\')),
            "between" => Event::Content(op!('≬')),

            _ => todo!(),
        })
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn em_sized_delim(&mut self, size: f32) -> Result<Event<'a>> {
        let delimiter = lex::delimiter(self.current_string().ok_or(ParserError::Delimiter)?)?;
        Ok(Event::Content(op!(delimiter, {size: Some((size, DimensionUnit::Em))})))
    }

    /// Override the `font_state` to the given font variant, and return the next event.
    fn font_override(&mut self, font: Font) -> Result<Event<'a>> {
        self.current_group_mut().font_state = Some(font);
        self.next_unwrap()
    }


    /// Override the `font_state` for the argument to the command.
    fn font_group(&mut self, font: Option<Font>) -> Result<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ParserError::Argument)?)?;
        match argument {
            Argument::Token(Token::Character(c)) => self.handle_char_token(c),
            Argument::Token(Token::ControlSequence(cs)) => self.handle_primitive(cs),
            Argument::Group(g) => {
                self.instruction_stack.push(Instruction::Substring {
                    content: g,
                    pop_internal_group: true,
                });
                self.group_stack.push(GroupNesting {
                    font_state: font,
                    group_type: GroupType::Internal,
                });
                self.next_unwrap()
            }
        }
    }

    /// Accent commands. parse the argument, and overset the accent.
    fn accent(&mut self, accent: Content) -> Result<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ParserError::Argument)?)?;
        self.instruction_stack.extend([
            Instruction::Event(Event::Content(accent)),
            Instruction::Event(Event::Infix(Infix::Overscript)),
        ]);
        match argument {
            Argument::Token(Token::Character(c)) => self.handle_char_token(c),
            Argument::Token(Token::ControlSequence(cs)) => self.handle_primitive(cs),
            Argument::Group(g) => {
                self.instruction_stack.push(Instruction::Substring {
                    content: g,
                    pop_internal_group: true,
                });
                self.group_stack.push(GroupNesting {
                    font_state: self.current_group().font_state,
                    group_type: GroupType::Internal,
                });
                self.next_unwrap()
            }
        }
    }

    /// Underscript commands. parse the argument, and underset the accent.
    fn underscript(&mut self, content: Content) -> Result<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ParserError::Argument)?)?;
        self.instruction_stack.extend([
            Instruction::Event(Event::Content(content)),
            Instruction::Event(Event::Infix(Infix::Underscript)),
        ]);
        match argument {
            Argument::Token(Token::Character(c)) => self.handle_char_token(c),
            Argument::Token(Token::ControlSequence(cs)) => self.handle_primitive(cs),
            Argument::Group(g) => {
                self.instruction_stack.push(Instruction::Substring {
                    content: g,
                    pop_internal_group: true,
                });
                self.group_stack.push(GroupNesting {
                    font_state: self.current_group().font_state,
                    group_type: GroupType::Internal,
                });
                self.next_unwrap()
            }
        }
    }
}


// TODO implementations:
// `*` ending commands
// `begingroup` and `endgroup`: https://tex.stackexchange.com/a/191533
// `sc` (small caps) font: https://tug.org/texinfohtml/latex2e.html#index-_005csc
// `bmod`, `pod`, `pmod`, `centerdot`

// Unimplemented primitives:
// `sl` (slanted) font: https://tug.org/texinfohtml/latex2e.html#index-_005csl
// `bbit` (double-struck italic) font
// `symliteral` wtf is this? (in unicode-math)

// Currently unhandled:
// - `relax`
// - `kern`, `mkern`
// - `hskip`
// - `\ ` (control space)
// - `raise`, `lower`
// - `char`
// - `hbox`, `mbox`?
// - `vcenter`
// - `rule`
// - `math_` atoms
// - `limits`, `nolimits` (only after Op)
// - `mathchoice` (TeXbook p. 151)
// - `displaystyle`, `textstyle`, `scriptstyle`, `scriptscriptstyle`
// - `over`, `atop`
// - `allowbreak`
