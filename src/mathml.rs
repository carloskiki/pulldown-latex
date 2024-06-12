//! A simple MathML Core renderer.
//!
//! This crate provides a "simple" `mathml` renderer which is available through the
//! [`push_mathml`] and [`write_mathml`] functions.

use std::{io, iter::Peekable};

use crate::{
    attribute::{tex_to_css_em, Font},
    config::{DisplayMode, RenderConfig},
    event::{
        ArrayColumn, ColorChange, ColorTarget, Content, DelimiterType, Event, Grouping,
        ScriptPosition, ScriptType, StateChange, Style, Visual,
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
        let env_stack = Vec::with_capacity(16);
        Self {
            input: input.peekable(),
            writer,
            config,
            env_stack,
            state_stack,
        }
    }

    fn open_tag(&mut self, tag: &str, additional_style: Option<&str>) -> io::Result<()> {
        let State {
            text_color,
            border_color,
            background_color,
            style,
            font: _,
        } = *self.state();
        write!(self.writer, "<{}", tag)?;
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
        Ok(())
    }

    fn state(&self) -> &State<'a> {
        self.state_stack.last().expect("state stack is empty")
    }

    fn write_event(&mut self, event: Result<Event<'a>, E>) -> io::Result<()> {
        match event {
            Ok(Event::Content(content)) => self.write_content(content, false),
            // TODO: environments.
            // Gather: Does not accept alignments, only newlines. eveything is centered.
            // Align: Columns do the following:
            //     right, left, space, right, left, space ...
            // Array: requires a column specification.
            // Cases: Only one alignment allowed, has considerable (probably \quad) space between columns.
            Ok(Event::Begin(grouping)) => {
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
                let mut new_state = State::default();

                let env_group = match grouping {
                    Grouping::Internal | Grouping::Normal => {
                        new_state.font = font;
                        self.open_tag("mrow", None)?;
                        self.writer.write_all(b">")?;
                        EnvGrouping::Normal
                        
                    }
                    Grouping::Relation => {
                        new_state.font = font;
                        self.open_tag("mrow", None)?;
                        self.writer.write_all(b">")?;
                        EnvGrouping::Relation
                    },
                    Grouping::LeftRight(opening, closing) => {
                        new_state.font = font;
                        self.open_tag("mrow", None)?;
                        self.writer.write_all(b">")?;
                        if let Some(delim) = opening {
                            self.open_tag("mo", None)?;
                            self.writer.write_all(b" stretchy=\"true\">")?;
                            let mut buf = [0u8; 4];
                            self.writer
                                .write_all(delim.encode_utf8(&mut buf).as_bytes())?;
                            self.writer.write_all(b"</mo>")?;
                        }
                        EnvGrouping::LeftRight { closing }
                    }
                    Grouping::Align => {
                        self.open_tag("mtable", None)?;
                        self.writer.write_all(b"><mtr><mtd style=\"text-align: right\">")?;
                        EnvGrouping::Align { left_align: false }
                    },
                    Grouping::Matrix => {
                        self.open_tag("mtable", None)?;
                        self.writer.write_all(b"><mtr><mtd>")?;
                        EnvGrouping::Matrix
                    }
                    Grouping::Cases => {
                        self.open_tag("mrow", None)?;
                        self.writer.write_all(b">")?;
                        self.writer.write_all(b"><mo stretchy=\"true\">{</mo><mtable><mtr><mtd style=\"text-align: left\">")?;
                        EnvGrouping::Cases { used_align: false }
                    }
                    Grouping::Array(cols) => {
                        let mut index = 0;
                        let additional_style = match (cols.first(), cols.last()) {
                            (Some(ArrayColumn::VerticalLine), Some(ArrayColumn::VerticalLine)) =>  {
                                index += 1;
                                Some(
                                "border-left: 0.06em solid; border-right: 0.06em solid; border-collapse: collapse;"
                                )
                            },
                            (Some(ArrayColumn::VerticalLine), None) => {
                                index += 1;
                                Some(
                                    "border-left: 0.06em solid; border-collapse: collapse;"
                                    )
                            }
                            (None, Some(ArrayColumn::VerticalLine)) => Some("border-right: 0.06em solid; border-collapse: collapse;"),
                            _ => None,
                        };
                        self.open_tag("mtable", additional_style)?;
                        self.writer.write_all(b"><mtr><mtd")?;
                        match cols[index] {
                            ArrayColumn::Left => self.writer.write_all(b" style=\"text-align: left\""),
                            ArrayColumn::Right => self.writer.write_all(b" style=\"text-align: right\""),
                            ArrayColumn::Center => self.writer.write_all(b">"),
                            ArrayColumn::VerticalLine => panic!("vertical line in place of column alignment"),
                        }?;
                        EnvGrouping::Array { cols, cols_index: index + 1 }
                    }
                };
                self.state_stack.push(new_state);
                self.env_stack
                    .push(Environment::from(env_group));
                Ok(())
            }
            Ok(Event::End) => {
                let env = self
                    .env_stack
                    .pop()
                    .expect("cannot pop an environment in group end");
                let Environment::Group(grouping) = env else {
                    panic!("unexpected environment in group end");
                };
                self.state_stack
                    .pop()
                    .expect("cannot pop a state in group end");
                match grouping {
                    EnvGrouping::Normal | EnvGrouping::Relation => {
                        self.writer.write_all(b"</mrow>")
                    }
                    EnvGrouping::LeftRight { closing } => {
                        if let Some(delim) = closing {
                            self.open_tag("mo", None)?;
                            self.writer.write_all(b" stretchy=\"true\">")?;
                            let mut buf = [0u8; 4];
                            self.writer
                                .write_all(delim.encode_utf8(&mut buf).as_bytes())?;
                            self.writer.write_all(b"</mo>")?;
                        }
                        self.writer.write_all(b"</mrow>")
                    }
                    EnvGrouping::Matrix | EnvGrouping::Align { .. } | EnvGrouping::Array { .. } => {
                        self.writer.write_all(b"</mtd></mtr></mtable>")
                    }
                    EnvGrouping::Cases { .. } => {
                        self.writer.write_all(b"</mtd></mtr></mtable></mrow>")
                    }
                }
            }
            Ok(Event::Visual(visual)) => {
                if visual == Visual::Negation {
                    match self.input.peek() {
                        Some(Ok(Event::Content(
                            content @ Content::Ordinary { .. }
                            | content @ Content::Relation { .. }
                            | content @ Content::BinaryOp { .. }
                            | content @ Content::LargeOp { .. }
                            | content @ Content::Delimiter { .. }
                            | content @ Content::Punctuation(_),
                        ))) => {
                            let content = *content;
                            self.write_content(content, true)?;
                            self.input.next();
                        }
                        _ => {
                            self.open_tag("mrow", Some("background: linear-gradient(to top left, rgba(0,0,0,0) 0%, rgba(0,0,0,0) calc(50% - 0.8px), rgba(0,0,0,1) 50%, rgba(0,0,0,0) calc(50% + 0.8px), rgba(0,0,0,0) 100%)"))?;
                            self.writer.write_all(b">")?;
                        }
                    }
                    return Ok(());
                }

                let env = Environment::from(visual);
                self.env_stack.push(env);
                self.open_tag(visual_tag(visual), None)?;
                if let Visual::Fraction(Some(dim)) = visual {
                    write!(self.writer, " linethickness=\"{}em\"", tex_to_css_em(dim))?;
                }

                self.writer.write_all(b">")
            }

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
                let env = Environment::from((ty, above_below));
                self.env_stack.push(env);
                self.open_tag(script_tag(ty, above_below), None)?;
                self.writer.write_all(b">")
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
            Ok(Event::NewLine) => {
                *self.state_stack.last_mut().expect("state stack is empty") = State::default();
                match self.env_stack.last_mut() {
                    Some(Environment::Group(EnvGrouping::Cases { .. })) => self
                        .writer
                        .write_all(b"</mtd></mtr><mtr><mtd style=\"text-align: left\">"),
                    Some(Environment::Group(EnvGrouping::Matrix)) => {
                        self.writer.write_all(b"</mtd></mtr><mtr><mtd>")
                    }
                    Some(Environment::Group(EnvGrouping::Align { left_align })) => {
                        *left_align = false;
                        self.writer
                            .write_all(b"</mtd></mtr><mtr><mtd style=\"text-align: left\">")
                    }
                    Some(Environment::Group(EnvGrouping::Array { cols, cols_index })) => {
                        *cols_index = (cols.first() == Some(&ArrayColumn::VerticalLine)) as usize + 1;
                        self.writer.write_all(b"</mtd></mtr><mtr><mtd")?;
                        match cols[*cols_index - 1] {
                            ArrayColumn::Left => self.writer.write_all(b" style=\"text-align: left\""),
                            ArrayColumn::Right => self.writer.write_all(b" style=\"text-align: right\">"),
                            ArrayColumn::Center => self.writer.write_all(b">"),
                            ArrayColumn::VerticalLine => panic!("vertical line in place of column alignment"),
                        }
                    }
                    _ => panic!("math env does not support newlines"),
                }
            }
            Ok(Event::Alignment) => {
                *self.state_stack.last_mut().expect("state stack is empty") = State::default();
                match self.env_stack.last_mut() {
                    // Left align both
                    Some(Environment::Group(EnvGrouping::Cases { used_align: false })) => self
                        .writer
                        .write_all(b"</mtd><mtd style=\"text-align: left\">"),
                    // Center align all
                    Some(Environment::Group(EnvGrouping::Matrix)) => {
                        self.writer.write_all(b"</mtd><mtd>")
                    }
                    Some(Environment::Group(EnvGrouping::Array {
                        cols,
                        cols_index,
                    })) => {
                        self.writer.write_all(b"</mtd><mtd style=\"")?;
                        if cols[*cols_index] == ArrayColumn::VerticalLine {
                            self.writer.write_all(b"border-left: 0.06em solid; ")?;
                            *cols_index += 1;
                        }
                        self.writer.write_all(match cols[*cols_index] {
                            ArrayColumn::Left => b"text-align: left\">",
                            ArrayColumn::Right => b"text-align: right\">",
                            ArrayColumn::Center => b"\">",
                            ArrayColumn::VerticalLine => panic!("vertical line in place of column alignment"),
                        })?;
                        *cols_index += 1;
                        Ok(())
                    },
                    Some(Environment::Group(EnvGrouping::Align { left_align })) => {
                        *left_align = !*left_align;
                        write!(
                            self.writer,
                            "</mtd><mtd style=\"text-align: {}\">",
                            if *left_align { "left" } else { "right" }
                        )
                    }
                    _ => panic!("alignment outside of environment"),
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
                self.writer.write_all(b"</mtext></merror>")
            }
        }
    }

    fn write_content(&mut self, content: Content<'a>, negate: bool) -> io::Result<()> {
        let mut buf = [0u8; 4];
        match content {
            Content::Text(text) => {
                self.open_tag("mtext", None)?;
                self.writer.write_all(b">")?;
                self.writer.write_all(text.as_bytes())?;
                self.writer.write_all(b"</mtext>")
            }
            Content::Number(number) => {
                self.open_tag("mn", None)?;
                self.writer.write_all(b">")?;
                let buf = &mut [0u8; 4];
                number.chars().try_for_each(|c| {
                    let content = self.state().font.map_or(c, |v| v.map_char(c));
                    let bytes = content.encode_utf8(buf);
                    self.writer.write_all(bytes.as_bytes())
                })?;
                self.writer.write_all(b"</mn>")
            }
            // TODO: script shenanigans and parens vs. no parens.
            Content::Function(str) => {
                self.open_tag("mi", None)?;
                self.writer.write_all(if str.chars().count() == 1 {
                    b" mathvariant=\"normal\">"
                } else {
                    b">"
                })?;
                self.writer.write_all(str.as_bytes())?;
                self.writer.write_all(b"</mi>")

                // TODO: Add function application symbol when no paren is there.
                // let to_append = "<mo>\u{2061}</mo><mspace width=\"0.1667em\" />";
            }
            Content::Ordinary { content, stretchy } => {
                if stretchy {
                    self.writer.write_all(b"<mo stretchy=\"true\">")?;
                    self.writer
                        .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                    self.writer.write_all(b"</mo>")
                } else {
                    self.open_tag("mi", None)?;
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
                    if negate {
                        self.writer.write_all("\u{20D2}".as_bytes())?;
                    }
                    self.writer.write_all(b"</mi>")
                }
            }
            // TODO: Follow plain tex's rules for spacing:
            // Varing is a binary when:
            // 1. preceded by closing
            // 3. preceded by punctuation
            // 4. preceded by number
            // 5. preceded by normal
            // and:
            // 1. followed by closing
            // 4. followed by number
            // 5. followed by normal
            //
            // If the current item is a Bin atom, and if this was the first atom in the list,
            // or if the most recent previous atom was Bin, Op, Rel, Open, or Punct, change the current
            // Bin to Ord and continue with Rule 14. Otherwise continue with Rule 17.
            //
            // If the current item is a Rel or Close or Punct atom, and if the most recent previous atom
            // was Bin, change that previous Bin to Ord. Continue with Rule 17.
            // TODO: The common code of all this should be refactored.
            Content::BinaryOp { content, small } => {
                self.open_tag("mo", small.then_some("font-size: 70%"))?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                self.writer.write_all(b"</mo>")
            }
            Content::Relation {
                content,
                unicode_variant,
                small,
            } => {
                self.open_tag("mo", small.then_some("font-size: 70%"))?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if unicode_variant {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                if negate {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                self.writer.write_all(b"</mo>")
            }

            Content::LargeOp { content, small } => {
                self.open_tag("mo", None)?;
                if small {
                    self.writer.write_all(b" largeop=\"false\"")?;
                }
                self.writer.write_all(b" movablelimits=\"false\">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                self.writer.write_all(b"</mo>")
            }
            Content::Delimiter { content, size, ty } => {
                self.open_tag("mo", None)?;
                write!(self.writer, " stretchy=\"{}\"", ty == DelimiterType::Fence)?;
                if let Some(size) = size {
                    write!(
                        self.writer,
                        "minsize=\"{}em\" maxsize=\"{}em\"",
                        size.to_em(),
                        size.to_em()
                    )?;
                }
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                self.writer.write_all(b"</mo>")
            }
            Content::Punctuation(content) => {
                self.open_tag("mo", None)?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{20D2}".as_bytes())?;
                }
                self.writer.write_all(b"</mo>")
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

            while let Some((tag, count)) = self.env_stack.last_mut().and_then(|env| match env {
                Environment::Group(_) => None,
                Environment::Visual { ty, count } => Some((visual_tag(*ty), count)),
                Environment::Script {
                    ty,
                    above_below,
                    count,
                } => Some((script_tag(*ty, *above_below), count)),
            }) {
                if *count == 0 {
                    self.writer.write_all(b"</")?;
                    self.writer.write_all(tag.as_bytes())?;
                    self.writer.write_all(b">")?;
                    self.env_stack.pop();
                    continue;
                }
                *count -= 1;
                break;
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

#[derive(Debug, Clone, PartialEq)]
enum EnvGrouping {
    Normal,
    Relation,
    LeftRight {
        closing: Option<char>,
    },
    Array {
        cols: Box<[ArrayColumn]>,
        cols_index: usize,
    },
    Matrix,
    Cases {
        used_align: bool,
    },
    Align {
        left_align: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Environment {
    Group(EnvGrouping),
    Visual {
        ty: Visual,
        count: u8,
    },
    Script {
        ty: ScriptType,
        above_below: bool,
        count: u8,
    },
}

impl From<EnvGrouping> for Environment {
    fn from(v: EnvGrouping) -> Self {
        Self::Group(v)
    }
}

impl From<(ScriptType, bool)> for Environment {
    fn from((ty, above_below): (ScriptType, bool)) -> Self {
        let count = match ty {
            ScriptType::Subscript => 2,
            ScriptType::Superscript => 2,
            ScriptType::SubSuperscript => 3,
        };
        Self::Script {
            ty,
            above_below,
            count,
        }
    }
}

impl From<Visual> for Environment {
    fn from(v: Visual) -> Self {
        let count = match v {
            Visual::SquareRoot => 1,
            Visual::Root => 2,
            Visual::Fraction(_) => 2,
            Visual::Negation => 1,
        };
        Self::Visual { ty: v, count }
    }
}

fn script_tag(ty: ScriptType, above_below: bool) -> &'static str {
    match (ty, above_below) {
        (ScriptType::Subscript, false) => "msub",
        (ScriptType::Superscript, false) => "msup",
        (ScriptType::SubSuperscript, false) => "msubsup",
        (ScriptType::Subscript, true) => "munder",
        (ScriptType::Superscript, true) => "mover",
        (ScriptType::SubSuperscript, true) => "munderover",
    }
}

fn visual_tag(visual: Visual) -> &'static str {
    match visual {
        Visual::Root => "mroot",
        Visual::Fraction(_) => "mfrac",
        Visual::SquareRoot => "msqrt",
        Visual::Negation => "mrow",
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct State<'a> {
    font: Option<Font>,
    text_color: Option<&'a str>,
    border_color: Option<&'a str>,
    background_color: Option<&'a str>,
    style: Option<Style>,
}

/// Takes a [`Parser`], or any `Iterator<Item = Result<Event<'_>, E>>` as input, and renders a
/// string of MathML into the input string.
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
