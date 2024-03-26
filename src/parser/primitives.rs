//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use crate::{
    attribute::{DimensionUnit, Font},
    event::{Content, Event, Identifier, Operator, Visual},
};

use super::{
    lex,
    operator_table::{is_delimiter, is_operator},
    Argument, ErrorKind, GroupType, InnerResult, Instruction, Parser, Token,
};

/// Return an `Operator` event with the given content and default modifiers.
macro_rules! op {
    ($content:expr) => {
        Operator {
            content: $content,
            ..Default::default()
        }
    };
    ($content:expr, {$($field:ident: $value:expr),*}) => {
        Operator {
            content: $content,
            $($field: $value,)*
            ..Default::default()
        }
    };
}

macro_rules! ensure_eq {
    ($left:expr, $right:expr, $err:expr) => {
        if $left != $right {
            return Err($err);
        }
    };
}

// NOTE/TODO: Currently, things like `\it_a` do not error.

impl<'a> Parser<'a> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` or `%` character is given
    pub(crate) fn handle_char_token(&mut self, token: char) -> InnerResult<()> {
        let instruction = Instruction::Event(match token {
            '\\' => panic!("(internal error: please report) the `\\` character should never be observed as a token"),
            '%' => panic!("(internal error: please report) the `%` character should never be observed as a token"),
            '_' => return Err(ErrorKind::SubscriptAsToken),
            '^' => return Err(ErrorKind::SuperscriptAsToken),
            '$' => return Err(ErrorKind::MathShift),
            '#' => return Err(ErrorKind::HashSign),
            '&' => return Err(ErrorKind::AlignmentChar),
            '{' => {
                self.group_stack.push(GroupType::Brace);
                Event::BeginGroup
            },
            '}' => {
                ensure_eq!(self.group_stack.pop(), Some(GroupType::Brace), ErrorKind::UnbalancedGroup(Some(GroupType::Brace)));
                Event::EndGroup
            },
            '\'' => Event::Content(Content::Operator(op!('′'))),

            c if is_delimiter(c) => Event::Content(Content::Operator(op!(c, {stretchy: Some(false)}))),
            c if is_operator(c) => Event::Content(Content::Operator(op!(c))),
            '0'..='9' => Event::Content(Content::Number(Identifier::Char(token))),
            c => ident(c),
        });
        self.buffer.push(instruction);
        Ok(())
    }

    /// Handle a supported control sequence, pushing instructions to the provided stack.
    pub(crate) fn handle_primitive(&mut self, control_sequence: &'a str) -> InnerResult<()> {
        let event = match control_sequence {
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
            "alpha" => ident('α'),
            "beta" => ident('β'),
            "gamma" => ident('γ'),
            "delta" => ident('δ'),
            "epsilon" => ident('ϵ'),
            "varepsilon" => ident('ε'),
            "zeta" => ident('ζ'),
            "eta" => ident('η'),
            "theta" => ident('θ'),
            "vartheta" => ident('ϑ'),
            "iota" => ident('ι'),
            "kappa" => ident('κ'),
            "lambda" => ident('λ'),
            "mu" => ident('µ'),
            "nu" => ident('ν'),
            "xi" => ident('ξ'),
            "pi" => ident('π'),
            "varpi" => ident('ϖ'),
            "rho" => ident('ρ'),
            "varrho" => ident('ϱ'),
            "sigma" => ident('σ'),
            "varsigma" => ident('ς'),
            "tau" => ident('τ'),
            "upsilon" => ident('υ'),
            "phi" => ident('φ'),
            "varphi" => ident('ϕ'),
            "chi" => ident('χ'),
            "psi" => ident('ψ'),
            "omega" => ident('ω'),
            // Uppercase Greek letters
            "Alpha" => ident('Α'),
            "Beta" => ident('Β'),
            "Gamma" => ident('Γ'),
            "Delta" => ident('Δ'),
            "Epsilon" => ident('Ε'),
            "Zeta" => ident('Ζ'),
            "Eta" => ident('Η'),
            "Theta" => ident('Θ'),
            "Iota" => ident('Ι'),
            "Kappa" => ident('Κ'),
            "Lambda" => ident('Λ'),
            "Mu" => ident('Μ'),
            "Nu" => ident('Ν'),
            "Xi" => ident('Ξ'),
            "Pi" => ident('Π'),
            "Rho" => ident('Ρ'),
            "Sigma" => ident('Σ'),
            "Tau" => ident('Τ'),
            "Upsilon" => ident('Υ'),
            "Phi" => ident('Φ'),
            "Chi" => ident('Χ'),
            "Psi" => ident('Ψ'),
            "Omega" => ident('Ω'),
            // Hebrew letters
            "aleph" => ident('ℵ'),
            "beth" => ident('ℶ'),
            "gimel" => ident('ℷ'),
            "daleth" => ident('ℸ'),
            // Other symbols
            "eth" => ident('ð'),
            "ell" => ident('ℓ'),
            "nabla" => ident('∇'),
            "partial" => ident('⅁'),
            "Finv" => ident('Ⅎ'),
            "Game" => ident('ℷ'),
            "hbar" | "hslash" => ident('ℏ'),
            "imath" => ident('ı'),
            "jmath" => ident('ȷ'),
            "Im" => ident('ℑ'),
            "Re" => ident('ℜ'),
            "wp" => ident('℘'),
            "Bbbk" => ident('𝕜'),
            "Angstrom" => ident('Å'),
            "backepsilon" => ident('϶'),

            ////////////////////////
            // Font state changes //
            ////////////////////////
            // LaTeX native absolute font changes (old behavior a.k.a NFSS 1)
            "bf" => font_override(Font::Bold),
            "cal" => font_override(Font::Script),
            "it" => font_override(Font::Italic),
            "rm" => font_override(Font::UpRight),
            "sf" => font_override(Font::SansSerif),
            "tt" => font_override(Font::Monospace),
            // amsfonts font changes (old behavior a.k.a NFSS 1)
            // unicode-math font changes (old behavior a.k.a NFSS 1)
            // changes, as described in https://mirror.csclub.uwaterloo.ca/CTAN/macros/unicodetex/latex/unicode-math/unicode-math.pdf
            // (section. 3.1)
            "mathbf" | "symbf" | "mathbfup" | "symbfup" => {
                return self.font_group(Some(Font::Bold))
            }
            "mathcal" | "symcal" | "mathup" | "symup" => {
                return self.font_group(Some(Font::Script))
            }
            "mathit" | "symit" => return self.font_group(Some(Font::Italic)),
            "mathrm" | "symrm" => return self.font_group(Some(Font::UpRight)),
            "mathsf" | "symsf" | "mathsfup" | "symsfup" => {
                return self.font_group(Some(Font::SansSerif))
            }
            "mathtt" | "symtt" => return self.font_group(Some(Font::Monospace)),
            "mathbb" | "symbb" => return self.font_group(Some(Font::DoubleStruck)),
            "mathfrak" | "symfrak" => return self.font_group(Some(Font::Fraktur)),
            "mathbfcal" | "symbfcal" => return self.font_group(Some(Font::BoldScript)),
            "mathsfit" | "symsfit" => return self.font_group(Some(Font::SansSerifItalic)),
            "mathbfit" | "symbfit" => return self.font_group(Some(Font::BoldItalic)),
            "mathbffrak" | "symbffrak" => return self.font_group(Some(Font::BoldFraktur)),
            "mathbfsfup" | "symbfsfup" => return self.font_group(Some(Font::BoldSansSerif)),
            "mathbfsfit" | "symbfsfit" => return self.font_group(Some(Font::SansSerifBoldItalic)),
            "mathnormal" | "symnormal" => return self.font_group(None),

            //////////////////////////////
            // Delimiter size modifiers //
            //////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => return self.em_sized_delim(1.2),
            "Big" | "Bigl" | "Bigr" | "Bigm" => return self.em_sized_delim(1.8),
            "bigg" | "biggl" | "biggr" | "biggm" => return self.em_sized_delim(2.4),
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => return self.em_sized_delim(3.0),

            "left" => {
                let curr_str = self.current_string()?.ok_or(ErrorKind::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    Event::BeginGroup
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string()?.ok_or(ErrorKind::Delimiter)?)?;
                    self.buffer.extend([
                        Instruction::Event(Event::BeginGroup),
                        Instruction::Event(Event::Content(Content::Operator(op!(delimiter)))),
                    ]);
                    self.group_stack.push(GroupType::LeftRight);
                    return Ok(());
                }
            }
            "middle" => {
                let delimiter =
                    lex::delimiter(self.current_string()?.ok_or(ErrorKind::Delimiter)?)?;
                operator(op!(delimiter))
            }
            "right" => {
                let curr_str = self.current_string()?.ok_or(ErrorKind::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    Event::EndGroup
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string()?.ok_or(ErrorKind::Delimiter)?)?;
                    self.buffer.extend([
                        Instruction::Event(Event::Content(Content::Operator(op!(delimiter)))),
                        Instruction::Event(Event::EndGroup),
                    ]);
                    ensure_eq!(
                        self.group_stack.pop(),
                        Some(GroupType::LeftRight),
                        ErrorKind::UnbalancedGroup(Some(GroupType::LeftRight))
                    );
                    return Ok(());
                }
            }

            ///////////////////
            // Big Operators //
            ///////////////////
            "sum" => operator(op!('∑')),
            "prod" => operator(op!('∏')),
            "coprod" => operator(op!('∐')),
            "int" => operator(op!('∫')),
            "iint" => operator(op!('∬')),
            "intop" => operator(op!('∫')),
            "iiint" => operator(op!('∭')),
            "smallint" => operator(op!('∫')),
            "iiiint" => operator(op!('⨌')),
            "intcap" => operator(op!('⨙')),
            "intcup" => operator(op!('⨚')),
            "oint" => operator(op!('∮')),
            "varointclockwise" => operator(op!('∲')),
            "intclockwise" => operator(op!('∱')),
            "oiint" => operator(op!('∯')),
            "pointint" => operator(op!('⨕')),
            "rppolint" => operator(op!('⨒')),
            "scpolint" => operator(op!('⨓')),
            "oiiint" => operator(op!('∰')),
            "intlarhk" => operator(op!('⨗')),
            "sqint" => operator(op!('⨖')),
            "intx" => operator(op!('⨘')),
            "intbar" => operator(op!('⨍')),
            "intBar" => operator(op!('⨎')),
            "fint" => operator(op!('⨏')),
            "bigoplus" => operator(op!('⨁')),
            "bigotimes" => operator(op!('⨂')),
            "bigvee" => operator(op!('⋁')),
            "bigwedge" => operator(op!('⋀')),
            "bigodot" => operator(op!('⨀')),
            "bigcap" => operator(op!('⋂')),
            "biguplus" => operator(op!('⨄')),
            "bigcup" => operator(op!('⋃')),
            "bigsqcup" => operator(op!('⨆')),
            "bigsqcap" => operator(op!('⨅')),
            "bigtimes" => operator(op!('⨉')),

            /////////////
            // Accents //
            /////////////
            "acute" => return self.accent(op!('´')),
            "bar" | "overline" => return self.accent(op!('‾')),
            "underbar" | "underline" => return self.underscript(op!('_')),
            "breve" => return self.accent(op!('˘')),
            "check" => return self.accent(op!('ˇ', {stretchy: Some(false)})),
            "dot" => return self.accent(op!('˙')),
            "ddot" => return self.accent(op!('¨')),
            "grave" => return self.accent(op!('`')),
            "hat" => return self.accent(op!('^', {stretchy: Some(false)})),
            "tilde" => return self.accent(op!('~', {stretchy: Some(false)})),
            "vec" => return self.accent(op!('→', {stretchy: Some(false)})),
            "mathring" => return self.accent(op!('˚')),

            // Arrows
            "overleftarrow" => return self.accent(op!('←')),
            "underleftarrow" => return self.underscript(op!('←')),
            "overrightarrow" => return self.accent(op!('→')),
            "Overrightarrow" => return self.accent(op!('⇒')),
            "underrightarrow" => return self.underscript(op!('→')),
            "overleftrightarrow" => return self.accent(op!('↔')),
            "underleftrightarrow" => return self.underscript(op!('↔')),
            "overleftharpoon" => return self.accent(op!('↼')),
            "overrightharpoon" => return self.accent(op!('⇀')),

            // Wide ops
            "widecheck" => return self.accent(op!('ˇ')),
            "widehat" => return self.accent(op!('^')),
            "widetilde" => return self.accent(op!('~')),
            "wideparen" | "overparen" => return self.accent(op!('⏜')),

            // Groups
            "overgroup" => return self.accent(op!('⏠')),
            "undergroup" => return self.underscript(op!('⏡')),
            "overbrace" => return self.accent(op!('⏞')),
            "underbrace" => return self.underscript(op!('⏟')),
            "underparen" => return self.underscript(op!('⏝')),

            // Primes
            "prime" => operator(op!('′')),
            "dprime" => operator(op!('″')),
            "trprime" => operator(op!('‴')),
            "qprime" => operator(op!('⁗')),
            "backprime" => operator(op!('‵')),
            "backdprime" => operator(op!('‶')),
            "backtrprime" => operator(op!('‷')),

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
            "~" | "nobreakspace" => Event::Content(Content::Text(Identifier::Str("&nbsp;"))),
            // Variable spacing
            "kern" => {
                let dimension =
                    lex::dimension(self.current_string()?.ok_or(ErrorKind::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "hskip" => {
                let glue = lex::glue(self.current_string()?.ok_or(ErrorKind::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "mkern" => {
                let dimension =
                    lex::math_dimension(self.current_string()?.ok_or(ErrorKind::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "mskip" => {
                let glue = lex::math_glue(self.current_string()?.ok_or(ErrorKind::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "hspace" => {
                let Argument::Group(mut argument) =
                    lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?
                else {
                    return Err(ErrorKind::DimensionArgument);
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
            "forall" => operator(op!('∀')),
            "complement" => operator(op!('∁')),
            "therefore" => operator(op!('∴')),
            "emptyset" => operator(op!('∅')),
            "exists" => operator(op!('∃')),
            "subset" => operator(op!('⊂')),
            "because" => operator(op!('∵')),
            "varnothing" => operator(op!('⌀')),
            "nexists" => operator(op!('∄')),
            "supset" => operator(op!('⊃')),
            "mapsto" => operator(op!('↦')),
            "implies" => operator(op!('⟹')),
            "in" => operator(op!('∈')),
            "mid" => operator(op!('∣')),
            "to" => operator(op!('→')),
            "impliedby" => operator(op!('⟸')),
            "ni" => operator(op!('∋')),
            "land" => operator(op!('∧')),
            "gets" => operator(op!('←')),
            "iff" => operator(op!('⟺')),
            "notni" => operator(op!('∌')),
            "neg" | "lnot" => operator(op!('¬')),
            "strictif" => operator(op!('⥽')),
            "strictfi" => operator(op!('⥼')),

            //////////////////////
            // Binary Operators //
            //////////////////////
            "ldotp" => operator(op!('.')),
            "cdotp" => operator(op!('·')),
            "cdot" => operator(op!('⋅')),
            "centerdot" => operator(op!('·')),
            "circ" => operator(op!('∘')),
            "circledast" => operator(op!('⊛')),
            "circledcirc" => operator(op!('⊚')),
            "circleddash" => operator(op!('⊝')),
            "bigcirc" => operator(op!('◯')),
            "leftthreetimes" => operator(op!('⋋')),
            "rhd" => operator(op!('⊳')),
            "lhd" => operator(op!('⊲')),
            "leftouterjoin" => operator(op!('⟕')),
            "rightouterjoin" => operator(op!('⟖')),
            "rightthreetimes" => operator(op!('⋌')),
            "rtimes" => operator(op!('⋊')),
            "ltimes" => operator(op!('⋉')),
            "leftmodels" => operator(op!('⊨')),
            "amalg" => operator(op!('⨿')),
            "ast" => operator(op!('*')),
            "asymp" => operator(op!('≍')),
            "And" => operator(op!('&')),
            "lor" => operator(op!('∨')),
            "setminus" => operator(op!('∖')),
            "Cup" => operator(op!('⋓')),
            "cup" => operator(op!('∪')),
            "sqcup" => operator(op!('⊔')),
            "sqcap" => operator(op!('⊓')),
            "lessdot" => operator(op!('⋖')),
            "smallsetminus" => operator(op!('∖', {size: Some((0.7, DimensionUnit::Em))})),
            "barwedge" => operator(op!('⌅')),
            "curlyvee" => operator(op!('⋎')),
            "curlywedge" => operator(op!('⋏')),
            "sslash" => operator(op!('⫽')),
            "bowtie" | "Join" => operator(op!('⋈')),
            "div" => operator(op!('÷')),
            "mp" => operator(op!('∓')),
            "times" => operator(op!('×')),
            "boxdot" => operator(op!('⊡')),
            "divideontimes" => operator(op!('⋇')),
            "odot" => operator(op!('⊙')),
            "unlhd" => operator(op!('⊴')),
            "boxminus" => operator(op!('⊟')),
            "dotplus" => operator(op!('∔')),
            "ominus" => operator(op!('⊖')),
            "unrhd" => operator(op!('⊵')),
            "boxplus" => operator(op!('⊞')),
            "doublebarwedge" => operator(op!('⩞')),
            "oplus" => operator(op!('⊕')),
            "uplus" => operator(op!('⊎')),
            "boxtimes" => operator(op!('⊠')),
            "doublecap" => operator(op!('⋒')),
            "otimes" => operator(op!('⊗')),
            "vee" => operator(op!('∨')),
            "veebar" => operator(op!('⊻')),
            "Cap" => operator(op!('⋒')),
            "fullouterjoin" => operator(op!('⟗')),
            "parr" => operator(op!('⅋')),
            "wedge" => operator(op!('∧')),
            "cap" => operator(op!('∩')),
            "gtrdot" => operator(op!('⋗')),
            "pm" => operator(op!('±')),
            "with" => operator(op!('&')),
            "intercal" => operator(op!('⊺')),
            "wr" => operator(op!('≀')),
            ///////////////
            // Fractions //
            ///////////////
            "frac" => {
                self.buffer
                    .push(Instruction::Event(Event::Visual(Visual::Fraction(None))));
                let first_arg = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
                self.handle_argument(first_arg)?;
                let second_arg = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
                self.handle_argument(second_arg)?;
                return Ok(());
            }

            "angle" => ident('∠'),
            "approx" => operator(op!('≈')),
            "approxeq" => operator(op!('≊')),
            "aPproxcolon" => {
                self.multi_event([
                    Event::Content(Content::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    Event::Content(Content::Operator(op! {
                        ':',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "approxcoloncolon" => {
                self.multi_event([
                    Event::Content(Content::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    Event::Content(Content::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    })),
                    Event::Content(Content::Operator(
                        op! {':', {left_space: Some((0., DimensionUnit::Em))}},
                    )),
                ]);
                return Ok(());
            }
            "backsim" => operator(op!('∽')),
            "backsimeq" => operator(op!('⋍')),
            "backslash" => ident('\\'),
            "between" => operator(op!('≬')),

            ///////////////////
            // Miscellaneous //
            ///////////////////
            "#" | "%" | "&" | "$" | "_" => ident(
                control_sequence
                    .chars()
                    .next()
                    .expect("the control sequence contains one of the matched characters"),
            ),
            "|" => operator(op!('∥', {stretchy: Some(false)})),
            "text" => {
                let argument = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
                self.buffer
                    .push(Instruction::Event(Event::Content(Content::Text(
                        match argument {
                            Argument::Token(Token::Character(c)) => Identifier::Char(c),
                            Argument::Group(inner) => Identifier::Str(inner),
                            _ => return Err(ErrorKind::TextModeControlSequence),
                        },
                    ))));
                return Ok(());
            }
            "begingroup" => {
                self.group_stack.push(GroupType::BeginGroup);
                Event::BeginGroup
            }
            "endgroup" => {
                ensure_eq!(
                    self.group_stack.pop(),
                    Some(GroupType::BeginGroup),
                    ErrorKind::UnbalancedGroup(Some(GroupType::BeginGroup))
                );
                Event::EndGroup
            }

            // Spacing
            c if c.trim_start().is_empty() => {
                Event::Content(Content::Text(Identifier::Str("&nbsp;")))
            }

            _ => return Err(ErrorKind::UnknownPrimitive),
        };
        self.buffer.push(Instruction::Event(event));
        Ok(())
    }

    /// Handle a control sequence that outputs more than one event.
    fn multi_event<const N: usize>(&mut self, events: [Event<'a>; N]) {
        self.buffer.push(Instruction::Event(Event::BeginGroup));
        self.buffer
            .extend(events.iter().map(|event| Instruction::Event(*event)));
        self.buffer.push(Instruction::Event(Event::EndGroup));
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn em_sized_delim(&mut self, size: f32) -> InnerResult<()> {
        let delimiter = lex::delimiter(self.current_string()?.ok_or(ErrorKind::Delimiter)?)?;
        self.buffer
            .push(Instruction::Event(Event::Content(Content::Operator(
                op!(delimiter, {size: Some((size, DimensionUnit::Em))}),
            ))));
        Ok(())
    }

    /// Override the `font_state` for the argument to the command.
    fn font_group(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
        self.buffer.extend([
            Instruction::Event(Event::BeginGroup),
            Instruction::Event(Event::FontChange(font)),
        ]);
        match argument {
            Argument::Token(token) => {
                match token {
                    Token::ControlSequence(cs) => self.handle_primitive(cs)?,
                    Token::Character(c) => self.handle_char_token(c)?,
                };
            }
            Argument::Group(group) => {
                self.buffer.push(Instruction::Substring(group));
            }
        };
        self.buffer.push(Instruction::Event(Event::EndGroup));
        Ok(())
    }

    /// Accent commands. parse the argument, and overset the accent.
    fn accent(&mut self, accent: Operator) -> InnerResult<()> {
        let argument = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
        self.buffer
            .push(Instruction::Event(Event::Visual(Visual::Overscript)));
        self.handle_argument(argument)?;
        self.buffer
            .push(Instruction::Event(Event::Content(Content::Operator(
                accent,
            ))));
        Ok(())
    }

    /// Underscript commands. parse the argument, and underset the accent.
    fn underscript(&mut self, content: Operator) -> InnerResult<()> {
        let argument = lex::argument(self.current_string()?.ok_or(ErrorKind::Argument)?)?;
        self.buffer
            .push(Instruction::Event(Event::Visual(Visual::Underscript)));
        self.handle_argument(argument)?;
        self.buffer
            .push(Instruction::Event(Event::Content(Content::Operator(
                content,
            ))));

        Ok(())
    }
}

#[inline]
fn font_override(font: Font) -> Event<'static> {
    Event::FontChange(Some(font))
}

#[inline]
fn ident(ident: char) -> Event<'static> {
    Event::Content(Content::Identifier(Identifier::Char(ident)))
}

#[inline]
fn operator(operator: Operator) -> Event<'static> {
    Event::Content(Content::Operator(operator))
}

// TODO implementations:
// `sc` (small caps) font: https://tug.org/texinfohtml/latex2e.html#index-_005csc
// `bmod`, `pod`, `pmod`, `centerdot`
// - `relax`
// - `raise`, `lower`
// - `char`
// - `hbox`, `mbox`?
// - `vcenter`
// - `rule`
// - `math_` atoms
// - `limits`, `nolimits` (only after Op)
// - `mathchoice` (TeXbook p. 151)
// - `displaystyle`, `textstyle`, `scriptstyle`, `scriptscriptstyle`

// Unimplemented primitives:
// `sl` (slanted) font: https://tug.org/texinfohtml/latex2e.html#index-_005csl
// `bbit` (double-struck italic) font
// `symliteral` wtf is this? (in unicode-math)
