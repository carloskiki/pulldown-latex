//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use crate::{
    attribute::{DimensionUnit, Font},
    event::{Content, Event, Identifier, Infix, Operator},
    parse::{lex, GroupNesting, GroupType, Parser, Result},
    Argument, Token,
};

use super::{
    operator_table::{is_delimiter, is_operator},
    Instruction, ParseError,
};

// Return a `Content::Identifier` event with the given content and font variant.
macro_rules! ident {
    ($content:expr) => {
        Event::Content(Content::Identifier(Identifier::Char {
            content: $content,
            variant: None,
        }))
    };
    ($content:expr, $self_:ident) => {
        Event::Content(Content::Identifier(Identifier::Char {
            content: $content,
            variant: $self_.current_group().font_state,
        }))
    };
}

/// Return an `Operator` event with the given content and default modifiers.
macro_rules! op {
    ($content:expr) => {
        Event::Content(Content::Operator(Operator {
            content: $content,
            ..Default::default()
        }))
    };
    ($content:expr, {$($field:ident: $value:expr),*}) => {
        Event::Content(Content::Operator(Operator {
            content: $content,
            $($field: $value,)*
            ..Default::default()
        }))
    };
}

/// Return a delimiter with the given size from the next character in the parser.
macro_rules! sized_delim {
    ($size:literal, $self_:ident) => {{
        let delimiter = lex::delimiter($self_.current_string()?)?;
        Event::Content(Content::Operator(Operator {
            content: delimiter,
            stretchy: None,
            moveable_limits: None,
            left_space: None,
            right_space: None,
            size: Some(($size, DimensionUnit::Em)),
        }))
    }};
}

/// Override the `font_state` to the given font variant.
macro_rules! font_override {
    ($font:ident, $self_:ident) => {{
        $self_.current_group_mut().font_state = Some(Font::$font);
        $self_.next_unwrap()?
    }};
}

/// Override the `font_state` for the argument to the command.
macro_rules! font_group {
    ($self_:ident) => {{
        let argument = lex::argument($self_.current_string()?)?;
        match argument {
            Argument::Token(Token::Character(c)) => $self_.handle_char_token(c)?,
            Argument::Token(Token::ControlSequence(cs)) => $self_.handle_primitive(cs)?,
            Argument::Group(g) => {
                $self_.instruction_stack.push(Instruction::Substring {
                    content: g,
                    pop_internal_group: true,
                });
                $self_.group_stack.push(GroupNesting {
                    font_state: None,
                    group_type: GroupType::Internal,
                });
                $self_.next_unwrap()?
            }
        }
    }};
    ($font:ident, $self_:ident) => {{
        let argument = lex::argument($self_.current_string()?)?;
        match argument {
            Argument::Token(Token::Character(c)) => $self_.handle_char_token(c)?,
            Argument::Token(Token::ControlSequence(cs)) => $self_.handle_primitive(cs)?,
            Argument::Group(g) => {
                $self_.instruction_stack.push(Instruction::Substring {
                    content: g,
                    pop_internal_group: true,
                });
                $self_.group_stack.push(GroupNesting {
                    font_state: Some(Font::$font),
                    group_type: GroupType::Internal,
                });
                $self_.next_unwrap()?
            }
        }
    }};
}

/// Accent commands. parse the argument, and overset the accent.
macro_rules! accent {
    ($accent:literal, $self_:ident) => {{
        accent!($accent, $self_, {})
    }};
    ($accent:literal, $self_:ident, $opts:tt) => {{
        let argument = lex::argument($self_.current_string()?)?;
        $self_.instruction_stack.extend([
            Instruction::Event(op!($accent, $opts)),
            Instruction::Event(Event::Infix(Infix::Overscript)),
        ]);
        match argument {
            Argument::Token(Token::Character(c)) => $self_.handle_char_token(c)?,
            Argument::Token(Token::ControlSequence(cs)) => $self_.handle_primitive(cs)?,
            Argument::Group(substr) => {
                $self_
                    .instruction_stack
                    .push(Instruction::Event(Event::EndGroup));
                $self_.instruction_stack.push(Instruction::Substring {
                    content: substr,
                    pop_internal_group: false,
                });
                Event::BeginGroup
            }
        }
    }};
}

/// Underscript commands. parse the argument, and underset the accent.
macro_rules! underscript {
    ($accent:literal, $self_:ident) => {{
        let argument = lex::argument($self_.current_string()?)?;
        $self_.instruction_stack.extend([
            Instruction::Event(op!($accent)),
            Instruction::Event(Event::Infix(Infix::Underscript)),
        ]);
        match argument {
            Argument::Token(Token::Character(c)) => $self_.handle_char_token(c)?,
            Argument::Token(Token::ControlSequence(cs)) => $self_.handle_primitive(cs)?,
            Argument::Group(substr) => {
                $self_
                    .instruction_stack
                    .push(Instruction::Event(Event::EndGroup));
                $self_.instruction_stack.push(Instruction::Substring {
                    content: substr,
                    pop_internal_group: false,
                });
                Event::BeginGroup
            }
        }
    }};
}

impl<'a> Parser<'a> {
    /// Handle a character token, returning a corresponding event.
    ///
    /// This function specially treats numbers as `mi`.
    ///
    /// ## Panics
    /// - This function will panic if the `\` character is given
    pub fn handle_char_token(&mut self, token: char) -> Result<Event<'a>> {
        // TODO: bracket table
        Ok(match token {
            '\\' => panic!("this function does not handle control sequences"),
            '%' => {
                let content = self.current_string()?;
                if let Some((_, rest)) = content.split_once('\n') {
                    *content = rest;
                } else {
                    *content = "";
                };
                return self.next_unwrap();
            }
            '{' => {
                self.group_stack.push(GroupNesting {
                    font_state: self.current_group().font_state,
                    group_type: GroupType::Brace,
                });
                Event::BeginGroup
            },
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
            },
            '_' => Event::Infix(Infix::Subscript),
            '^' => Event::Infix(Infix::Superscript),
            '$' => return Err(ParseError::MathShift),
            '#' => return Err(ParseError::HashSign),
            '&' => return Err(ParseError::AlignmentChar),
            // TODO: check for double and triple primes
            '\'' => op!('′'),

            c if is_delimiter(c) => op!(c, {stretchy: Some(false)}),
            c if is_operator(c) => op!(c),
            // TODO: handle every character correctly.
            c => ident!(c),
        })
    }

    /// Handle a control sequence, returning a corresponding event.
    ///
    /// 1. If the control sequence is user defined, apply the corresponding definition.
    /// 2. If the event is a primitive, apply the corresponding primitive.
    /// 3. If the control sequence is not defined, return an error.
    pub fn handle_primitive(&mut self, control_sequence: &'a str) -> Result<Event<'a>> {
        Ok(match control_sequence {
            "#" | "%" | "&" | "$" | "_" => Event::Content(Content::Identifier(Identifier::Char {
                content: control_sequence.chars().next().unwrap(),
                variant: None,
            })),
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
            "alpha" => ident!('α', self),
            "beta" => ident!('β', self),
            "gamma" => ident!('γ', self),
            "delta" => ident!('δ', self),
            "epsilon" => ident!('ϵ', self),
            "varepsilon" => ident!('ε', self),
            "zeta" => ident!('ζ', self),
            "eta" => ident!('η', self),
            "theta" => ident!('θ', self),
            "vartheta" => ident!('ϑ', self),
            "iota" => ident!('ι', self),
            "kappa" => ident!('κ', self),
            "lambda" => ident!('λ', self),
            "mu" => ident!('µ', self),
            "nu" => ident!('ν', self),
            "xi" => ident!('ξ', self),
            "pi" => ident!('π', self),
            "varpi" => ident!('ϖ', self),
            "rho" => ident!('ρ', self),
            "varrho" => ident!('ϱ', self),
            "sigma" => ident!('σ', self),
            "varsigma" => ident!('ς', self),
            "tau" => ident!('τ', self),
            "upsilon" => ident!('υ', self),
            "phi" => ident!('φ', self),
            "varphi" => ident!('ϕ', self),
            "chi" => ident!('χ', self),
            "psi" => ident!('ψ', self),
            "omega" => ident!('ω', self),
            // Uppercase Greek letters
            "Alpha" => ident!('Α', self),
            "Beta" => ident!('Β', self),
            "Gamma" => ident!('Γ', self),
            "Delta" => ident!('Δ', self),
            "Epsilon" => ident!('Ε', self),
            "Zeta" => ident!('Ζ', self),
            "Eta" => ident!('Η', self),
            "Theta" => ident!('Θ', self),
            "Iota" => ident!('Ι', self),
            "Kappa" => ident!('Κ', self),
            "Lambda" => ident!('Λ', self),
            "Mu" => ident!('Μ', self),
            "Nu" => ident!('Ν', self),
            "Xi" => ident!('Ξ', self),
            "Pi" => ident!('Π', self),
            "Rho" => ident!('Ρ', self),
            "Sigma" => ident!('Σ', self),
            "Tau" => ident!('Τ', self),
            "Upsilon" => ident!('Υ', self),
            "Phi" => ident!('Φ', self),
            "Chi" => ident!('Χ', self),
            "Psi" => ident!('Ψ', self),
            "Omega" => ident!('Ω', self),
            // Hebrew letters
            "aleph" => ident!('ℵ'),
            "beth" => ident!('ℶ'),
            "gimel" => ident!('ℷ'),
            "daleth" => ident!('ℸ'),
            // Other symbols
            "eth" => ident!('ð'),
            "ell" => ident!('ℓ'),
            "nabla" => ident!('∇'),
            "partial" => ident!('⅁'),
            "Finv" => ident!('Ⅎ'),
            "Game" => ident!('ℷ'),
            "hbar" | "hslash" => ident!('ℏ'),
            "imath" => ident!('ı'),
            "jmath" => ident!('ȷ'),
            "Im" => ident!('ℑ'),
            "Re" => ident!('ℜ'),
            "wp" => ident!('℘'),
            "Bbbk" => ident!('𝕜'),
            "Angstrom" => ident!('Å'),

            ////////////////////////
            // Font state changes //
            ////////////////////////
            // LaTeX native absolute font changes (old behavior a.k.a NFSS 1)
            "bf" => font_override!(Bold, self),
            "cal" => font_override!(Script, self),
            "it" => font_override!(Italic, self),
            "rm" => font_override!(UpRight, self),
            "sf" => font_override!(SansSerif, self),
            "tt" => font_override!(Monospace, self),
            // amsfonts font changes (old behavior a.k.a NFSS 1)
            // unicode-math font changes (old behavior a.k.a NFSS 1)
            // TODO: Make it so that there is a different between `\sym_` and `\math_` font
            // changes, as described in https://mirror.csclub.uwaterloo.ca/CTAN/macros/unicodetex/latex/unicode-math/unicode-math.pdf
            // (section. 3.1)
            "mathbf" | "symbf" | "mathbfup" | "symbfup" => font_group!(Bold, self),
            "mathcal" | "symcal" | "mathup" | "symup" => font_group!(Script, self),
            "mathit" | "symit" => font_group!(Italic, self),
            "mathrm" | "symrm" => font_group!(UpRight, self),
            "mathsf" | "symsf" | "mathsfup" | "symsfup" => font_group!(SansSerif, self),
            "mathtt" | "symtt" => font_group!(Monospace, self),
            "mathbb" | "symbb" => font_group!(DoubleStruck, self),
            "mathfrak" | "symfrak" => font_group!(Fraktur, self),
            "mathbfcal" | "symbfcal" => font_group!(BoldScript, self),
            "mathsfit" | "symsfit" => font_group!(SansSerifItalic, self),
            "mathbfit" | "symbfit" => font_group!(BoldItalic, self),
            "mathbffrak" | "symbffrak" => font_group!(BoldFraktur, self),
            "mathbfsfup" | "symbfsfup" => font_group!(BoldSansSerif, self),
            "mathbfsfit" | "symbfsfit" => font_group!(SansSerifBoldItalic, self),
            "mathnormal" | "symnormal" => font_group!(self),

            "|" => ident!('∥'),
            "angle" => ident!('∠'),

            "approx" => op!('≈'),
            "approxeq" => op!('≊'),
            "approxcolon" => {
                self.instruction_stack.push(Instruction::Event(op! {
                    ':',
                    {left_space: Some((0., DimensionUnit::Em))}
                }));
                op! {
                    '≈',
                    {right_space: Some((0., DimensionUnit::Em))}
                }
            }
            "approxcoloncolon" => {
                self.instruction_stack.push(Instruction::Event(
                    op! {':', {left_space: Some((0., DimensionUnit::Em))}},
                ));
                self.instruction_stack.push(Instruction::Event(op! {
                    ':',
                    {
                        left_space: Some((0., DimensionUnit::Em)),
                        right_space: Some((0., DimensionUnit::Em))
                    }
                }));
                op! {
                    '≈',
                    {right_space: Some((0., DimensionUnit::Em))}
                }
            }
            "ast" => op!('*'),
            "asymp" => op!('≍'),
            "amalg" => op!('⨿'),
            "And" => op!('&'),

            "backepsilon" => ident!('϶'),
            "backsim" => op!('∽'),
            "backsimeq" => op!('⋍'),
            "backslash" => ident!('\\'),
            "barwedge" => op!('⌅'),
            "because" => op!('∵'),
            "between" => op!('≬'),

            //////////////////////////////
            // Delimiter size modifiers //
            //////////////////////////////
            // Sizes taken from `texzilla`
            // Big left and right seem to not care about which delimiter is used. i.e., \bigl) and \bigr) are the same.
            "big" | "bigl" | "bigr" | "bigm" => sized_delim!(1.2, self),
            "Big" | "Bigl" | "Bigr" | "Bigm" => sized_delim!(1.8, self),
            "bigg" | "biggl" | "biggr" | "biggm" => sized_delim!(2.4, self),
            "Bigg" | "Biggl" | "Biggr" | "Biggm" => sized_delim!(3.0, self),

            // TODO: maybe use something else than an internal group for this?
            "left" => {
                let curr_str = self.current_string()?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    return self.next_unwrap();
                }
                let delimiter = lex::delimiter(self.current_string()?)?;
                self.group_stack.push(GroupNesting {
                    font_state: None,
                    group_type: GroupType::Internal,
                });
                op!(delimiter)
            }
            "middle" => {
                let delimiter = lex::delimiter(self.current_string()?)?;
                op!(delimiter)
            }
            "right" => {
                let curr_str = self.current_string()?;
                if let Some(rest) = curr_str.strip_prefix('.') {
                    *curr_str = rest;
                    return self.next_unwrap();
                }
                let delimiter = lex::delimiter(curr_str)?;
                let group = self.group_stack.pop();
                assert!(matches!(
                    group,
                    Some(GroupNesting {
                        group_type: GroupType::Internal,
                        ..
                    })
                ));

                op!(delimiter)
            }

            ///////////////////
            // Big Operators //
            ///////////////////
            "sum" => op!('∑'),
            "prod" => op!('∏'),
            "coprod" => op!('∐'),
            "int" => op!('∫'),
            "iint" => op!('∬'),
            "intop" => op!('∫'),
            "iiint" => op!('∭'),
            "smallint" => op!('∫'),
            "iiiint" => op!('⨌'),
            "intcap" => op!('⨙'),
            "intcup" => op!('⨚'),
            "oint" => op!('∮'),
            "varointclockwise" => op!('∲'),
            "intclockwise" => op!('∱'),
            "oiint" => op!('∯'),
            "pointint" => op!('⨕'),
            "rppolint" => op!('⨒'),
            "scpolint" => op!('⨓'),
            "oiiint" => op!('∰'),
            "intlarhk" => op!('⨗'),
            "sqint" => op!('⨖'),
            "intx" => op!('⨘'),
            "intbar" => op!('⨍'),
            "intBar" => op!('⨎'),
            "fint" => op!('⨏'),
            "bigoplus" => op!('⨁'),
            "bigotimes" => op!('⨂'),
            "bigvee" => op!('⋁'),
            "bigwedge" => op!('⋀'),
            "bigodot" => op!('⨀'),
            "bigcap" => op!('⋂'),
            "biguplus" => op!('⨄'),
            "bigcup" => op!('⋃'),
            "bigsqcup" => op!('⨆'),
            "bigsqcap" => op!('⨅'),
            "bigtimes" => op!('⨉'),

            /////////////
            // Accents //
            /////////////
            "acute" => accent!('´', self),
            "bar" | "overline" => accent!('‾', self),
            "underbar" | "underline" => underscript!('_', self),
            "breve" => accent!('˘', self),
            "check" => accent!('ˇ', self, {stretchy: Some(false)}),
            "dot" => accent!('˙', self),
            "ddot" => accent!('¨', self),
            "grave" => accent!('`', self),
            "hat" => accent!('^', self, {stretchy: Some(false)}),
            "tilde" => accent!('~', self, {stretchy: Some(false)}),
            "vec" => accent!('→', self, {stretchy: Some(false)}),
            "mathring" => accent!('˚', self),

            // Arrows
            "overleftarrow" => accent!('←', self),
            "underleftarrow" => underscript!('←', self),
            "overrightarrow" => accent!('→', self),
            "Overrightarrow" => accent!('⇒', self),
            "underrightarrow" => underscript!('→', self),
            "overleftrightarrow" => accent!('↔', self),
            "underleftrightarrow" => underscript!('↔', self),
            "overleftharpoon" => accent!('↼', self),
            "overrightharpoon" => accent!('⇀', self),

            // Wide accents
            "widecheck" => accent!('ˇ', self),
            "widehat" => accent!('^', self),
            "widetilde" => accent!('~', self),
            "wideparen" | "overparen" => accent!('⏜', self),

            // Groups
            "overgroup" => accent!('⏠', self),
            "undergroup" => underscript!('⏡', self),
            "overbrace" => accent!('⏞', self),
            "underbrace" => underscript!('⏟', self),
            "underparen" => underscript!('⏝', self),

            // Primes
            "prime" => op!('′'),
            "dprime" => op!('″'),
            "trprime" => op!('‴'),
            "qprime" => op!('⁗'),
            "backprime" => op!('‵'),
            "backdprime" => op!('‶'),
            "backtrprime" => op!('‷'),

            _ => todo!(),
        })
    }
}

// TODO implementations:
//
// `begingroup` and `endgroup`: https://tex.stackexchange.com/a/191533
// `sc` (small caps) font: https://tug.org/texinfohtml/latex2e.html#index-_005csc

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
