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

/// Return a `Content::Identifier` event with the given content and font variant.
///
/// If self is not provided, the font variant is set to `None`.
macro_rules! ident {
    ($content:expr) => {
        Content::Identifier(Identifier::Char($content))
    };
}

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

// NOTE/TODO: what if there is something such as `a_\pi_\pi` would the current implementation parse
// it normally since the subscript `pi` automatically parses another subscript? Yes, and this is a
// problem!!!
// How do we handle:
// - `__`:  handle char returns an error.
// - `_\frac{a}{b}`: Parse the base into the staging buffer, parse the superscript into the stack,
// and parse the subscript into the staging buffer on top of the base. Then drain the subscript from
// the staging buffer, and extend it to the stack, and then drain the base and extend it to the
// stack.
// - `\it _a`: In the `next` function, always parse the next token in the staging buffer, and then
// always check for suffixes. This solves the issues with `\mathcal{...}_a` and etc.

// TODO: Have an handler for multi-event primitives, because they must be grouped.
// TODO: Most of hepler methods such as `operator` or `ident` could be implemented as normal functions.

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
                let grouping = self.group_stack.pop().ok_or(ErrorKind::UnbalancedGroup(None))?;
                ensure_eq!(grouping, GroupType::Brace, ErrorKind::UnbalancedGroup(Some(grouping)));
                Event::EndGroup
            },
            // TODO: check for double and triple primes
            '\'' => Event::Content(Content::Operator(op!('′'))),

            c if is_delimiter(c) => Event::Content(Content::Operator(op!(c, {stretchy: Some(false)}))),
            c if is_operator(c) => Event::Content(Content::Operator(op!(c))),
            '0'..='9' => Event::Content(Content::Number(Identifier::Char(token))),
            // TODO: handle every character correctly.
            c => Event::Content(ident!(c)),
        });
        self.stack().push(instruction);
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
            "bf" => self.font_override(Font::Bold),
            "cal" => self.font_override(Font::Script),
            "it" => self.font_override(Font::Italic),
            "rm" => self.font_override(Font::UpRight),
            "sf" => self.font_override(Font::SansSerif),
            "tt" => self.font_override(Font::Monospace),
            // amsfonts font changes (old behavior a.k.a NFSS 1)
            // unicode-math font changes (old behavior a.k.a NFSS 1)
            // TODO: Make it so that there is a different between `\sym_` and `\math_` font
            // changes, as described in https://mirror.csclub.uwaterloo.ca/CTAN/macros/unicodetex/latex/unicode-math/unicode-math.pdf
            // (section. 3.1)
            "mathbf" | "symbf" | "mathbfup" | "symbfup" => self.font_group(Some(Font::Bold))?,
            "mathcal" | "symcal" | "mathup" | "symup" => self.font_group(Some(Font::Script))?,
            "mathit" | "symit" => self.font_group(Some(Font::Italic))?,
            "mathrm" | "symrm" => self.font_group(Some(Font::UpRight))?,
            "mathsf" | "symsf" | "mathsfup" | "symsfup" => {
                self.font_group(Some(Font::SansSerif))?
            }
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
            "#" | "%" | "&" | "$" | "_" => ident(
                control_sequence
                    .chars()
                    .next()
                    .expect("the control sequence contains one of the matched characters"),
            ),
            "|" => operator(op!('∥', {stretchy: Some(false)})),

            //////////////////////////////
            // Delimiter size modifiers //
            //////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => self.em_sized_delim(1.2)?,
            "Big" | "Bigl" | "Bigr" | "Bigm" => self.em_sized_delim(1.8)?,
            "bigg" | "biggl" | "biggr" | "biggm" => self.em_sized_delim(2.4)?,
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => self.em_sized_delim(3.0)?,

            // TODO: Fix these 3 they do not work!!!
            "left" => {
                let curr_str = self.current_string().ok_or(ErrorKind::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string().ok_or(ErrorKind::Delimiter)?)?;
                    self.stack()
                        .push(Instruction::Event(Event::Content(Content::Operator(op!(
                            delimiter
                        )))));
                }
                Event::BeginGroup
            }
            "middle" => {
                let delimiter = lex::delimiter(self.current_string().ok_or(ErrorKind::Delimiter)?)?;
                Event::Content(Content::Operator(op!(delimiter)))
            }
            "right" => {
                let curr_str = self.current_string().ok_or(ErrorKind::Delimiter)?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    Event::EndGroup
                } else {
                    let delimiter =
                        lex::delimiter(self.current_string().ok_or(ErrorKind::Delimiter)?)?;
                    self.stack().push(Instruction::Event(Event::EndGroup));
                    Event::Content(Content::Operator(op!(delimiter)))
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
            "~" | "nobreakspace" => Event::Content(Content::Text("&nbsp;")),
            // Variable spacing
            "kern" => {
                let dimension = lex::dimension(self.current_string().ok_or(ErrorKind::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "hskip" => {
                let glue = lex::glue(self.current_string().ok_or(ErrorKind::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "mkern" => {
                let dimension =
                    lex::math_dimension(self.current_string().ok_or(ErrorKind::Dimension)?)?;
                Event::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "mskip" => {
                let glue = lex::math_glue(self.current_string().ok_or(ErrorKind::Glue)?)?;
                Event::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "hspace" => {
                let Argument::Group(mut argument) =
                    lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?
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
                let first_arg = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
                self.handle_argument(first_arg)?;
                let second_arg = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
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

            // Spacing
            c if c.trim_start().is_empty() => Event::Content(Content::Text("&nbsp;")),

            _ => return Err(ErrorKind::UnknownPrimitive),
        };
        self.stack().push(Instruction::Event(event));
        Ok(())
    }

    /// Handle a control sequence that outputs more than one event.
    fn multi_event<const N: usize>(&mut self, events: [Event<'a>; N]) -> InnerResult<()> {
        self.buffer.push(Instruction::Event(Event::BeginGroup));
        self.buffer
            .extend(events.iter().map(|event| Instruction::Event(*event)));
        self.buffer.push(Instruction::Event(Event::EndGroup));
        Ok(())
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn em_sized_delim(&mut self, size: f32) -> InnerResult<()> {
        let delimiter = lex::delimiter(self.current_string().ok_or(ErrorKind::Delimiter)?)?;
        self.buffer
            .push(Instruction::Event(Event::Content(Content::Operator(
                op!(delimiter, {size: Some((size, DimensionUnit::Em))}),
            ))));
        Ok(())
    }

    /// Override the `font_state` for the argument to the command.
    fn font_group(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
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
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
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
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
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

// Fonts are handled by the renderer using groups.
// In font group, a group is opened, the font state is set, and the argument is parsed.
// In frac, always use groups for safety.
// In accent, always use groups for safety.
// Everywhere, we can't go wrong using groups.
//
//
// Expanded macros are owned strings, and to fetch the context of an error, we use the previous
// string in the stack. INVARIANT: an expanded macro must always have a source that is its neigbour
// in the stack. That is because macro expansion does not output anything other than the expanded
// macro to the top of the stack. Example: [... (Other stuff), &'a str (source), String (macro), String (macro)]
//
//
// Comments must be checked when parsing an argument, but are left in the string in order to have a
// continuous string.

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
