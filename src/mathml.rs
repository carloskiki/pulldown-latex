//! A simple MathML Core renderer.
//!
//! This crate provides a "simple" `mathml` renderer which is available through the
//! [`push_mathml`] and [`write_mathml`] functions.

use std::{
    collections::VecDeque,
    io::{self, Write},
};

use crate::{
    config::{DisplayMode, RenderConfig},
    event::{
        ArrayColumn, ColorChange, ColorTarget, ColumnAlignment, Content, DelimiterType,
        EnvironmentFlow, Event, Font, Grouping, Line, ScriptPosition, ScriptType, StateChange,
        Style, Visual,
    },
};

struct MathmlWriter<'a, I: Iterator, W> {
    input: ManyPeek<I>,
    writer: W,
    config: RenderConfig<'a>,
    env_stack: Vec<Environment>,
    state_stack: Vec<State>,
    previous_atom: Option<Atom>,
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
            input: ManyPeek::new(input),
            writer,
            config,
            env_stack,
            state_stack,
            previous_atom: None,
        }
    }

    fn open_tag(&mut self, tag: &str, classes: Option<&str>) -> io::Result<()> {
        let State {
            text_color,
            border_color,
            background_color,
            style,
            font: _,
        } = *self.state();
        write!(self.writer, "<{}", tag)?;
        if let Some(style) = style {
            if !matches!(
                self.env_stack.last(),
                Some(
                    Environment::Script {
                        ty: ScriptType::Subscript | ScriptType::Superscript,
                        count: 0,
                        ..
                    } | Environment::Script {
                        ty: ScriptType::SubSuperscript,
                        count: 0 | 1,
                        ..
                    }
                )
            ) {
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
        }

        let prefix = |style_written: &mut bool| {
            if !*style_written {
                *style_written = true;
                " style=\""
            } else {
                "; "
            }
        };

        let mut style_written = false;
        if let Some((r, g, b)) = text_color {
            write!(self.writer, " style=\"color: rgb({r} {g} {b})")?;
            style_written = true;
        }
        if let Some((r, g, b)) = border_color {
            write!(
                self.writer,
                "{}border: 0.06em solid rgb({r} {g} {b})",
                prefix(&mut style_written)
            )?;
        }
        if let Some((r, g, b)) = background_color {
            write!(
                self.writer,
                "{}background-color: rgb({r} {g} {b})",
                prefix(&mut style_written)
            )?;
        }
        if style_written {
            self.writer.write_all(b"\"")?;
        }
        if let Some(classes) = classes {
            write!(self.writer, " class=\"{}\"", classes)?;
        }
        Ok(())
    }

    fn write_event(&mut self, event: Result<Event<'a>, E>) -> io::Result<()> {
        match event {
            Ok(Event::Content(content)) => self.write_content(content, false),
            Ok(Event::Begin(grouping)) => {
                // Mathematical environments do something different with state compared to things
                // like left/right and {}.
                //
                // - They do not inherit state from their parent, their state is reset to default
                // upon entering.
                // - State changes occuring within them are also reset when crossing alignments or
                // newlines (`&` or `\\`).
                //
                // For these reasons, they don't use the `open_tag` method.
                // TODO: Make `StateChange::Style` be maintained across math_env groups.
                self.previous_atom = None;
                if grouping.is_math_env() {
                    self.state_stack.push(State::default())
                } else {
                    let last_state = *self.state();
                    self.state_stack.push(last_state);
                    while let Some(Ok(Event::StateChange(state_change))) = self.input.peek_first() {
                        let state_change = *state_change;
                        self.handle_state_change(state_change);
                        self.input.next();
                    }
                    self.open_tag("mrow", None)?;
                    self.writer.write_all(b">")?;
                    // Every state appliable to the style of the mrow is reset, i.e., everything
                    // except font.
                    *self.state_stack.last_mut().expect("state stack is empty") = State {
                        font: self.state().font,
                        ..State::default()
                    };
                }

                macro_rules! env_horizontal_lines {
                    () => {
                        if let Some(Ok(Event::EnvironmentFlow(EnvironmentFlow::StartLines {
                            lines,
                        }))) = self.input.peek_first()
                        {
                            env_horizontal_lines(&mut self.writer, lines)?;
                            self.input.next();
                        }
                    };
                }

                let env_group = match grouping {
                    Grouping::Normal => EnvGrouping::Normal,
                    Grouping::LeftRight(opening, closing) => {
                        if let Some(delim) = opening {
                            self.open_tag("mo", None)?;
                            self.writer.write_all(b" stretchy=\"true\">")?;
                            let mut buf = [0u8; 4];
                            self.writer
                                .write_all(delim.encode_utf8(&mut buf).as_bytes())?;
                            self.writer.write_all(b"</mo>")?;
                        }
                        self.previous_atom = Some(Atom::Open);
                        EnvGrouping::LeftRight { closing }
                    }
                    Grouping::Align { eq_numbers } => {
                        self.writer
                            .write_all(b"<mtable class=\"menv-alignlike menv-align")?;
                        if eq_numbers {
                            self.writer.write_all(b" menv-with-eqn")?;
                        }
                        self.writer.write_all(b"\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Align
                    }
                    Grouping::Matrix { alignment } => {
                        self.writer.write_all(b"<mtable class=\"menv-arraylike")?;
                        self.writer.write_all(match alignment {
                            ColumnAlignment::Left => b" menv-cells-left\"",
                            ColumnAlignment::Center => b"\"",
                            ColumnAlignment::Right => b" menv-cells-right\"",
                        })?;
                        self.writer.write_all(b"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Matrix
                    }
                    Grouping::Cases { left } => {
                        self.writer.write_all(b"<mrow>")?;
                        if left {
                            self.writer.write_all(b"<mo stretchy=\"true\">{</mo>")?;
                        }
                        self.writer
                            .write_all(b"<mtable class=\"menv-cells-left menv-cases\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Cases {
                            left,
                            used_align: false,
                        }
                    }
                    Grouping::Array(cols) => {
                        self.writer
                            .write_all(b"<mtable class=\"menv-arraylike\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b">")?;
                        let index = array_newline(&mut self.writer, &cols)?;
                        EnvGrouping::Array {
                            cols,
                            cols_index: index,
                        }
                    }
                    Grouping::Aligned => {
                        self.writer
                            .write_all(b"<mtable class=\"menv-alignlike menv-align\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Align
                    }
                    Grouping::SubArray { alignment } => {
                        self.writer.write_all(b"<mtable")?;
                        match alignment {
                            crate::event::ColumnAlignment::Left => {
                                self.writer.write_all(b" class=\"menv-cells-left\"")?
                            }
                            crate::event::ColumnAlignment::Center => (),
                            crate::event::ColumnAlignment::Right => {
                                self.writer.write_all(b" class=\"menv-cells-right\"")?
                            }
                        }
                        self.writer.write_all(b"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::SubArray
                    }
                    Grouping::Alignat { pairs, eq_numbers } => {
                        self.writer.write_all(b"<mtable class=\"menv-alignlike")?;
                        if eq_numbers {
                            self.writer.write_all(b" menv-with-eqn")?;
                        }
                        self.writer.write_all(b"\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Alignat {
                            pairs,
                            columns_used: 0,
                        }
                    }
                    Grouping::Alignedat { pairs } => {
                        self.writer.write_all(b"<mtable class=\"menv-alignlike\"")?;
                        self.writer.write_all(b"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Alignat {
                            pairs,
                            columns_used: 0,
                        }
                    }
                    Grouping::Gather { eq_numbers } => {
                        self.writer.write_all(b"<mtable")?;
                        if eq_numbers {
                            self.writer.write_all(b" class=\"menv-with-eqn\"")?;
                        }
                        self.writer.write_all(b"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Gather
                    }
                    Grouping::Gathered => {
                        self.writer.write_all(b"<mtable><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Gather
                    }
                    Grouping::Multline => {
                        self.writer
                            .write_all(b"<mtable class=\"menv-multline\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Multline
                    }
                    Grouping::Split => {
                        self.writer
                            .write_all(b"<mtable class=\"menv-alignlike\"><mtr")?;
                        env_horizontal_lines!();
                        self.writer.write_all(b"><mtd>")?;
                        EnvGrouping::Split { used_align: false }
                    }
                    Grouping::Equation { eq_numbers } => {
                        self.writer.write_all(b"<mtable")?;
                        if eq_numbers {
                            self.writer.write_all(b" class=\"menv-with-eqn\"")?;
                        }
                        self.writer.write_all(b"><mtr><mtd>")?;
                        EnvGrouping::Equation
                    },
                };
                self.env_stack.push(Environment::from(env_group));
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
                self.previous_atom = Some(Atom::Inner);
                match grouping {
                    EnvGrouping::Normal => self.writer.write_all(b"</mrow>"),
                    EnvGrouping::LeftRight { closing } => {
                        if let Some(delim) = closing {
                            self.open_tag("mo", None)?;
                            self.writer.write_all(b" stretchy=\"true\">")?;
                            let mut buf = [0u8; 4];
                            self.writer
                                .write_all(delim.encode_utf8(&mut buf).as_bytes())?;
                            self.writer.write_all(b"</mo>")?;
                        }
                        self.previous_atom = Some(Atom::Close);
                        self.writer.write_all(b"</mrow>")
                    }
                    EnvGrouping::Matrix
                    | EnvGrouping::Align
                    | EnvGrouping::SubArray
                    | EnvGrouping::Gather
                    | EnvGrouping::Multline
                    | EnvGrouping::Equation
                    | EnvGrouping::Split { .. }
                    | EnvGrouping::Alignat { .. } => {
                        self.writer.write_all(b"</mtd></mtr></mtable>")
                    }
                    EnvGrouping::Array { cols, cols_index } => {
                        self.writer.write_all(b"</mtd>")?;
                        cols[cols_index..]
                            .iter()
                            .map_while(|col| match col {
                                ArrayColumn::Separator(line) => Some(line),
                                _ => None,
                            })
                            .try_for_each(|line| {
                                self.writer.write_all(match line {
                                    Line::Solid => {
                                        b"<mtd class=\"menv-right-solid menv-border-only\"></mtd>"
                                    }
                                    Line::Dashed => {
                                        b"<mtd class=\"menv-right-dashed menv-border-only\"></mtd>"
                                    }
                                })
                            })?;
                        self.writer.write_all(b"</mtr></mtable>")
                    }
                    EnvGrouping::Cases { left, .. } => {
                        self.writer.write_all(b"</mtd></mtr></mtable>")?;
                        if !left {
                            self.writer.write_all(b"<mo stretchy=\"true\">}</mo>")?;
                        }
                        self.writer.write_all(b"</mrow>")
                    }
                }
            }
            Ok(Event::Visual(visual)) => {
                if visual == Visual::Negation {
                    match self.input.peek_first() {
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
                            self.open_tag("mrow", Some("mop-negated"))?;
                            self.writer.write_all(b">")?;
                            self.env_stack.push(Environment::from(visual));
                        }
                    }
                    return Ok(());
                }

                let env = Environment::from(visual);
                self.env_stack.push(env);
                self.open_tag(visual_tag(visual), None)?;
                if let Visual::Fraction(Some(dim)) = visual {
                    write!(self.writer, " linethickness=\"{}\"", dim)?;
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

            Ok(Event::Space { width, height }) => {
                if let Some(width) = width {
                    write!(self.writer, "<mspace width=\"{}\"", width)?;
                    if width.value < 0. {
                        write!(self.writer, " style=\"margin-left: {}\"", width)?;
                    }
                }
                if let Some(height) = height {
                    write!(self.writer, " height=\"{}\"", height)?;
                }
                self.writer.write_all(b" />")
            }
            Ok(Event::StateChange(state_change)) => {
                self.handle_state_change(state_change);
                Ok(())
            }
            Ok(Event::EnvironmentFlow(EnvironmentFlow::NewLine {
                spacing,
                horizontal_lines,
            })) => {
                *self.state_stack.last_mut().expect("state stack should not be empty") = State::default();
                self.previous_atom = None;

                if let Some(Environment::Group(EnvGrouping::Array { cols, cols_index })) =
                    self.env_stack.last()
                {
                    array_close_line(&mut self.writer, &cols[*cols_index..])?;
                } else if let Some(Environment::Group(EnvGrouping::Equation { .. })) = self.env_stack.last() {
                    // LaTeX does _nothing_ when a newline is encountered in an eqution, we do the
                    // same thing.
                    return Ok(());
                } else {
                    self.writer.write_all(b"</mtd></mtr><mtr")?;
                }

                if let Some(spacing) = spacing {
                    write!(self.writer, " style=\"height: {}\">", spacing)?;
                    if let Some(Environment::Group(EnvGrouping::Array { cols, cols_index })) =
                        self.env_stack.last_mut()
                    {
                        let mut index = array_newline(&mut self.writer, cols)?;
                        while index < *cols_index {
                            array_align(&mut self.writer, cols, &mut index)?;
                        }
                        array_close_line(&mut self.writer, &cols[index..])?;
                    } else {
                        self.writer
                            .write_all(b"<mtd class=\"menv-nonumber\"></mtd></mtr><mtr")?;
                    }
                }
                env_horizontal_lines(&mut self.writer, &horizontal_lines)?;

                match self.env_stack.last_mut() {
                    Some(Environment::Group(
                        EnvGrouping::Cases { used_align, .. } | EnvGrouping::Split { used_align },
                    )) => {
                        *used_align = false;
                        self.writer.write_all(b"><mtd>")
                    }
                    Some(Environment::Group(
                        EnvGrouping::Matrix
                        | EnvGrouping::Align
                        | EnvGrouping::Gather
                        | EnvGrouping::SubArray
                        | EnvGrouping::Multline,
                    )) => self.writer.write_all(b"><mtd>"),
                    Some(Environment::Group(EnvGrouping::Array { cols, cols_index })) => {
                        self.writer.write_all(b">")?;
                        let new_index = array_newline(&mut self.writer, cols)?;
                        *cols_index = new_index;
                        Ok(())
                    }
                    Some(Environment::Group(EnvGrouping::Alignat { columns_used, .. })) => {
                        *columns_used = 0;
                        self.writer.write_all(b"><mtd>")
                    }

                    _ => panic!("newline not allowed in current environment"),
                }
            }
            Ok(Event::EnvironmentFlow(EnvironmentFlow::Alignment)) => {
                *self.state_stack.last_mut().expect("state stack is empty") = State::default();
                self.previous_atom = None;
                match self.env_stack.last_mut() {
                    Some(Environment::Group(
                        EnvGrouping::Cases {
                            used_align: false, ..
                        }
                        | EnvGrouping::Split { used_align: false },
                    )) => self.writer.write_all(b"</mtd><mtd>"),
                    Some(Environment::Group(EnvGrouping::Align | EnvGrouping::Matrix)) => {
                        self.writer.write_all(b"</mtd><mtd>")
                    }
                    Some(Environment::Group(EnvGrouping::Alignat {
                        pairs,
                        columns_used,
                    })) if *columns_used / 2 <= *pairs => {
                        *columns_used += 1;
                        self.writer.write_all(b"</mtd><mtd>")
                    }
                    Some(Environment::Group(EnvGrouping::Array { cols, cols_index })) => {
                        array_align(&mut self.writer, cols, cols_index)
                    }
                    _ => panic!("alignment not allowed in current environment"),
                }
            }

            Ok(Event::EnvironmentFlow(EnvironmentFlow::StartLines { .. })) => {
                panic!("unexpected StartLines event found")
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
                let trimmed = text.trim();
                if text.starts_with(char::is_whitespace) {
                    self.writer.write_all(b"&nbsp;")?;
                }
                self.writer.write_all(trimmed.as_bytes())?;
                if text.ends_with(char::is_whitespace) {
                    self.writer.write_all(b"&nbsp;")?;
                }
                self.set_previous_atom(Atom::Ord);
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
                self.set_previous_atom(Atom::Ord);
                self.writer.write_all(b"</mn>")
            }
            Content::Function(str) => {
                if matches!(
                    self.previous_atom,
                    Some(Atom::Inner | Atom::Close | Atom::Ord)
                ) {
                    self.writer
                        .write_all("<mspace width=\"0.1667em\" />".as_bytes())?;
                }

                self.open_tag("mi", None)?;
                self.writer.write_all(if str.chars().count() == 1 {
                    b" mathvariant=\"normal\">"
                } else {
                    b">"
                })?;
                self.writer.write_all(str.as_bytes())?;
                self.set_previous_atom(Atom::Op);
                self.writer.write_all(b"</mi>")?;

                if let Some(Environment::Script { fn_application, .. }) = self.env_stack.last_mut()
                {
                    *fn_application = true;
                } else if let Some(atom) = self.next_atom() {
                    self.writer.write_all("<mo>\u{2061}</mo>".as_bytes())?;
                    if !matches!(atom, Atom::Open | Atom::Punct | Atom::Close) {
                        self.writer
                            .write_all("<mspace width=\"0.1667em\" />".as_bytes())?;
                    }
                };

                Ok(())
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
                        self.writer.write_all("\u{0338}".as_bytes())?;
                    }
                    self.set_previous_atom(Atom::Ord);
                    self.writer.write_all(b"</mi>")
                }
            }
            // TexBook p. 153 and 157 for math classes.
            // TexBook p. 170 for spacing.
            // TexBook p. 438-443 for type setting rules (especially important for Binary Ops)
            Content::BinaryOp { content, small } => {
                let tag = if matches!(
                    self.previous_atom,
                    Some(Atom::Inner | Atom::Close | Atom::Ord)
                ) && !matches!(
                    self.env_stack.last(),
                    Some(
                        Environment::Script { .. }
                            | Environment::Visual {
                                ty: Visual::Root | Visual::Fraction(_) | Visual::SquareRoot,
                                ..
                            }
                    )
                ) && matches!(
                    self.next_atom(),
                    Some(Atom::Inner | Atom::Bin | Atom::Op | Atom::Ord | Atom::Open)
                ) {
                    self.set_previous_atom(Atom::Bin);
                    "mo"
                } else {
                    self.set_previous_atom(Atom::Ord);
                    "mi"
                };

                self.open_tag(tag, small.then_some("small"))?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{0338}".as_bytes())?;
                }
                write!(self.writer, "</{}>", tag)
            }
            Content::Relation { content, small } => {
                let mut buf = [0; 8];
                self.open_tag("mo", small.then_some("small"))?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8_to_buf(&mut buf))?;
                if negate {
                    self.writer.write_all("\u{0338}".as_bytes())?;
                }
                self.set_previous_atom(Atom::Rel);
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
                    self.writer.write_all("\u{0338}".as_bytes())?;
                }
                self.set_previous_atom(Atom::Op);
                self.writer.write_all(b"</mo>")
            }
            Content::Delimiter { content, size, ty } => {
                self.open_tag("mo", None)?;
                write!(
                    self.writer,
                    " symmetric=\"{0}\" stretchy=\"{0}\"",
                    ty == DelimiterType::Fence || size.is_some()
                )?;
                if let Some(size) = size {
                    write!(
                        self.writer,
                        "minsize=\"{0}em\" maxsize=\"{0}em\"",
                        size.to_em()
                    )?;
                }

                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{0338}".as_bytes())?;
                }
                self.set_previous_atom(match ty {
                    DelimiterType::Open => Atom::Open,
                    DelimiterType::Fence => Atom::Punct,
                    DelimiterType::Close => Atom::Close,
                });
                self.writer.write_all(b"</mo>")
            }
            Content::Punctuation(content) => {
                self.open_tag("mo", None)?;
                self.writer.write_all(b">")?;
                self.writer
                    .write_all(content.encode_utf8(&mut buf).as_bytes())?;
                if negate {
                    self.writer.write_all("\u{0338}".as_bytes())?;
                }
                self.set_previous_atom(Atom::Punct);
                self.writer.write_all(b"</mo>")
            }
        }
    }

    fn handle_state_change(&mut self, state_change: StateChange) {
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
    }

    fn set_previous_atom(&mut self, atom: Atom) {
        if !matches!(
            self.env_stack.last(),
            Some(
                Environment::Visual {
                    ty: Visual::Root | Visual::Fraction(_),
                    count: 0
                } | Environment::Script {
                    ty: ScriptType::Subscript | ScriptType::Superscript,
                    count: 0,
                    ..
                } | Environment::Script {
                    ty: ScriptType::SubSuperscript,
                    count: 0 | 1,
                    ..
                }
            )
        ) {
            self.previous_atom = Some(atom);
        }
    }

    fn next_atom(&mut self) -> Option<Atom> {
        let mut index = 0;
        loop {
            let next = match self.input.peeked_nth(index) {
                None => self.input.peek_next()?,
                Some(next) => {
                    index += 1;
                    next
                }
            };

            break match next {
                Ok(
                    Event::StateChange(_)
                    | Event::Space { .. }
                    | Event::Visual(Visual::Negation)
                    | Event::Script { .. },
                ) => continue,
                Ok(Event::End | Event::EnvironmentFlow(_)) | Err(_) => None,
                Ok(Event::Visual(_) | Event::Begin(_)) => Some(Atom::Inner),
                Ok(Event::Content(content)) => match content {
                    Content::BinaryOp { .. } => Some(Atom::Bin),
                    Content::LargeOp { .. } => Some(Atom::Op),
                    Content::Relation { .. } => Some(Atom::Rel),
                    Content::Delimiter {
                        ty: DelimiterType::Open,
                        ..
                    } => Some(Atom::Open),
                    Content::Delimiter {
                        ty: DelimiterType::Close,
                        ..
                    } => Some(Atom::Close),
                    Content::Punctuation(_) => Some(Atom::Punct),
                    _ => Some(Atom::Ord),
                },
            };
        }
    }

    fn state(&self) -> &State {
        self.state_stack.last().expect("state stack is empty")
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
            self.writer.write_all(b"<semantics><mrow>")?;
        }

        while let Some(event) = self.input.next() {
            self.write_event(event)?;

            while let Some((tag, count, fn_application)) =
                self.env_stack.last_mut().and_then(|env| match env {
                    Environment::Group(_) => None,
                    Environment::Visual { ty, count } => Some((visual_tag(*ty), count, None)),
                    Environment::Script {
                        ty,
                        above_below,
                        count,
                        fn_application,
                    } => Some((script_tag(*ty, *above_below), count, Some(*fn_application))),
                })
            {
                if *count != 0 {
                    *count -= 1;
                    break;
                }
                self.writer.write_all(b"</")?;
                self.writer.write_all(tag.as_bytes())?;
                self.writer.write_all(b">")?;
                self.set_previous_atom(Atom::Inner);
                self.env_stack.pop();

                if fn_application.unwrap_or(false) {
                    if let Some(atom) = self.next_atom() {
                        self.writer.write_all("<mo>\u{2061}</mo>".as_bytes())?;

                        if !matches!(atom, Atom::Open | Atom::Punct | Atom::Close) {
                            self.writer
                                .write_all("<mspace width=\"0.1667em\" />".as_bytes())?;
                        }
                    }
                }
            }
        }

        if !self.env_stack.is_empty() || self.state_stack.len() != 1 {
            panic!("unbalanced environment stack or state stack");
        }

        if let Some(annotation) = self.config.annotation {
            self.writer.write_all(b"</mrow>")?;
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

fn array_newline<W: Write>(writer: &mut W, cols: &[ArrayColumn]) -> io::Result<usize> {
    let mut index = 0;
    writer.write_all(b"<mtd")?;
    cols.windows(2)
        .map_while(|window| match window[..2] {
            [ArrayColumn::Separator(line), ArrayColumn::Separator(_)] => Some(line),
            _ => None,
        })
        .try_for_each(|line| {
            index += 1;
            writer.write_all(match line {
                Line::Solid => b" class=\"menv-left-solid menv-border-only\"></mtd><mtd",
                Line::Dashed => b" class=\"menv-left-dashed menv-border-only\"></mtd><mtd",
            })
        })?;

    let to_append: &[u8] = match (cols.get(index), cols.get(index + 1)) {
        (Some(ArrayColumn::Separator(line)), Some(ArrayColumn::Column(col))) => {
            writer.write_all(match (line, col) {
                (Line::Solid, ColumnAlignment::Left) => b" class=\"menv-left-solid cell-left",
                (Line::Solid, ColumnAlignment::Center) => b" class=\"menv-left-solid",
                (Line::Solid, ColumnAlignment::Right) => b" class=\"menv-left-solid cell-right",
                (Line::Dashed, ColumnAlignment::Left) => b" class=\"menv-left-dashed cell-left",
                (Line::Dashed, ColumnAlignment::Center) => b" class=\"menv-left-dashed",
                (Line::Dashed, ColumnAlignment::Right) => b" class=\"menv-left-dashed cell-right",
            })?;
            index += 2;

            if let Some(ArrayColumn::Separator(line)) = cols.get(index) {
                index += 1;
                match line {
                    Line::Solid => b" menv-right-solid\">",
                    Line::Dashed => b" menv-right-dashed\">",
                }
            } else {
                b"\">"
            }
        }
        (Some(ArrayColumn::Column(col)), Some(ArrayColumn::Separator(line))) => {
            index += 2;
            match (col, line) {
                (ColumnAlignment::Left, Line::Solid) => b" class=\"cell-left menv-right-solid\">",
                (ColumnAlignment::Left, Line::Dashed) => b" class=\"cell-left menv-right-dashed\">",
                (ColumnAlignment::Center, Line::Solid) => b" class=\"menv-right-solid\">",
                (ColumnAlignment::Center, Line::Dashed) => b" class=\"menv-right-dashed\">",
                (ColumnAlignment::Right, Line::Solid) => b" class=\"cell-right menv-right-solid\">",
                (ColumnAlignment::Right, Line::Dashed) => {
                    b" class=\"cell-right menv-right-dashed\">"
                }
            }
        }
        (Some(ArrayColumn::Column(col)), _) => {
            index += 1;
            match col {
                ColumnAlignment::Left => b" class=\"cell-left\">",
                ColumnAlignment::Center => b">",
                ColumnAlignment::Right => b" class=\"cell-right\">",
            }
        }
        (None, None) => b">",
        _ => unreachable!(),
    };
    writer.write_all(to_append)?;

    Ok(index)
}

fn array_align<W: Write>(
    writer: &mut W,
    cols: &[ArrayColumn],
    cols_index: &mut usize,
) -> io::Result<()> {
    writer.write_all(b"</mtd><mtd")?;
    cols[*cols_index..]
        .iter()
        .map_while(|col| match col {
            ArrayColumn::Separator(line) => Some(line),
            _ => None,
        })
        .try_for_each(|line| {
            *cols_index += 1;
            writer.write_all(match line {
                Line::Solid => b" class=\"menv-right-solid menv-border-only\"></mtd><mtd",
                Line::Dashed => b" class=\"menv-right-dashed menv-border-only\"></mtd><mtd",
            })
        })?;

    let to_append: &[u8] = match (cols[*cols_index], cols.get(*cols_index + 1)) {
        (ArrayColumn::Column(col), Some(ArrayColumn::Separator(line))) => {
            *cols_index += 2;
            match (col, line) {
                (ColumnAlignment::Left, Line::Solid) => b" class=\"cell-left menv-right-solid\">",
                (ColumnAlignment::Left, Line::Dashed) => b" class=\"cell-left menv-right-dashed\">",
                (ColumnAlignment::Center, Line::Solid) => b" class=\"menv-right-solid\">",
                (ColumnAlignment::Center, Line::Dashed) => b" class=\"menv-right-dashed\">",
                (ColumnAlignment::Right, Line::Solid) => b" class=\"cell-right menv-right-solid\">",
                (ColumnAlignment::Right, Line::Dashed) => {
                    b" class=\"cell-right menv-right-dashed\">"
                }
            }
        }
        (ArrayColumn::Column(col), _) => {
            *cols_index += 1;
            match col {
                ColumnAlignment::Left => b" class=\"cell-left\">",
                ColumnAlignment::Center => b">",
                ColumnAlignment::Right => b" class=\"cell-right\">",
            }
        }
        (ArrayColumn::Separator(_), _) => unreachable!(),
    };
    writer.write_all(to_append)
}

fn array_close_line<W: Write>(writer: &mut W, rest_cols: &[ArrayColumn]) -> io::Result<()> {
    writer.write_all(b"</mtd>")?;
    rest_cols
        .iter()
        .map_while(|col| match col {
            ArrayColumn::Separator(line) => Some(line),
            _ => None,
        })
        .try_for_each(|line| {
            writer.write_all(match line {
                Line::Solid => b"<mtd class=\"menv-right-solid menv-border-only\"></mtd>",
                Line::Dashed => b"<mtd class=\"menv-right-dashed menv-border-only\"></mtd>",
            })
        })?;
    writer.write_all(b"</mtr><mtr")
}

fn env_horizontal_lines<W: Write>(writer: &mut W, lines: &[Line]) -> io::Result<()> {
    let mut iter = lines.iter();
    if let Some(last_line) = iter.next_back() {
        iter.try_for_each(|line| {
            writer.write_all(match line {
                Line::Solid => {
                    b" class=\"menv-hline\"><mtd class=\"menv-nonumber\"></mtd></mtr><mtr"
                }
                Line::Dashed => {
                    b" class=\"menv-hdashline\"><mtd class=\"menv-nonumber\"></mtd></mtr><mtr"
                }
            })
        })?;
        writer.write_all(match last_line {
            Line::Solid => b" class=\"menv-hline\"",
            Line::Dashed => b" class=\"menv-hdashline\"",
        })?;
    };
    Ok(())
}

enum Atom {
    Bin,
    Op,
    Rel,
    Open,
    Close,
    Punct,
    Ord,
    Inner,
}

#[derive(Debug, Clone, PartialEq)]
enum EnvGrouping {
    Normal,
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
        left: bool,
    },
    Align,
    Alignat {
        pairs: u16,
        columns_used: u16,
    },
    SubArray,
    Gather,
    Multline,
    Split {
        used_align: bool,
    },
    Equation,
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
        fn_application: bool,
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
            fn_application: false,
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
struct State {
    font: Option<Font>,
    text_color: Option<(u8, u8, u8)>,
    border_color: Option<(u8, u8, u8)>,
    background_color: Option<(u8, u8, u8)>,
    style: Option<Style>,
}

struct ManyPeek<I: Iterator> {
    iter: I,
    peeked: VecDeque<I::Item>,
}

impl<I: Iterator> ManyPeek<I> {
    fn new(iter: I) -> Self {
        Self {
            iter,
            peeked: VecDeque::new(),
        }
    }

    fn peek_next(&mut self) -> Option<&I::Item> {
        self.peeked.push_back(self.iter.next()?);
        self.peeked.back()
    }

    fn peeked_nth(&self, n: usize) -> Option<&I::Item> {
        self.peeked.get(n)
    }

    fn peek_first(&mut self) -> Option<&I::Item> {
        if self.peeked.is_empty() {
            self.peek_next()
        } else {
            self.peeked.front()
        }
    }
}

impl<I: Iterator> Iterator for ManyPeek<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.peeked.pop_front().or_else(|| self.iter.next())
    }
}

impl Font {
    /// Map a character to its mathvariant equivalent.
    fn map_char(self, c: char) -> char {
        char::from_u32(match (self, c) {
            // Bold Script mappings
            (Font::BoldScript, 'A'..='Z') => c as u32 + 0x1D48F,
            (Font::BoldScript, 'a'..='z') => c as u32 + 0x1D489,

            // Bold Italic mappings
            (Font::BoldItalic, 'A'..='Z') => c as u32 + 0x1D427,
            (Font::BoldItalic, 'a'..='z') => c as u32 + 0x1D421,
            (Font::BoldItalic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D38B
            }
            (Font::BoldItalic, '\u{03F4}') => c as u32 + 0x1D339,
            (Font::BoldItalic, '\u{2207}') => c as u32 + 0x1B52E,
            (Font::BoldItalic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D385,
            (Font::BoldItalic, '\u{2202}') => c as u32 + 0x1B54D,
            (Font::BoldItalic, '\u{03F5}') => c as u32 + 0x1D35B,
            (Font::BoldItalic, '\u{03D1}') => c as u32 + 0x1D380,
            (Font::BoldItalic, '\u{03F0}') => c as u32 + 0x1D362,
            (Font::BoldItalic, '\u{03D5}') => c as u32 + 0x1D37E,
            (Font::BoldItalic, '\u{03F1}') => c as u32 + 0x1D363,
            (Font::BoldItalic, '\u{03D6}') => c as u32 + 0x1D37F,

            // Bold mappings
            (Font::Bold, 'A'..='Z') => c as u32 + 0x1D3BF,
            (Font::Bold, 'a'..='z') => c as u32 + 0x1D3B9,
            (Font::Bold, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => c as u32 + 0x1D317,
            (Font::Bold, '\u{03F4}') => c as u32 + 0x1D2C5,
            (Font::Bold, '\u{2207}') => c as u32 + 0x1B4BA,
            (Font::Bold, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D311,
            (Font::Bold, '\u{2202}') => c as u32 + 0x1B4D9,
            (Font::Bold, '\u{03F5}') => c as u32 + 0x1D2E7,
            (Font::Bold, '\u{03D1}') => c as u32 + 0x1D30C,
            (Font::Bold, '\u{03F0}') => c as u32 + 0x1D2EE,
            (Font::Bold, '\u{03D5}') => c as u32 + 0x1D30A,
            (Font::Bold, '\u{03F1}') => c as u32 + 0x1D2EF,
            (Font::Bold, '\u{03D6}') => c as u32 + 0x1D30B,
            (Font::Bold, '\u{03DC}' | '\u{03DD}') => c as u32 + 0x1D7CA,
            (Font::Bold, '0'..='9') => c as u32 + 0x1D79E,

            // Fraktur mappings
            (Font::Fraktur, 'A' | 'B' | 'D'..='G' | 'J'..='Q' | 'S'..='Y') => c as u32 + 0x1D4C3,
            (Font::Fraktur, 'C') => c as u32 + 0x20EA,
            (Font::Fraktur, 'H' | 'I') => c as u32 + 0x20C4,
            (Font::Fraktur, 'R') => c as u32 + 0x20CA,
            (Font::Fraktur, 'Z') => c as u32 + 0x20CE,
            (Font::Fraktur, 'a'..='z') => c as u32 + 0x1D4BD,

            // Script mappings
            (Font::Script, 'A' | 'C' | 'D' | 'G' | 'J' | 'K' | 'N'..='Q' | 'S'..='Z') => {
                c as u32 + 0x1D45B
            }
            (Font::Script, 'B') => c as u32 + 0x20EA,
            (Font::Script, 'E' | 'F') => c as u32 + 0x20EB,
            (Font::Script, 'H') => c as u32 + 0x20C3,
            (Font::Script, 'I') => c as u32 + 0x20C7,
            (Font::Script, 'L') => c as u32 + 0x20C6,
            (Font::Script, 'M') => c as u32 + 0x20E6,
            (Font::Script, 'R') => c as u32 + 0x20C9,
            (Font::Script, 'a'..='d' | 'f' | 'h'..='n' | 'p'..='z') => c as u32 + 0x1D455,
            (Font::Script, 'e') => c as u32 + 0x20CA,
            (Font::Script, 'g') => c as u32 + 0x20A3,
            (Font::Script, 'o') => c as u32 + 0x20C5,

            // Monospace mappings
            (Font::Monospace, 'A'..='Z') => c as u32 + 0x1D62F,
            (Font::Monospace, 'a'..='z') => c as u32 + 0x1D629,
            (Font::Monospace, '0'..='9') => c as u32 + 0x1D7C6,

            // Sans Serif mappings
            (Font::SansSerif, 'A'..='Z') => c as u32 + 0x1D55F,
            (Font::SansSerif, 'a'..='z') => c as u32 + 0x1D559,
            (Font::SansSerif, '0'..='9') => c as u32 + 0x1D7B2,

            // Double Struck mappings
            (Font::DoubleStruck, 'A' | 'B' | 'D'..='G' | 'I'..='M' | 'O' | 'S'..='Y') => {
                c as u32 + 0x1D4F7
            }
            (Font::DoubleStruck, 'C') => c as u32 + 0x20BF,
            (Font::DoubleStruck, 'H') => c as u32 + 0x20C5,
            (Font::DoubleStruck, 'N') => c as u32 + 0x20C7,
            (Font::DoubleStruck, 'P' | 'Q') => c as u32 + 0x20C9,
            (Font::DoubleStruck, 'R') => c as u32 + 0x20CB,
            (Font::DoubleStruck, 'Z') => c as u32 + 0x20CA,
            (Font::DoubleStruck, 'a'..='z') => c as u32 + 0x1D4F1,
            (Font::DoubleStruck, '0'..='9') => c as u32 + 0x1D7A8,

            // Italic mappings
            (Font::Italic, 'A'..='Z') => c as u32 + 0x1D3F3,
            (Font::Italic, 'a'..='g' | 'i'..='z') => c as u32 + 0x1D3ED,
            (Font::Italic, 'h') => c as u32 + 0x20A6,
            (Font::Italic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => c as u32 + 0x1D351,
            (Font::Italic, '\u{03F4}') => c as u32 + 0x1D2FF,
            (Font::Italic, '\u{2207}') => c as u32 + 0x1B4F4,
            (Font::Italic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D34B,
            (Font::Italic, '\u{2202}') => c as u32 + 0x1B513,
            (Font::Italic, '\u{03F5}') => c as u32 + 0x1D321,
            (Font::Italic, '\u{03D1}') => c as u32 + 0x1D346,
            (Font::Italic, '\u{03F0}') => c as u32 + 0x1D328,
            (Font::Italic, '\u{03D5}') => c as u32 + 0x1D344,
            (Font::Italic, '\u{03F1}') => c as u32 + 0x1D329,
            (Font::Italic, '\u{03D6}') => c as u32 + 0x1D345,

            // Bold Fraktur mappings
            (Font::BoldFraktur, 'A'..='Z') => c as u32 + 0x1D52B,
            (Font::BoldFraktur, 'a'..='z') => c as u32 + 0x1D525,

            // Sans Serif Bold Italic mappings
            (Font::SansSerifBoldItalic, 'A'..='Z') => c as u32 + 0x1D5FB,
            (Font::SansSerifBoldItalic, 'a'..='z') => c as u32 + 0x1D5F5,
            (Font::SansSerifBoldItalic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D3FF
            }
            (Font::SansSerifBoldItalic, '\u{03F4}') => c as u32 + 0x1D3AD,
            (Font::SansSerifBoldItalic, '\u{2207}') => c as u32 + 0x1B5A2,
            (Font::SansSerifBoldItalic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D3F9,
            (Font::SansSerifBoldItalic, '\u{2202}') => c as u32 + 0x1B5C1,
            (Font::SansSerifBoldItalic, '\u{03F5}') => c as u32 + 0x1D3CF,
            (Font::SansSerifBoldItalic, '\u{03D1}') => c as u32 + 0x1D3F4,
            (Font::SansSerifBoldItalic, '\u{03F0}') => c as u32 + 0x1D3D6,
            (Font::SansSerifBoldItalic, '\u{03D5}') => c as u32 + 0x1D3F2,
            (Font::SansSerifBoldItalic, '\u{03F1}') => c as u32 + 0x1D3D7,
            (Font::SansSerifBoldItalic, '\u{03D6}') => c as u32 + 0x1D3F3,

            // Sans Serif Italic mappings
            (Font::SansSerifItalic, 'A'..='Z') => c as u32 + 0x1D5D7,
            (Font::SansSerifItalic, 'a'..='z') => c as u32 + 0x1D5C1,

            // Bold Sans Serif mappings
            (Font::BoldSansSerif, 'A'..='Z') => c as u32 + 0x1D593,
            (Font::BoldSansSerif, 'a'..='z') => c as u32 + 0x1D58D,
            (Font::BoldSansSerif, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D3C5
            }
            (Font::BoldSansSerif, '\u{03F4}') => c as u32 + 0x1D373,
            (Font::BoldSansSerif, '\u{2207}') => c as u32 + 0x1B568,
            (Font::BoldSansSerif, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D3BF,
            (Font::BoldSansSerif, '\u{2202}') => c as u32 + 0x1B587,
            (Font::BoldSansSerif, '\u{03F5}') => c as u32 + 0x1D395,
            (Font::BoldSansSerif, '\u{03D1}') => c as u32 + 0x1D3BA,
            (Font::BoldSansSerif, '\u{03F0}') => c as u32 + 0x1D39C,
            (Font::BoldSansSerif, '\u{03D5}') => c as u32 + 0x1D3B8,
            (Font::BoldSansSerif, '\u{03F1}') => c as u32 + 0x1D39D,
            (Font::BoldSansSerif, '\u{03D6}') => c as u32 + 0x1D3B9,
            (Font::BoldSansSerif, '0'..='9') => c as u32 + 0x1D7BC,

            // Upright mappings (map to themselves) and unknown mappings
            (_, _) => c as u32,
        })
        .expect("character not in Unicode (developer error)")
    }
}

/// Takes a [`Parser`], or any `Iterator<Item = Result<Event<'_>, E>>` as input, and renders
/// the MathML into the given string.
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
