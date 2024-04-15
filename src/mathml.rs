//! A simple `mathml` renderer.
//!
//! This crate provides a "simple" `mathml` renderer which is available through the
//! [`push_mathml`] and [`write_mathml`] functions.

use std::io;

use crate::{
    attribute::{tex_to_css_em, Font},
    config::{DisplayMode, RenderConfig},
    event::{Content, Event, Identifier, Operator, ScriptPosition, ScriptType, Visual},
};

// TODO: Someone stupid and tired wrote this, please refactor.
struct MathmlWriter<'a, I, W> {
    input: I,
    writer: W,
    font_state: Vec<Option<Font>>,
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
        let mut font_state = Vec::with_capacity(16);
        font_state.push(None);
        Self {
            input,
            writer,
            font_state,
            config,
        }
    }

    fn write(mut self) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // How is the writer used?:
        // - using `write_all` with a utf-8 string.
        // - uwing `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
        //      they are formatted using the `Display` trait.

        write!(
            self.writer,
            "<math display=\"{}\"",
            self.config.display_mode
        )?;
        if self.config.xml {
            self.writer
                .write_all(b" xmlns=\"http://www.w3.org/1998/Math/MathML\"")?;
        }
        self.writer.write_all(b">")?;
        if self.config.annotation.is_some() {
            self.writer.write_all(b"<semantics>")?;
        }

        while let Some(event) = self.input.next() {
            self.write_event(event, false)?;
        }
        if let Some(annotation) = self.config.annotation {
            write!(
                self.writer,
                "<annotation encoding=\"application/x-tex\">{}</annotation>",
                annotation
            )?;
            self.writer.write_all(b"</semantics>")?;
        }
        self.writer.write_all(b"</math>")?;

        Ok(())
    }

    fn write_event(&mut self, event: Result<Event<'a>, E>, required: bool) -> io::Result<()> {
        // SAFETY: This function respects the invariants of the MathmlWriter
        match event {
            Ok(Event::Content(content)) => match content {
                Content::Text(text) => {
                    self.writer.write_all(b"<mtext>")?;
                    self.writer.write_all(text.as_bytes())?;
                    self.writer.write_all(b"</mtext>")
                }
                Content::Number(number) => {
                    self.writer.write_all(b"<mn>")?;
                    let buf = &mut [0u8; 4];
                    number.chars().try_for_each(|c| {
                        let content = self.get_font()?.map_or(c, |v| v.map_char(c));
                        let bytes = content.encode_utf8(buf);
                        self.writer.write_all(bytes.as_bytes())?;
                        Ok::<(), io::Error>(())
                    })?;
                    self.writer.write_all(b"</mn>")
                }
                Content::Identifier(ident) => match ident {
                    Identifier::Str(str) => {
                        self.writer.write_all(if str.chars().count() == 1 {
                            b"<mi mathvariant=\"normal\">"
                        } else {
                            b"<mi>"
                        })?;
                        self.writer.write_all(str.as_bytes())?;
                        self.writer.write_all(b"</mi>")
                    }
                    Identifier::Char(content) => self.write_char_ident(content, false),
                },
                Content::Operator(op) => self.write_operator(op, false),
            },
            Ok(Event::BeginGroup) => {
                self.writer.write_all(b"<mrow>")?;
                self.font_state
                    .push(*self.font_state.last().ok_or(io::Error::other(
                        "unbalanced use of grouping in `FontChange` events, no font state found",
                    ))?);
                loop {
                    let event =
                        self.next_else("expected `EndGroup` event before the end of the input")?;
                    if event.as_ref().is_ok_and(|e| e == &Event::EndGroup) {
                        self.font_state.pop();
                        break;
                    }
                    self.write_event(event, false)?;
                }
                self.writer.write_all(b"</mrow>")
            }
            // This should always be reached in the process of the `BeginGroup` event, and thus we
            // should most likely output and error if it is reached here.
            Ok(Event::EndGroup) => Err(io::Error::other(
                "unbalanced use of `BeginGroup` and `EndGroup` events",
            )),
            Ok(Event::Visual(visual)) => match visual {
                Visual::Fraction(dim) => {
                    self.writer.write_all(b"<mfrac")?;
                    if let Some(dim) = dim {
                        write!(self.writer, " linethickness=\"{}em\"", tex_to_css_em(dim))?;
                    }
                    self.writer.write_all(b">")?;
                    let err = "expected two elements after a `Fraction` event";
                    let first = self.next_else(err)?;
                    self.write_event(first, true)?;
                    let second = self.next_else(err)?;
                    self.write_event(second, true)?;
                    self.writer.write_all(b"</mfrac>")
                }
                Visual::SquareRoot => {
                    let argument =
                        self.next_else("expected an element after a `SquareRoot` event")?;
                    self.writer.write_all(b"<msqrt>")?;
                    self.write_event(argument, true)?;
                    self.writer.write_all(b"</msqrt>")
                }
                Visual::Root => {
                    let err = "expected two elements after a `Root` event";
                    self.writer.write_all(b"<mroot>")?;
                    let radicand = self.next_else(err)?;
                    self.write_event(radicand, true)?;
                    let index = self.next_else(err)?;
                    self.write_event(index, true)?;
                    self.writer.write_all(b"</mroot>")
                }
                Visual::Negation => {
                    let next = self.next_else("expected an element after a `Negation` event")?;
                    match next {
                        Ok(Event::Content(Content::Operator(op))) => self.write_operator(op, true),
                        Ok(Event::Content(Content::Identifier(Identifier::Char(c)))) => {
                            self.write_char_ident(c, true)
                        }
                        _ => {
                            self.writer.write_all(b"<mrow style=\"background: linear-gradient(to top left, rgba(0,0,0,0) 0%, rgba(0,0,0,0) calc(50% - 0.8px), rgba(0,0,0,1) 50%, rgba(0,0,0,0) calc(50% + 0.8px), rgba(0,0,0,0) 100%);\">")?;
                            self.write_event(next, true)?;
                            self.writer.write_all(b"</mrow>")
                        }
                    }
                }
            },

            Ok(Event::Script { ty, position }) => {
                let above_below = match position {
                    ScriptPosition::Right => false,
                    ScriptPosition::AboveBelow => true,
                    ScriptPosition::Movable => self.config.display_mode == DisplayMode::Block,
                };
                match (ty, above_below) {
                    (ScriptType::Subscript, false) => {
                        let err = "expected two elements after a `Subscript` event";
                        self.writer.write_all(b"<msub>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let subscript = self.next_else(err)?;
                        self.write_event(subscript, true)?;
                        self.writer.write_all(b"</msub>")
                    }
                    (ScriptType::Superscript, false) => {
                        let err = "expected two elements after a `Superscript` event";
                        self.writer.write_all(b"<msup>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let superscript = self.next_else(err)?;
                        self.write_event(superscript, true)?;
                        self.writer.write_all(b"</msup>")
                    }
                    (ScriptType::SubSuperscript, false) => {
                        let err = "expected three elements after a `SubSuperscript` event";
                        self.writer.write_all(b"<msubsup>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let subscript = self.next_else(err)?;
                        self.write_event(subscript, true)?;
                        let superscript = self.next_else(err)?;
                        self.write_event(superscript, true)?;
                        self.writer.write_all(b"</msubsup>")
                    }
                    (ScriptType::Subscript, true) => {
                        let err = "expected two elements after a `Undercript` event";
                        self.writer.write_all(b"<munder>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let underscript = self.next_else(err)?;
                        self.write_event(underscript, true)?;
                        self.writer.write_all(b"</munder>")
                    }
                    (ScriptType::Superscript, true) => {
                        let err = "expected two elements after a `Overscript` event";
                        self.writer.write_all(b"<mover>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let overscript = self.next_else(err)?;
                        self.write_event(overscript, true)?;
                        self.writer.write_all(b"</mover>")
                    }
                    (ScriptType::SubSuperscript, true) => {
                        let err = "expected three elements after a `UnderOver` event";
                        self.writer.write_all(b"<munderover>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base, true)?;
                        let underscript = self.next_else(err)?;
                        self.write_event(underscript, true)?;
                        let overscript = self.next_else(err)?;
                        self.write_event(overscript, true)?;
                        self.writer.write_all(b"</munderover>")
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
                self.writer.write_all(b" />")
            }
            // TODO: This is not okay, it does not work
            Ok(Event::StateChange(state_change)) => {
                todo!()
                // let font_state = self.font_state.last_mut().ok_or(io::Error::other(
                //     "unbalanced use of grouping in `FontChange` events, no font state found",
                // ))?;
                // *font_state = font;
                // if required {
                //     self.writer.write_all(b"<mrow />")?;
                // }
                // Ok(())
            }
            Err(e) => {
                let error_color = self.config.error_color;
                write!(
                    self.writer,
                    "<merror style=\"border-color: #{:x}{:x}{:x}\"><mtext>",
                    error_color.0, error_color.1, error_color.2
                )?;
                self.writer.write_all(e.to_string().as_bytes())?;
                self.writer.write_all(b"</mtext></merror>")
            }
        }
    }

    fn next_else(&mut self, err: &str) -> io::Result<Result<Event<'a>, E>> {
        self.input.next().ok_or(io::Error::other(err))
    }

    fn get_font(&self) -> io::Result<Option<Font>> {
        self.font_state.last().copied().ok_or(io::Error::other(
            "unbalanced use of grouping in `FontChange` events, no font state found",
        ))
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
    ) -> io::Result<()> {
        self.writer.write_all(b"<mo")?;
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

    fn write_char_ident(&mut self, content: char, negate: bool) -> io::Result<()> {
        let content = match (
            self.get_font()?,
            self.config.math_style.should_be_upright(content),
        ) {
            (Some(Font::UpRight), _) | (None, true) => {
                self.writer.write_all(b"<mi mathvariant=\"normal\">")?;
                content
            }
            (Some(font), _) => {
                self.writer.write_all(b"<mi>")?;
                font.map_char(content)
            }
            _ => {
                self.writer.write_all(b"<mi>")?;
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
