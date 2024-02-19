//! A module that implements the behavior of every primitive of the supported LaTeX syntax. This
//! includes every primitive macro and active character.

use crate::{
    ast::{Content, Event, Grouping, Identifier, Infix},
    attribute::{tex_to_css_units, DimensionUnit},
    parse::{Parser, Result}, Argument, Token,
};

use super::Instruction;

macro_rules! ident {
    ($content:literal) => {
        Event::Content(Content::Identifier(Identifier::Char {
            content: $content,
            is_normal: false,
        }))
    };
    ($content:literal, $normal:literal) => {
        Event::Content(Content::Identifier(Identifier::Char {
            content: $content,
            is_normal: $normal,
        }))
    };
}

macro_rules! op {
    ($content:literal) => {
        Event::Content(Content::Operator {
            content: $content,
            stretchy: None,
            moveable_limits: None,
            left_space: None,
            right_space: None,
        })
    };
}

impl<'a> Parser<'a> {
    /// Handle a control sequence, returning a corresponding event.
    ///
    /// 1. If the control sequence is user defined, apply the corresponding definition.
    /// 2. If the event is a primitive, apply the corresponding primitive.
    /// 3. If the control sequence is not defined, return an error.
    pub fn handle_primitive(&mut self, control_sequence: &'a str) -> Result<Event<'a>> {
        dbg!(control_sequence);
        Ok(match control_sequence {
            "#" | "%" | "&" | "$" | "_" => Event::Content(Content::Identifier(Identifier::Char {
                content: control_sequence.chars().next().unwrap(),
                is_normal: true,
            })),
            "arccos" | "cos" | "csc" | "exp" | "ker" | "sinh" | "arcsin" | "cosh" | "deg"
            | "lg" | "ln" | "arctan" | "cot" | "det" | "hom" | "log" | "sec" | "tan" | "arg"
            | "coth" | "dim" | "sin" | "tanh" => {
                Event::Content(Content::Identifier(Identifier::Str(control_sequence)))
            }
            // TODO: The following have `under` subscripts in display math: Pr sup liminf max inf gcd limsup min

            // Lowercase Greek letters
            "alpha" => ident!('α'),
            "beta" => ident!('β'),
            "gamma" => ident!('γ'),
            "delta" => ident!('δ'),
            "epsilon" => ident!('ϵ'),
            "varepsilon" => ident!('ε'),
            "zeta" => ident!('ζ'),
            "eta" => ident!('η'),
            "theta" => ident!('θ'),
            "vartheta" => ident!('ϑ'),
            "iota" => ident!('ι'),
            "kappa" => ident!('κ'),
            "lambda" => ident!('λ'),
            "mu" => ident!('µ'),
            "nu" => ident!('ν'),
            "xi" => ident!('ξ'),
            "pi" => ident!('π'),
            "varpi" => ident!('ϖ'),
            "rho" => ident!('ρ'),
            "varrho" => ident!('ϱ'),
            "sigma" => ident!('σ'),
            "varsigma" => ident!('ς'),
            "tau" => ident!('τ'),
            "upsilon" => ident!('υ'),
            "phi" => ident!('φ'),
            "varphi" => ident!('ϕ'),
            "chi" => ident!('χ'),
            "psi" => ident!('ψ'),
            "omega" => ident!('ω'),
            // Uppercase Greek letters
            "Alpha" => ident!('Α'),
            "Beta" => ident!('Β'),
            "Gamma" => ident!('Γ'),
            "Delta" => ident!('Δ'),
            "Epsilon" => ident!('Ε'),
            "Zeta" => ident!('Ζ'),
            "Eta" => ident!('Η'),
            "Theta" => ident!('Θ'),
            "Iota" => ident!('Ι'),
            "Kappa" => ident!('Κ'),
            "Lambda" => ident!('Λ'),
            "Mu" => ident!('Μ'),
            "Nu" => ident!('Ν'),
            "Xi" => ident!('Ξ'),
            "Pi" => ident!('Π'),
            "Rho" => ident!('Ρ'),
            "Sigma" => ident!('Σ'),
            "Tau" => ident!('Τ'),
            "Upsilon" => ident!('Υ'),
            "Phi" => ident!('Φ'),
            "Chi" => ident!('Χ'),
            "Psi" => ident!('Ψ'),
            "Omega" => ident!('Ω'),

            "|" => ident!('∥'),
            "angle" => ident!('∠'),
            "aleph" => ident!('ℵ'),

            "approx" => op!('≈'),
            "approxeq" => op!('≊'),
            "approxcolon" => {
                self.instruction_stack
                    .push(Instruction::Event(Event::Content(Content::Operator {
                        content: ':',
                        stretchy: None,
                        moveable_limits: None,
                        left_space: Some((0., DimensionUnit::Em)),
                        right_space: None,
                    })));
                Event::Content(Content::Operator {
                    content: '≈',
                    stretchy: None,
                    moveable_limits: None,
                    left_space: None,
                    right_space: Some((0., DimensionUnit::Em)),
                })
            }
            "approxcoloncolon" => {
                self.instruction_stack
                    .push(Instruction::Event(Event::Content(Content::Operator {
                        content: ':',
                        stretchy: None,
                        moveable_limits: None,
                        left_space: Some((0., DimensionUnit::Em)),
                        right_space: None,
                    })));
                self.instruction_stack
                    .push(Instruction::Event(Event::Content(Content::Operator {
                        content: ':',
                        stretchy: None,
                        moveable_limits: None,
                        left_space: Some((0., DimensionUnit::Em)),
                        right_space: Some((0., DimensionUnit::Em)),
                    })));
                Event::Content(Content::Operator {
                    content: '≈',
                    stretchy: None,
                    moveable_limits: None,
                    left_space: None,
                    right_space: Some((0., DimensionUnit::Em)),
                })
            }
            "ast" => op!('*'),
            "asymp" => op!('≍'),
            "amalg" => op!('⨿'),
            "And" => op!('&'),

            "backepsilon" => ident!('϶'),
            "backprime" => ident!('‵'),
            "backsim" => op!('∽'),
            "backsimeq" => op!('⋍'),
            "backslash" => ident!('\\'),
            "bar" => {
                self.instruction_stack.push(Instruction::Event(op!('‾')));
                let argument = self.argument()?;
                match argument {
                    Argument::Token(Token::Character(c)) => self.handle_char_token(c)?,
                    Argument::Token(Token::ControlSequence(cs)) => self.handle_primitive(cs)?,
                    Argument::Group(substr) => {
                        self.instruction_stack.push(Instruction::Event(Event::EndGroup));
                        self.instruction_stack.push(Instruction::Substring(substr));
                        Event::Begin(Grouping::Group)
                    }
                }
            }
            "barwedge" => op!('⌅'),
            _ => todo!(),
        })
    }
}

// Currently unhandled:
// - `global`
// - `relax`
// - `begingroup`, `endgroup`
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
