//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use crate::{
    attribute::{DimensionUnit, Font},
    event::{Content, Event, Identifier, Operator, Visual},
};

use super::{
    lex,
    operator_table::{is_delimiter, is_operator},
    Argument, ErrorKind, GroupType, InnerResult, Instruction, Parser,
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

// TODO: have an handler for multi-event primitives, because they must be grouped.

impl<'a> Parser<'a> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` or `%` character is given
    pub(crate) fn handle_char_token(
        &mut self,
        token: char,
    ) -> InnerResult<()> {
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
    pub(crate) fn handle_primitive(
        &mut self,
        control_sequence: &'a str,
    ) -> InnerResult<()> {
        let event = match control_sequence {
            "arccos" | "cos" | "csc" | "exp" | "ker" | "sinh" | "arcsin" | "cosh" | "deg"
            | "lg" | "ln" | "arctan" | "cot" | "det" | "hom" | "log" | "sec" | "tan" | "arg"
            | "coth" | "dim" | "sin" | "tanh" => Event::Content(Content::Identifier(
                Identifier::Str(control_sequence),
            )),
            // TODO: The following have `under` subscripts in display math: Pr sup liminf max inf gcd limsup min

            /////////////////////////
            // Non-Latin Alphabets //
            /////////////////////////
            // Lowercase Greek letters
            "alpha" => self.ident('α'),
            "beta" => self.ident('β'),
            "gamma" => self.ident('γ'),
            "delta" => self.ident('δ'),
            "epsilon" => self.ident('ϵ'),
            "varepsilon" => self.ident('ε'),
            "zeta" => self.ident('ζ'),
            "eta" => self.ident('η'),
            "theta" => self.ident('θ'),
            "vartheta" => self.ident('ϑ'),
            "iota" => self.ident('ι'),
            "kappa" => self.ident('κ'),
            "lambda" => self.ident('λ'),
            "mu" => self.ident('µ'),
            "nu" => self.ident('ν'),
            "xi" => self.ident('ξ'),
            "pi" => self.ident('π'),
            "varpi" => self.ident('ϖ'),
            "rho" => self.ident('ρ'),
            "varrho" => self.ident('ϱ'),
            "sigma" => self.ident('σ'),
            "varsigma" => self.ident('ς'),
            "tau" => self.ident('τ'),
            "upsilon" => self.ident('υ'),
            "phi" => self.ident('φ'),
            "varphi" => self.ident('ϕ'),
            "chi" => self.ident('χ'),
            "psi" => self.ident('ψ'),
            "omega" => self.ident('ω'),
            // Uppercase Greek letters
            "Alpha" => self.ident('Α'),
            "Beta" => self.ident('Β'),
            "Gamma" => self.ident('Γ'),
            "Delta" => self.ident('Δ'),
            "Epsilon" => self.ident('Ε'),
            "Zeta" => self.ident('Ζ'),
            "Eta" => self.ident('Η'),
            "Theta" => self.ident('Θ'),
            "Iota" => self.ident('Ι'),
            "Kappa" => self.ident('Κ'),
            "Lambda" => self.ident('Λ'),
            "Mu" => self.ident('Μ'),
            "Nu" => self.ident('Ν'),
            "Xi" => self.ident('Ξ'),
            "Pi" => self.ident('Π'),
            "Rho" => self.ident('Ρ'),
            "Sigma" => self.ident('Σ'),
            "Tau" => self.ident('Τ'),
            "Upsilon" => self.ident('Υ'),
            "Phi" => self.ident('Φ'),
            "Chi" => self.ident('Χ'),
            "Psi" => self.ident('Ψ'),
            "Omega" => self.ident('Ω'),
            // Hebrew letters
            "aleph" => self.ident('ℵ'),
            "beth" => self.ident('ℶ'),
            "gimel" => self.ident('ℷ'),
            "daleth" => self.ident('ℸ'),
            // Other symbols
            "eth" => self.ident('ð'),
            "ell" => self.ident('ℓ'),
            "nabla" => self.ident('∇'),
            "partial" => self.ident('⅁'),
            "Finv" => self.ident('Ⅎ'),
            "Game" => self.ident('ℷ'),
            "hbar" | "hslash" => self.ident('ℏ'),
            "imath" => self.ident('ı'),
            "jmath" => self.ident('ȷ'),
            "Im" => self.ident('ℑ'),
            "Re" => self.ident('ℜ'),
            "wp" => self.ident('℘'),
            "Bbbk" => self.ident('𝕜'),
            "Angstrom" => self.ident('Å'),
            "backepsilon" => self.ident('϶'),

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
            "mathbf" | "symbf" | "mathbfup" | "symbfup" => {
                self.font_group(Some(Font::Bold))?
            }
            "mathcal" | "symcal" | "mathup" | "symup" => {
                self.font_group(Some(Font::Script))?
            }
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
            "mathbfsfit" | "symbfsfit" => {
                self.font_group(Some(Font::SansSerifBoldItalic))?
            }
            "mathnormal" | "symnormal" => self.font_group(None)?,

            //////////////////
            // Miscellanous //
            //////////////////
            "#" | "%" | "&" | "$" | "_" => self.ident(
                control_sequence
                    .chars()
                    .next()
                    .expect("the control sequence contains one of the matched characters"),
            ),
            "|" => self.operator(op!('∥', {stretchy: Some(false)})),

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
                    self.stack().push(Instruction::Event(Event::Content(Content::Operator(op!(
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
            "sum" => self.operator(op!('∑')),
            "prod" => self.operator(op!('∏')),
            "coprod" => self.operator(op!('∐')),
            "int" => self.operator(op!('∫')),
            "iint" => self.operator(op!('∬')),
            "intop" => self.operator(op!('∫')),
            "iiint" => self.operator(op!('∭')),
            "smallint" => self.operator(op!('∫')),
            "iiiint" => self.operator(op!('⨌')),
            "intcap" => self.operator(op!('⨙')),
            "intcup" => self.operator(op!('⨚')),
            "oint" => self.operator(op!('∮')),
            "varointclockwise" => self.operator(op!('∲')),
            "intclockwise" => self.operator(op!('∱')),
            "oiint" => self.operator(op!('∯')),
            "pointint" => self.operator(op!('⨕')),
            "rppolint" => self.operator(op!('⨒')),
            "scpolint" => self.operator(op!('⨓')),
            "oiiint" => self.operator(op!('∰')),
            "intlarhk" => self.operator(op!('⨗')),
            "sqint" => self.operator(op!('⨖')),
            "intx" => self.operator(op!('⨘')),
            "intbar" => self.operator(op!('⨍')),
            "intBar" => self.operator(op!('⨎')),
            "fint" => self.operator(op!('⨏')),
            "bigoplus" => self.operator(op!('⨁')),
            "bigotimes" => self.operator(op!('⨂')),
            "bigvee" => self.operator(op!('⋁')),
            "bigwedge" => self.operator(op!('⋀')),
            "bigodot" => self.operator(op!('⨀')),
            "bigcap" => self.operator(op!('⋂')),
            "biguplus" => self.operator(op!('⨄')),
            "bigcup" => self.operator(op!('⋃')),
            "bigsqcup" => self.operator(op!('⨆')),
            "bigsqcap" => self.operator(op!('⨅')),
            "bigtimes" => self.operator(op!('⨉')),

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
            "prime" => self.operator(op!('′')),
            "dprime" => self.operator(op!('″')),
            "trprime" => self.operator(op!('‴')),
            "qprime" => self.operator(op!('⁗')),
            "backprime" => self.operator(op!('‵')),
            "backdprime" => self.operator(op!('‶')),
            "backtrprime" => self.operator(op!('‷')),

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
            "forall" => self.operator(op!('∀')),
            "complement" => self.operator(op!('∁')),
            "therefore" => self.operator(op!('∴')),
            "emptyset" => self.operator(op!('∅')),
            "exists" => self.operator(op!('∃')),
            "subset" => self.operator(op!('⊂')),
            "because" => self.operator(op!('∵')),
            "varnothing" => self.operator(op!('⌀')),
            "nexists" => self.operator(op!('∄')),
            "supset" => self.operator(op!('⊃')),
            "mapsto" => self.operator(op!('↦')),
            "implies" => self.operator(op!('⟹')),
            "in" => self.operator(op!('∈')),
            "mid" => self.operator(op!('∣')),
            "to" => self.operator(op!('→')),
            "impliedby" => self.operator(op!('⟸')),
            "ni" => self.operator(op!('∋')),
            "land" => self.operator(op!('∧')),
            "gets" => self.operator(op!('←')),
            "iff" => self.operator(op!('⟺')),
            "notni" => self.operator(op!('∌')),
            "neg" | "lnot" => self.operator(op!('¬')),
            "strictif" => self.operator(op!('⥽')),
            "strictfi" => self.operator(op!('⥼')),

            //////////////////////
            // Binary Operators //
            //////////////////////
            "ldotp" => self.operator(op!('.')),
            "cdotp" => self.operator(op!('·')),
            "cdot" => self.operator(op!('⋅')),
            "centerdot" => self.operator(op!('·')),
            "circ" => self.operator(op!('∘')),
            "circledast" => self.operator(op!('⊛')),
            "circledcirc" => self.operator(op!('⊚')),
            "circleddash" => self.operator(op!('⊝')),
            "bigcirc" => self.operator(op!('◯')),
            "leftthreetimes" => self.operator(op!('⋋')),
            "rhd" => self.operator(op!('⊳')),
            "lhd" => self.operator(op!('⊲')),
            "leftouterjoin" => self.operator(op!('⟕')),
            "rightouterjoin" => self.operator(op!('⟖')),
            "rightthreetimes" => self.operator(op!('⋌')),
            "rtimes" => self.operator(op!('⋊')),
            "ltimes" => self.operator(op!('⋉')),
            "leftmodels" => self.operator(op!('⊨')),
            "amalg" => self.operator(op!('⨿')),
            "ast" => self.operator(op!('*')),
            "asymp" => self.operator(op!('≍')),
            "And" => self.operator(op!('&')),
            "lor" => self.operator(op!('∨')),
            "setminus" => self.operator(op!('∖')),
            "Cup" => self.operator(op!('⋓')),
            "cup" => self.operator(op!('∪')),
            "sqcup" => self.operator(op!('⊔')),
            "sqcap" => self.operator(op!('⊓')),
            "lessdot" => self.operator(op!('⋖')),
            "smallsetminus" => self.operator(op!('∖', {size: Some((0.7, DimensionUnit::Em))})),
            "barwedge" => self.operator(op!('⌅')),
            "curlyvee" => self.operator(op!('⋎')),
            "curlywedge" => self.operator(op!('⋏')),
            "sslash" => self.operator(op!('⫽')),
            "bowtie" | "Join" => self.operator(op!('⋈')),
            "div" => self.operator(op!('÷')),
            "mp" => self.operator(op!('∓')),
            "times" => self.operator(op!('×')),
            "boxdot" => self.operator(op!('⊡')),
            "divideontimes" => self.operator(op!('⋇')),
            "odot" => self.operator(op!('⊙')),
            "unlhd" => self.operator(op!('⊴')),
            "boxminus" => self.operator(op!('⊟')),
            "dotplus" => self.operator(op!('∔')),
            "ominus" => self.operator(op!('⊖')),
            "unrhd" => self.operator(op!('⊵')),
            "boxplus" => self.operator(op!('⊞')),
            "doublebarwedge" => self.operator(op!('⩞')),
            "oplus" => self.operator(op!('⊕')),
            "uplus" => self.operator(op!('⊎')),
            "boxtimes" => self.operator(op!('⊠')),
            "doublecap" => self.operator(op!('⋒')),
            "otimes" => self.operator(op!('⊗')),
            "vee" => self.operator(op!('∨')),
            "veebar" => self.operator(op!('⊻')),
            "Cap" => self.operator(op!('⋒')),
            "fullouterjoin" => self.operator(op!('⟗')),
            "parr" => self.operator(op!('⅋')),
            "wedge" => self.operator(op!('∧')),
            "cap" => self.operator(op!('∩')),
            "gtrdot" => self.operator(op!('⋗')),
            "pm" => self.operator(op!('±')),
            "with" => self.operator(op!('&')),
            "intercal" => self.operator(op!('⊺')),
            "wr" => self.operator(op!('≀')),
            ///////////////
            // Fractions //
            ///////////////
            "frac" => {
                todo!()
            }

            "angle" => self.ident('∠'),
            "approx" => self.operator(op!('≈')),
            "approxeq" => self.operator(op!('≊')),
            "approxcolon" => {
                self.stack().extend([
                    Instruction::Event(Event::EndGroup),
                    Instruction::Event(Event::Content(Content::Operator(op! {
                        ':',
                        {left_space: Some((0., DimensionUnit::Em))}
                    }))),
                    Instruction::Event(Event::Content(Content::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    }))),
                ]);
                Event::BeginGroup
            }
            "approxcoloncolon" => {
                self.stack().extend([
                    Instruction::Event(Event::EndGroup),
                    Instruction::Event(Event::Content(Content::Operator(
                        op! {':', {left_space: Some((0., DimensionUnit::Em))}},
                    ))),
                    Instruction::Event(Event::Content(Content::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    }))),
                    Instruction::Event(Event::Content(Content::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    }))),
                ]);
                Event::BeginGroup
            }
            "backsim" => self.operator(op!('∽')),
            "backsimeq" => self.operator(op!('⋍')),
            "backslash" => self.ident('\\'),
            "between" => self.operator(op!('≬')),

            // Spacing
            c if c.trim_start().is_empty() => Event::Content(Content::Text("&nbsp;")),

            _ => return Err(ErrorKind::UnknownPrimitive),
        };
        self.stack().push(Instruction::Event(event));
        Ok(())
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn em_sized_delim(&mut self, size: f32) -> InnerResult<Event<'a>> {
        let delimiter = lex::delimiter(self.current_string().ok_or(ErrorKind::Delimiter)?)?;
        Ok(Event::Content(Content::Operator(
            op!(delimiter, {size: Some((size, DimensionUnit::Em))}),
        )))
    }

    /// Override the `font_state` to the given font variant, and return the next event.
    fn font_override(&mut self, font: Font) -> Event<'a> {
        Event::FontChange(Some(font))
    }

    /// Override the `font_state` for the argument to the command.
    fn font_group(
        &mut self,
        font: Option<Font>,
    ) -> InnerResult<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
        self.handle_argument(argument)?;
        // Kind of silly, we could inline `handle_argument` here and not push the
        // BeginGroup
        let stack = self.stack();
        stack.pop();
        stack.extend([Instruction::Event(Event::FontChange(font))]);
        Ok(Event::BeginGroup)
    }

    /// Accent commands. parse the argument, and overset the accent.
    fn accent(
        &mut self,
        accent: Operator,
    ) -> InnerResult<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
        self.stack().push(Instruction::Event(Event::Content(Content::Operator(
            accent,
        ))));
        self.handle_argument(argument)?;
        Ok(Event::Visual(Visual::Overscript))
    }

    /// Underscript commands. parse the argument, and underset the accent.
    fn underscript(
        &mut self,
        content: Operator,
    ) -> InnerResult<Event<'a>> {
        let argument = lex::argument(self.current_string().ok_or(ErrorKind::Argument)?)?;
        self.stack().push(Instruction::Event(Event::Content(Content::Operator(
            content,
        ))));

        self.handle_argument(argument)?;
        Ok(Event::Visual(Visual::Underscript))
    }

    fn ident(&mut self, ident: char) -> Event<'a> {
        Event::Content(Content::Identifier(Identifier::Char(ident)))
    }

    fn operator(&mut self, operator: Operator) -> Event<'a> {
        Event::Content(Content::Operator(operator))
    }
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
