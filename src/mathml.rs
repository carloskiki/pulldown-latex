use std::io;

use crate::{
    attribute::{tex_to_css_em, Font},
    config::{DisplayMode, RenderConfig},
    event::{Content, Event, Identifier, Operator, ScriptPosition, ScriptType, Visual},
};

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
            self.write_event(event)?;
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

    // TODO: Some complete dickhead wrote this, and now I need to change it.
    // We shouldn't call `next`, we should handle a whole ass group if there is one.
    fn write_event(&mut self, event: Result<Event<'a>, E>) -> io::Result<()> {
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
                Content::Identifier(ident) => {
                    match ident {
                        Identifier::Str(str) => {
                            self.writer.write_all(if str.chars().count() == 1 {
                                b"<mi mathvariant=\"normal\">"
                            } else {
                                b"<mi>"
                            })?;
                            self.writer.write_all(str.as_bytes())?;
                        }
                        Identifier::Char(content) => {
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
                        }
                    }
                    self.writer.write_all(b"</mi>")
                }
                Content::Operator(Operator {
                    content,
                    stretchy,
                    deny_movable_limits,
                    left_space,
                    right_space,
                    size,
                }) => {
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
                    self.writer.write_all(b"</mo>")
                }
            },
            Ok(Event::BeginGroup) => {
                self.writer.write_all(b"<mrow>")?;
                self.font_state
                    .push(*self.font_state.last().ok_or(io::Error::other(
                        "unbalanced use of grouping in `FontChange` events, no font state found",
                    ))?);
                loop {
                    self.next_else("expected `EndGroup` event before the end of the input")?;
                    if event.as_ref().is_ok_and(|e| e == &Event::EndGroup) {
                        self.font_state.pop();
                        break;
                    }
                    self.write_event(event)?;
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
                    let err = "expected two elements after a `Fraction` event";
                    let first = self.next_else(err)?;
                    self.write_event(first)?;
                    let second = self.next_else(err)?;
                    self.write_event(second)?;
                    self.writer.write_all(b"</mfrac>")
                }
                Visual::SquareRoot => {
                    let argument = self.next_else("expected an element after a `SquareRoot` event")?;
                    self.writer.write_all(b"<msqrt>")?;
                    self.write_event(argument)?;
                    self.writer.write_all(b"</msqrt>")
                }
                Visual::Root => {
                    let err = "expected two elements after a `Root` event";
                    self.writer.write_all(b"<mroot>")?;
                    let radicand = self.next_else(err)?;
                    self.write_event(radicand)?;
                    let index = self.next_else(err)?;
                    self.write_event(index)?;
                    self.writer.write_all(b"</mroot>")
                }
                Visual::Negation => {
                    let next = self.next_else("expected an element after a `Negation` event")?;
                    if matches!(next, Ok(Event::Content(Content::Operator(_) | Content::Identifier(Identifier::Char(_))))) {
                        todo!()
                    } 
                    todo!()
                }
            },

            Ok(Event::Script {
                ty,
                position,
            }) => {
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
                        self.write_event(base)?;
                        let subscript = self.next_else(err)?;
                        self.write_event(subscript)?;
                        self.writer.write_all(b"</msub>")
                    }
                    (ScriptType::Superscript, false) => {
                        let err = "expected two elements after a `Superscript` event";
                        self.writer.write_all(b"<msup>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base)?;
                        let superscript = self.next_else(err)?;
                        self.write_event(superscript)?;
                        self.writer.write_all(b"</msup>")
                    }
                    (ScriptType::SubSuperscript, false) => {
                        let err = "expected three elements after a `SubSuperscript` event";
                        self.writer.write_all(b"<msubsup>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base)?;
                        let subscript = self.next_else(err)?;
                        self.write_event(subscript)?;
                        let superscript = self.next_else(err)?;
                        self.write_event(superscript)?;
                        self.writer.write_all(b"</msubsup>")
                    }
                    (ScriptType::Subscript, true) => {
                        let err = "expected two elements after a `Undercript` event";
                        self.writer.write_all(b"<munder>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base)?;
                        let underscript = self.next_else(err)?;
                        self.write_event(underscript)?;
                        self.writer.write_all(b"</munder>")
                    }
                    (ScriptType::Superscript, true) => {
                        let err = "expected two elements after a `Overscript` event";
                        self.writer.write_all(b"<mover>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base)?;
                        let overscript = self.next_else(err)?;
                        self.write_event(overscript)?;
                        self.writer.write_all(b"</mover>")
                    }
                    (ScriptType::SubSuperscript, true) => {
                        let err = "expected three elements after a `UnderOver` event";
                        self.writer.write_all(b"<munderover>")?;
                        let base = self.next_else(err)?;
                        self.write_event(base)?;
                        let underscript = self.next_else(err)?;
                        self.write_event(underscript)?;
                        let overscript = self.next_else(err)?;
                        self.write_event(overscript)?;
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
            Ok(Event::FontChange(font)) => {
                let font_state = self.font_state.last_mut().ok_or(io::Error::other(
                    "unbalanced use of grouping in `FontChange` events, no font state found",
                ))?;
                *font_state = font;
                let next_event = self.input.next().ok_or(io::Error::other(
                    "missing following event in use of grouping in `FontChange` events",
                ))?;
                self.write_event(next_event)
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
        self.input.next().ok_or(io::Error::other(
                err
        ))
    }

    fn get_font(&self) -> io::Result<Option<Font>> {
        self.font_state.last().copied().ok_or(io::Error::other(
            "unbalanced use of grouping in `FontChange` events, no font state found",
        ))
    }
}

/// Takes a [`Parser`], or any `Iterator<Item = Result<Event<'_>, E>>`, as input and renders a
/// string of MathML into the given string.
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
pub fn write_mathml<'a, I, W, E>(writer: W, parser: I, config: RenderConfig<'a>) -> io::Result<()>
where
    I: Iterator<Item = Result<Event<'a>, E>>,
    W: io::Write,
    E: std::error::Error,
{
    MathmlWriter::new(parser, writer, config).write()
}
