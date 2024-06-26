//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use core::panic;

use crate::{
    attribute::{DimensionUnit, Font},
    event::{
       Grouping as G,
       Operator as O,
       Event as E,
       Style as S,
       Visual as V,
       Content as C,
       Identifier as ID,
       ScriptType as ST,
       ScriptPosition as SP, 
       StateChange as SC,
       ColorTarget as CT,
       ColorChange as CC,
       Grouping,
    },
};

use super::{
    lex,
    tables::{control_sequence_delimiter_map, is_char_delimiter, is_operator, is_primitive_color, token_to_delim},
    Argument, CharToken, ErrorKind, InnerResult, Instruction as I, Parser, Token,
};

/// Return an `Operator` event with the given content and default modifiers.
macro_rules! op {
    ($content:expr) => {
        O {
            content: $content,
            ..Default::default()
        }
    };
    ($content:expr, {$($field:ident: $value:expr),*}) => {
        O {
            content: $content,
            $($field: $value,)*
            ..Default::default()
        }
    };
}

impl<'a> Parser<'a> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` or `%` character is given
    pub(super) fn handle_char_token(&mut self, token: CharToken<'a>) -> InnerResult<()> {
        let instruction = I::Event(match token.into() {
            '\\' => panic!("(internal error: please report) the `\\` character should never be observed as a token"),
            '%' => panic!("(internal error: please report) the `%` character should never be observed as a token"),
            '_' => {
                let script = E::Script {
                    ty: self.rhs_suffixes(true)?,
                    position: SP::Right,
                };
                self.buffer.extend([
                    I::Event(script),
                    I::Event(E::Begin(G::Internal)),
                ]);
                self.state.skip_suffixes = true;
                E::End
            }
            '^' => {
                let script = E::Script {
                    ty: self.rhs_suffixes(false)?,
                    position: SP::Right,
                };
                self.buffer.extend([
                    I::Event(script),
                    I::Event(E::Begin(G::Internal)),
                ]);
                self.state.skip_suffixes = true;
                E::End
            }
            '$' => return Err(ErrorKind::MathShift),
            '#' => return Err(ErrorKind::HashSign),
            '&' if self.allows_alignment() => E::Alignment,
            '{' => {
                let str = self.current_string();
                let group = lex::group_content(str, "{", "}")?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::SubGroup { content: group, allows_alignment: false },
                    I::Event(E::End)
                ]);
                return Ok(())
            },
            '}' => {
                return Err(ErrorKind::UnbalancedGroup(None))
            },
            '\'' => E::Content(C::Operator(op!('′'))),

            c if is_char_delimiter(c) => E::Content(C::Operator(op!(c, {stretchy: Some(false)}))),
            c if is_operator(c) => E::Content(C::Operator(op!(c))),
            
            '0'..='9' => {
                let content = token.as_str();
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
                *self.current_string() = rest;
                self.buffer
                    .push(I::Event(E::Content(C::Number(number))));
                return Ok(())
            }
            c => ident(c),
        });
        self.buffer.push(instruction);
        Ok(())
    }

    /// Handle a supported control sequence, pushing instructions to the provided stack.
    pub(super) fn handle_primitive(&mut self, control_sequence: &'a str) -> InnerResult<()> {
        let event = match control_sequence {
            "arccos" | "cos" | "csc" | "exp" | "ker" | "sinh" | "arcsin" | "cosh" | "deg"
            | "lg" | "ln" | "arctan" | "cot" | "det" | "hom" | "log" | "sec" | "tan" | "arg"
            | "coth" | "dim" | "sin" | "tanh" | "sgn" => {
                E::Content(C::Identifier(ID::Str(control_sequence)))
            }
            "lim" | "Pr" | "sup" | "liminf" | "max" | "inf" | "gcd" | "limsup" | "min" => {
                self.state.allow_suffix_modifiers = true;
                self.state.above_below_suffix_default = true;
                E::Content(C::Identifier(ID::Str(control_sequence)))
            }
            "operatorname" => {
                self.state.allow_suffix_modifiers = true;
                let argument = lex::argument(self.current_string())?;
                match argument {
                    Argument::Token(Token::ControlSequence(_)) => {
                        return Err(ErrorKind::ControlSequenceAsArgument)
                    }
                    Argument::Token(Token::Character(char_)) => {
                        E::Content(C::Identifier(ID::Str(char_.as_str())))
                    }
                    Argument::Group(content) => {
                        E::Content(C::Identifier(ID::Str(content)))
                    }
                }
            }
            "bmod" => E::Content(C::Identifier(ID::Str("mod"))),
            "pmod" => {
                let argument = lex::argument(self.current_string())?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Internal)),
                    I::Event(operator(op!('('))),
                ]);
                self.handle_argument(argument)?;
                self.buffer.extend([
                    I::Event(operator(op!(')'))),
                    I::Event(E::End),
                ]);
                return Ok(());
            }

            // TODO: Operators with '*', for operatorname* and friends

            /////////////////////////
            // Non-Latin Alphabets //
            /////////////////////////
            // Lowercase Greek letters
            "alpha" => ident('α'),
            "beta" => ident('β'),
            "gamma" => ident('γ'),
            "delta" => ident('δ'),
            "epsilon" => ident('ϵ'),
            "zeta" => ident('ζ'),
            "eta" => ident('η'),
            "theta" => ident('θ'),
            "iota" => ident('ι'),
            "kappa" => ident('κ'),
            "lambda" => ident('λ'),
            "mu" => ident('µ'),
            "nu" => ident('ν'),
            "xi" => ident('ξ'),
            "pi" => ident('π'),
            "rho" => ident('ρ'),
            "sigma" => ident('σ'),
            "tau" => ident('τ'),
            "upsilon" => ident('υ'),
            "phi" => ident('φ'),
            "chi" => ident('χ'),
            "psi" => ident('ψ'),
            "omega" => ident('ω'),
            "omicron" => ident('ο'),
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
            "Omicron" => ident('Ο'),
            // Lowercase Greek Variants
            "varepsilon" => ident('ε'),
            "vartheta" => ident('ϑ'),
            "varkappa" => ident('ϰ'),
            "varrho" => ident('ϱ'),
            "varsigma" => ident('ς'),
            "varpi" => ident('ϖ'),
            "varphi" => ident('ϕ'),
            // Uppercase Greek Variants
            "varGamma" => ident('𝛤'),
            "varDelta" => ident('𝛥'),
            "varTheta" => ident('𝛩'),
            "varLambda" => ident('𝛬'),
            "varXi" => ident('𝛯'),
            "varPi" => ident('𝛱'),
            "varSigma" => ident('𝛴'),
            "varUpsilon" => ident('𝛶'),
            "varPhi" => ident('𝛷'),
            "varPsi" => ident('𝛹'),
            "varOmega" => ident('𝛺'),

            // Hebrew letters
            "aleph" => ident('ℵ'),
            "beth" => ident('ℶ'),
            "gimel" => ident('ℷ'),
            "daleth" => ident('ℸ'),
            // Other symbols
            "digamma" => ident('ϝ'),
            "eth" => ident('ð'),
            "ell" => ident('ℓ'),
            "nabla" => ident('∇'),
            "partial" => ident('∂'),
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

            ///////////////////////////
            // Symbols & Punctuation //
            ///////////////////////////
            "dots" => if self.current_string().trim_start().starts_with(['.', ',']) {
                operator(op!('…'))
            } else {
                operator(op!('⋯'))
            }
            "ldots" | "dotso" | "dotsc" => operator(op!('…')),
            "cdots" | "dotsi" | "dotsm" | "dotsb" | "idotsin" => operator(op!('⋯')),
            "ddots" => operator(op!('⋱')),
            "iddots" => operator(op!('⋰')),
            "vdots" => operator(op!('⋮')),
            "mathellipsis" => operator(op!('…')),
            "infty" => ident('∞'),
            "checkmark" => ident('✓'),
            "ballotx" => ident('✗'),
            "dagger" | "dag" => ident('†'),
            "ddagger" | "ddag" => ident('‡'),
            "angle" => ident('∠'),
            "measuredangle" => ident('∡'),
            "lq" => ident('‘'),
            "Box" => ident('□'),
            "sphericalangle" => ident('∢'),
            "square" => ident('□'),
            "top" => ident('⊤'),
            "rq" => ident('′'),
            "blacksquare" => ident('■'),
            "bot" => ident('⊥'),
            "triangledown" => ident('▽'),
            "Bot" => ident('⫫'),
            "triangleleft" => ident('◃'),
            "triangleright" => ident('▹'),
            "cent" => ident('¢'),
            "colon" | "ratio" | "vcentcolon" => ident(':'),
            "bigtriangledown" => ident('▽'),
            "pounds" | "mathsterling" => ident('£'),
            "bigtriangleup" => ident('△'),
            "blacktriangle" => ident('▲'),
            "blacktriangledown" => ident('▼'),
            "yen" => ident('¥'),
            "blacktriangleleft" => ident('◀'),
            "euro" => ident('€'),
            "blacktriangleright" => ident('▶'),
            "Diamond" => ident('◊'),
            "degree" => ident('°'),
            "lozenge" => ident('◊'),
            "blacklozenge" => ident('⧫'),
            "mho" => ident('℧'),
            "bigstar" => ident('★'),
            "diagdown" => ident('╲'),
            "maltese" => ident('✠'),
            "diagup" => ident('╱'),
            "P" => ident('¶'),
            "clubsuit" => ident('♣'),
            "varclubsuit" => ident('♧'),
            "S" => ident('§'),
            "diamondsuit" => ident('♢'),
            "vardiamondsuit" => ident('♦'),
            "copyright" => ident('©'),
            "heartsuit" => ident('♡'),
            "varheartsuit" => ident('♥'),
            "circledR" => ident('®'),
            "spadesuit" => ident('♠'),
            "varspadesuit" => ident('♤'),
            "circledS" => ident('Ⓢ'),
            "female" => ident('♀'),
            "male" => ident('♂'),
            "astrosun" => ident('☉'),
            "sun" => ident('☼'),
            "leftmoon" => ident('☾'),
            "rightmoon" => ident('☽'),
            "smiley" => ident('☺'),
            "Earth" => ident('⊕'),
            "flat" => ident('♭'),
            "standardstate" => ident('⦵'),
            "natural" => ident('♮'),
            "sharp" => ident('♯'),
            "permil" => ident('‰'),
            "QED" => ident('∎'),
            "lightning" => ident('↯'),
            "diameter" => ident('⌀'),

            ////////////////////////
            // Font state changes //
            ////////////////////////
            // LaTeX native absolute font changes (old behavior a.k.a NFSS 1)
            "bf" => self.font_change(Font::Bold),
            "cal" => self.font_change(Font::Script),
            "it" => self.font_change(Font::Italic),
            "rm" => self.font_change(Font::UpRight),
            "sf" => self.font_change(Font::SansSerif),
            "tt" => self.font_change(Font::Monospace),
            // amsfonts font changes (old behavior a.k.a NFSS 1)
            // unicode-math font changes (old behavior a.k.a NFSS 1)
            // changes, as described in https://mirror.csclub.uwaterloo.ca/CTAN/macros/unicodetex/latex/unicode-math/unicode-math.pdf
            // (section. 3.1)
            "mathbf" | "symbf" | "mathbfup" | "symbfup" | "boldsymbol" => {
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

            ////////////////////////
            // Style state change //
            ////////////////////////
            "displaystyle" => self.style_change(S::Display),
            "textstyle" => self.style_change(S::Text),
            "scriptstyle" => self.style_change(S::Script),
            "scriptscriptstyle" => self.style_change(S::ScriptScript),

            ////////////////////////
            // Color state change //
            ////////////////////////
            "color" => {
                let Argument::Group(color) =
                    lex::argument(self.current_string())?
                else {
                    return Err(ErrorKind::Argument);
                };
                self.state.skip_suffixes = true;
                
                if !is_primitive_color(color) {
                    return Err(ErrorKind::UnknownColor);
                }
                E::StateChange(SC::Color(CC {
                    color,
                    target: CT::Text,
                }))
            },
            "textcolor" => {
                let str = self.current_string();
                let Argument::Group(color) =
                    lex::argument(str)?
                else {
                    return Err(ErrorKind::Argument);
                };
                
                if !is_primitive_color(color) {
                    return Err(ErrorKind::UnknownColor);
                }
                let modified = lex::argument(str)?;

                self.buffer.extend([I::Event(E::Begin(G::Normal)), I::Event(E::StateChange(SC::Color(CC {
                    color,
                    target: CT::Text,
                })))]);
                self.handle_argument(modified)?;
                E::End
            }
            "colorbox" => {
                let Argument::Group(color) =
                    lex::argument(self.current_string())?
                else {
                    return Err(ErrorKind::Argument);
                };
                if !is_primitive_color(color) {
                    return Err(ErrorKind::UnknownColor);
                }
                self.buffer.extend([I::Event(E::Begin(G::Normal)), I::Event(E::StateChange(SC::Color(CC {
                    color,
                    target: CT::Background,
                })))]);
                self.text_argument()?;
                E::End
            }
            "fcolorbox" => {
                let str = self.current_string();
                let Argument::Group(frame_color) =
                    lex::argument(str)?
                else {
                    return Err(ErrorKind::Argument);
                };
                let Argument::Group(background_color) =
                    lex::argument(str)?
                else {
                    return Err(ErrorKind::Argument);
                };
                if !is_primitive_color(frame_color) || !is_primitive_color(background_color) {
                    return Err(ErrorKind::UnknownColor);
                }
                self.buffer.extend([I::Event(E::Begin(G::Normal)), I::Event(E::StateChange(SC::Color(CC {
                    color: frame_color,
                    target: CT::Text,
                }))), I::Event(E::StateChange(SC::Color(CC {
                    color: background_color,
                    target: CT::Background,
                })))]);
                self.text_argument()?;
                E::End
            },

            ///////////////////////////////
            // Delimiters size modifiers //
            ///////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => return self.em_sized_delim(1.2),
            "Big" | "Bigl" | "Bigr" | "Bigm" => return self.em_sized_delim(1.8),
            "bigg" | "biggl" | "biggr" | "biggm" => return self.em_sized_delim(2.4),
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => return self.em_sized_delim(3.0),

            "left" => {
                let curr_str = self.current_string();
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    self.buffer.push(I::Event(E::Begin(G::LeftRight)));
                } else {
                    let delimiter =
                        lex::delimiter(curr_str)?;
                    self.buffer.extend([
                        I::Event(E::Begin(G::LeftRight)),
                        I::Event(E::Content(C::Operator(op!(delimiter)))),
                    ]);
                }

                let curr_str = self.current_string();
                let group_content = lex::group_content(curr_str, r"\left", r"\right")?;
                let delim = if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    None
                } else {
                    let delimiter =
                        lex::delimiter(curr_str)?;
                    Some(E::Content(C::Operator(op!(delimiter))))
                };

                self.buffer.push(I::SubGroup { content: group_content, allows_alignment: false });
                if let Some(delim) = delim {
                    self.buffer.push(I::Event(delim));
                }
                self.buffer.push(I::Event(E::End));

                return Ok(());
            }
            "middle" => {
                let delimiter = lex::delimiter(self.current_string())?;
                operator(op!(delimiter))
            }
            "right" => {
                return Err(ErrorKind::UnbalancedGroup(None));
            }

            ///////////////////
            // Big Operators //
            ///////////////////
            // NOTE: All of the following operators allow limit modifiers.
            // The following operators have above and below limits by default.
            "sum" => self.big_operator(op!('∑', {deny_movable_limits: true}), true),
            "prod" => self.big_operator(op!('∏', {deny_movable_limits: true}), true),
            "coprod" => self.big_operator(op!('∐', {deny_movable_limits: true}), true),
            "bigvee" => self.big_operator(op!('⋁', {deny_movable_limits: true}), true),
            "bigwedge" => self.big_operator(op!('⋀', {deny_movable_limits: true}), true),
            "bigcup" => self.big_operator(op!('⋃', {deny_movable_limits: true}), true),
            "bigcap" => self.big_operator(op!('⋂', {deny_movable_limits: true}), true),
            "biguplus" => self.big_operator(op!('⨄', {deny_movable_limits: true}), true),
            "bigoplus" => self.big_operator(op!('⨁', {deny_movable_limits: true}), true),
            "bigotimes" => self.big_operator(op!('⨂', {deny_movable_limits: true}), true),
            "bigodot" => self.big_operator(op!('⨀', {deny_movable_limits: true}), true),
            "bigsqcup" => self.big_operator(op!('⨆', {deny_movable_limits: true}), true),
            "bigsqcap" => self.big_operator(op!('⨅', {deny_movable_limits: true}), true),
            "bigtimes" => self.big_operator(op!('⨉', {deny_movable_limits: true}), true),
            "intop" => self.big_operator(op!('∫'), true),
            // The following operators do not have above and below limits by default.
            "int" => self.big_operator(op!('∫'), false),
            "iint" => self.big_operator(op!('∬'), false),
            "iiint" => self.big_operator(op!('∭'), false),
            "smallint" => {
                self.big_operator(op!('∫', {size: Some((0.7, DimensionUnit::Em))}), false)
            }
            "iiiint" => self.big_operator(op!('⨌'), false),
            "intcap" => self.big_operator(op!('⨙'), false),
            "intcup" => self.big_operator(op!('⨚'), false),
            "oint" => self.big_operator(op!('∮'), false),
            "varointclockwise" => self.big_operator(op!('∲'), false),
            "intclockwise" => self.big_operator(op!('∱'), false),
            "oiint" => self.big_operator(op!('∯'), false),
            "pointint" => self.big_operator(op!('⨕'), false),
            "rppolint" => self.big_operator(op!('⨒'), false),
            "scpolint" => self.big_operator(op!('⨓'), false),
            "oiiint" => self.big_operator(op!('∰'), false),
            "intlarhk" => self.big_operator(op!('⨗'), false),
            "sqint" => self.big_operator(op!('⨖'), false),
            "intx" => self.big_operator(op!('⨘'), false),
            "intbar" => self.big_operator(op!('⨍'), false),
            "intBar" => self.big_operator(op!('⨎'), false),
            "fint" => self.big_operator(op!('⨏'), false),

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
            "," | "thinspace" => E::Space {
                width: Some((3. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            ">" | ":" | "medspace" => E::Space {
                width: Some((4. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            ";" | "thickspace" => E::Space {
                width: Some((5. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "enspace" => E::Space {
                width: Some((0.5, DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "quad" => E::Space {
                width: Some((1., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "qquad" => E::Space {
                width: Some((2., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "mathstrut" => E::Space {
                width: None,
                height: Some((0.7, DimensionUnit::Em)),
                depth: Some((0.3, DimensionUnit::Em)),
            },
            "~" | "nobreakspace" => E::Content(C::Text("&nbsp;")),
            // Variable spacing
            "kern" => {
                let dimension = lex::dimension(self.current_string())?;
                E::Space {
                    width: Some(dimension),
                    height: None,
                    depth: None,
                }
            }
            "hskip" => {
                let glue = lex::glue(self.current_string())?;
                E::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            "mkern" => {
                let dimension =
                    lex::dimension(self.current_string())?;
                if dimension.1 == DimensionUnit::Mu {
                    E::Space {
                        width: Some(dimension),
                        height: None,
                        depth: None,
                    }
                } else {
                    return Err(ErrorKind::MathUnit);
                }
            }
            "mskip" => {
                let glue = lex::glue(self.current_string())?;
                if glue.0.1 == DimensionUnit::Mu
                    && glue.1.map_or(true, |(_, unit)| unit == DimensionUnit::Mu)
                    && glue.2.map_or(true, |(_, unit)| unit == DimensionUnit::Mu) {
                    E::Space {
                        width: Some(glue.0),
                        height: None,
                        depth: None,
                    }
                } else {
                    return Err(ErrorKind::MathUnit);
                }
            }
            "hspace" => {
                let Argument::Group(mut argument) =
                    lex::argument(self.current_string())?
                else {
                    return Err(ErrorKind::DimensionArgument);
                };
                let glue = lex::glue(&mut argument)?;
                E::Space {
                    width: Some(glue.0),
                    height: None,
                    depth: None,
                }
            }
            // Negative spacing
            "!" | "negthinspace" => E::Space {
                width: Some((-3. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "negmedspace" => E::Space {
                width: Some((-4. / 18., DimensionUnit::Em)),
                height: None,
                depth: None,
            },
            "negthickspace" => E::Space {
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
            "in" | "isin" => operator(op!('∈')),
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
            "bullet" => operator(op!('∙')),
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
            "circledvert" => operator(op!('⦶')),
            "blackhourglass" => operator(op!('⧗')),
            "circlehbar" => operator(op!('⦵')),
            "operp" => operator(op!('⦹')),
            "boxast" => operator(op!('⧆')),
            "concavediamond" => operator(op!('⟡')),
            "boxbox" => operator(op!('⧈')),
            "concavediamondtickleft" => operator(op!('⟢')),
            "oslash" => operator(op!('⊘')),
            "boxcircle" => operator(op!('⧇')),
            "concavediamondtickright" => operator(op!('⟣')),
            "diamond" => operator(op!('⋄')),
            "Otimes" => operator(op!('⨷')),
            "hourglass" => operator(op!('⧖')),
            "otimeshat" => operator(op!('⨶')),
            "triangletimes" => operator(op!('⨻')),
            "lozengeminus" => operator(op!('⟠')),
            "star" => operator(op!('⋆')),
            "obar" => operator(op!('⌽')),
            "triangle" | "vartriangle" => operator(op!('△')),
            "obslash" => operator(op!('⦸')),
            "triangleminus" => operator(op!('⨺')),
            "odiv" => operator(op!('⨸')),
            "triangleplus" => operator(op!('⨹')),
            "circledequal" => operator(op!('⊜')),
            "ogreaterthan" => operator(op!('⧁')),
            "whitesquaretickleft" => operator(op!('⟤')),
            "circledparallel" => operator(op!('⦷')),
            "olessthan" => operator(op!('⧀')),
            "whitesquaretickright" => operator(op!('⟥')),

            ///////////////
            // Relations //
            ///////////////
            "eqcirc" => operator(op!('≖')),
            "lessgtr" => operator(op!('≶')),
            "smile" | "sincoh" => operator(op!('⌣')),
            "eqcolon" | "minuscolon" => operator(op!('∹')),
            "lesssim" => operator(op!('≲')),
            "sqsubset" => operator(op!('⊏')),
            "ll" => operator(op!('≪')),
            "sqsubseteq" => operator(op!('⊑')),
            "eqqcolon" => operator(op!('≕')),
            "lll" => operator(op!('⋘')),
            "sqsupset" => operator(op!('⊐')),
            "llless" => operator(op!('⋘')),
            "sqsupseteq" => operator(op!('⊒')),
            "approx" => operator(op!('≈')),
            "eqdef" => operator(op!('≝')),
            "lt" => operator(op!('<')),
            "stareq" => operator(op!('≛')),
            "approxeq" => operator(op!('≊')),
            "eqsim" => operator(op!('≂')),
            "measeq" => operator(op!('≞')),
            "Subset" => operator(op!('⋐')),
            "arceq" => operator(op!('≘')),
            "eqslantgtr" => operator(op!('⪖')),
            "eqslantless" => operator(op!('⪕')),
            "models" => operator(op!('⊨')),
            "subseteq" => operator(op!('⊆')),
            "backcong" => operator(op!('≌')),
            "equiv" => operator(op!('≡')),
            "multimap" => operator(op!('⊸')),
            "subseteqq" => operator(op!('⫅')),
            "fallingdotseq" => operator(op!('≒')),
            "multimapboth" => operator(op!('⧟')),
            "succ" => operator(op!('≻')),
            "backsim" => operator(op!('∽')),
            "frown" => operator(op!('⌢')),
            "multimapinv" => operator(op!('⟜')),
            "succapprox" => operator(op!('⪸')),
            "backsimeq" => operator(op!('⋍')),
            "ge" => operator(op!('≥')),
            "origof" => operator(op!('⊶')),
            "succcurlyeq" => operator(op!('≽')),
            "between" => operator(op!('≬')),
            "geq" => operator(op!('≥')),
            "owns" => operator(op!('∋')),
            "succeq" => operator(op!('⪰')),
            "bumpeq" => operator(op!('≏')),
            "geqq" => operator(op!('≧')),
            "parallel" => operator(op!('∥')),
            "succsim" => operator(op!('≿')),
            "Bumpeq" => operator(op!('≎')),
            "geqslant" => operator(op!('⩾')),
            "perp" => operator(op!('⟂')),
            "Supset" => operator(op!('⋑')),
            "circeq" => operator(op!('≗')),
            "gg" => operator(op!('≫')),
            "Perp" => operator(op!('⫫')),
            "coh" => operator(op!('⌢')),
            "ggg" => operator(op!('⋙')),
            "pitchfork" => operator(op!('⋔')),
            "supseteq" => operator(op!('⊇')),
            "gggtr" => operator(op!('⋙')),
            "prec" => operator(op!('≺')),
            "supseteqq" => operator(op!('⫆')),
            "gt" => operator(op!('>')),
            "precapprox" => operator(op!('⪷')),
            "thickapprox" => operator(op!('≈')),
            "gtrapprox" => operator(op!('⪆')),
            "preccurlyeq" => operator(op!('≼')),
            "thicksim" => operator(op!('∼')),
            "gtreqless" => operator(op!('⋛')),
            "preceq" => operator(op!('⪯')),
            "trianglelefteq" => operator(op!('⊴')),
            "coloneqq" | "colonequals" => operator(op!('≔')),
            "gtreqqless" => operator(op!('⪌')),
            "precsim" => operator(op!('≾')),
            "triangleq" => operator(op!('≜')),
            "Coloneqq" | "coloncolonequals" => operator(op!('⩴')),
            "gtrless" => operator(op!('≷')),
            "propto" => operator(op!('∝')),
            "trianglerighteq" => operator(op!('⊵')),
            "gtrsim" => operator(op!('≳')),
            "questeq" => operator(op!('≟')),
            "varpropto" => operator(op!('∝')),
            "imageof" => operator(op!('⊷')),
            "cong" => operator(op!('≅')),
            "risingdotseq" => operator(op!('≓')),
            "vartriangleleft" => operator(op!('⊲')),
            "curlyeqprec" => operator(op!('⋞')),
            "scoh" => operator(op!('⌢')),
            "vartriangleright" => operator(op!('⊳')),
            "curlyeqsucc" => operator(op!('⋟')),
            "le" => operator(op!('≤')),
            "shortmid" => operator(op!('∣', {size:Some((0.7, DimensionUnit::Em))})),
            "shortparallel" => operator(op!('∥', {size:Some((0.7, DimensionUnit::Em))})),
            "vdash" => operator(op!('⊢')),
            "dashv" => operator(op!('⊣')),
            "leq" => operator(op!('≤')),
            "vDash" => operator(op!('⊨')),
            "dblcolon" | "coloncolon" => operator(op!('∷')),
            "leqq" => operator(op!('≦')),
            "sim" => operator(op!('∼')),
            "Vdash" => operator(op!('⊩')),
            "doteq" => operator(op!('≐')),
            "leqslant" => operator(op!('⩽')),
            "simeq" => operator(op!('≃')),
            "Dash" => operator(op!('⊫')),
            "Doteq" => operator(op!('≑')),
            "lessapprox" => operator(op!('⪅')),
            "Vvdash" => operator(op!('⊪')),
            "doteqdot" => operator(op!('≑')),
            "lesseqgtr" => operator(op!('⋚')),
            "smallfrown" => operator(op!('⌢')),
            "veeeq" => operator(op!('≚')),
            "eqeq" => operator(op!('⩵')),
            "lesseqqgtr" => operator(op!('⪋')),
            "smallsmile" => operator(op!('⌣', {size:Some((0.7, DimensionUnit::Em))})),
            "wedgeq" => operator(op!('≙')),
            "Eqcolon" | "minuscoloncolon" => {
                self.multi_event([
                    E::Content(C::Operator(
                        op!('−', {right_space: Some((0., DimensionUnit::Em))}),
                    )),
                    E::Content(C::Operator(op!('∷'))),
                ]);
                return Ok(());
            }
            "Eqqcolon" => {
                self.multi_event([
                    E::Content(C::Operator(
                        op!('=', {right_space: Some((0., DimensionUnit::Em))}),
                    )),
                    E::Content(C::Operator(op!('∷'))),
                ]);
                return Ok(());
            }
            "approxcolon" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        ':',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "colonapprox" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        '≈',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "approxcoloncolon" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        '≈',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    })),
                    E::Content(C::Operator(
                        op! {':', {left_space: Some((0., DimensionUnit::Em))}},
                    )),
                ]);
                return Ok(());
            }
            "Colonapprox" | "coloncolonapprox" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    })),
                    E::Content(C::Operator(op! {
                        '≈',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "coloneq" | "colonminus" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        '-',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "Coloneq" | "coloncolonminus" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    })),
                    E::Content(C::Operator(op! {
                        '-',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "colonsim" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        '∼',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            "Colonsim" | "coloncolonsim" => {
                self.multi_event([
                    E::Content(C::Operator(op! {
                        ':',
                        {right_space: Some((0., DimensionUnit::Em))}
                    })),
                    E::Content(C::Operator(op! {
                        ':',
                        {
                            left_space: Some((0., DimensionUnit::Em)),
                            right_space: Some((0., DimensionUnit::Em))
                        }
                    })),
                    E::Content(C::Operator(op! {
                        '∼',
                        {left_space: Some((0., DimensionUnit::Em))}
                    })),
                ]);
                return Ok(());
            }
            // Negated relations
            "gnapprox" => operator(op!('⪊')),
            "ngeqslant" => operator(op!('≱')),
            "nsubset" => operator(op!('⊄')),
            "nVdash" => operator(op!('⊮')),
            "gneq" => operator(op!('⪈')),
            "ngtr" => operator(op!('≯')),
            "nsubseteq" => operator(op!('⊈')),
            "precnapprox" => operator(op!('⪹')),
            "gneqq" => operator(op!('≩')),
            "nleq" => operator(op!('≰')),
            "nsubseteqq" => operator(op!('⊈')),
            "precneqq" => operator(op!('⪵')),
            "gnsim" => operator(op!('⋧')),
            "nleqq" => operator(op!('≰')),
            "nsucc" => operator(op!('⊁')),
            "precnsim" => operator(op!('⋨')),
            "nleqslant" => operator(op!('≰')),
            "nsucceq" => operator(op!('⋡')),
            "subsetneq" => operator(op!('⊊')),
            "lnapprox" => operator(op!('⪉')),
            "nless" => operator(op!('≮')),
            "nsupset" => operator(op!('⊅')),
            "subsetneqq" => operator(op!('⫋')),
            "lneq" => operator(op!('⪇')),
            "nmid" => operator(op!('∤')),
            "nsupseteq" => operator(op!('⊉')),
            "succnapprox" => operator(op!('⪺')),
            "lneqq" => operator(op!('≨')),
            "notin" => operator(op!('∉')),
            "nsupseteqq" => operator(op!('⊉')),
            "succneqq" => operator(op!('⪶')),
            "lnsim" => operator(op!('⋦')),
            "ntriangleleft" => operator(op!('⋪')),
            "succnsim" => operator(op!('⋩')),
            "nparallel" => operator(op!('∦')),
            "ntrianglelefteq" => operator(op!('⋬')),
            "supsetneq" => operator(op!('⊋')),
            "ncong" => operator(op!('≆')),
            "nprec" => operator(op!('⊀')),
            "ntriangleright" => operator(op!('⋫')),
            "supsetneqq" => operator(op!('⫌')),
            "ne" => operator(op!('≠')),
            "npreceq" => operator(op!('⋠')),
            "ntrianglerighteq" => operator(op!('⋭')),
            "neq" => operator(op!('≠')),
            "nshortmid" => operator(op!('∤')),
            "nvdash" => operator(op!('⊬')),
            "ngeq" => operator(op!('≱')),
            "nshortparallel" => operator(op!('∦', {size: Some((0.7, DimensionUnit::Em))})),
            "nvDash" => operator(op!('⊭')),
            "varsupsetneq" => operator(op!('⊋')),
            "ngeqq" => operator(op!('≱')),
            "nsim" => operator(op!('≁')),
            "nVDash" => operator(op!('⊯')),
            "varsupsetneqq" => operator(op!('⫌', {unicode_variant: true})),
            "varsubsetneqq" => operator(op!('⫋', {unicode_variant: true})),
            "varsubsetneq" => operator(op!('⊊', {unicode_variant: true})),
            "gvertneqq" => operator(op!('≩', {unicode_variant: true})),
            "lvertneqq" => operator(op!('≨', {unicode_variant: true})),

            ////////////
            // Arrows //
            ////////////
            "circlearrowleft" => operator(op!('↺')),
            "Leftrightarrow" => operator(op!('⇔')),
            "restriction" => operator(op!('↾')),
            "circlearrowright" => operator(op!('↻')),
            "leftrightarrows" => operator(op!('⇆')),
            "rightarrow" => operator(op!('→')),
            "curvearrowleft" => operator(op!('↶')),
            "leftrightharpoons" => operator(op!('⇋')),
            "Rightarrow" => operator(op!('⇒')),
            "curvearrowright" => operator(op!('↷')),
            "leftrightsquigarrow" => operator(op!('↭')),
            "rightarrowtail" => operator(op!('↣')),
            "dashleftarrow" => operator(op!('⇠')),
            "Lleftarrow" => operator(op!('⇚')),
            "rightharpoondown" => operator(op!('⇁')),
            "dashrightarrow" => operator(op!('⇢')),
            "longleftarrow" => operator(op!('⟵')),
            "rightharpoonup" => operator(op!('⇀')),
            "downarrow" => operator(op!('↓')),
            "Longleftarrow" => operator(op!('⟸')),
            "rightleftarrows" => operator(op!('⇄')),
            "Downarrow" => operator(op!('⇓')),
            "longleftrightarrow" => operator(op!('⟷')),
            "rightleftharpoons" => operator(op!('⇌')),
            "downdownarrows" => operator(op!('⇊')),
            "Longleftrightarrow" => operator(op!('⟺')),
            "rightrightarrows" => operator(op!('⇉')),
            "downharpoonleft" => operator(op!('⇃')),
            "longmapsto" => operator(op!('⟼')),
            "rightsquigarrow" => operator(op!('⇝')),
            "downharpoonright" => operator(op!('⇂')),
            "longrightarrow" => operator(op!('⟶')),
            "Rrightarrow" => operator(op!('⇛')),
            "Longrightarrow" => operator(op!('⟹')),
            "Rsh" => operator(op!('↱')),
            "hookleftarrow" => operator(op!('↩')),
            "looparrowleft" => operator(op!('↫')),
            "searrow" => operator(op!('↘')),
            "hookrightarrow" => operator(op!('↪')),
            "looparrowright" => operator(op!('↬')),
            "swarrow" => operator(op!('↙')),
            "Lsh" => operator(op!('↰')),
            "mapsfrom" => operator(op!('↤')),
            "twoheadleftarrow" => operator(op!('↞')),
            "twoheadrightarrow" => operator(op!('↠')),
            "leadsto" => operator(op!('⇝')),
            "nearrow" => operator(op!('↗')),
            "uparrow" => operator(op!('↑')),
            "leftarrow" => operator(op!('←')),
            "nleftarrow" => operator(op!('↚')),
            "Uparrow" => operator(op!('⇑')),
            "Leftarrow" => operator(op!('⇐')),
            "nLeftarrow" => operator(op!('⇍')),
            "updownarrow" => operator(op!('↕')),
            "leftarrowtail" => operator(op!('↢')),
            "nleftrightarrow" => operator(op!('↮')),
            "Updownarrow" => operator(op!('⇕')),
            "leftharpoondown" => operator(op!('↽')),
            "nLeftrightarrow" => operator(op!('⇎')),
            "upharpoonleft" => operator(op!('↿')),
            "leftharpoonup" => operator(op!('↼')),
            "nrightarrow" => operator(op!('↛')),
            "upharpoonright" => operator(op!('↾')),
            "leftleftarrows" => operator(op!('⇇')),
            "nRightarrow" => operator(op!('⇏')),
            "upuparrows" => operator(op!('⇈')),
            "leftrightarrow" => operator(op!('↔')),
            "nwarrow" => operator(op!('↖')),

            ///////////////
            // Fractions //
            ///////////////
            "frac" => {
                return self.fraction_like(None);
            }
            // TODO: better errors for this
            "genfrac" => {
                let str = self.current_string();
                let ldelim_argument = lex::argument(str)?;
                let ldelim = match ldelim_argument {
                    Argument::Token(token) => Some(token_to_delim(token).ok_or(ErrorKind::Delimiter)?),
                    Argument::Group(group) => if group.is_empty() {
                        None
                    } else {
                        return Err(ErrorKind::Delimiter);
                    },
                };
                let rdelim_argument = lex::argument(str)?;
                let rdelim = match rdelim_argument {
                    Argument::Token(token) => Some(token_to_delim(token).ok_or(ErrorKind::Delimiter)?),
                    Argument::Group(group) => if group.is_empty() {
                        None
                    } else {
                        return Err(ErrorKind::Delimiter);
                    },
                };
                let bar_size_argument = lex::argument(str)?;
                let bar_size = match bar_size_argument {
                    Argument::Token(_) => return Err(ErrorKind::DimensionArgument),
                    Argument::Group("") => None,
                    Argument::Group(mut group) => lex::dimension(&mut group).and_then(|dim| {
                        if group.is_empty() {
                            Ok(Some(dim))
                        } else {
                            Err(ErrorKind::DimensionArgument)
                        }
                    })?,
                };
                let display_style_argument = lex::argument(str)?;
                let display_style = match display_style_argument {
                    Argument::Token(t) => match t {
                            Token::ControlSequence(_) => return Err(ErrorKind::Argument),
                            Token::Character(c) => Some(match c.into() {
                                '0' => S::Display,
                                '1' => S::Text,
                                '2' => S::Script,
                                '3' => S::ScriptScript,
                                _ => return Err(ErrorKind::Argument),
                            }),
                    },
                    Argument::Group(group) => {
                        match group {
                            "0" => Some(S::Display),
                            "1" => Some(S::Text),
                            "2" => Some(S::Script),
                            "3" => Some(S::ScriptScript),
                            "" => None,
                            _ => return Err(ErrorKind::Argument),
                        }
                    }
                };

                self.buffer.push(I::Event(E::Begin(G::LeftRight)));
                if let Some(style) = display_style {
                    self.buffer.push(I::Event(E::StateChange(SC::Style(style))));
                }
                if let Some(ldelim) = ldelim {
                    self.buffer.push(I::Event(E::Content(C::Operator(op!(ldelim)))));
                }
                
                self.fraction_like(bar_size)?;
                
                if let Some(rdelim) = rdelim {
                    self.buffer.push(I::Event(E::Content(C::Operator(op!(rdelim)))));
                }
                self.buffer.push(I::Event(E::End));
                return Ok(())
            }
            "binom" => {
                self.buffer.extend([I::Event(E::Begin(G::LeftRight)),
                                    I::Event(E::Content(C::Operator(op!('('))))]);
                self.fraction_like(None)?;
                self.buffer.extend([I::Event(E::Content(C::Operator(op!(')')))),
                                    I::Event(E::End)]);
                return Ok(())
            }
            "cfrac" => {
                self.buffer.extend([I::Event(E::Begin(G::Internal)),
                                    I::Event(E::StateChange(SC::Style(S::Display)))]);
                self.fraction_like(None)?;
                self.buffer.push(I::Event(E::End));
                return Ok(())
            }
            "tfrac" => {
                self.buffer.extend([I::Event(E::Begin(G::Internal)),
                                    I::Event(E::StateChange(SC::Style(S::Text)))]);
                self.fraction_like(None)?;
                self.buffer.push(I::Event(E::End));
                return Ok(())
            }
            "dfrac" => {
                self.buffer.extend([I::Event(E::Begin(G::Internal)),
                                    I::Event(E::StateChange(SC::Style(S::Script)))]);
                self.fraction_like(None)?;
                self.buffer.push(I::Event(E::End));
                return Ok(())
            }
            "overset" => {
                self.buffer.push(I::Event(E::Script {
                    ty: ST::Superscript,
                    position: SP::AboveBelow,
                }));
                let over = lex::argument(self.current_string())?;
                self.handle_argument(over)?;
                let base = lex::argument(self.current_string())?;
                self.handle_argument(base)?;
                return Ok(());
            }
            "underset" => {
                self.buffer.push(I::Event(E::Script {
                    ty: ST::Subscript,
                    position: SP::AboveBelow,
                }));
                let under = lex::argument(self.current_string())?;
                self.handle_argument(under)?;
                let base = lex::argument(self.current_string())?;
                self.handle_argument(base)?;
                return Ok(());
            }

            //////////////
            // Radicals //
            //////////////
            "sqrt" => {
                if let Some(index) =
                    lex::optional_argument(self.current_string())?
                {
                    self.buffer
                        .push(I::Event(E::Visual(V::Root)));
                    let arg = lex::argument(self.current_string())?;
                    self.handle_argument(arg)?;
                    self.buffer.push(I::SubGroup {
                        content: index,
                        allows_alignment: false,
                    });
                } else {
                    self.buffer
                        .push(I::Event(E::Visual(V::SquareRoot)));
                    let arg = lex::argument(self.current_string())?;
                    self.handle_argument(arg)?;
                }
                return Ok(());
            }
            "surd" => {
                self.multi_event([
                    E::Visual(V::SquareRoot),
                    E::Space {
                        width: Some((0., DimensionUnit::Em)),
                        height: Some((0.7, DimensionUnit::Em)),
                        depth: None,
                    },
                ]);
                return Ok(());
            }

            "backslash" => ident('\\'),

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
            "text" => return self.text_argument(),
            "not" => {
                self.buffer
                    .push(I::Event(E::Visual(V::Negation)));
                let argument = lex::argument(self.current_string())?;
                self.handle_argument(argument)?;
                return Ok(());
            }
            "char" => {
                let number = lex::unsigned_integer(self.current_string())?;
                if number > 255 {
                    return Err(ErrorKind::InvalidCharNumber);
                }
                E::Content(C::Identifier(ID::Char(
                                char::from_u32(number as u32).expect("the number is a valid char since it is less than 256")
                                )))
            },
            "relax" => {
                return if self.state.invalidate_relax {
                    Err(ErrorKind::Relax)
                } else {
                    Ok(())
                }
            }

            "begingroup" => {
                let str = self
                    .current_string();
                let group = lex::group_content(str, "begingroup", "endgroup")?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::SubGroup { content: group, allows_alignment: false },
                    I::Event(E::End),
                ]);
                return Ok(());
            }
            "endgroup" => return Err(ErrorKind::UnbalancedGroup(None)),

            "begin" => {
                let Argument::Group(argument) = lex::argument(self.current_string())? else {
                    return Err(ErrorKind::Argument);
                };
                let mut closing = None;
                let environment = match argument {
                    "array" => Grouping::Array,
                    "matrix" => Grouping::Matrix,
                    "pmatrix" => {
                        self.buffer.extend([
                            I::Event(E::Begin(G::LeftRight)),
                            I::Event(E::Content(C::Operator(op!('(')))),
                        ]);
                        closing = Some(op!(')'));
                        Grouping::Matrix
                    },
                    "bmatrix" => {
                        self.buffer.extend([
                            I::Event(E::Begin(G::LeftRight)),
                            I::Event(E::Content(C::Operator(op!('[')))),
                        ]);
                        closing = Some(op!(']'));
                        Grouping::Matrix
                    },
                    "vmatrix" => {
                        self.buffer.extend([
                            I::Event(E::Begin(G::LeftRight)),
                            I::Event(E::Content(C::Operator(op!('|')))),
                        ]);
                        closing = Some(op!('|'));
                        Grouping::Matrix
                    },
                    "Vmatrix" => {
                        self.buffer.extend([
                            I::Event(E::Begin(G::LeftRight)),
                            I::Event(E::Content(C::Operator(op!('‖')))),
                        ]);
                        closing = Some(op!('‖'));
                        Grouping::Matrix
                    },
                    "Bmatrix" => {
                        self.buffer.extend([
                            I::Event(E::Begin(G::LeftRight)),
                            I::Event(E::Content(C::Operator(op!('{')))),
                        ]);
                        closing = Some(op!('}'));
                        Grouping::Matrix
                    },
                    "cases" => Grouping::Cases,
                    "align" => Grouping::Align,
                    _ => return Err(ErrorKind::Environment),
                };
                // TODO: correctly spot deeper environment of the same type.
                let content = lex::group_content(
                    self.current_string(),
                    &format!(r"\begin{{{argument}}}"),
                    &format!(r"\end{{{argument}}}")
                )?;
                self.buffer.extend([
                    I::Event(E::Begin(environment)),
                    I::SubGroup { content, allows_alignment: true },
                    I::Event(E::End)
                ]);
                if let Some(closing) = closing {
                    self.buffer.extend([
                        I::Event(E::Content(C::Operator(closing))),
                        I::Event(E::End)
                    ]);
                }
                return Ok(());
            }
            "end" => return Err(ErrorKind::UnbalancedGroup(None)),
            "\\" | "cr" if self.allows_alignment() => E::NewLine,

            // Delimiters
            cs if control_sequence_delimiter_map(cs).is_some() => {
                operator(op!(control_sequence_delimiter_map(cs).unwrap(), {stretchy: Some(false)}))
            }

            // Spacing
            c if c.trim_start().is_empty() => E::Content(C::Text("&nbsp;")),

            _ => return Err(ErrorKind::UnknownPrimitive),
        };
        self.buffer.push(I::Event(event));
        Ok(())
    }

    /// Handle a control sequence that outputs more than one event.
    fn multi_event<const N: usize>(&mut self, events: [E<'a>; N]) {
        self.buffer.push(I::Event(E::Begin(G::Internal)));
        self.buffer
            .extend(events.into_iter().map(I::Event));
        self.buffer.push(I::Event(E::End));
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn em_sized_delim(&mut self, size: f32) -> InnerResult<()> {
        let current = self.current_string();
        let delimiter = lex::delimiter(current)?;
        self.buffer
            .push(I::Event(E::Content(C::Operator(
                op!(delimiter, {size: Some((size, DimensionUnit::Em))}),
            ))));
        Ok(())
    }

    /// Override the `font_state` for the argument to the command.
    fn font_group(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(self.current_string())?;
        self.buffer.extend([
            I::Event(E::Begin(G::Internal)),
            I::Event(E::StateChange(SC::Font(font))),
        ]);
        match argument {
            Argument::Token(token) => {
                match token {
                    Token::ControlSequence(cs) => self.handle_primitive(cs)?,
                    Token::Character(c) => self.handle_char_token(c)?,
                };
            }
            Argument::Group(group) => {
                self.buffer.push(I::SubGroup { content: group, allows_alignment: false });
            }
        };
        self.buffer.push(I::Event(E::End));
        Ok(())
    }

    /// Accent commands. parse the argument, and overset the accent.
    fn accent(&mut self, accent: O) -> InnerResult<()> {
        let argument = lex::argument(self.current_string())?;
        self.buffer.push(I::Event(E::Script {
            ty: ST::Superscript,
            position: SP::AboveBelow,
        }));
        self.handle_argument(argument)?;
        self.buffer
            .push(I::Event(E::Content(C::Operator(
                accent,
            ))));
        Ok(())
    }

    /// Underscript commands. parse the argument, and underset the accent.
    fn underscript(&mut self, content: O) -> InnerResult<()> {
        let argument = lex::argument(self.current_string())?;
        self.buffer.push(I::Event(E::Script {
            ty: ST::Subscript,
            position: SP::AboveBelow,
        }));
        self.handle_argument(argument)?;
        self.buffer
            .push(I::Event(E::Content(C::Operator(
                content,
            ))));

        Ok(())
    }

    fn big_operator(&mut self, op: O, above_below: bool) -> E<'a> {
        self.state.allow_suffix_modifiers = true;
        self.state.above_below_suffix_default = above_below;
        operator(op)
    }

    fn font_change(&mut self, font: Font) -> E<'a> {
        self.state.skip_suffixes = true;
        E::StateChange(SC::Font(Some(font)))
    }

    fn style_change(&mut self, style: S) -> E<'a> {
        self.state.skip_suffixes = true;
        E::StateChange(SC::Style(style))
    }

    fn text_argument(&mut self) -> InnerResult<()> {
        let argument = lex::argument(self.current_string())?;
        self.buffer
            .push(I::Event(E::Content(C::Text(
                match argument {
                    Argument::Token(Token::Character(c)) => c.as_str(),
                    Argument::Group(inner) => inner,
                    _ => return Err(ErrorKind::ControlSequenceAsArgument),
                },
            ))));
        Ok(())
    }

    fn fraction_like(&mut self, bar_size: Option<(f32, DimensionUnit)>) -> InnerResult<()> {
        self.buffer.push(I::Event(E::Visual(V::Fraction(bar_size))));
        let numerator = lex::argument(self.current_string())?;
        self.handle_argument(numerator)?;
        let denominator = lex::argument(self.current_string())?;
        self.handle_argument(denominator)?;
        Ok(())
    }
}

#[inline]
fn ident(ident: char) -> E<'static> {
    E::Content(C::Identifier(ID::Char(ident)))
}

#[inline]
fn operator(operator: O) -> E<'static> {
    E::Content(C::Operator(operator))
}

// TODO implementations:
// - `raise`, `lower`
// - `hbox`, `mbox`?
// - `vcenter`
// - `rule`
// - `math_` atoms
// - `mathchoice` (TeXbook p. 151)

// Unimplemented primitives:
// `sl` (slanted) font: https://tug.org/texinfohtml/latex2e.html#index-_005csl
// `bbit` (double-struck italic) font
// `symliteral` wtf is this? (in unicode-math)
// `sc` (small caps) font: https://tug.org/texinfohtml/latex2e.html#index-_005csc
