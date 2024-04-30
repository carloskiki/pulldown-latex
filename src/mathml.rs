//! A simple `mathml` renderer.
//!
//! This crate provides a "simple" `mathml` renderer which is available through the
//! [`push_mathml`] and [`write_mathml`] functions.

use std::{io, iter::Peekable};

use crate::{
    attribute::{tex_to_css_em, Font},
    config::{DisplayMode, RenderConfig},
    event::{
        ColorChange, ColorTarget, Content, Event, Identifier, Operator, ScriptPosition, ScriptType,
        StateChange, Style, Visual,
    },
};

struct MathmlWriter<'a, I: Iterator, W> {
    input: Peekable<I>,
    writer: W,
    config: RenderConfig<'a>,
    env_stack: Vec<Environment>,
    state_stack: Vec<State<'a>>,
}

impl<'a, I, W, E> MathmlWriter<'a, I, W>
where
    I: Iterator<Item = Result<Event<'a>, E>>,
    W: io::Write,
    E: std::error::Error,
{
    fn new(input: I, writer: W, config: RenderConfig<'a>) -> Self {
        // Size of the buffer is arbitrary for performance guess.
        let mut state_stack = Vec::with_capacity(16);
        state_stack.push(State {
            font: None,
            text_color: None,
            border_color: None,
            background_color: None,
            style: None,
        });
        let env_stack = Vec::with_capacity(32);
        Self {
            input: input.peekable(),
            writer,
            config,
            env_stack,
            state_stack,
        }
    }

    fn open_tag(
        &mut self,
        tag: &str,
        additional_style: Option<&str>,
        close: bool,
    ) -> io::Result<()> {
        let State {
            text_color,
            border_color,
            background_color,
            style,
            font: _,
        } = *self.state();
        write!(self.writer, "<{}", tag)?;
        if text_color.is_none()
            && border_color.is_none()
            && background_color.is_none()
            && style.is_none()
        {
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
        if let Some(background_color) = background_color {
            if style_written {
                write!(self.writer, "; background-color: {}", background_color)?;
            } else {
                write!(
                    self.writer,
                    "style=\"background-color: {}",
                    background_color
                )?;
                style_written = true;
            }
        }
        if let Some(additional_style) = additional_style {
            if style_written {
                write!(self.writer, "; {}", additional_style)?;
            } else {
                write!(self.writer, "style=\"{}\"", additional_style)?;
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

    fn state(&self) -> &State<'a> {
        self.state_stack.last().expect("state stack is empty")
    }

    fn write_event(&mut self, event: Result<Event<'a>, E>) -> io::Result<()> {
        match event {
            Ok(Event::Content(content)) => match content {
                Content::Text(text) => {
                    self.open_tag("mtext", None, true)?;
                    self.writer.write_all(text.as_bytes())?;
                    self.writer.write_all(b"</mtext>")
                }
                Content::Number(number) => {
                    self.open_tag("mn", None, true)?;
                    let buf = &mut [0u8; 4];
                    number.chars().try_for_each(|c| {
                        let content = self.state().font.map_or(c, |v| v.map_char(c));
                        let bytes = content.encode_utf8(buf);
                        self.writer.write_all(bytes.as_bytes())
                    })?;
                    self.writer.write_all(b"</mn>")
                }
                Content::Identifier(ident) => match ident {
                    Identifier::Str(str) => {
                        self.open_tag("mi", None, false)?;
                        self.writer.write_all(if str.chars().count() == 1 {
                            b" mathvariant=\"normal\">"
                        } else {
                            b">"
                        })?;
                        self.writer.write_all(str.as_bytes())?;
                        self.writer.write_all(b"</mi>")
                    }
                    Identifier::Char(content) => {
                        self.open_tag("mi", None, false)?;
                        let content = match (
                            self.state().font,
                            self.config.math_style.should_be_upright(content),
                        ) {
                            (Some(Font::UpRight), _) | (None, true) => {
                                self.writer.write_all(b" mathvariant=\"normal\">")?;
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
                        if self.env_stack.last().map(|env| env.env) == Some(EnvironmentType::Negate)
                        {
                            self.writer.write_all("\u{0338}".as_bytes())?;
                            self.env_stack.pop();
                        }
                        self.writer.write_all(b"</mi>")
                    }
                },
                Content::Operator(Operator {
                    content,
                    stretchy,
                    deny_movable_limits,
                    unicode_variant,
                    left_space,
                    right_space,
                    size,
                }) => {
                    self.open_tag("mo", None, false)?;
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
                    let bytes = content
                        .encode_utf8(buf)
                        .as_bytes();
                    self.writer.write_all(bytes)?;
                    if unicode_variant {
                        self.writer.write_all("\u{20D2}".as_bytes())?;
                    }
                    if self.env_stack.last().map(|env| env.env) == Some(EnvironmentType::Negate) {
                        self.writer.write_all("\u{0338}".as_bytes())?;
                        self.env_stack.pop();
                    }
                    self.writer.write_all(b"</mo>")
                }
            },
            Ok(Event::BeginGroup) => {
                let mut font = self.state().font;
                let old_state = self.state_stack.last_mut().expect("state stack is empty");
                while let Some(Ok(Event::StateChange(state_change))) = self.input.peek() {
                    match state_change {
                        StateChange::Font(new_font) => font = *new_font,
                        StateChange::Color(ColorChange { color, target }) => match target {
                            ColorTarget::Text => old_state.text_color = Some(color),
                            ColorTarget::Background => old_state.background_color = Some(color),
                            ColorTarget::Border => old_state.border_color = Some(color),
                        },
                        StateChange::Style(style) => old_state.style = Some(*style),
                    }
                    self.input.next();
                }
                self.state_stack.push(State {
                    font,
                    text_color: None,
                    border_color: None,
                    background_color: None,
                    style: None,
                });
                self.env_stack
                    .push(Environment::new(EnvironmentType::Group));
                self.open_tag("mrow", None, true)
            }
            Ok(Event::EndGroup) => {
                let env = self
                    .env_stack
                    .pop()
                    .expect("cannot pop an environment in group end");
                if env.env != EnvironmentType::Group {
                    panic!("unexpected environment in group end");
                }
                self.state_stack
                    .pop()
                    .expect("cannot pop a state in group end");
                self.writer.write_all(b"</mrow>")
            }
            Ok(Event::Visual(visual)) => match visual {
                Visual::Fraction(dim) => {
                    self.env_stack
                        .push(Environment::new(EnvironmentType::Fraction));
                    self.open_tag("mfrac", None, false)?;
                    if let Some(dim) = dim {
                        write!(self.writer, " linethickness=\"{}em\"", tex_to_css_em(dim))?;
                    }
                    self.writer.write_all(b">")
                }
                Visual::SquareRoot => {
                    self.env_stack.push(Environment::new(EnvironmentType::Sqrt));
                    self.open_tag("msqrt", None, true)
                }
                Visual::Root => {
                    self.env_stack.push(Environment::new(EnvironmentType::Root));
                    self.open_tag("mroot", None, true)
                }
                Visual::Negation => {
                    self.env_stack
                        .push(Environment::new(EnvironmentType::Negate));
                    if !matches!(
                        self.input
                            .peek()
                            .expect("need to be a next event after negation"),
                        Ok(Event::Content(Content::Operator(_)))
                            | Ok(Event::Content(Content::Identifier(Identifier::Char(_))))
                    ) {
                        self.open_tag("mrow", Some("background: linear-gradient(to top left, rgba(0,0,0,0) 0%, rgba(0,0,0,0) calc(50% - 0.8px), rgba(0,0,0,1) 50%, rgba(0,0,0,0) calc(50% + 0.8px), rgba(0,0,0,0) 100%)"), true)
                    } else {
                        Ok(())
                    }
                }
            },

            Ok(Event::Script { ty, position }) => {
                let state = self.state();
                let above_below = match position {
                    ScriptPosition::Right => false,
                    ScriptPosition::AboveBelow => true,
                    ScriptPosition::Movable => {
                        state.style == Some(Style::Display)
                            || (state.style.is_none()
                                && self.config.display_mode == DisplayMode::Block)
                    }
                };
                let env = EnvironmentType::Script(ty, above_below);
                self.env_stack.push(Environment::new(env));
                self.open_tag(env.tag(), None, true)
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
            Ok(Event::StateChange(state_change)) => {
                let state = self.state_stack.last_mut().expect("state stack is empty");
                match state_change {
                    StateChange::Font(font) => state.font = font,
                    StateChange::Color(ColorChange { color, target }) => match target {
                        ColorTarget::Text => state.text_color = Some(color),
                        ColorTarget::Border => state.border_color = Some(color),
                        ColorTarget::Background => state.background_color = Some(color),
                    },
                    StateChange::Style(style) => state.style = Some(style),
                }
                Ok(())
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

    fn write(mut self) -> io::Result<()> {
        // Safety: this function must only write valid utf-8 to the writer.
        // How is the writer used?:
        // - using `write_all` with a utf-8 string.
        // - using `write!` with a utf-8 string, and the parameters must all be valid utf-8 since
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

            if let Some(Environment { env, count }) = self.env_stack.last_mut() {
                if *count == Some(0) {
                    self.writer.write_all(b"</")?;
                    self.writer.write_all(env.tag().as_bytes())?;
                    self.writer.write_all(b">")?;
                    self.env_stack.pop();
                } else if let Some(count) = count {
                    *count -= 1;
                }
            }
        }
        if !self.env_stack.is_empty() || self.state_stack.len() != 1 {
            panic!("unbalanced environment stack or state stack");
        }

        if let Some(annotation) = self.config.annotation {
            write!(
                self.writer,
                "<annotation encoding=\"application/x-tex\">{}</annotation>",
                annotation
            )?;
            self.writer.write_all(b"</semantics>")?;
        }
        self.writer.write_all(b"</math>")
    }
}

#[derive(Debug, Clone, Copy)]
struct Environment {
    env: EnvironmentType,
    count: Option<u8>,
}

impl Environment {
    fn new(env: EnvironmentType) -> Self {
        Self {
            env,
            count: match env {
                EnvironmentType::Group => None,
                EnvironmentType::Fraction => Some(2),
                EnvironmentType::Root => Some(2),
                EnvironmentType::Sqrt => Some(1),
                EnvironmentType::Negate => Some(1),
                EnvironmentType::Script(ScriptType::Subscript, _) => Some(2),
                EnvironmentType::Script(ScriptType::Superscript, _) => Some(2),
                EnvironmentType::Script(ScriptType::SubSuperscript, _) => Some(3),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EnvironmentType {
    Group,
    Fraction,
    Root,
    Sqrt,
    Negate,
    Script(ScriptType, bool),
}

impl EnvironmentType {
    fn tag(&self) -> &'static str {
        match self {
            EnvironmentType::Group => "mrow",
            EnvironmentType::Fraction => "mfrac",
            EnvironmentType::Root => "mroot",
            EnvironmentType::Sqrt => "msqrt",
            EnvironmentType::Negate => "mrow",
            EnvironmentType::Script(ScriptType::Subscript, false) => "msub",
            EnvironmentType::Script(ScriptType::Superscript, false) => "msup",
            EnvironmentType::Script(ScriptType::SubSuperscript, false) => "msubsup",
            EnvironmentType::Script(ScriptType::Subscript, true) => "munder",
            EnvironmentType::Script(ScriptType::Superscript, true) => "mover",
            EnvironmentType::Script(ScriptType::SubSuperscript, true) => "munderover",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct State<'a> {
    font: Option<Font>,
    text_color: Option<&'a str>,
    border_color: Option<&'a str>,
    background_color: Option<&'a str>,
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
