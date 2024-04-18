//! A simple `mathml` renderer.
//!
//! This crate provides a "simple" `mathml` renderer which is available through the
//! [`push_mathml`] and [`write_mathml`] functions.

use std::io;

use crate::{
    attribute::{tex_to_css_em, Font},
    config::{DisplayMode, RenderConfig},
    event::{ColorChange, ColorTarget, Content, Event, Identifier, Operator, ScriptPosition, ScriptType, StateChange, Style, Visual},
};

// TODO: Someone stupid and tired wrote this, please refactor.
struct MathmlWriter<'a, I, W> {
    input: I,
    writer: W,
    config: RenderConfig<'a>,
}

impl<'a, I, W, E> MathmlWriter<'a, I, W>
where
    I: Iterator<Item = Result<Event<'a>, E>>,
    W: io::Write,
    E: std::error::Error,
{
    fn new(input: I, writer: W, config: RenderConfig<'a>) -> Self {
        // Size of the buffer is arbitrary for performance guess.
        Self {
            input,
            writer,
            config,
        }
    }

    fn write_operator(
        &mut self,
        Operator {
            content,
            stretchy,
            unicode_variant,
            deny_movable_limits,
            left_space,
            right_space,
            size,
        }: Operator,
        negate: bool,
        state: State<'a>,
    ) -> io::Result<()> {
        self.open_tag("mo", false, state)?;
        if let Some(stretchy) = stretchy {
            write!(self.writer, " stretchy=\"{}\"", stretchy)?;
        }
        if deny_movable_limits {
            self.writer.write_all(b" movablelimits=\"false\"")?;
        }
        if let Some(left_space) = left_space {
            write!(self.writer, " lspace=\"{}em\"", tex_to_css_em(left_space))?;
        }
        if let Some(right_space) = right_space {
            write!(self.writer, " rspace=\"{}em\"", tex_to_css_em(right_space))?;
        }
        if let Some(size) = size {
            let size = tex_to_css_em(size);
            write!(self.writer, " minsize=\"{}em\"", size)?;
            write!(self.writer, " maxsize=\"{}em\"", size)?;
        }
        self.writer.write_all(b">")?;
        let buf = &mut [0u8; 4];
        let bytes = content.encode_utf8(buf).as_bytes();
        self.writer.write_all(bytes)?;
        if unicode_variant {
            self.writer.write_all("\u{20D2}".as_bytes())?;
        }
        if negate {
            self.writer.write_all("\u{0338}".as_bytes())?;
        }
        self.writer.write_all(b"</mo>")
    }

    fn write_char_ident(&mut self, content: char, negate: bool, state: State<'a>) -> io::Result<()> {
        self.open_tag("mi", false, state)?;
        let content = match (
            state.font,
            self.config.math_style.should_be_upright(content),
        ) {
            (Some(Font::UpRight), _) | (None, true) => {
                self.writer.write_all(b"mathvariant=\"normal\">")?;
                content
            }
            (Some(font), _) => {
                self.writer.write_all(b">")?;
                font.map_char(content)
            }
            _ => {
                self.writer.write_all(b">")?;
                content
            }
        };

        let buf = &mut [0u8; 4];
        let bytes = content.encode_utf8(buf);
        self.writer.write_all(bytes.as_bytes())?;
        if negate {
            self.writer.write_all("\u{0338}".as_bytes())?;
        }
        self.writer.write_all(b"</mi>")
    }

    fn open_tag(
        &mut self,
        tag: &str,
        close: bool,
        State {
            text_color,
            border_color,
            style,
            ..
        }: State,
    ) -> io::Result<()> {
        write!(self.writer, "<{}", tag)?;
        if text_color.is_none() && border_color.is_none() && style.is_none() {
            if close {
                return self.writer.write_all(b">");
            } else {
                return Ok(());
            }
        }

        let mut style_written = false;
        if let Some(text_color) = text_color {
            write!(self.writer, "style=\"color: {}", text_color)?;
        }
        if let Some(border_color) = border_color {
            if style_written {
                write!(self.writer, "; border: 0.06em solid {}", border_color)?;
            } else {
                write!(self.writer, "style=\"border: 0.06em solid {}", border_color)?;
                style_written = true;
            }
        }
        if style_written {
            self.writer.write_all(b"\"")?;
        }
        if let Some(style) = style {
            let args = match style {
                Style::Display => (true, 0),
                Style::Text => (false, 0),
                Style::Script => (false, 1),
                Style::ScriptScript => (false, 2),
            };
            write!(
                self.writer,
                " displaystyle=\"{}\" scriptlevel=\"{}\"",
                args.0, args.1
            )?;
        }
        if close {
            self.writer.write_all(b">")?;
        }
        Ok(())
    }

    fn write(mut self) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // How is the writer used?:
        // - using `write_all` with a utf-8 string.
        // - using `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
        //      they are formatted using the `Display` trait.
        let mut state_stack = Vec::with_capacity(16);
        state_stack.push(State {
            font: None,
            text_color: None,
            border_color: None,
            style: None,
        });
        let mut env_stack = Vec::with_capacity(32);
        
        while let Some(event) = self.input.next() {
            let state = *state_stack.last().expect("state stack is empty");

            match event {
                Ok(Event::Content(content)) => match content {
                    Content::Text(text) => {
                        self.open_tag("mtext", true, state)?;
                        self.writer.write_all(text.as_bytes())?;
                        self.writer.write_all(b"</mtext>")?;
                    }
                    Content::Number(number) => {
                        self.open_tag("mn", true, state)?;
                        let buf = &mut [0u8; 4];
                        number.chars().try_for_each(|c| {
                            let content = state.font.map_or(c, |v| v.map_char(c));
                            let bytes = content.encode_utf8(buf);
                            self.writer.write_all(bytes.as_bytes())
                        })?;
                        self.writer.write_all(b"</mn>")?;
                    }
                    Content::Identifier(ident) => match ident {
                        Identifier::Str(str) => {
                            self.open_tag("mi", false, state)?;
                            self.writer.write_all(if str.chars().count() == 1 {
                                b"mathvariant=\"normal\">"
                            } else {
                                b">"
                            })?;
                            self.writer.write_all(str.as_bytes())?;
                            self.writer.write_all(b"</mi>")?;
                        }
                        Identifier::Char(content) => {
                            self.write_char_ident(content, false, state)?;
                        }
                    },
                    Content::Operator(op) => {
                        self.write_operator(op, false, state)?;
                    }
                },
                Ok(Event::BeginGroup) => {
                    // TODO: Check for state changes at the begining of a group which could be
                    // optimized out into the initial elements' attributes.
                    self.open_tag("mrow", true, state)?;
                    state_stack.push(State {
                        font: state.font,
                        text_color: None,
                        border_color: None,
                        style: None,
                    });
                    env_stack.push(None);
                }
                Ok(Event::EndGroup) => {
                    let env = env_stack.pop().expect("cannot pop an environment in group end");
                    if env.is_some() {
                        panic!("unexpected environment in group end");
                    }
                    state_stack.pop().expect("cannot pop a state in group end");
                    self.writer.write_all(b"</mrow>")?;
                }
                Ok(Event::Visual(visual)) => match visual {
                    Visual::Fraction(dim) => {
                        self.open_tag("mfrac", false, state)?;
                        if let Some(dim) = dim {
                            write!(self.writer, " linethickness=\"{}em\"", tex_to_css_em(dim))?;
                        }
                        self.writer.write_all(b">")?;
                        env_stack.push(Some((2, "mfrac")));
                    }
                    Visual::SquareRoot => {
                        self.open_tag("msqrt", true, state)?;
                        env_stack.push(Some((1, "msqrt")));
                    }
                    Visual::Root => {
                        self.open_tag("mroot", true, state)?;
                        env_stack.push(Some((2, "mroot")));
                    }
                    Visual::Negation => {
                        todo!()
                        // let next = self.next_else("expected an element after a `Negation` event")?;
                        // match next {
                        //     Ok(Event::Content(Content::Operator(op))) => self.write_operator(op, true)
                        //     Ok(Event::Content(Content::Identifier(Identifier::Char(c)))) => {
                        //         self.write_char_ident(c, true),
                        //     }
                        //     _ => {
                        //         self.writer.write_all(b"<mrow style=\"background: linear-gradient(to top left, rgba(0,0,0,0) 0%, rgba(0,0,0,0) calc(50% - 0.8px), rgba(0,0,0,1) 50%, rgba(0,0,0,0) calc(50% + 0.8px), rgba(0,0,0,0) 100%);\">")?;
                        //         self.write_event(next, true)?;
                        //         self.writer.write_all(b"</mrow>")?;
                        //     }
                        // }?;
                    }
                },

                Ok(Event::Script { ty, position }) => {
                    let above_below = match position {
                        ScriptPosition::Right => false,
                        ScriptPosition::AboveBelow => true,
                        ScriptPosition::Movable => {
                            state.style == Some(Style::Display)
                                || (state.style.is_none()
                                    && self.config.display_mode == DisplayMode::Block)
                        }
                    };
                    match (ty, above_below) {
                        (ScriptType::Subscript, false) => {
                            self.open_tag("msub", true, state)?;
                            env_stack.push(Some((2, "msub")));
                        }
                        (ScriptType::Superscript, false) => {
                            self.open_tag("msup", true, state)?;
                            env_stack.push(Some((2, "msup")));
                        }
                        (ScriptType::SubSuperscript, false) => {
                            self.open_tag("msubsup", true, state)?;
                            env_stack.push(Some((3, "msubsup")));
                        }
                        (ScriptType::Subscript, true) => {
                            self.open_tag("munder", true, state)?;
                            env_stack.push(Some((2, "munder")));
                        }
                        (ScriptType::Superscript, true) => {
                            self.open_tag("mover", true, state)?;
                            env_stack.push(Some((2, "mover")));
                        }
                        (ScriptType::SubSuperscript, true) => {
                            self.open_tag("munderover", true, state)?;
                            env_stack.push(Some((3, "munderover")));
                        }
                    }
                }

                Ok(Event::Space {
                    width,
                    height,
                    depth,
                }) => {
                    if let Some(width) = width {
                        write!(self.writer, "<mspace width=\"{}em\"", tex_to_css_em(width))?;
                        if width.0 < 0.0 {
                            write!(
                                self.writer,
                                " style=\"margin-left: {}em\"",
                                tex_to_css_em(width)
                            )?;
                        }
                    }
                    if let Some(height) = height {
                        write!(self.writer, " height=\"{}em\"", tex_to_css_em(height))?;
                    }
                    if let Some(depth) = depth {
                        write!(self.writer, " depth=\"{}em\"", tex_to_css_em(depth))?;
                    }
                    self.writer.write_all(b" />")?;
                }
                Ok(Event::StateChange(state_change)) => {
                    let state = state_stack.last_mut().expect("state stack is empty");
                    match state_change {
                        StateChange::Font(font) => state.font = font,
                        StateChange::Color(ColorChange {
                            color,
                            target,
                        }) => match target {
                            ColorTarget::Text => state.text_color = Some(color),
                            ColorTarget::Border => state.border_color = Some(color),
                            ColorTarget::Background => todo!(),
                        },
                        StateChange::Style(style) => state.style = Some(style),
                    }
                }
                Err(e) => {
                    let error_color = self.config.error_color;
                    write!(
                        self.writer,
                        "<merror style=\"border-color: #{:x}{:x}{:x}\"><mtext>",
                        error_color.0, error_color.1, error_color.2
                    )?;
                    self.writer.write_all(e.to_string().as_bytes())?;
                    self.writer.write_all(b"</mtext></merror>")?;
                }
            }

            if let Some(Some((count, tag))) = env_stack.last_mut() {
                *count -= 1;
                if *count == 0 {
                    self.writer.write_all(b"</")?;
                    self.writer.write_all(tag.as_bytes())?;
                    self.writer.write_all(b">")?;
                    env_stack.pop();
                }
            }
        }
        if !env_stack.is_empty() || state_stack.len() != 1 {
            eprintln!("{:?}", env_stack);
            eprintln!("{:?}", state_stack);
            panic!("unbalanced environment stack or state stack");
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct State<'a> {
    font: Option<Font>,
    text_color: Option<&'a str>,
    border_color: Option<&'a str>,
    style: Option<Style>,
}

/// Takes a [`Parser`], or any `Iterator<Item = Result<Event<'_>, E>>`, as input and renders a
/// string of MathML into the given string.
///
/// [`Parser`]: crate::parser::Parser
pub fn push_mathml<'a, I, E>(
    string: &mut String,
    parser: I,
    config: RenderConfig<'a>,
) -> io::Result<()>
where
    I: Iterator<Item = Result<Event<'a>, E>>,
    E: std::error::Error,
{
    // SAFETY: The MathmlWriter guarantees that all writes to the writer are valid utf-8.
    MathmlWriter::new(parser, unsafe { string.as_mut_vec() }, config).write()
}

/// Takes a [`Parser`], or any `Iterator<Item = Result<Event<'_>, E>>`, as input and renders the
/// MathML into the given writer.
///
/// [`Parser`]: crate::parser::Parser
pub fn write_mathml<'a, I, W, E>(writer: W, parser: I, config: RenderConfig<'a>) -> io::Result<()>
where
    I: Iterator<Item = Result<Event<'a>, E>>,
    W: io::Write,
    E: std::error::Error,
{
    MathmlWriter::new(parser, writer, config).write()
}
