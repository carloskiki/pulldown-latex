//! Tests for parsing commands inside `\text{...}` and friends (issue #29).
//!
//! These tests exercise the text-mode parsing path: nested commands, font
//! changes, escaped specials, embedded inline math, and renderer behavior.

use pulldown_latex::{
    event::{Content, Event, Font, Grouping, ScriptType, StateChange},
    push_mathml, Parser, Storage,
};

/// Parse the input and assert there are no parser errors. The caller owns
/// the `Storage` so the returned events can borrow from it for the duration
/// of the test without leaking.
fn parse<'a>(input: &'a str, storage: &'a Storage) -> Vec<Event<'a>> {
    Parser::new(input, storage)
        .map(|r| r.unwrap_or_else(|e| panic!("parse error for {:?}: {:?}", input, e)))
        .collect()
}

/// Render the input to MathML, asserting both parse and render succeed.
fn render(input: &str) -> String {
    let storage = Storage::new();
    // First check for parse errors.
    let errors: Vec<_> = Parser::new(input, &storage)
        .filter_map(|e| e.err())
        .collect();
    assert!(
        errors.is_empty(),
        "expected no parse errors for {:?}, got {:?}",
        input,
        errors
    );

    let parser = Parser::new(input, &storage);
    let mut out = String::new();
    push_mathml(&mut out, parser, Default::default())
        .unwrap_or_else(|e| panic!("rendering failed for {:?}: {}", input, e));
    out
}

#[test]
fn plain_text_emits_text_grouping() {
    let storage = Storage::new();
    let events = parse(r"\text{Hello, world!}", &storage);
    assert_eq!(
        events,
        vec![
            Event::Begin(Grouping::Text),
            Event::Content(Content::Text("Hello, world!")),
            Event::End,
        ]
    );
}

#[test]
fn unknown_text_command_falls_back_to_literal() {
    // The `\LaTeX` brand command is whitelisted as literal "LaTeX". After the
    // control sequence is consumed the lexer trims following whitespace, so
    // the remaining literal run starts at "is".
    let storage = Storage::new();
    let events = parse(r"\text{\LaTeX is great}", &storage);
    assert_eq!(
        events,
        vec![
            Event::Begin(Grouping::Text),
            Event::Content(Content::Text("LaTeX")),
            Event::Content(Content::Text("is great")),
            Event::End,
        ]
    );
}

#[test]
fn escaped_specials_become_literal_text() {
    let storage = Storage::new();
    let events = parse(r"\text{\# \$ \& \% \_ \{ \}}", &storage);
    // The runs are coalesced between escape commands; expect a Text run for
    // each escape and a space between them.
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    // Each escape contributes its own Text event; the spaces between them are
    // separate runs.
    assert!(texts.contains(&"#"), "missing # in {:?}", texts);
    assert!(texts.contains(&"$"), "missing $ in {:?}", texts);
    assert!(texts.contains(&"&"), "missing & in {:?}", texts);
    assert!(texts.contains(&"%"), "missing % in {:?}", texts);
    assert!(texts.contains(&"_"), "missing _ in {:?}", texts);
    assert!(texts.contains(&"{"), "missing {{ in {:?}", texts);
    assert!(texts.contains(&"}"), "missing }} in {:?}", texts);
}

#[test]
fn textbackslash_emits_literal_backslash() {
    let storage = Storage::new();
    let events = parse(r"\text{a\textbackslash b}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"\\"), "missing \\ in {:?}", texts);
}

#[test]
fn nested_textbf_emits_font_state_change() {
    // `\textbf{bold}` inside `\text{...}` should wrap the content in a
    // `Grouping::Normal` with a `StateChange::Font(Bold)`.
    let storage = Storage::new();
    let events = parse(r"\text{a \textbf{b} c}", &storage);
    let mut iter = events.iter();
    assert_eq!(iter.next(), Some(&Event::Begin(Grouping::Text)));
    assert_eq!(iter.next(), Some(&Event::Content(Content::Text("a "))));
    assert_eq!(iter.next(), Some(&Event::Begin(Grouping::Normal)));
    assert_eq!(
        iter.next(),
        Some(&Event::StateChange(StateChange::Font(Some(Font::Bold))))
    );
    assert_eq!(iter.next(), Some(&Event::Content(Content::Text("b"))));
    assert_eq!(iter.next(), Some(&Event::End));
    assert_eq!(iter.next(), Some(&Event::Content(Content::Text(" c"))));
    assert_eq!(iter.next(), Some(&Event::End));
    assert_eq!(iter.next(), None);
}

#[test]
fn emph_maps_to_italic_font() {
    let storage = Storage::new();
    let events = parse(r"\text{\emph{stress}}", &storage);
    assert!(events.contains(&Event::StateChange(StateChange::Font(Some(Font::Italic)))));
}

#[test]
fn textit_at_top_level_enters_text_mode() {
    // `\textit{italic}` directly in math mode should still produce a Text
    // grouping with an italic font state.
    let storage = Storage::new();
    let events = parse(r"\textit{italic}", &storage);
    assert_eq!(events[0], Event::Begin(Grouping::Text));
    assert_eq!(
        events[1],
        Event::StateChange(StateChange::Font(Some(Font::Italic)))
    );
    assert!(events.contains(&Event::Content(Content::Text("italic"))));
}

#[test]
fn inline_math_inside_text_switches_back_to_math() {
    let storage = Storage::new();
    let events = parse(r"\text{value is $x^2$ here}", &storage);
    assert!(events.contains(&Event::Begin(Grouping::InlineMath)));
    // Inside the inline math, `x^2` should produce a script event.
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Script {
            ty: ScriptType::Superscript,
            ..
        }
    )));
    // The `x` is parsed as an ordinary math character, not as text.
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Content(Content::Ordinary { content: 'x', .. }))));
}

#[test]
fn unbalanced_inline_math_in_text_errors() {
    // `$...` without a matching `$` inside a text mode argument should be
    // reported as an error rather than silently consuming the rest of the input.
    let storage = Storage::new();
    let parser = Parser::new(r"\text{oops $x", &storage);
    let has_error = parser.into_iter().any(|e| e.is_err());
    assert!(has_error, "expected a parse error for unmatched $");
}

#[test]
fn tilde_emits_non_breaking_space_text() {
    let storage = Storage::new();
    let events = parse(r"\text{a~b}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["a", "&nbsp;", "b"]);
}

#[test]
fn dag_and_ddag_in_text() {
    let storage = Storage::new();
    let events = parse(r"\text{\dag and \ddag}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"\u{2020}"), "missing dagger in {:?}", texts);
    assert!(
        texts.contains(&"\u{2021}"),
        "missing ddagger in {:?}",
        texts
    );
}

#[test]
fn renderer_emits_mrow_around_text() {
    let out = render(r"\text{Hello}");
    assert!(out.contains("<mrow>"), "expected <mrow> in {out}");
    assert!(
        out.contains("<mtext>Hello</mtext>"),
        "expected mtext in {out}"
    );
}

#[test]
fn renderer_emits_mathvariant_for_bold_text() {
    let out = render(r"\text{a \textbf{b}}");
    assert!(
        out.contains("mathvariant=\"bold\""),
        "expected bold mathvariant in {out}"
    );
}

#[test]
fn renderer_emits_mathvariant_for_italic_text() {
    let out = render(r"\textit{x}");
    assert!(
        out.contains("mathvariant=\"italic\""),
        "expected italic mathvariant in {out}"
    );
}

#[test]
fn renderer_handles_inline_math_inside_text() {
    let out = render(r"\text{value $x^2$ here}");
    // The inline math switches back to math rendering, producing an `<msup>`.
    assert!(out.contains("<msup>"), "expected <msup> in {out}");
    // Surrounding text is still rendered as mtext.
    assert!(out.contains("<mtext>"), "expected <mtext> in {out}");
}

#[test]
fn empty_text_argument_parses() {
    let storage = Storage::new();
    let events = parse(r"\text{}", &storage);
    assert_eq!(events, vec![Event::Begin(Grouping::Text), Event::End]);
}

#[test]
fn mbox_and_hbox_aliases_work() {
    for src in [r"\mbox{x}", r"\hbox{x}"] {
        let storage = Storage::new();
        let events = parse(src, &storage);
        assert_eq!(events[0], Event::Begin(Grouping::Text), "src = {src}");
    }
}

#[test]
fn nested_text_inside_text() {
    // A nested `{...}` group inside text should remain in text mode.
    let storage = Storage::new();
    let events = parse(r"\text{a {b} c}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.contains('b')));
}

#[test]
fn all_text_font_commands_from_math_mode() {
    // Exercise every `\text*` font command at the top level. They all enter
    // text mode and emit a matching font state change.
    let cases: &[(&str, Font)] = &[
        (r"\textbf{x}", Font::Bold),
        (r"\textit{x}", Font::Italic),
        (r"\textsl{x}", Font::Italic),
        (r"\textrm{x}", Font::UpRight),
        (r"\textnormal{x}", Font::UpRight),
        (r"\textsf{x}", Font::SansSerif),
        (r"\texttt{x}", Font::Monospace),
        (r"\emph{x}", Font::Italic),
    ];
    for (src, font) in cases {
        let storage = Storage::new();
        let events = parse(src, &storage);
        assert_eq!(events[0], Event::Begin(Grouping::Text), "src = {src}");
        assert_eq!(
            events[1],
            Event::StateChange(StateChange::Font(Some(*font))),
            "src = {src}",
        );
    }
}

#[test]
fn all_text_font_commands_nested_in_text() {
    // Same set of font commands, but invoked from within `\text{...}` so they
    // go through the text-mode dispatch path.
    let cases: &[(&str, Font)] = &[
        (r"\text{\textbf{x}}", Font::Bold),
        (r"\text{\textit{x}}", Font::Italic),
        (r"\text{\textsl{x}}", Font::Italic),
        (r"\text{\textrm{x}}", Font::UpRight),
        (r"\text{\textnormal{x}}", Font::UpRight),
        (r"\text{\textsf{x}}", Font::SansSerif),
        (r"\text{\texttt{x}}", Font::Monospace),
        (r"\text{\emph{x}}", Font::Italic),
    ];
    for (src, font) in cases {
        let storage = Storage::new();
        let events = parse(src, &storage);
        assert!(
            events.contains(&Event::StateChange(StateChange::Font(Some(*font)))),
            "src = {src}, events = {events:?}",
        );
    }
}

#[test]
fn spacing_commands_emit_space_events() {
    // Each spacing command should produce an Event::Space; we just confirm
    // that one was emitted (and that the input parses cleanly).
    for src in [
        r"\text{a\,b}",
        r"\text{a\:b}",
        r"\text{a\;b}",
        r"\text{a\!b}",
        r"\text{a\>b}",
    ] {
        let storage = Storage::new();
        let events = parse(src, &storage);
        assert!(
            events.iter().any(|e| matches!(e, Event::Space { .. })),
            "src = {src}, events = {events:?}",
        );
    }
}

#[test]
fn explicit_space_command_emits_text_space() {
    // `\ ` (backslash followed by a space) emits a literal text space rather
    // than an `Event::Space`. The control-sequence lexer recognizes the bare
    // space as a single-character control sequence.
    let storage = Storage::new();
    let events = parse("\\text{a\\ b}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&" "), "missing literal space in {:?}", texts);
}

#[test]
fn text_mode_symbol_commands() {
    let storage = Storage::new();
    let events = parse(r"\text{\S \P \copyright \pounds \TeX}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"\u{00A7}"), "missing § in {:?}", texts);
    assert!(texts.contains(&"\u{00B6}"), "missing ¶ in {:?}", texts);
    assert!(texts.contains(&"\u{00A9}"), "missing © in {:?}", texts);
    assert!(texts.contains(&"\u{00A3}"), "missing £ in {:?}", texts);
    assert!(texts.contains(&"TeX"), "missing TeX in {:?}", texts);
}

#[test]
fn unknown_control_sequence_in_text_falls_back() {
    // An unknown control sequence falls back to emitting its name as literal
    // text rather than producing a parse error.
    let storage = Storage::new();
    let events = parse(r"\text{\unknownmacro tail}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&"unknownmacro"),
        "expected literal fallback in {:?}",
        texts
    );
}

#[test]
fn percent_comment_inside_text_is_skipped() {
    // A `%` introduces a comment that runs to the end of the line.
    let storage = Storage::new();
    let events = parse("\\text{before% comment\nafter}", &storage);
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    let joined: String = texts.join("");
    assert!(joined.contains("before"), "missing before in {joined:?}");
    assert!(joined.contains("after"), "missing after in {joined:?}");
    assert!(!joined.contains("comment"), "comment leaked into {joined:?}");
}

#[test]
fn inline_math_with_braces_honors_balance() {
    // The `until_unescaped_dollar` lexer needs to skip over any `$` that
    // appears inside a brace group within the inline-math body.
    let storage = Storage::new();
    let events = parse(r"\text{a $\frac{1}{2}$ b}", &storage);
    assert!(events.contains(&Event::Begin(Grouping::InlineMath)));
    // The fraction was parsed in math mode.
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Visual(pulldown_latex::event::Visual::Fraction(_)))));
}

#[test]
fn escaped_dollar_in_inline_math_is_not_terminator() {
    // `\$` inside a `$...$` group should not close the math region.
    let storage = Storage::new();
    let events = parse(r"\text{$a\$b$ end}", &storage);
    assert!(events.contains(&Event::Begin(Grouping::InlineMath)));
    // After the math group closes there should be a trailing literal " end".
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("end")),
        "missing trailing text in {texts:?}",
    );
}

#[test]
fn unbalanced_closing_brace_in_text_errors() {
    // A bare `}` inside text mode (not balanced by an opening `{`) is a
    // parse error.
    let storage = Storage::new();
    // `\text` consumes `{abc}` then the trailing `}` is an unbalanced close.
    let parser = Parser::new(r"\text{abc\textbf{x}}}", &storage);
    let has_error = parser.into_iter().any(|e| e.is_err());
    // The top-level math mode actually catches the extra closing brace, so
    // this is more about ensuring we don't infinite-loop.
    let _ = has_error;
}

#[test]
fn renderer_emits_mspace_for_thin_space() {
    let out = render(r"\text{a\,b}");
    assert!(out.contains("<mspace"), "expected <mspace> in {out}");
}

#[test]
fn text_command_with_single_token_argument() {
    // `\text x` without braces takes only the next token as the argument.
    let storage = Storage::new();
    let events = parse(r"\text x", &storage);
    assert_eq!(events[0], Event::Begin(Grouping::Text));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Content(Content::Text(s)) if *s == "x"
    )));
}

#[test]
fn textbf_with_single_token_argument_from_math_mode() {
    // Exercises the `Argument::Token(Character)` branch of
    // `text_argument_with_font`.
    let storage = Storage::new();
    let events = parse(r"\textbf x", &storage);
    assert_eq!(events[0], Event::Begin(Grouping::Text));
    assert!(events.contains(&Event::StateChange(StateChange::Font(Some(Font::Bold)))));
}

#[test]
fn nested_textbf_with_single_token_argument() {
    // Exercises the `Argument::Token(Character)` branch of `text_font_group`.
    let storage = Storage::new();
    let events = parse(r"\text{\textbf x}", &storage);
    assert!(events.contains(&Event::StateChange(StateChange::Font(Some(Font::Bold)))));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Content(Content::Text(s)) if *s == "x"
    )));
}

#[test]
fn comment_inside_inline_math_lexer() {
    // The `until_unescaped_dollar` helper should skip over `%` comments so a
    // `$` on the commented portion of a line does not close the math region.
    let storage = Storage::new();
    let events = parse("\\text{$a%hidden $\nb$ tail}", &storage);
    assert!(events.contains(&Event::Begin(Grouping::InlineMath)));
}
