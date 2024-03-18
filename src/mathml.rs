use std::io;

use crate::{
    attribute::{tex_to_css_units, Font},
    event::{Content, Event, Identifier, Operator, Visual},
};

struct MathmlWriter<I, W> {
    input: I,
    writer: W,
    font_state: Vec<Option<Font>>,
}

// TODO: Should we make css unit conversion produce a string directly?
// TODO: Handle configuration of the writer
impl<'a, I, W> MathmlWriter<I, W>
where
    I: Iterator<Item = Event<'a>>,
    W: io::Write,
{
    fn new(input: I, writer: W) -> Self {
        // Size of the buffer is arbitrary for performance guess.
        let mut font_state = Vec::with_capacity(16);
        font_state.push(None);
        Self {
            input,
            writer,
            font_state,
        }
    }

    fn write(mut self) -> io::Result<()> {
        while let Some(event) = self.input.next() {
            self.write_event(event)?;
        }
        Ok(())
    }

    fn write_event(&mut self, event: Event<'a>) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // How is the writer used?:
        // - using `write_all` with a utf-8 string.
        // - uwing `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
        //      they are formatted using the `Display` trait.
        match event {
            Event::Content(content) => match content {
                Content::Text(str) => {
                    self.writer.write_all(b"<mtext>")?;
                    self.writer.write_all(str.as_bytes())?;
                    self.writer.write_all(b"</mtext>")
                }
                Content::Number(content) => {
                    self.writer.write_all(b"<mn>")?;
                    let buf = &mut [0u8; 4];
                    content.chars().try_for_each(|c| {
                        let content = self.get_font()?.map_or(c, |v| v.map_char(c));
                        let bytes = content.encode_utf8(buf);
                        self.writer.write_all(bytes.as_bytes())?;
                        Ok::<(), io::Error>(())
                    })?;
                    self.writer.write_all(b"</mn>")
                }
                Content::Identifier(ident) => {
                    self.writer.write_all(b"<mi>")?;
                    match ident {
                        Identifier::Str(str) => self.writer.write_all(str.as_bytes())?,
                        Identifier::Char(content) => {
                            let buf = &mut [0u8; 4];
                            // TODO: Handle the config of ISO vs. LaTeX vs. French vs. Upright
                            let content = self.get_font()?.map_or(content, |v| v.map_char(content));
                            let bytes = content.encode_utf8(buf);
                            self.writer.write_all(bytes.as_bytes())?;
                        }
                    }
                    self.writer.write_all(b"</mi>")
                }
                Content::Operator(Operator {
                    content,
                    stretchy,
                    moveable_limits,
                    left_space,
                    right_space,
                    size,
                }) => {
                    self.writer.write_all(b"<mo")?;
                    if let Some(stretchy) = stretchy {
                        write!(self.writer, " stretchy=\"{}\"", stretchy)?;
                    }
                    if let Some(moveable_limits) = moveable_limits {
                        write!(self.writer, " movablelimits=\"{}\"", moveable_limits)?;
                    }
                    if let Some(left_space) = left_space {
                        let (left_space, unit) = tex_to_css_units(left_space);
                        write!(self.writer, " lspace=\"{}{}\"", left_space, unit)?;
                    }
                    if let Some(right_space) = right_space {
                        let (right_space, unit) = tex_to_css_units(right_space);
                        write!(self.writer, " rspace=\"{}{}\"", right_space, unit)?;
                    }
                    if let Some(size) = size {
                        let (size, unit) = tex_to_css_units(size);
                        write!(self.writer, " minsize=\"{}{}\"", size, unit)?;
                        write!(self.writer, " maxsize=\"{}{}\"", size, unit)?;
                    }
                    self.writer.write_all(b">")?;
                    let buf = &mut [0u8; 4];
                    let bytes = content.encode_utf8(buf).as_bytes();
                    self.writer.write_all(bytes)?;
                    self.writer.write_all(b"</mo>")
                }
            },
            Event::BeginGroup => {
                self.writer.write_all(b"<mrow>")?;
                self.font_state
                    .push(*self.font_state.last().ok_or(io::Error::other(
                        "unbalanced use of grouping in `FontChange` events, no font state found",
                    ))?);
                loop {
                    let Some(event) = self.input.next() else {
                        return Err(io::Error::other(
                            "expected `EndGroup` event before the end of the input",
                        ));
                    };
                    let stop = event == Event::EndGroup;
                    if stop {
                        self.font_state.pop();
                        break;
                    }
                    self.write_event(event)?;
                }
                self.writer.write_all(b"</mrow>")
            }
            // This should always be reached in the process of the `BeginGroup` event, and thus we
            // should most likely output and error if it is reached here.
            Event::EndGroup => Err(io::Error::other(
                "unbalanced use of `BeginGroup` and `EndGroup` events",
            )),
            Event::Visual(Visual::Fraction(dim)) => {
                let (Some(first), Some(second)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Fraction` event",
                    ));
                };
                self.writer.write_all(b"<mfrac")?;
                if let Some(dim) = dim {
                    let (dim, unit) = tex_to_css_units(dim);
                    write!(self.writer, " linethickness=\"{}{}\"", dim, unit)?;
                }
                self.write_event(first)?;
                self.write_event(second)?;
                self.writer.write_all(b"</mfrac>")
            }
            Event::Visual(Visual::SquareRoot) => {
                let Some(argument) = self.input.next() else {
                    return Err(io::Error::other(
                        "expected two components after a `Root` event",
                    ));
                };
                self.writer.write_all(b"<msqrt>")?;
                self.write_event(argument)?;
                self.writer.write_all(b"</msqrt>")
            }
            Event::Visual(Visual::Subscript) => {
                let (Some(base), Some(subscript)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Subscript` event",
                    ));
                };
                self.writer.write_all(b"<msub>")?;
                self.write_event(base)?;
                self.write_event(subscript)?;
                self.writer.write_all(b"</msub>")
            }
            Event::Visual(Visual::Superscript) => {
                let (Some(base), Some(superscript)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Superscript` event",
                    ));
                };
                self.writer.write_all(b"<msup>")?;
                self.write_event(base)?;
                self.write_event(superscript)?;
                self.writer.write_all(b"</msup>")
            }
            Event::Visual(Visual::SubSuperscript) => {
                let (Some(base), Some(subscript), Some(superscript)) =
                    (self.input.next(), self.input.next(), self.input.next())
                else {
                    return Err(io::Error::other(
                        "expected three components after a `SubSuperscript` event",
                    ));
                };
                self.writer.write_all(b"<msubsup>")?;
                self.write_event(base)?;
                self.write_event(subscript)?;
                self.write_event(superscript)?;
                self.writer.write_all(b"</msubsup>")
            }
            Event::Visual(Visual::Overscript) => {
                let (Some(base), Some(overscript)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Overscript` event",
                    ));
                };
                self.writer.write_all(b"<mover>")?;
                self.write_event(base)?;
                self.write_event(overscript)?;
                self.writer.write_all(b"</mover>")
            }
            Event::Visual(Visual::Underscript) => {
                let (Some(base), Some(underscript)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Underscript` event",
                    ));
                };
                self.writer.write_all(b"<munder>")?;
                self.write_event(base)?;
                self.write_event(underscript)?;
                self.writer.write_all(b"</munder>")
            }
            Event::Visual(Visual::UnderOverscript) => {
                let (Some(base), Some(underscript), Some(overscript)) =
                    (self.input.next(), self.input.next(), self.input.next())
                else {
                    return Err(io::Error::other(
                        "expected three components after a `UnderOverscript` event",
                    ));
                };
                self.writer.write_all(b"<munderover>")?;
                self.write_event(base)?;
                self.write_event(underscript)?;
                self.write_event(overscript)?;
                self.writer.write_all(b"</munderover>")
            }
            Event::Space {
                width,
                height,
                depth,
            } => {
                if let Some(width) = width {
                    let (width, unit) = tex_to_css_units(width);
                    write!(self.writer, "<mspace width=\"{}{}\"", width, unit)?;
                    if width < 0.0 {
                        write!(self.writer, " style=\"margin-left: {}{}\"", width, unit)?;
                    }
                }
                if let Some(height) = height {
                    let (height, unit) = tex_to_css_units(height);
                    write!(self.writer, " height=\"{}{}\"", height, unit)?;
                }
                if let Some(depth) = depth {
                    let (depth, unit) = tex_to_css_units(depth);
                    write!(self.writer, " depth=\"{}{}\"", depth, unit)?;
                }
                self.writer.write_all(b" />")
            }
            Event::FontChange(font) => {
                let font_state = self.font_state.last_mut().ok_or(io::Error::other(
                    "unbalanced use of grouping in `FontChange` events, no font state found",
                ))?;
                *font_state = font;
                let next_event = self.input.next().ok_or(io::Error::other(
                    "missing following event in use of grouping in `FontChange` events",
                ))?;
                self.write_event(next_event)
            }
        }
    }

    fn get_font(&self) -> io::Result<Option<Font>> {
        self.font_state
            .last()
            .copied()
            .ok_or(io::Error::other("unbalanced use of grouping in `FontChange` events, no font state found"))
    }
}

/// Takes a [`Parser`] as input and returns a string of MathML.
pub fn push_html<'a, I>(string: &mut String, parser: I) -> io::Result<()>
where
    I: Iterator<Item = Event<'a>>,
{
    MathmlWriter::new(parser, unsafe { string.as_mut_vec() }).write()
}
