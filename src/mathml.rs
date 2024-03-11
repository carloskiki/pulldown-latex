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

    pub fn write(mut self) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // Where the writer is written to:
        // - using `write_all` with a utf-8 string.
        // - uwing `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
        //      they are formatted using the `Display` trait.
        //
        // TODO: remove expect in favor of error handling.

        // If the stream of events was not modified after being created \
        // by the parser, this is a bug, and should be reported."));
    }

    fn handle_event(&mut self, event: Event<'a>) -> io::Result<()> {
        match event {
            Event::Content(content) => match content {
                Content::Text(str) => {
                    self.writer.write_all(b"<mtext>")?;
                    self.writer.write_all(str.as_bytes())?;
                    self.writer.write_all(b"</mtext>")?;
                }
                Content::Number(content) => {
                    self.writer.write_all(b"<mn>")?;
                    let buf = &mut [0u8; 4];
                    content.chars().try_for_each(|c| {
                        let content = self.get_font().map_or(c, |v| v.map_char(c));
                        let bytes = content.encode_utf8(buf);
                        self.writer.write_all(bytes.as_bytes())?;
                        Ok::<(), io::Error>(())
                    })?;
                    self.writer.write_all(b"</mn>")?;
                }
                Content::Identifier(ident) => {
                    self.writer.write_all(b"<mi>")?;
                    match ident {
                        Identifier::Str(str) => self.writer.write_all(str.as_bytes())?,
                        Identifier::Char(content) => {
                            let buf = &mut [0u8; 4];
                            // TODO: Handle the config of ISO vs. LaTeX vs. French vs. Upright
                            let content = self.get_font().map_or(content, |v| v.map_char(content));
                            let bytes = content.encode_utf8(buf);
                            self.writer.write_all(bytes.as_bytes())?;
                        }
                    }
                    self.writer.write_all(b"</mi>")?;
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
                    self.writer.write_all(b"</mo>")?;
                }
            },
            Event::BeginGroup => {
                self.writer.write_all(b"<mrow>")?;
                loop {
                    let Some(event) = self.input.next() else {
                        return Err(io::Error::other(
                            "expected `EndGroup` event before the end of the input.",
                        ));
                    };
                    let stop = event == Event::EndGroup;
                    self.handle_event(event)?;
                    if stop {
                        break;
                    }
                }
                self.writer.write_all(b"</mrow>")?;
            }
            // This should always be reached in the process of the `BeginGroup` event, and thus we
            // should most likely output and error if it is reached here.
            Event::EndGroup => {
                return Err(io::Error::other(
                    "unbalanced use of `BeginGroup` and `EndGroup` events.",
                ));
            }
            Event::Visual(Visual::Fraction(dim)) => {
                let (Some(first), Some(second)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Fraction` event.",
                    ));
                };
                self.writer.write_all(b"<mfrac")?;
                if let Some(dim) = dim {
                    let (dim, unit) = tex_to_css_units(dim);
                    write!(self.writer, " linethickness=\"{}{}\"", dim, unit)?;
                }
                self.handle_event(first)?;
                self.handle_event(second)?;
                self.writer.write_all(b"</mfrac>")?;
            }
            Event::Visual(Visual::SquareRoot) => {
                let Some(degree) = self.input.next() else {
                    return Err(io::Error::other(
                        "expected two components after a `Root` event.",
                    ));
                };
                self.writer.write_all(b"<msqrt>")?;
                self.handle_event(degree)?;
                self.writer.write_all(b"</msqrt>")?;
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
                self.writer.write_all(b" />")?;
            }
            Event::FontChange(font) => {
                let font_state = self.font_state.last_mut().expect("(internal error: please report) unbalanced use of `FontChange` events.");
                *font_state = font;
                self.
            },
        }
        Ok(())
    }

    fn get_font(&self) -> Option<Font> {
        *self
            .font_state
            .last()
            .expect("(internal error: please report) unbalanced use of `FontChange` events.")
    }
}

/// Takes a [`Parser`] as input and returns a string of MathML.
pub fn push_html<'a, I>(string: &mut String, parser: I) -> io::Result<()>
where
    I: Iterator<Item = Event<'a>>,
{
    MathmlWriter::new(parser, unsafe { string.as_mut_vec() }).write()
}
