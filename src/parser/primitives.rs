//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use core::panic;

use crate::event::{
    ArrayColumn as AC, ColorChange as CC, ColorTarget as CT, ColumnAlignment, Content as C,
    DelimiterSize, DelimiterType, Dimension, DimensionUnit, EnvironmentFlow, Event as E, Font,
    Grouping as G, GroupingKind, Line, MatrixType, RelationContent, ScriptPosition as SP,
    ScriptType as ST, StateChange as SC, Style as S, Visual as V,
};

use super::{
    lex,
    tables::{
        char_delimiter_map, control_sequence_delimiter_map, is_binary, is_relation, token_to_delim,
    },
    AlignmentCount, Argument, CharToken, ErrorKind, InnerParser, InnerResult, Instruction as I,
    Token,
};

impl<'b, 'store> InnerParser<'b, 'store> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` or `%` character is given
    pub(super) fn handle_char_token(&mut self, token: CharToken<'store>) -> InnerResult<()> {
        let instruction = I::Event(match token.into() {
            '\\' => panic!("(internal error: please report) the `\\` character should never be observed as a token"),
            '%' => panic!("(internal error: please report) the `%` character should never be observed as a token"),
            '_' => {
                if self.state.handling_argument {
                    return Err(ErrorKind::ScriptAsArgument)
                }
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                ]);
                self.content = token.as_str();
                E::End
            }
            '^' => {
                if self.state.handling_argument {
                    return Err(ErrorKind::ScriptAsArgument)
                }
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                ]);
                self.content = token.as_str();
                E::End
            }
            '$' => return Err(ErrorKind::MathShift),
            '#' => return Err(ErrorKind::HashSign),
            '&' if self
                    .state
                    .allowed_alignment_count
                    .as_deref()
                    .is_some_and(AlignmentCount::can_increment) && !self.state.handling_argument => {
                       self
                           .state
                           .allowed_alignment_count
                           .as_mut()
                           .expect("we have checked that `allowed_alignment_count` is Some")
                           .increment();
                        E::EnvironmentFlow(EnvironmentFlow::Alignment)
                    },
            '&' => return Err(ErrorKind::Alignment),
            '{' => {
                let str = &mut self.content;
                let group = lex::group_content(str, GroupingKind::Normal)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::SubGroup { content: group, allowed_alignment_count: None, text_mode: self.state.text_mode },
                    I::Event(E::End)
                ]);
                return Ok(())
            },
            '}' => {
                return Err(ErrorKind::UnbalancedGroup(None))
            },

            '~' => {
                E::Content(C::Text("&nbsp;"))
            },

            '0'..='9' => {
                let content = token.as_str();
                let len = if self.state.handling_argument {
                    1
                } else {
                    let mut len = content
                        .chars()
                        .skip(1)
                        .take_while(|&c| matches!(c, '.' | ',' | '0'..='9'))
                        .count()
                        + 1;
                    if matches!(content.as_bytes()[len - 1], b'.' | b',') {
                        len -= 1;
                    }
                    len
                };
                let (number, rest) = content.split_at(len);
                self.content = rest;
                self.buffer
                    .push(I::Event(E::Content(C::Number(number))));
                return Ok(())
            }
            // Punctuation
            '.' | ',' | ';' => E::Content(C::Punctuation(token.into())),
            '\'' => ordinary('′'),
            '-' => binary('−'),
            '*' => binary('∗'),
            c if is_binary(c) => binary(c),
            c if is_relation(c) => relation(c),
            c if char_delimiter_map(c).is_some() => {
                let (content, ty) = char_delimiter_map(c).unwrap();
                if ty == DelimiterType::Fence {
                    ordinary(content)
                } else {
                    E::Content(C::Delimiter {
                        content,
                        size: None,
                        ty,
                    })
                }
            }
            c => ordinary(c),
        });
        self.buffer.push(instruction);
        Ok(())
    }

    /// Handle a supported control sequence, pushing instructions to the provided stack.
    pub(super) fn handle_primitive(&mut self, control_sequence: &'store str) -> InnerResult<()> {
        let event = match control_sequence {
            "arccos" | "cos" | "csc" | "exp" | "ker" | "sinh" | "arcsin" | "cosh" | "deg"
            | "lg" | "ln" | "arctan" | "cot" | "det" | "hom" | "log" | "sec" | "tan" | "arg"
            | "coth" | "dim" | "sin" | "tanh" | "sgn" => E::Content(C::Function(control_sequence)),
            "lim" | "Pr" | "sup" | "max" | "inf" | "gcd" | "min" => {
                self.state.allow_script_modifiers = true;
                self.state.script_position = SP::Movable;
                E::Content(C::Function(control_sequence))
            }
            "liminf" => {
                self.state.allow_script_modifiers = true;
                self.state.script_position = SP::Movable;
                E::Content(C::Function("lim inf"))
            }
            "limsup" => {
                self.state.allow_script_modifiers = true;
                self.state.script_position = SP::Movable;
                E::Content(C::Function("lim sup"))
            }

            "operatorname" => {
                self.state.allow_script_modifiers = true;
                let argument = lex::argument(&mut self.content)?;
                match argument {
                    Argument::Token(Token::ControlSequence(_)) => {
                        return Err(ErrorKind::ControlSequenceAsArgument)
                    }
                    Argument::Token(Token::Character(char_)) => {
                        E::Content(C::Function(char_.as_str()))
                    }
                    Argument::Group(content) => E::Content(C::Function(content)),
                }
            }
            "bmod" => E::Content(C::Function("mod")),
            "pmod" => {
                let argument = lex::argument(&mut self.content)?;
                self.buffer.extend([
                    I::Event(E::Space {
                        width: Some(Dimension::new(1., DimensionUnit::Em)),
                        height: None,
                    }),
                    I::Event(E::Begin(G::Normal)),
                    I::Event(E::Content(C::Delimiter {
                        content: '(',
                        size: None,
                        ty: DelimiterType::Open,
                    })),
                    I::Event(E::Content(C::Function("mod"))),
                ]);
                self.handle_argument(argument)?;
                self.buffer.extend([
                    I::Event(E::End),
                    I::Event(E::Content(C::Delimiter {
                        content: ')',
                        size: None,
                        ty: DelimiterType::Close,
                    })),
                ]);
                return Ok(());
            }

            // TODO: Operators with '*', for operatorname* and friends

            ////////////////////////////////
            // Atom-type (\math*) commands //
            ////////////////////////////////
            // These commands set the math class of their argument, which
            // influences spacing in MathML output.
            "mathord" => return self.atom_group(AtomClass::Ord),
            "mathop" => {
                self.state.allow_script_modifiers = true;
                self.state.script_position = SP::Movable;
                return self.atom_group(AtomClass::Op);
            }
            "mathbin" => return self.atom_group(AtomClass::Bin),
            "mathrel" => return self.atom_group(AtomClass::Rel),
            "mathopen" => return self.atom_group(AtomClass::Open),
            "mathclose" => return self.atom_group(AtomClass::Close),
            "mathpunct" => return self.atom_group(AtomClass::Punct),
            "mathinner" => return self.atom_group(AtomClass::Inner),

            /////////////////////////
            // Non-Latin Alphabets //
            /////////////////////////
            // Lowercase Greek letters
            "alpha" => ordinary('α'),
            "beta" => ordinary('β'),
            "gamma" => ordinary('γ'),
            "delta" => ordinary('δ'),
            "epsilon" => ordinary('ϵ'),
            "zeta" => ordinary('ζ'),
            "eta" => ordinary('η'),
            "theta" => ordinary('θ'),
            "iota" => ordinary('ι'),
            "kappa" => ordinary('κ'),
            "lambda" => ordinary('λ'),
            "mu" => ordinary('µ'),
            "nu" => ordinary('ν'),
            "xi" => ordinary('ξ'),
            "pi" => ordinary('π'),
            "rho" => ordinary('ρ'),
            "sigma" => ordinary('σ'),
            "tau" => ordinary('τ'),
            "upsilon" => ordinary('υ'),
            "phi" => ordinary('ϕ'),
            "chi" => ordinary('χ'),
            "psi" => ordinary('ψ'),
            "omega" => ordinary('ω'),
            "omicron" => ordinary('ο'),
            // Uppercase Greek letters
            "Alpha" => ordinary('Α'),
            "Beta" => ordinary('Β'),
            "Gamma" => ordinary('Γ'),
            "Delta" => ordinary('Δ'),
            "Epsilon" => ordinary('Ε'),
            "Zeta" => ordinary('Ζ'),
            "Eta" => ordinary('Η'),
            "Theta" => ordinary('Θ'),
            "Iota" => ordinary('Ι'),
            "Kappa" => ordinary('Κ'),
            "Lambda" => ordinary('Λ'),
            "Mu" => ordinary('Μ'),
            "Nu" => ordinary('Ν'),
            "Xi" => ordinary('Ξ'),
            "Pi" => ordinary('Π'),
            "Rho" => ordinary('Ρ'),
            "Sigma" => ordinary('Σ'),
            "Tau" => ordinary('Τ'),
            "Upsilon" => ordinary('Υ'),
            "Phi" => ordinary('Φ'),
            "Chi" => ordinary('Χ'),
            "Psi" => ordinary('Ψ'),
            "Omega" => ordinary('Ω'),
            "Omicron" => ordinary('Ο'),
            // Lowercase Greek Variants
            "varepsilon" => ordinary('ε'),
            "vartheta" => ordinary('ϑ'),
            "varkappa" => ordinary('ϰ'),
            "varrho" => ordinary('ϱ'),
            "varsigma" => ordinary('ς'),
            "varpi" => ordinary('ϖ'),
            "varphi" => ordinary('φ'),
            // Uppercase Greek Variants
            "varGamma" => ordinary('𝛤'),
            "varDelta" => ordinary('𝛥'),
            "varTheta" => ordinary('𝛩'),
            "varLambda" => ordinary('𝛬'),
            "varXi" => ordinary('𝛯'),
            "varPi" => ordinary('𝛱'),
            "varSigma" => ordinary('𝛴'),
            "varUpsilon" => ordinary('𝛶'),
            "varPhi" => ordinary('𝛷'),
            "varPsi" => ordinary('𝛹'),
            "varOmega" => ordinary('𝛺'),

            // Hebrew letters
            "aleph" => ordinary('ℵ'),
            "beth" => ordinary('ℶ'),
            "gimel" => ordinary('ℷ'),
            "daleth" => ordinary('ℸ'),
            // Other symbols
            "digamma" => ordinary('ϝ'),
            "eth" => ordinary('ð'),
            "ell" => ordinary('ℓ'),
            "nabla" => ordinary('∇'),
            "partial" => ordinary('∂'),
            "Finv" => ordinary('Ⅎ'),
            "Game" => ordinary('ℷ'),
            "hbar" | "hslash" => ordinary('ℏ'),
            "imath" => ordinary('ı'),
            "jmath" => ordinary('ȷ'),
            "Im" => ordinary('ℑ'),
            "Re" => ordinary('ℜ'),
            "wp" => ordinary('℘'),
            "Bbbk" => ordinary('𝕜'),
            "Angstrom" => ordinary('Å'),
            "backepsilon" => ordinary('϶'),

            ///////////////////////////
            // Symbols & Punctuation //
            ///////////////////////////
            "dots" => {
                if self.content.trim_start().starts_with(['.', ',']) {
                    ordinary('…')
                } else {
                    ordinary('⋯')
                }
            }
            "ldots" | "dotso" | "dotsc" => ordinary('…'),
            "cdots" | "dotsi" | "dotsm" | "dotsb" | "idotsin" => ordinary('⋯'),
            "ddots" => ordinary('⋱'),
            "iddots" => ordinary('⋰'),
            "vdots" => ordinary('⋮'),
            "mathellipsis" => ordinary('…'),
            "infty" => ordinary('∞'),
            "checkmark" => ordinary('✓'),
            "ballotx" => ordinary('✗'),
            "dagger" | "dag" => ordinary('†'),
            "ddagger" | "ddag" => ordinary('‡'),
            "angle" => ordinary('∠'),
            "measuredangle" => ordinary('∡'),
            "lq" => ordinary('‘'),
            "Box" => ordinary('□'),
            "sphericalangle" => ordinary('∢'),
            "square" => ordinary('□'),
            "top" => ordinary('⊤'),
            "rq" => ordinary('′'),
            "blacksquare" => ordinary('■'),
            "bot" => ordinary('⊥'),
            "triangledown" => ordinary('▽'),
            "Bot" => ordinary('⫫'),
            "triangleleft" => ordinary('◃'),
            "triangleright" => ordinary('▹'),
            "cent" => ordinary('¢'),
            "colon" | "ratio" | "vcentcolon" => ordinary(':'),
            "bigtriangledown" => ordinary('▽'),
            "pounds" | "mathsterling" => ordinary('£'),
            "bigtriangleup" => ordinary('△'),
            "blacktriangle" => ordinary('▲'),
            "blacktriangledown" => ordinary('▼'),
            "yen" => ordinary('¥'),
            "blacktriangleleft" => ordinary('◀'),
            "euro" => ordinary('€'),
            "blacktriangleright" => ordinary('▶'),
            "Diamond" => ordinary('◊'),
            "degree" => ordinary('°'),
            "lozenge" => ordinary('◊'),
            "blacklozenge" => ordinary('⧫'),
            "mho" => ordinary('℧'),
            "bigstar" => ordinary('★'),
            "diagdown" => ordinary('╲'),
            "maltese" => ordinary('✠'),
            "diagup" => ordinary('╱'),
            "P" => ordinary('¶'),
            "clubsuit" => ordinary('♣'),
            "varclubsuit" => ordinary('♧'),
            "S" => ordinary('§'),
            "diamondsuit" => ordinary('♢'),
            "vardiamondsuit" => ordinary('♦'),
            "copyright" => ordinary('©'),
            "heartsuit" => ordinary('♡'),
            "varheartsuit" => ordinary('♥'),
            "circledR" => ordinary('®'),
            "spadesuit" => ordinary('♠'),
            "varspadesuit" => ordinary('♤'),
            "circledS" => ordinary('Ⓢ'),
            "female" => ordinary('♀'),
            "male" => ordinary('♂'),
            "astrosun" => ordinary('☉'),
            "sun" => ordinary('☼'),
            "leftmoon" => ordinary('☾'),
            "rightmoon" => ordinary('☽'),
            "smiley" => ordinary('☺'),
            "Earth" => ordinary('⊕'),
            "flat" => ordinary('♭'),
            "standardstate" => ordinary('⦵'),
            "natural" => ordinary('♮'),
            "sharp" => ordinary('♯'),
            "permil" => ordinary('‰'),
            "QED" => ordinary('∎'),
            "lightning" => ordinary('↯'),
            "diameter" => ordinary('⌀'),
            "leftouterjoin" => ordinary('⟕'),
            "rightouterjoin" => ordinary('⟖'),
            "concavediamond" => ordinary('⟡'),
            "concavediamondtickleft" => ordinary('⟢'),
            "concavediamondtickright" => ordinary('⟣'),
            "fullouterjoin" => ordinary('⟗'),
            "triangle" | "vartriangle" => ordinary('△'),
            "whitesquaretickleft" => ordinary('⟤'),
            "whitesquaretickright" => ordinary('⟥'),

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
            "mathcal" | "symcal" => return self.font_group(Some(Font::Script)),
            "mathit" | "symit" => return self.font_group(Some(Font::Italic)),
            "mathrm" | "symrm" | "mathup" | "symup" => return self.font_group(Some(Font::UpRight)),
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
                let Argument::Group(color) = lex::argument(&mut self.content)? else {
                    return Err(ErrorKind::Argument);
                };
                self.state.skip_scripts = true;

                let color = lex::color(color).ok_or(ErrorKind::UnknownColor)?;
                E::StateChange(SC::Color(CC {
                    color,
                    target: CT::Text,
                }))
            }
            "textcolor" => {
                let str = &mut self.content;
                let Argument::Group(color) = lex::argument(str)? else {
                    return Err(ErrorKind::Argument);
                };

                let color = lex::color(color).ok_or(ErrorKind::UnknownColor)?;
                let modified = lex::argument(str)?;

                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::Event(E::StateChange(SC::Color(CC {
                        color,
                        target: CT::Text,
                    }))),
                ]);
                self.handle_argument(modified)?;
                E::End
            }
            "colorbox" => {
                let Argument::Group(color) = lex::argument(&mut self.content)? else {
                    return Err(ErrorKind::Argument);
                };

                let color = lex::color(color).ok_or(ErrorKind::UnknownColor)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::Event(E::StateChange(SC::Color(CC {
                        color,
                        target: CT::Background,
                    }))),
                ]);
                self.text_argument()?;
                E::End
            }
            "fcolorbox" => {
                let str = &mut self.content;
                let Argument::Group(frame_color) = lex::argument(str)? else {
                    return Err(ErrorKind::Argument);
                };
                let Argument::Group(background_color) = lex::argument(str)? else {
                    return Err(ErrorKind::Argument);
                };

                let frame_color = lex::color(frame_color).ok_or(ErrorKind::UnknownColor)?;
                let background_color =
                    lex::color(background_color).ok_or(ErrorKind::UnknownColor)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::Event(E::StateChange(SC::Color(CC {
                        color: frame_color,
                        target: CT::Border,
                    }))),
                    I::Event(E::StateChange(SC::Color(CC {
                        color: background_color,
                        target: CT::Background,
                    }))),
                ]);
                self.text_argument()?;
                E::End
            }

            ///////////////////////////////
            // Delimiters size modifiers //
            ///////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => return self.sized_delim(DelimiterSize::Big),
            "Big" | "Bigl" | "Bigr" | "Bigm" => return self.sized_delim(DelimiterSize::BIG),
            "bigg" | "biggl" | "biggr" | "biggm" => return self.sized_delim(DelimiterSize::Bigg),
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => return self.sized_delim(DelimiterSize::BIGG),

            "left" => {
                let curr_str = &mut self.content;
                let opening = if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    None
                } else {
                    Some(lex::delimiter(curr_str)?.0)
                };

                let curr_str = &mut self.content;
                let group_content = lex::group_content(curr_str, GroupingKind::LeftRight)?;
                let closing = if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    None
                } else {
                    Some(lex::delimiter(curr_str)?.0)
                };

                self.buffer.extend([
                    I::Event(E::Begin(G::LeftRight(opening, closing))),
                    I::SubGroup {
                        content: group_content,
                        allowed_alignment_count: None,
                        text_mode: false,
                    },
                    I::Event(E::End),
                ]);

                return Ok(());
            }
            // TODO: Check the conditions for this op. Does it need to be
            // within a left-right group?
            "middle" => {
                let delimiter = lex::delimiter(&mut self.content)?;
                E::Content(C::Delimiter {
                    content: delimiter.0,
                    size: Some(DelimiterSize::Big),
                    ty: DelimiterType::Fence,
                })
            }
            "right" => {
                return Err(ErrorKind::UnbalancedGroup(None));
            }

            ///////////////////
            // Big Operators //
            ///////////////////
            // NOTE: All of the following operators allow limit modifiers.
            // The following operators have above and below limits by default.
            "sum" => self.large_op('∑', true),
            "prod" => self.large_op('∏', true),
            "coprod" => self.large_op('∐', true),
            "bigvee" => self.large_op('⋁', true),
            "bigwedge" => self.large_op('⋀', true),
            "bigcup" => self.large_op('⋃', true),
            "bigcap" => self.large_op('⋂', true),
            "biguplus" => self.large_op('⨄', true),
            "bigoplus" => self.large_op('⨁', true),
            "bigotimes" => self.large_op('⨂', true),
            "bigodot" => self.large_op('⨀', true),
            "bigsqcup" => self.large_op('⨆', true),
            "bigsqcap" => self.large_op('⨅', true),
            "bigtimes" => self.large_op('⨉', true),
            "intop" => self.large_op('∫', true),
            // The following operators do not have above and below limits by default.
            "int" => self.large_op('∫', false),
            "iint" => self.large_op('∬', false),
            "iiint" => self.large_op('∭', false),
            "smallint" => {
                self.state.allow_script_modifiers = true;
                E::Content(C::LargeOp {
                    content: '∫',
                    small: true,
                })
            }
            "iiiint" => self.large_op('⨌', false),
            "intcap" => self.large_op('⨙', false),
            "intcup" => self.large_op('⨚', false),
            "oint" => self.large_op('∮', false),
            "varointclockwise" => self.large_op('∲', false),
            "intclockwise" => self.large_op('∱', false),
            "oiint" => self.large_op('∯', false),
            "pointint" => self.large_op('⨕', false),
            "rppolint" => self.large_op('⨒', false),
            "scpolint" => self.large_op('⨓', false),
            "oiiint" => self.large_op('∰', false),
            "intlarhk" => self.large_op('⨗', false),
            "sqint" => self.large_op('⨖', false),
            "intx" => self.large_op('⨘', false),
            "intbar" => self.large_op('⨍', false),
            "intBar" => self.large_op('⨎', false),
            "fint" => self.large_op('⨏', false),

            /////////////
            // Accents //
            /////////////
            "acute" => return self.accent('´', false),
            "bar" | "overline" => return self.accent('‾', false),
            "underbar" | "underline" => return self.underscript('_'),
            "breve" => return self.accent('˘', false),
            "check" => return self.accent('ˇ', false),
            "dot" => return self.accent('˙', false),
            "ddot" => return self.accent('¨', false),
            "grave" => return self.accent('`', false),
            "hat" => return self.accent('^', false),
            "tilde" => return self.accent('~', false),
            "vec" => return self.accent('→', false),
            "mathring" => return self.accent('˚', false),

            // Arrows
            "overleftarrow" => return self.accent('←', true),
            "underleftarrow" => return self.underscript('←'),
            "overrightarrow" => return self.accent('→', true),
            "Overrightarrow" => return self.accent('⇒', true),
            "underrightarrow" => return self.underscript('→'),
            "overleftrightarrow" => return self.accent('↔', true),
            "underleftrightarrow" => return self.underscript('↔'),
            "overleftharpoon" => return self.accent('↼', true),
            "overrightharpoon" => return self.accent('⇀', true),

            // Wide ops
            "widecheck" => return self.accent('ˇ', true),
            "widehat" => return self.accent('^', true),
            "widetilde" => return self.accent('~', true),
            "wideparen" | "overparen" => return self.accent('⏜', true),

            // Groups
            "overgroup" => {
                self.state.script_position = SP::AboveBelow;
                return self.accent('⏠', true);
            }
            "undergroup" => {
                self.state.script_position = SP::AboveBelow;
                return self.underscript('⏡');
            }
            "overbrace" => {
                self.state.script_position = SP::AboveBelow;
                return self.accent('⏞', true);
            }
            "underbrace" => {
                self.state.script_position = SP::AboveBelow;
                return self.underscript('⏟');
            }
            "underparen" => {
                self.state.script_position = SP::AboveBelow;
                return self.underscript('⏝');
            }
            "overbracket" => {
                self.state.script_position = SP::AboveBelow;
                return self.accent('⎴', true);
            }
            "underbracket" => {
                self.state.script_position = SP::AboveBelow;
                return self.underscript('⎵');
            }

            // Primes
            "prime" => ordinary('′'),
            "dprime" => ordinary('″'),
            "trprime" => ordinary('‴'),
            "qprime" => ordinary('⁗'),
            "backprime" => ordinary('‵'),
            "backdprime" => ordinary('‶'),
            "backtrprime" => ordinary('‷'),

            /////////////
            // Spacing //
            /////////////
            "," | "thinspace" => E::Space {
                width: Some(Dimension::new(3. / 18., DimensionUnit::Em)),
                height: None,
            },
            ">" | ":" | "medspace" => E::Space {
                width: Some(Dimension::new(4. / 18., DimensionUnit::Em)),
                height: None,
            },
            ";" | "thickspace" => E::Space {
                width: Some(Dimension::new(5. / 18., DimensionUnit::Em)),
                height: None,
            },
            "enspace" => E::Space {
                width: Some(Dimension::new(0.5, DimensionUnit::Em)),
                height: None,
            },
            "quad" => E::Space {
                width: Some(Dimension::new(1., DimensionUnit::Em)),
                height: None,
            },
            "qquad" => E::Space {
                width: Some(Dimension::new(2., DimensionUnit::Em)),
                height: None,
            },
            "mathstrut" => E::Space {
                width: None,
                height: Some(Dimension::new(0.7, DimensionUnit::Em)),
            },
            "~" | "nobreakspace" => E::Content(C::Text("&nbsp;")),
            // Variable spacing
            "kern" => {
                let dimension = lex::dimension(&mut self.content)?;
                E::Space {
                    width: Some(dimension),
                    height: None,
                }
            }
            "hskip" => {
                let glue = lex::glue(&mut self.content)?;
                E::Space {
                    width: Some(glue.0),
                    height: None,
                }
            }
            "mkern" => {
                let dimension = lex::dimension(&mut self.content)?;
                if dimension.unit == DimensionUnit::Mu {
                    E::Space {
                        width: Some(dimension),
                        height: None,
                    }
                } else {
                    return Err(ErrorKind::MathUnit);
                }
            }
            "mskip" => {
                let glue = lex::glue(&mut self.content)?;
                if glue.0.unit == DimensionUnit::Mu
                    && glue
                        .1
                        .map_or(true, |Dimension { unit, .. }| unit == DimensionUnit::Mu)
                    && glue
                        .2
                        .map_or(true, |Dimension { unit, .. }| unit == DimensionUnit::Mu)
                {
                    E::Space {
                        width: Some(glue.0),
                        height: None,
                    }
                } else {
                    return Err(ErrorKind::MathUnit);
                }
            }
            "hspace" => {
                let Argument::Group(mut argument) = lex::argument(&mut self.content)? else {
                    return Err(ErrorKind::DimensionArgument);
                };
                let glue = lex::glue(&mut argument)?;
                E::Space {
                    width: Some(glue.0),
                    height: None,
                }
            }
            // Negative spacing
            "!" | "negthinspace" => E::Space {
                width: Some(Dimension::new(-3. / 18., DimensionUnit::Em)),
                height: None,
            },
            "negmedspace" => E::Space {
                width: Some(Dimension::new(-4. / 18., DimensionUnit::Em)),
                height: None,
            },
            "negthickspace" => E::Space {
                width: Some(Dimension::new(-5. / 18., DimensionUnit::Em)),
                height: None,
            },

            ////////////////////////
            // Logic & Set Theory //
            ////////////////////////
            "forall" => ordinary('∀'),
            "exists" => ordinary('∃'),
            "complement" => ordinary('∁'),
            "nexists" => ordinary('∄'),
            "neg" | "lnot" => ordinary('¬'),

            "therefore" => relation('∴'),
            "because" => relation('∵'),
            "subset" => relation('⊂'),
            "supset" => relation('⊃'),
            "strictif" => relation('⥽'),
            "strictfi" => relation('⥼'),
            "mapsto" => relation('↦'),
            "implies" => relation('⟹'),
            "mid" => relation('∣'),
            "to" => relation('→'),
            "impliedby" => relation('⟸'),
            "in" | "isin" => relation('∈'),
            "ni" => relation('∋'),
            "gets" => relation('←'),
            "iff" => relation('⟺'),
            "notni" => relation('∌'),

            "land" => binary('∧'),

            "emptyset" => ordinary('∅'),
            "varnothing" => ordinary('⌀'),

            //////////////////////
            // Binary Operators //
            //////////////////////
            "ldotp" => binary('.'),
            "cdotp" => binary('·'),
            "cdot" => binary('⋅'),
            "centerdot" => binary('·'),
            "circ" => binary('∘'),
            "bullet" => binary('∙'),
            "circledast" => binary('⊛'),
            "circledcirc" => binary('⊚'),
            "circleddash" => binary('⊝'),
            "bigcirc" => binary('◯'),
            "leftthreetimes" => binary('⋋'),
            "rhd" => binary('⊳'),
            "lhd" => binary('⊲'),
            "rightthreetimes" => binary('⋌'),
            "rtimes" => binary('⋊'),
            "ltimes" => binary('⋉'),
            "leftmodels" => binary('⊨'),
            "amalg" => binary('⨿'),
            "ast" => binary('*'),
            "asymp" => binary('≍'),
            "And" | "with" => binary('&'),
            "lor" => binary('∨'),
            "setminus" => binary('∖'),
            "Cup" => binary('⋓'),
            "cup" => binary('∪'),
            "sqcup" => binary('⊔'),
            "sqcap" => binary('⊓'),
            "lessdot" => binary('⋖'),
            "smallsetminus" => E::Content(C::BinaryOp {
                content: '∖',
                small: false,
            }),
            "barwedge" => binary('⌅'),
            "curlyvee" => binary('⋎'),
            "curlywedge" => binary('⋏'),
            "sslash" => binary('⫽'),
            "div" => binary('÷'),
            "mp" => binary('∓'),
            "times" => binary('×'),
            "boxdot" => binary('⊡'),
            "divideontimes" => binary('⋇'),
            "odot" => binary('⊙'),
            "unlhd" => binary('⊴'),
            "boxminus" => binary('⊟'),
            "dotplus" => binary('∔'),
            "ominus" => binary('⊖'),
            "unrhd" => binary('⊵'),
            "boxplus" => binary('⊞'),
            "doublebarwedge" => binary('⩞'),
            "oplus" => binary('⊕'),
            "uplus" => binary('⊎'),
            "boxtimes" => binary('⊠'),
            "doublecap" => binary('⋒'),
            "otimes" => binary('⊗'),
            "vee" => binary('∨'),
            "veebar" => binary('⊻'),
            "Cap" => binary('⋒'),
            "parr" => binary('⅋'),
            "wedge" => binary('∧'),
            "cap" => binary('∩'),
            "gtrdot" => binary('⋗'),
            "pm" => binary('±'),
            "intercal" => binary('⊺'),
            "wr" => binary('≀'),
            "circledvert" => binary('⦶'),
            "blackhourglass" => binary('⧗'),
            "circlehbar" => binary('⦵'),
            "operp" => binary('⦹'),
            "boxast" => binary('⧆'),
            "boxbox" => binary('⧈'),
            "oslash" => binary('⊘'),
            "boxcircle" => binary('⧇'),
            "diamond" => binary('⋄'),
            "Otimes" => binary('⨷'),
            "hourglass" => binary('⧖'),
            "otimeshat" => binary('⨶'),
            "triangletimes" => binary('⨻'),
            "lozengeminus" => binary('⟠'),
            "star" => binary('⋆'),
            "obar" => binary('⌽'),
            "obslash" => binary('⦸'),
            "triangleminus" => binary('⨺'),
            "odiv" => binary('⨸'),
            "triangleplus" => binary('⨹'),
            "circledequal" => binary('⊜'),
            "ogreaterthan" => binary('⧁'),
            "circledparallel" => binary('⦷'),
            "olessthan" => binary('⧀'),

            ///////////////
            // Relations //
            ///////////////
            "eqcirc" => relation('≖'),
            "lessgtr" => relation('≶'),
            "smile" | "sincoh" => relation('⌣'),
            "eqcolon" | "minuscolon" => relation('∹'),
            "lesssim" => relation('≲'),
            "sqsubset" => relation('⊏'),
            "ll" => relation('≪'),
            "sqsubseteq" => relation('⊑'),
            "eqqcolon" => relation('≕'),
            "lll" => relation('⋘'),
            "sqsupset" => relation('⊐'),
            "llless" => relation('⋘'),
            "sqsupseteq" => relation('⊒'),
            "approx" => relation('≈'),
            "eqdef" => relation('≝'),
            "lt" => relation('<'),
            "stareq" => relation('≛'),
            "approxeq" => relation('≊'),
            "eqsim" => relation('≂'),
            "measeq" => relation('≞'),
            "Subset" => relation('⋐'),
            "arceq" => relation('≘'),
            "eqslantgtr" => relation('⪖'),
            "eqslantless" => relation('⪕'),
            "models" => relation('⊨'),
            "subseteq" => relation('⊆'),
            "backcong" => relation('≌'),
            "equiv" => relation('≡'),
            "multimap" => relation('⊸'),
            "subseteqq" => relation('⫅'),
            "fallingdotseq" => relation('≒'),
            "multimapboth" => relation('⧟'),
            "succ" => relation('≻'),
            "backsim" => relation('∽'),
            "frown" => relation('⌢'),
            "multimapinv" => relation('⟜'),
            "succapprox" => relation('⪸'),
            "backsimeq" => relation('⋍'),
            "ge" => relation('≥'),
            "origof" => relation('⊶'),
            "succcurlyeq" => relation('≽'),
            "between" => relation('≬'),
            "geq" => relation('≥'),
            "owns" => relation('∋'),
            "succeq" => relation('⪰'),
            "bumpeq" => relation('≏'),
            "geqq" => relation('≧'),
            "parallel" => relation('∥'),
            "succsim" => relation('≿'),
            "Bumpeq" => relation('≎'),
            "geqslant" => relation('⩾'),
            "perp" => relation('⟂'),
            "Supset" => relation('⋑'),
            "circeq" => relation('≗'),
            "gg" => relation('≫'),
            "Perp" => relation('⫫'),
            "coh" => relation('⌢'),
            "ggg" => relation('⋙'),
            "pitchfork" => relation('⋔'),
            "supseteq" => relation('⊇'),
            "gggtr" => relation('⋙'),
            "prec" => relation('≺'),
            "supseteqq" => relation('⫆'),
            "gt" => relation('>'),
            "precapprox" => relation('⪷'),
            "thickapprox" => relation('≈'),
            "gtrapprox" => relation('⪆'),
            "preccurlyeq" => relation('≼'),
            "thicksim" => relation('∼'),
            "gtreqless" => relation('⋛'),
            "preceq" => relation('⪯'),
            "trianglelefteq" => relation('⊴'),
            "coloneqq" | "colonequals" => relation('≔'),
            "gtreqqless" => relation('⪌'),
            "precsim" => relation('≾'),
            "triangleq" => relation('≜'),
            "Coloneqq" | "coloncolonequals" => relation('⩴'),
            "gtrless" => relation('≷'),
            "propto" => relation('∝'),
            "trianglerighteq" => relation('⊵'),
            "gtrsim" => relation('≳'),
            "questeq" => relation('≟'),
            "varpropto" => relation('∝'),
            "imageof" => relation('⊷'),
            "cong" => relation('≅'),
            "risingdotseq" => relation('≓'),
            "vartriangleleft" => relation('⊲'),
            "curlyeqprec" => relation('⋞'),
            "scoh" => relation('⌢'),
            "vartriangleright" => relation('⊳'),
            "curlyeqsucc" => relation('⋟'),
            "le" => relation('≤'),
            "shortmid" => E::Content(C::Relation {
                content: RelationContent::single_char('∣'),
                small: true,
            }),
            "shortparallel" => E::Content(C::Relation {
                content: RelationContent::single_char('∥'),
                small: true,
            }),
            "vdash" => relation('⊢'),
            "dashv" => relation('⊣'),
            "leq" => relation('≤'),
            "vDash" => relation('⊨'),
            "dblcolon" | "coloncolon" => relation('∷'),
            "leqq" => relation('≦'),
            "sim" => relation('∼'),
            "Vdash" => relation('⊩'),
            "doteq" => relation('≐'),
            "leqslant" => relation('⩽'),
            "simeq" => relation('≃'),
            "Dash" => relation('⊫'),
            "Doteq" => relation('≑'),
            "lessapprox" => relation('⪅'),
            "Vvdash" => relation('⊪'),
            "doteqdot" => relation('≑'),
            "lesseqgtr" => relation('⋚'),
            "smallfrown" => relation('⌢'),
            "veeeq" => relation('≚'),
            "eqeq" => relation('⩵'),
            "lesseqqgtr" => relation('⪋'),
            "smallsmile" => E::Content(C::Relation {
                content: RelationContent::single_char('⌣'),
                small: true,
            }),
            "wedgeq" => relation('≙'),
            "bowtie" | "Join" => relation('⋈'),
            // Negated relations
            "gnapprox" => relation('⪊'),
            "ngeqslant" => relation('≱'),
            "nsubset" => relation('⊄'),
            "nVdash" => relation('⊮'),
            "gneq" => relation('⪈'),
            "ngtr" => relation('≯'),
            "nsubseteq" => relation('⊈'),
            "precnapprox" => relation('⪹'),
            "gneqq" => relation('≩'),
            "nleq" => relation('≰'),
            "nsubseteqq" => relation('⊈'),
            "precneqq" => relation('⪵'),
            "gnsim" => relation('⋧'),
            "nleqq" => relation('≰'),
            "nsucc" => relation('⊁'),
            "precnsim" => relation('⋨'),
            "nleqslant" => relation('≰'),
            "nsucceq" => relation('⋡'),
            "subsetneq" => relation('⊊'),
            "lnapprox" => relation('⪉'),
            "nless" => relation('≮'),
            "nsupset" => relation('⊅'),
            "subsetneqq" => relation('⫋'),
            "lneq" => relation('⪇'),
            "nmid" => relation('∤'),
            "nsupseteq" => relation('⊉'),
            "succnapprox" => relation('⪺'),
            "lneqq" => relation('≨'),
            "notin" => relation('∉'),
            "nsupseteqq" => relation('⊉'),
            "succneqq" => relation('⪶'),
            "lnsim" => relation('⋦'),
            "ntriangleleft" => relation('⋪'),
            "succnsim" => relation('⋩'),
            "nparallel" => relation('∦'),
            "ntrianglelefteq" => relation('⋬'),
            "supsetneq" => relation('⊋'),
            "ncong" => relation('≆'),
            "nprec" => relation('⊀'),
            "ntriangleright" => relation('⋫'),
            "supsetneqq" => relation('⫌'),
            "ne" => relation('≠'),
            "npreceq" => relation('⋠'),
            "ntrianglerighteq" => relation('⋭'),
            "neq" => relation('≠'),
            "nshortmid" => E::Content(C::Relation {
                content: RelationContent::single_char('∤'),
                small: true,
            }),
            "nvdash" => relation('⊬'),
            "ngeq" => relation('≱'),
            "nshortparallel" => E::Content(C::Relation {
                content: RelationContent::single_char('∦'),
                small: true,
            }),
            "nvDash" => relation('⊭'),
            "ngeqq" => relation('≱'),
            "nsim" => relation('≁'),
            "nVDash" => relation('⊯'),
            "varsupsetneqq" => multirelation('⫌', '\u{fe00}'),
            "varsubsetneqq" => multirelation('⫋', '\u{fe00}'),
            "varsubsetneq" => multirelation('⊊', '\u{fe00}'),
            "varsupsetneq" => multirelation('⊋', '\u{fe00}'),
            "gvertneqq" => multirelation('≩', '\u{fe00}'),
            "lvertneqq" => multirelation('≨', '\u{fe00}'),
            "Eqcolon" | "minuscoloncolon" => multirelation('−', '∷'),
            "Eqqcolon" => multirelation('=', '∷'),
            "approxcolon" => multirelation('≈', ':'),
            "colonapprox" => multirelation(':', '≈'),
            "approxcoloncolon" => multirelation('≈', '∷'),
            "Colonapprox" | "coloncolonapprox" => multirelation('∷', '≈'),
            "coloneq" | "colonminus" => multirelation(':', '−'),
            "Coloneq" | "coloncolonminus" => multirelation('∷', '−'),
            "colonsim" => multirelation(':', '∼'),
            "Colonsim" | "coloncolonsim" => multirelation('∷', '∼'),

            ////////////
            // Arrows //
            ////////////
            "circlearrowleft" => relation('↺'),
            "Leftrightarrow" => relation('⇔'),
            "restriction" => relation('↾'),
            "circlearrowright" => relation('↻'),
            "leftrightarrows" => relation('⇆'),
            "rightarrow" => relation('→'),
            "curvearrowleft" => relation('↶'),
            "leftrightharpoons" => relation('⇋'),
            "Rightarrow" => relation('⇒'),
            "curvearrowright" => relation('↷'),
            "leftrightsquigarrow" => relation('↭'),
            "rightarrowtail" => relation('↣'),
            "dashleftarrow" => relation('⇠'),
            "Lleftarrow" => relation('⇚'),
            "rightharpoondown" => relation('⇁'),
            "dashrightarrow" => relation('⇢'),
            "longleftarrow" => relation('⟵'),
            "rightharpoonup" => relation('⇀'),
            "downarrow" => relation('↓'),
            "Longleftarrow" => relation('⟸'),
            "rightleftarrows" => relation('⇄'),
            "Downarrow" => relation('⇓'),
            "longleftrightarrow" => relation('⟷'),
            "rightleftharpoons" => relation('⇌'),
            "downdownarrows" => relation('⇊'),
            "Longleftrightarrow" => relation('⟺'),
            "rightrightarrows" => relation('⇉'),
            "downharpoonleft" => relation('⇃'),
            "longmapsto" => relation('⟼'),
            "rightsquigarrow" => relation('⇝'),
            "downharpoonright" => relation('⇂'),
            "longrightarrow" => relation('⟶'),
            "Rrightarrow" => relation('⇛'),
            "Longrightarrow" => relation('⟹'),
            "Rsh" => relation('↱'),
            "hookleftarrow" => relation('↩'),
            "looparrowleft" => relation('↫'),
            "searrow" => relation('↘'),
            "hookrightarrow" => relation('↪'),
            "looparrowright" => relation('↬'),
            "swarrow" => relation('↙'),
            "Lsh" => relation('↰'),
            "mapsfrom" => relation('↤'),
            "twoheadleftarrow" => relation('↞'),
            "twoheadrightarrow" => relation('↠'),
            "leadsto" => relation('⇝'),
            "nearrow" => relation('↗'),
            "uparrow" => relation('↑'),
            "leftarrow" => relation('←'),
            "nleftarrow" => relation('↚'),
            "Uparrow" => relation('⇑'),
            "Leftarrow" => relation('⇐'),
            "nLeftarrow" => relation('⇍'),
            "updownarrow" => relation('↕'),
            "leftarrowtail" => relation('↢'),
            "nleftrightarrow" => relation('↮'),
            "Updownarrow" => relation('⇕'),
            "leftharpoondown" => relation('↽'),
            "nLeftrightarrow" => relation('⇎'),
            "upharpoonleft" => relation('↿'),
            "leftharpoonup" => relation('↼'),
            "nrightarrow" => relation('↛'),
            "upharpoonright" => relation('↾'),
            "leftleftarrows" => relation('⇇'),
            "nRightarrow" => relation('⇏'),
            "upuparrows" => relation('⇈'),
            "leftrightarrow" => relation('↔'),
            "nwarrow" => relation('↖'),
            "xleftarrow" => {
                let below = lex::optional_argument(&mut self.content);
                let above = lex::argument(&mut self.content)?;
                self.buffer.extend([
                    I::Event(E::Script {
                        ty: if below.is_some() {
                            ST::SubSuperscript
                        } else {
                            ST::Superscript
                        },
                        position: SP::AboveBelow,
                    }),
                    I::Event(relation('←')),
                ]);
                if let Some(below) = below {
                    self.handle_argument(Argument::Group(below))?;
                }
                self.handle_argument(above)?;
                return Ok(());
            }
            "xrightarrow" => {
                let below = lex::optional_argument(&mut self.content);
                let above = lex::argument(&mut self.content)?;
                self.buffer.extend([
                    I::Event(E::Script {
                        ty: if below.is_some() {
                            ST::SubSuperscript
                        } else {
                            ST::Superscript
                        },
                        position: SP::AboveBelow,
                    }),
                    I::Event(relation('→')),
                ]);
                if let Some(below) = below {
                    self.handle_argument(Argument::Group(below))?;
                }
                self.handle_argument(above)?;
                return Ok(());
            }

            ///////////////
            // Fractions //
            ///////////////
            "frac" => {
                return self.fraction_like(None, None, None, None);
            }
            // TODO: better errors for this
            "genfrac" => {
                let str = &mut self.content;
                let ldelim_argument = lex::argument(str)?;
                let ldelim = match ldelim_argument {
                    Argument::Token(token) => {
                        Some(token_to_delim(token).ok_or(ErrorKind::Delimiter)?)
                    }
                    Argument::Group(group) => {
                        if group.is_empty() {
                            None
                        } else {
                            return Err(ErrorKind::Delimiter);
                        }
                    }
                };
                let rdelim_argument = lex::argument(str)?;
                let rdelim = match rdelim_argument {
                    Argument::Token(token) => {
                        Some(token_to_delim(token).ok_or(ErrorKind::Delimiter)?)
                    }
                    Argument::Group(group) => {
                        if group.is_empty() {
                            None
                        } else {
                            return Err(ErrorKind::Delimiter);
                        }
                    }
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
                    Argument::Group(group) => match group {
                        "0" => Some(S::Display),
                        "1" => Some(S::Text),
                        "2" => Some(S::Script),
                        "3" => Some(S::ScriptScript),
                        "" => None,
                        _ => return Err(ErrorKind::Argument),
                    },
                };

                self.fraction_like(
                    ldelim.map(|d| d.0),
                    rdelim.map(|d| d.0),
                    bar_size,
                    display_style,
                )?;

                return Ok(());
            }
            "cfrac" | "dfrac" => {
                self.fraction_like(None, None, None, Some(S::Display))?;
                return Ok(());
            }
            "tfrac" => {
                self.fraction_like(None, None, None, Some(S::Text))?;
                return Ok(());
            }
            "binom" => {
                self.fraction_like(
                    Some('('),
                    Some(')'),
                    Some(Dimension::new(0., DimensionUnit::Em)),
                    None,
                )?;
                return Ok(());
            }
            "dbinom" => {
                self.fraction_like(
                    Some('('),
                    Some(')'),
                    Some(Dimension::new(0., DimensionUnit::Em)),
                    Some(S::Display),
                )?;
                return Ok(());
            }
            "tbinom" => {
                self.fraction_like(
                    Some('('),
                    Some(')'),
                    Some(Dimension::new(0., DimensionUnit::Em)),
                    Some(S::Text),
                )?;
                return Ok(());
            }
            "overset" | "stackrel" => {
                self.buffer.push(I::Event(E::Script {
                    ty: ST::Superscript,
                    position: SP::AboveBelow,
                }));
                let before_over_index = self.buffer.len();
                let over = lex::argument(&mut self.content)?;
                self.handle_argument(over)?;
                let over_events = self.buffer.split_off(before_over_index);
                let base = lex::argument(&mut self.content)?;
                self.handle_argument(base)?;
                self.buffer.extend(over_events);
                return Ok(());
            }
            "underset" => {
                self.buffer.push(I::Event(E::Script {
                    ty: ST::Subscript,
                    position: SP::AboveBelow,
                }));
                let before_under_index = self.buffer.len();
                let under = lex::argument(&mut self.content)?;
                self.handle_argument(under)?;
                let under_events = self.buffer.split_off(before_under_index);
                let base = lex::argument(&mut self.content)?;
                self.handle_argument(base)?;
                self.buffer.extend(under_events);
                return Ok(());
            }
            "buildrel" => {
                let mut over_content = lex::content_with_suffix(&mut self.content, r"\over")?;
                self.buffer.push(I::Event(E::Script {
                    ty: ST::Superscript,
                    position: SP::AboveBelow,
                }));
                let before_over_index = self.buffer.len();
                let over = lex::argument(&mut over_content)?;
                self.handle_argument(over)?;
                let over_events = self.buffer.split_off(before_over_index);
                let base = lex::argument(&mut self.content)?;
                self.handle_argument(base)?;
                self.buffer.extend(over_events);
                return Ok(());
            }
            "substack" => {
                let content = lex::brace_argument(&mut self.content)?;
                self.buffer.push(I::Event(E::Begin(G::SubArray {
                    alignment: ColumnAlignment::Center,
                })));
                self.buffer.push(I::SubGroup {
                    content,
                    allowed_alignment_count: Some(AlignmentCount::new(0)),
                    text_mode: false,
                });
                self.buffer.push(I::Event(E::End));
                return Ok(());
            }
            "sideset" => {
                let _left = lex::argument(&mut self.content)?;
                let _right = lex::argument(&mut self.content)?;
                let base = lex::argument(&mut self.content)?;
                self.handle_argument(base)?;
                return Ok(());
            }

            //////////////
            // Radicals //
            //////////////
            "sqrt" => {
                if let Some(index) = lex::optional_argument(&mut self.content) {
                    self.buffer.push(I::Event(E::Visual(V::Root)));
                    let arg = lex::argument(&mut self.content)?;
                    self.handle_argument(arg)?;
                    self.buffer.push(I::SubGroup {
                        content: index,
                        allowed_alignment_count: None,
                        text_mode: false,
                    });
                } else {
                    self.buffer.push(I::Event(E::Visual(V::SquareRoot)));
                    let arg = lex::argument(&mut self.content)?;
                    self.handle_argument(arg)?;
                }
                return Ok(());
            }
            "surd" => {
                self.buffer.extend([
                    I::Event(E::Visual(V::SquareRoot)),
                    I::Event(E::Space {
                        width: Some(Dimension::new(0., DimensionUnit::Em)),
                        height: Some(Dimension::new(0.7, DimensionUnit::Em)),
                    }),
                ]);
                return Ok(());
            }

            "backslash" => ordinary('\\'),

            ///////////////////
            // Miscellaneous //
            ///////////////////
            "#" | "%" | "&" | "$" | "_" => ordinary(
                control_sequence
                    .chars()
                    .next()
                    .expect("the control sequence contains one of the matched characters"),
            ),
            "|" => ordinary('∥'),
            "text" | "mbox" | "hbox" => return self.text_argument(),
            "textbf" => return self.text_argument_with_font(Some(Font::Bold)),
            "textit" | "emph" => return self.text_argument_with_font(Some(Font::Italic)),
            "textrm" | "textnormal" => return self.text_argument_with_font(Some(Font::UpRight)),
            "textsf" => return self.text_argument_with_font(Some(Font::SansSerif)),
            "texttt" => return self.text_argument_with_font(Some(Font::Monospace)),
            "textsl" => return self.text_argument_with_font(Some(Font::Italic)),
            "not" | "cancel" => {
                self.buffer.push(I::Event(E::Visual(V::Negation)));
                let argument = lex::argument(&mut self.content)?;
                self.handle_argument(argument)?;
                return Ok(());
            }
            "char" => {
                let number = lex::unsigned_integer(&mut self.content)?;
                if number > 255 {
                    return Err(ErrorKind::InvalidCharNumber);
                }
                E::Content(C::Ordinary {
                    content: char::from_u32(number as u32)
                        .expect("the number is a valid char since it is less than 256"),
                    stretchy: false,
                })
            }
            "relax" => {
                return if self.state.handling_argument {
                    Err(ErrorKind::Relax)
                } else {
                    Ok(())
                }
            }

            "begingroup" => {
                let group = lex::group_content(&mut self.content, GroupingKind::BeginEnd)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::SubGroup {
                        content: group,
                        allowed_alignment_count: None,
                        text_mode: self.state.text_mode,
                    },
                    I::Event(E::End),
                ]);
                return Ok(());
            }
            "endgroup" => return Err(ErrorKind::UnbalancedGroup(None)),

            "begin" => {
                let Argument::Group(argument) = lex::argument(&mut self.content)? else {
                    return Err(ErrorKind::Argument);
                };

                let mut style = None;
                let mut wrap: Option<(char, char)> = None;

                let (environment, align_count, grouping_kind) = match argument {
                    "array" => {
                        let (grouping, count) = self.array_environment()?;
                        (grouping, count, GroupingKind::Array { display: false })
                    }
                    "darray" => {
                        style = Some(S::Display);
                        let (grouping, count) = self.array_environment()?;
                        (grouping, count, GroupingKind::Array { display: true })
                    }
                    "matrix" => (
                        G::Matrix {
                            alignment: ColumnAlignment::Center,
                        },
                        u16::MAX,
                        GroupingKind::Matrix {
                            ty: MatrixType::Normal,
                            column_spec: false,
                        },
                    ),
                    "matrix*" => (
                        G::Matrix {
                            alignment: self
                                .optional_alignment()?
                                .unwrap_or(ColumnAlignment::Center),
                        },
                        u16::MAX,
                        GroupingKind::Matrix {
                            ty: MatrixType::Normal,
                            column_spec: true,
                        },
                    ),
                    "smallmatrix" => {
                        style = Some(S::Text);
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Small,
                                column_spec: false,
                            },
                        )
                    }
                    "pmatrix" => {
                        wrap = Some(('(', ')'));
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Parens,
                                column_spec: false,
                            },
                        )
                    }
                    "pmatrix*" => {
                        wrap = Some(('(', ')'));
                        (
                            G::Matrix {
                                alignment: self
                                    .optional_alignment()?
                                    .unwrap_or(ColumnAlignment::Center),
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Parens,
                                column_spec: true,
                            },
                        )
                    }
                    "bmatrix" => {
                        wrap = Some(('[', ']'));
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Brackets,
                                column_spec: false,
                            },
                        )
                    }
                    "bmatrix*" => {
                        wrap = Some(('[', ']'));
                        (
                            G::Matrix {
                                alignment: self
                                    .optional_alignment()?
                                    .unwrap_or(ColumnAlignment::Center),
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Brackets,
                                column_spec: true,
                            },
                        )
                    }
                    "vmatrix" => {
                        wrap = Some(('|', '|'));
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Vertical,
                                column_spec: false,
                            },
                        )
                    }
                    "vmatrix*" => {
                        wrap = Some(('|', '|'));
                        (
                            G::Matrix {
                                alignment: self
                                    .optional_alignment()?
                                    .unwrap_or(ColumnAlignment::Center),
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Vertical,
                                column_spec: true,
                            },
                        )
                    }
                    "Vmatrix" => {
                        wrap = Some(('‖', '‖'));
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::DoubleVertical,
                                column_spec: false,
                            },
                        )
                    }
                    "Vmatrix*" => {
                        wrap = Some(('‖', '‖'));
                        (
                            G::Matrix {
                                alignment: self
                                    .optional_alignment()?
                                    .unwrap_or(ColumnAlignment::Center),
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::DoubleVertical,
                                column_spec: true,
                            },
                        )
                    }
                    "Bmatrix" => {
                        wrap = Some(('{', '}'));
                        (
                            G::Matrix {
                                alignment: ColumnAlignment::Center,
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Braces,
                                column_spec: false,
                            },
                        )
                    }
                    "Bmatrix*" => {
                        wrap = Some(('{', '}'));
                        (
                            G::Matrix {
                                alignment: self
                                    .optional_alignment()?
                                    .unwrap_or(ColumnAlignment::Center),
                            },
                            u16::MAX,
                            GroupingKind::Matrix {
                                ty: MatrixType::Braces,
                                column_spec: true,
                            },
                        )
                    }
                    "cases" => (
                        G::Cases { left: true },
                        1,
                        GroupingKind::Cases {
                            left: true,
                            display: false,
                        },
                    ),
                    "dcases" => {
                        style = Some(S::Display);
                        (
                            G::Cases { left: true },
                            1,
                            GroupingKind::Cases {
                                left: true,
                                display: true,
                            },
                        )
                    }
                    "rcases" => (
                        G::Cases { left: false },
                        1,
                        GroupingKind::Cases {
                            left: false,
                            display: false,
                        },
                    ),
                    "drcases" => {
                        style = Some(S::Display);
                        (
                            G::Cases { left: false },
                            1,
                            GroupingKind::Cases {
                                left: false,
                                display: true,
                            },
                        )
                    }
                    "equation" => (
                        G::Equation { eq_numbers: true },
                        0,
                        GroupingKind::Equation { eq_numbers: true },
                    ),
                    "equation*" => (
                        G::Equation { eq_numbers: false },
                        0,
                        GroupingKind::Equation { eq_numbers: false },
                    ),
                    "align" => (
                        G::Align { eq_numbers: true },
                        u16::MAX,
                        GroupingKind::Align { eq_numbers: true },
                    ),
                    "align*" => (
                        G::Align { eq_numbers: false },
                        u16::MAX,
                        GroupingKind::Align { eq_numbers: false },
                    ),
                    "aligned" => (G::Aligned, u16::MAX, GroupingKind::Aligned),
                    "gather" => (
                        G::Gather { eq_numbers: true },
                        0,
                        GroupingKind::Gather { eq_numbers: true },
                    ),
                    "gather*" => (
                        G::Gather { eq_numbers: false },
                        0,
                        GroupingKind::Gather { eq_numbers: false },
                    ),
                    "gathered" => (G::Gathered, 0, GroupingKind::Gathered),
                    "alignat" => {
                        let pairs = match lex::argument(&mut self.content)? {
                            Argument::Group(mut content) => lex::unsigned_integer(&mut content),
                            _ => Err(ErrorKind::Argument),
                        }? as u16;
                        (
                            G::Alignat {
                                pairs,
                                eq_numbers: true,
                            },
                            (pairs * 2).saturating_sub(1),
                            GroupingKind::Alignat { eq_numbers: true },
                        )
                    }
                    "alignat*" => {
                        let pairs = match lex::argument(&mut self.content)? {
                            Argument::Group(mut content) => lex::unsigned_integer(&mut content),
                            _ => Err(ErrorKind::Argument),
                        }? as u16;
                        (
                            G::Alignat {
                                pairs,
                                eq_numbers: false,
                            },
                            (pairs * 2).saturating_sub(1),
                            GroupingKind::Alignat { eq_numbers: false },
                        )
                    }
                    "alignedat" => {
                        let pairs = match lex::argument(&mut self.content)? {
                            Argument::Group(mut content) => lex::unsigned_integer(&mut content),
                            _ => Err(ErrorKind::Argument),
                        }? as u16;
                        (
                            G::Alignedat { pairs },
                            (pairs * 2).saturating_sub(1),
                            GroupingKind::Alignedat,
                        )
                    }
                    "subarray" => {
                        let alignment = match lex::argument(&mut self.content)? {
                            Argument::Group("l") => ColumnAlignment::Left,
                            Argument::Group("c") => ColumnAlignment::Center,
                            Argument::Group("r") => ColumnAlignment::Right,
                            _ => return Err(ErrorKind::Argument),
                        };
                        (G::SubArray { alignment }, 0, GroupingKind::SubArray)
                    }
                    "multline" => (G::Multline, 0, GroupingKind::Multline),
                    "split" => (G::Split, 1, GroupingKind::Split),
                    _ => return Err(ErrorKind::Environment),
                };

                let wrap_used = if let Some((left, right)) = wrap {
                    self.buffer
                        .push(I::Event(E::Begin(G::LeftRight(Some(left), Some(right)))));
                    true
                } else {
                    false
                };

                let horizontal_lines = lex::horizontal_lines(&mut self.content);
                let content = lex::group_content(&mut self.content, grouping_kind)?;
                self.buffer.push(I::Event(E::Begin(environment)));
                if let Some(style) = style {
                    self.buffer.push(I::Event(E::StateChange(SC::Style(style))));
                }
                if horizontal_lines.len() > 0 {
                    self.buffer
                        .push(I::Event(E::EnvironmentFlow(EnvironmentFlow::StartLines {
                            lines: horizontal_lines,
                        })));
                }
                self.buffer.extend([
                    I::SubGroup {
                        content,
                        allowed_alignment_count: Some(AlignmentCount::new(align_count)),
                        text_mode: false,
                    },
                    I::Event(E::End),
                ]);

                if wrap_used {
                    self.buffer.push(I::Event(E::End));
                }
                return Ok(());
            }
            "end" => return Err(ErrorKind::UnbalancedGroup(None)),
            "\\" | "cr"
                if self.state.allowed_alignment_count.is_some()
                    && !self.state.handling_argument =>
            {
                self.state.allowed_alignment_count.as_mut().unwrap().reset();
                let additional_space =
                    if let Some(mut arg) = lex::optional_argument(&mut self.content) {
                        Some(lex::dimension(&mut arg)?)
                    } else {
                        None
                    };

                let horizontal_lines = lex::horizontal_lines(&mut self.content);
                E::EnvironmentFlow(EnvironmentFlow::NewLine {
                    spacing: additional_space,
                    horizontal_lines,
                })
            }
            "\\" | "cr" => return Err(ErrorKind::NewLine),

            // Delimiters
            cs if control_sequence_delimiter_map(cs).is_some() => {
                let (content, ty) = control_sequence_delimiter_map(cs).unwrap();
                E::Content(C::Delimiter {
                    content,
                    size: None,
                    ty,
                })
            }

            // Spacing
            c if c.trim_start().is_empty() => E::Content(C::Text("&nbsp;")),

            // Macros
            "def" => {
                let (cs, parameter_text, replacement_text) = lex::definition(&mut self.content)?;
                self.state.skip_scripts = true;
                return self
                    .macro_context
                    .define(cs, parameter_text, replacement_text);
            }
            "let" => {
                let (cs, token) = lex::let_assignment(&mut self.content)?;
                self.state.skip_scripts = true;
                self.macro_context.assign(cs, token);
                return Ok(());
            }
            "futurelet" => {
                let (cs, token, rest) = lex::futurelet_assignment(&mut self.content)?;
                self.state.skip_scripts = true;
                self.macro_context.assign(cs, token);
                self.content = rest;

                return Ok(());
            }
            "newcommand" => return self.new_command(Some(false)),
            "renewcommand" => return self.new_command(Some(true)),
            "providecommand" => return self.new_command(None),
            _ => return Err(ErrorKind::UnknownPrimitive),
        };
        self.buffer.push(I::Event(event));
        Ok(())
    }

    /// Return a delimiter with the given size from the next character in the parser.
    fn sized_delim(&mut self, size: DelimiterSize) -> InnerResult<()> {
        let current = &mut self.content;
        let (content, ty) = lex::delimiter(current)?;
        self.buffer.push(I::Event(E::Content(C::Delimiter {
            content,
            size: Some(size),
            ty,
        })));
        Ok(())
    }

    /// Implementation of the `\math<class>` atom-type commands (e.g. `\mathord`,
    /// `\mathrel`, `\mathbin`, `\mathop`, `\mathopen`, `\mathclose`, `\mathpunct`,
    /// `\mathinner`).
    ///
    /// These commands set the math class of their argument, which determines
    /// spacing in the rendered MathML output. When the argument is a single
    /// character, it is emitted as the matching [`Content`] variant. When the
    /// argument is a group, its contents are parsed normally and wrapped in
    /// an [`Event::Begin`]/[`Event::End`] pair so that the surrounding
    /// spacing is governed by the requested atom class.
    ///
    /// [`Content`]: crate::event::Content
    /// [`Event::Begin`]: crate::event::Event::Begin
    /// [`Event::End`]: crate::event::Event::End
    fn atom_group(&mut self, class: AtomClass) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;

        // Try to extract a single ASCII/Unicode character so we can emit a
        // directly-classified Content variant. This gives the most faithful
        // spacing for the common case (e.g. `\mathbin{+}`).
        let single_char: Option<char> = match argument {
            Argument::Token(Token::Character(char_)) => Some(char_.into()),
            Argument::Group(s) => {
                let s = s.trim();
                let mut chars = s.chars();
                let first = chars.next();
                match (first, chars.next()) {
                    (Some(c), None) => Some(c),
                    _ => None,
                }
            }
            Argument::Token(Token::ControlSequence(_)) => None,
        };

        if let Some(c) = single_char {
            let event = match class {
                AtomClass::Ord => ordinary(c),
                AtomClass::Op => E::Content(C::LargeOp {
                    content: c,
                    small: false,
                }),
                AtomClass::Bin => binary(c),
                AtomClass::Rel => relation(c),
                AtomClass::Open => E::Content(C::Delimiter {
                    content: c,
                    size: None,
                    ty: DelimiterType::Open,
                }),
                AtomClass::Close => E::Content(C::Delimiter {
                    content: c,
                    size: None,
                    ty: DelimiterType::Close,
                }),
                AtomClass::Punct => E::Content(C::Punctuation(c)),
                AtomClass::Inner => ordinary(c),
            };
            self.buffer.push(I::Event(event));
            return Ok(());
        }

        // Multi-character group (or a control sequence). For \mathop with a
        // textual group, emit it as a `Function` so it renders as a
        // multi-letter operator (mirroring `\operatorname`). Otherwise wrap
        // the parsed argument in a normal group; the inner content keeps its
        // own atom classes, but the whole construct presents as an
        // `Atom::Inner` for spacing purposes — which is what TeX does for
        // `\mathinner`, and is a reasonable approximation for the other
        // classes when given multi-token arguments.
        if matches!(class, AtomClass::Op) {
            if let Argument::Group(content) = argument {
                self.buffer.push(I::Event(E::Content(C::Function(content))));
                return Ok(());
            }
        }

        self.buffer.push(I::Event(E::Begin(G::Normal)));
        self.handle_argument(argument)?;
        self.buffer.push(I::Event(E::End));
        Ok(())
    }

    /// Override the `font_state` for the argument to the command.
    fn font_group(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        self.buffer.extend([
            I::Event(E::Begin(G::Normal)),
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
                self.buffer.push(I::SubGroup {
                    content: group,
                    allowed_alignment_count: None,
                    text_mode: self.state.text_mode,
                });
            }
        };
        self.buffer.push(I::Event(E::End));
        Ok(())
    }

    /// Accent commands. parse the argument, and overset the accent.
    fn accent(&mut self, accent: char, stretchy: bool) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        self.buffer.push(I::Event(E::Script {
            ty: ST::Superscript,
            position: SP::AboveBelow,
        }));
        self.handle_argument(argument)?;
        self.buffer.push(I::Event(E::Content(C::Ordinary {
            content: accent,
            stretchy,
        })));
        Ok(())
    }

    /// Underscript commands. parse the argument, and underset the accent.
    fn underscript(&mut self, content: char) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        self.buffer.push(I::Event(E::Script {
            ty: ST::Subscript,
            position: SP::AboveBelow,
        }));
        self.handle_argument(argument)?;
        self.buffer.push(I::Event(E::Content(C::Ordinary {
            content,
            stretchy: true,
        })));

        Ok(())
    }

    fn large_op(&mut self, op: char, movable: bool) -> E<'store> {
        self.state.allow_script_modifiers = true;
        self.state.script_position = if movable { SP::Movable } else { SP::Right };
        E::Content(C::LargeOp {
            content: op,
            small: false,
        })
    }

    fn font_change(&mut self, font: Font) -> E<'store> {
        self.state.skip_scripts = true;
        E::StateChange(SC::Font(Some(font)))
    }

    fn style_change(&mut self, style: S) -> E<'store> {
        self.state.skip_scripts = true;
        E::StateChange(SC::Style(style))
    }

    /// Like [`text_argument`], but wraps the text-mode content in a font state
    /// change. Used for `\textbf`, `\textit`, etc. when encountered in math mode.
    fn text_argument_with_font(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        let content = match argument {
            Argument::Token(Token::Character(c)) => c.as_str(),
            Argument::Group(inner) => inner,
            _ => return Err(ErrorKind::ControlSequenceAsArgument),
        };
        self.buffer.extend([
            I::Event(E::Begin(G::Text)),
            I::Event(E::StateChange(SC::Font(font))),
            I::SubGroup {
                content,
                allowed_alignment_count: None,
                text_mode: true,
            },
            I::Event(E::End),
        ]);
        Ok(())
    }

    /// Parse the argument to `\text{...}` (or another text-mode command), and
    /// emit a [`Grouping::Text`] enclosing a text-mode parse of its contents.
    fn text_argument(&mut self) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        let content = match argument {
            Argument::Token(Token::Character(c)) => c.as_str(),
            Argument::Group(inner) => inner,
            _ => return Err(ErrorKind::ControlSequenceAsArgument),
        };
        self.buffer.extend([
            I::Event(E::Begin(G::Text)),
            I::SubGroup {
                content,
                allowed_alignment_count: None,
                text_mode: true,
            },
            I::Event(E::End),
        ]);
        Ok(())
    }

    /// Parse a single element while in text mode and push its events to the buffer.
    ///
    /// In text mode, runs of literal characters are coalesced into a single
    /// [`Content::Text`] event. The `\` character starts a control sequence,
    /// `$` introduces an embedded math-mode group via [`Grouping::InlineMath`],
    /// and `~` produces a non-breaking space.
    pub(super) fn parse_text_element(&mut self) -> InnerResult<()> {
        if self.content.is_empty() {
            return Ok(());
        }

        let bytes = self.content.as_bytes();
        let first = bytes[0];

        // A run of literal characters is consumed up to the next special byte.
        // `{` and `}` also terminate a run because they begin/end a sub-group.
        if !matches!(first, b'\\' | b'$' | b'~' | b'{' | b'}' | b'%') {
            let mut idx = 1;
            while idx < bytes.len()
                && !matches!(bytes[idx], b'\\' | b'$' | b'~' | b'{' | b'}' | b'%')
            {
                idx += 1;
            }
            let (run, rest) = self.content.split_at(idx);
            self.content = rest;
            self.buffer.push(I::Event(E::Content(C::Text(run))));
            return Ok(());
        }

        match first {
            b'\\' => {
                self.content = &self.content[1..];
                let cs = lex::rhs_control_sequence(&mut self.content)?;
                self.handle_text_primitive(cs)?;
            }
            b'$' => {
                self.content = &self.content[1..];
                let math_content = lex::until_unescaped_dollar(&mut self.content)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::InlineMath)),
                    I::SubGroup {
                        content: math_content,
                        allowed_alignment_count: None,
                        text_mode: false,
                    },
                    I::Event(E::End),
                ]);
            }
            b'{' => {
                self.content = &self.content[1..];
                let group = lex::group_content(&mut self.content, GroupingKind::Normal)?;
                self.buffer.extend([
                    I::Event(E::Begin(G::Normal)),
                    I::SubGroup {
                        content: group,
                        allowed_alignment_count: None,
                        text_mode: true,
                    },
                    I::Event(E::End),
                ]);
            }
            b'}' => return Err(ErrorKind::UnbalancedGroup(None)),
            b'~' => {
                self.content = &self.content[1..];
                self.buffer.push(I::Event(E::Content(C::Text("&nbsp;"))));
            }
            b'%' => {
                // A comment runs to the end of the line; skip it.
                let bytes = self.content.as_bytes();
                let after = bytes
                    .iter()
                    .position(|&c| c == b'\n')
                    .map(|i| i + 1)
                    .unwrap_or(bytes.len());
                self.content = &self.content[after..];
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Handle a control sequence that appears in text mode. Falls back to a
    /// literal text rendering for unknown commands rather than erroring.
    fn handle_text_primitive(&mut self, cs: &'store str) -> InnerResult<()> {
        match cs {
            // Font-changing commands. They are scoped to their argument by
            // wrapping a font state change inside a `Grouping::Normal`.
            "textbf" => return self.text_font_group(Some(Font::Bold)),
            "textit" | "emph" => return self.text_font_group(Some(Font::Italic)),
            "textrm" | "textnormal" => return self.text_font_group(Some(Font::UpRight)),
            "textsf" => return self.text_font_group(Some(Font::SansSerif)),
            "texttt" => return self.text_font_group(Some(Font::Monospace)),
            "textsl" => return self.text_font_group(Some(Font::Italic)),

            // Escaped specials: emit the literal character.
            "#" | "$" | "&" | "%" | "_" | "{" | "}" => {
                self.buffer.push(I::Event(E::Content(C::Text(cs))));
            }
            "textbackslash" => {
                self.buffer.push(I::Event(E::Content(C::Text("\\"))));
            }

            // Brand commands.
            "LaTeX" => self.buffer.push(I::Event(E::Content(C::Text("LaTeX")))),
            "TeX" => self.buffer.push(I::Event(E::Content(C::Text("TeX")))),

            // Spacing commands.
            "," => self.buffer.push(I::Event(E::Space {
                width: Some(Dimension::new(3.0 / 18.0, DimensionUnit::Em)),
                height: None,
            })),
            ":" | ">" => self.buffer.push(I::Event(E::Space {
                width: Some(Dimension::new(4.0 / 18.0, DimensionUnit::Em)),
                height: None,
            })),
            ";" => self.buffer.push(I::Event(E::Space {
                width: Some(Dimension::new(5.0 / 18.0, DimensionUnit::Em)),
                height: None,
            })),
            "!" => self.buffer.push(I::Event(E::Space {
                width: Some(Dimension::new(-3.0 / 18.0, DimensionUnit::Em)),
                height: None,
            })),
            " " => self.buffer.push(I::Event(E::Content(C::Text(" ")))),

            // Symbols.
            "dag" | "dagger" => self.buffer.push(I::Event(E::Content(C::Text("\u{2020}")))),
            "ddag" | "ddagger" => self.buffer.push(I::Event(E::Content(C::Text("\u{2021}")))),
            "S" => self.buffer.push(I::Event(E::Content(C::Text("\u{00A7}")))),
            "P" => self.buffer.push(I::Event(E::Content(C::Text("\u{00B6}")))),
            "copyright" => self.buffer.push(I::Event(E::Content(C::Text("\u{00A9}")))),
            "pounds" => self.buffer.push(I::Event(E::Content(C::Text("\u{00A3}")))),

            // Unknown control sequence: emit it as literal text so the renderer
            // does not silently lose the input.
            _ => {
                self.buffer.push(I::Event(E::Content(C::Text(cs))));
            }
        }
        Ok(())
    }

    /// Wrap a text-mode argument in a `Grouping::Normal` scoped font change.
    fn text_font_group(&mut self, font: Option<Font>) -> InnerResult<()> {
        let argument = lex::argument(&mut self.content)?;
        self.buffer.extend([
            I::Event(E::Begin(G::Normal)),
            I::Event(E::StateChange(SC::Font(font))),
        ]);
        let content = match argument {
            Argument::Token(Token::Character(c)) => c.as_str(),
            Argument::Group(inner) => inner,
            _ => return Err(ErrorKind::ControlSequenceAsArgument),
        };
        self.buffer.push(I::SubGroup {
            content,
            allowed_alignment_count: None,
            text_mode: true,
        });
        self.buffer.push(I::Event(E::End));
        Ok(())
    }

    fn fraction_like(
        &mut self,
        open: Option<char>,
        close: Option<char>,
        bar_size: Option<Dimension>,
        style: Option<S>,
    ) -> InnerResult<()> {
        let open_close_group = open.is_some() || close.is_some();
        if open_close_group {
            self.buffer
                .push(I::Event(E::Begin(G::LeftRight(open, close))));
        }
        if let Some(style) = style {
            if !open_close_group {
                self.buffer.push(I::Event(E::Begin(G::Normal)));
            }
            self.buffer.push(I::Event(E::StateChange(SC::Style(style))));
        };

        self.buffer.push(I::Event(E::Visual(V::Fraction(bar_size))));
        let numerator = lex::argument(&mut self.content)?;
        self.handle_argument(numerator)?;
        let denominator = lex::argument(&mut self.content)?;
        self.handle_argument(denominator)?;
        if open_close_group || style.is_some() {
            self.buffer.push(I::Event(E::End));
        }

        Ok(())
    }

    fn array_environment(&mut self) -> InnerResult<(G, u16)> {
        let Argument::Group(array_columns_str) = lex::argument(&mut self.content)? else {
            return Err(ErrorKind::Argument);
        };

        let mut column_count: u16 = 0;
        let mut contains_column = false;
        let array_columns = array_columns_str
            .chars()
            .map(|c| {
                column_count += 1;
                Ok(match c {
                    'c' => {
                        contains_column = true;
                        AC::Column(ColumnAlignment::Center)
                    }
                    'l' => {
                        contains_column = true;
                        AC::Column(ColumnAlignment::Left)
                    }
                    'r' => {
                        contains_column = true;
                        AC::Column(ColumnAlignment::Right)
                    }
                    '|' => {
                        column_count -= 1;
                        AC::Separator(Line::Solid)
                    }
                    ':' => {
                        column_count -= 1;
                        AC::Separator(Line::Dashed)
                    }
                    _ => return Err(ErrorKind::Argument),
                })
            })
            .collect::<Result<_, _>>()?;

        if !contains_column {
            return Err(ErrorKind::ArrayNoColumns);
        }

        Ok((G::Array(array_columns), column_count.saturating_sub(1)))
    }

    fn optional_alignment(&mut self) -> InnerResult<Option<ColumnAlignment>> {
        let alignment = lex::optional_argument(&mut self.content);
        Ok(match alignment {
            Some("c") => Some(ColumnAlignment::Center),
            Some("l") => Some(ColumnAlignment::Left),
            Some("r") => Some(ColumnAlignment::Right),
            None => None,
            _ => return Err(ErrorKind::Argument),
        })
    }

    fn new_command(&mut self, should_already_exist: Option<bool>) -> InnerResult<()> {
        let mut group = lex::brace_argument(&mut self.content)?;
        let cs = lex::control_sequence(&mut group)?;

        if should_already_exist.is_some_and(|sae| sae != self.macro_context.contains(cs)) {
            return Err(if should_already_exist.unwrap() {
                ErrorKind::MacroNotDefined
            } else {
                ErrorKind::MacroAlreadyDefined
            });
        }

        let arg_count = (lex::optional_argument(&mut self.content).ok_or(ErrorKind::Argument)?)
            .parse::<u8>()
            .map_err(|_| ErrorKind::Number)?;
        let first_arg_default = lex::optional_argument(&mut self.content);
        if arg_count > 9 && arg_count >= first_arg_default.is_some() as u8 {
            return Err(ErrorKind::TooManyParams);
        }

        let replacement_text = lex::brace_argument(&mut self.content)?;

        if self.macro_context.contains(cs) && should_already_exist.is_none() {
            return Ok(());
        }
        self.macro_context
            .insert_command(cs, arg_count, first_arg_default, replacement_text)?;
        Ok(())
    }
}

#[inline]
fn ordinary(ident: char) -> E<'static> {
    E::Content(C::Ordinary {
        content: ident,
        stretchy: false,
    })
}

#[inline]
fn relation(rel: char) -> E<'static> {
    E::Content(C::Relation {
        content: RelationContent::single_char(rel),
        small: false,
    })
}

fn multirelation(first: char, second: char) -> E<'static> {
    E::Content(C::Relation {
        content: RelationContent::double_char(first, second),
        small: false,
    })
}

#[inline]
fn binary(op: char) -> E<'static> {
    E::Content(C::BinaryOp {
        content: op,
        small: false,
    })
}

/// Math atom classes used by the `\math<class>` family of commands.
///
/// These mirror the TeXbook's eight atom classes and are used by
/// [`InnerParser::atom_group`] to pick an appropriate [`Content`] variant
/// for the argument.
///
/// [`Content`]: crate::event::Content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomClass {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

// TODO implementations:
// - `raise`, `lower`
// - `hbox`, `mbox`?
// - `vcenter`
// - `rule`
// - `mathchoice` (TeXbook p. 151)

// Unimplemented primitives:
// `sl` (slanted) font: https://tug.org/texinfohtml/latex2e.html#index-_005csl
// `bbit` (double-struck italic) font
// `symliteral` wtf is this? (in unicode-math)
// `sc` (small caps) font: https://tug.org/texinfohtml/latex2e.html#index-_005csc
