use std::io;

use crate::{event::Event, attribute::tex_to_css_units};


struct MathmlWriter<I, W> {
    input: I,
    writer: W,
}

impl<'a, I, W> MathmlWriter<I, W>
    where
    I: IntoIterator<Item = Event<'a>>,
    W: io::Write,
{
    fn new(input: I, writer: W) -> Self {
        Self { input, writer }
    }

    fn write(mut self) -> io::Result<()> {
        for event in self.input {
            match event {
                Event::Content(_) => todo!(),
                Event::BeginGroup => self.writer.write_all(b"<mrow>")?,
                Event::EndGroup => self.writer.write_all(b"</mrow>")?,
                // If this event was recieved here, it is because no content was present before it.
                // e.g.: `5{_2}`
                Event::Infix(infix) => todo!(),
                Event::Visual(_) => todo!(),
                Event::Space { width, height, depth } => {
                    if let Some(width) = width {
                        let (width, unit) = tex_to_css_units(width);
                        self.writer.write_all(b"<mspace")?;
                        self.writer.write_all(b" width=\"")?;
                        self.writer.write_all(width.to_string().as_bytes())?;
                        self.writer.write_all(&unit.as_bytes())?;
                        self.writer.write_all(b"\"")?;
                        if width < 0.0 {
                            self.writer.write_all(b" style=\"margin-left: ")?;
                            self.writer.write_all(width.to_string().as_bytes())?;
                            self.writer.write_all(&unit.as_bytes())?;
                            self.writer.write_all(b"\"")?;
                        }
                    }
                    if let Some(height) = height {
                        let (height, unit) = tex_to_css_units(height);
                        self.writer.write_all(b" height=\"")?;
                        self.writer.write_all(height.to_string().as_bytes())?;
                        self.writer.write_all(&unit.as_bytes())?;
                        self.writer.write_all(b"\"")?;
                    }
                    if let Some(depth) = depth {
                        let (depth, unit) = tex_to_css_units(depth);
                        self.writer.write_all(b" depth=\"")?;
                        self.writer.write_all(depth.to_string().as_bytes())?;
                        self.writer.write_all(&unit.as_bytes())?;
                        self.writer.write_all(b"\"")?;
                    }
                    self.writer.write_all(b" />")?;
                },
            }
        }
        Ok(())
    }

    fn infixes(&mut self) -> io::Result<()> {
        todo!()
    }
}


/// Takes a [`Parser`] as input and returns a string of MathML.
pub fn push_html<'a, I>(string: &mut String, parser: I)
    where
    I: IntoIterator<Item = Event<'a>>,
{
}
