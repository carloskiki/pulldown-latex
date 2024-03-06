use std::io::{self, Write};

use crate::{
    attribute::tex_to_css_units,
    event::{Content, Event, Identifier, Visual, Operator},
};

struct MathmlWriter<I, W> {
    input: I,
    writer: W,
    buffer: Vec<u8>,
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
        Self {
            input,
            writer,
            buffer: Vec::with_capacity(4096),
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
        let first_event = self.input.next()
        self.handle_event(self.input.next().expect("empty input"))

        // If the stream of events was not modified after being created \
        // by the parser, this is a bug, and should be reported on GitHub."));
    }

    fn handle_event(&mut self, event: Event<'a>) -> io::Result<()> {
        if !matches!(event, Event::Infix(_)) {
            self.writer.write_all(&self.buffer[..])?;
            self.buffer.clear();
        }
        
        match event {
            Event::Content(content) => match content {
                Content::Text(str) => {
                    self.buffer.write_all(b"<mtext>")?;
                    self.buffer.write_all(str.as_bytes())?;
                    self.buffer.write_all(b"</mtext>")?;
                },
                Content::Number { content, variant } => {
                    self.buffer.write_all(b"<mn>")?;
                    let buf = &mut [0u8; 4];
                    content.chars().try_for_each(|c| {
                        let content = variant.map_or(c, |v| v.map_char(c));
                        let bytes = content.encode_utf8(buf);
                        self.buffer.write_all(bytes.as_bytes())?;
                        Ok::<(), io::Error>(())
                    })?;
                    self.buffer.write_all(b"</mn>")?;
                },
                Content::Identifier(ident) => {
                    self.buffer.write_all(b"<mi>")?;
                    match ident {
                        Identifier::Str(str) => self.buffer.write_all(str.as_bytes())?,
                        Identifier::Char { content, variant } => {
                            let buf = &mut [0u8; 4];
                            // TODO: Handle the config of ISO vs. LaTeX vs. French vs. Upright
                            let content = variant.map_or(content, |v| v.map_char(content));
                            let bytes = content.encode_utf8(buf);
                            self.buffer.write_all(bytes.as_bytes())?;
                        }
                    }
                    self.buffer.write_all(b"</mi>")?;
                }
                Content::Operator(Operator {
                    content,
                    stretchy,
                    moveable_limits,
                    left_space,
                    right_space,
                    size,
                }) => {
                    self.buffer.write_all(b"<mo")?;
                    if let Some(stretchy) = stretchy {
                        write!(self.buffer, " stretchy=\"{}\"", stretchy)?;
                    }
                    if let Some(moveable_limits) = moveable_limits {
                        write!(self.buffer, " movablelimits=\"{}\"", moveable_limits)?;
                    }
                    if let Some(left_space) = left_space {
                        let (left_space, unit) = tex_to_css_units(left_space);
                        write!(self.buffer, " lspace=\"{}{}\"", left_space, unit)?;
                    }
                    if let Some(right_space) = right_space {
                        let (right_space, unit) = tex_to_css_units(right_space);
                        write!(self.buffer, " rspace=\"{}{}\"", right_space, unit)?;
                    }
                    if let Some(size) = size {
                        let (size, unit) = tex_to_css_units(size);
                        write!(self.buffer, " minsize=\"{}{}\"", size, unit)?;
                        write!(self.buffer, " maxsize=\"{}{}\"", size, unit)?;
                    }
                    self.buffer.write_all(b">")?;
                    let buf = &mut [0u8; 4];
                    let bytes = content.encode_utf8(buf).as_bytes();
                    self.buffer.write_all(bytes)?;
                    self.buffer.write_all(b"</mo>")?;
                },
            },
            Event::BeginGroup => {
                self.buffer.write_all(b"<mrow>")?;
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
                self.buffer.write_all(b"</mrow>")?;
            }
            // This should always be reached in the process of the `BeginGroup` event, and thus we
            // should most likely output and error if it is reached here.
            Event::EndGroup => {
                return Err(io::Error::other(
                    "unbalanced use of `BeginGroup` and `EndGroup` events.",
                ));
            }
            Event::Infix(infix) => {
                if self.buffer.is_empty() {
                    
                }
            }
            Event::Visual(Visual::Fraction(dim)) => {
                let (Some(first), Some(second)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other(
                        "expected two components after a `Fraction` event.",
                    ));
                };
                self.buffer.write_all(b"<mfrac")?;
                if let Some(dim) = dim {
                    let (dim, unit) = tex_to_css_units(dim);
                    write!(self.buffer, " linethickness=\"{}{}\"", dim, unit)?;
                }
                self.handle_event(first)?;
                self.handle_event(second)?;
                self.buffer.write_all(b"</mfrac>")?;
            }
            Event::Visual(Visual::Root) => {
                let Some(degree) = self.input.next() else {
                    return Err(io::Error::other(
                        "expected two components after a `Root` event.",
                    ));
                };
                self.buffer.write_all(b"<msqrt>")?;
                self.handle_event(degree)?;
                self.buffer.write_all(b"</msqrt>")?;
            }
            Event::Space {
                width,
                height,
                depth,
            } => {
                if let Some(width) = width {
                    let (width, unit) = tex_to_css_units(width);
                    write!(self.buffer, "<mspace width=\"{}{}\"", width, unit)?;
                    if width < 0.0 {
                        write!(self.buffer, " style=\"margin-left: {}{}\"", width, unit)?;
                    }
                }
                if let Some(height) = height {
                    let (height, unit) = tex_to_css_units(height);
                    write!(self.buffer, " height=\"{}{}\"", height, unit)?;
                }
                if let Some(depth) = depth {
                    let (depth, unit) = tex_to_css_units(depth);
                    write!(self.buffer, " depth=\"{}{}\"", depth, unit)?;
                }
                self.buffer.write_all(b" />")?;
            }
        }
        Ok(())
    }
}

/// Takes a [`Parser`] as input and returns a string of MathML.
pub fn push_html<'a, I>(string: &mut String, parser: I) -> io::Result<()>
where
    I: Iterator<Item = Event<'a>>,
{
    MathmlWriter::new(parser, unsafe { string.as_mut_vec() }).write()
}
