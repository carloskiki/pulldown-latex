use std::io;

use crate::{
    attribute::tex_to_css_units,
    event::{Event, Visual},
};

struct MathmlWriter<I, W> {
    input: I,
    writer: W,
}

// TODO: Should we make css unit conversion produce a string directly?
impl<'a, I, W> MathmlWriter<I, W>
where
    I: Iterator<Item = Event<'a>>,
    W: io::Write,
{
    fn new(input: I, writer: W) -> Self {
        Self { input, writer }
    }

    pub fn write(mut self) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // Where the writer is written to:
        // - using `write_all` with a utf-8 string.
        // - uwing `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
        //      they are formatted using the `Display` trait.
        
        let mut events = self.input.into_iter().peekable();
        while let Some(event) = events.next() {}
        Ok(())
         // If the stream of events was not modified after being created \
         // by the parser, this is a bug, and should be reported on GitHub."));
    }

    fn handle_event(&mut self, event: Event<'a>) -> io::Result<()> {
        match event {
            Event::Content(_) => todo!(),
            Event::BeginGroup => {
                self.writer.write_all(b"<mrow>")?;
                while let Some(event) = self.input.next() {
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
                return Err(io::Error::other("unbalanced use of `BeginGroup` and `EndGroup` events."));
            }
            // If this event was recieved here, it is because no content was present before it.
            // e.g.: `5{_2}`
            Event::Infix(infix) => todo!(),

            Event::Visual(Visual::Fraction(dim)) => {
                let (Some(first), Some(second)) = (self.input.next(), self.input.next()) else {
                    return Err(io::Error::other("expected two components after a `Fraction` event."));
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
            Event::Visual(Visual::Root) => {
                let Some(degree) = self.input.next() else {
                    return Err(io::Error::other("expected two components after a `Root` event."));
                };
                self.writer.write_all(b"<msqrt>")?;
                self.handle_event(degree)?;
                self.writer.write_all(b"</msqrt>")?;
            },
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
