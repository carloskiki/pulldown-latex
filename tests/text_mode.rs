//! Tests for parsing commands inside `\text{...}` and friends (issue #29).
//!
//! These tests exercise the text-mode parsing path: nested commands, font
//! changes, escaped specials, embedded inline math, and renderer behavior.

use pulldown_latex::{
    event::{Content, Event, Font, Grouping, ScriptType, StateChange},
    push_mathml, Parser, Storage,
};

/// Parse the input and assert there are no parser errors.
///
/// Leaks a `Storage` so the returned events can borrow from it for the lifetime
/// of the test — fine for unit tests since each test is its own process scope.
fn parse(input: &str) -> Vec<Event<'_>> {
    let storage: &'static Storage = Box::leak(Box::new(Storage::new()));
    let parser = Parser::new(input, storage);
    parser
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
    let events = parse(r"\text{Hello, world!}");
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
    let events = parse(r"\text{\LaTeX is great}");
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
    let events = parse(r"\text{\# \$ \& \% \_ \{ \}}");
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
    let events = parse(r"\text{a\textbackslash b}");
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
    let events = parse(r"\text{a \textbf{b} c}");
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
    let events = parse(r"\text{\emph{stress}}");
    assert!(events.contains(&Event::StateChange(StateChange::Font(Some(Font::Italic)))));
}

#[test]
fn textit_at_top_level_enters_text_mode() {
    // `\textit{italic}` directly in math mode should still produce a Text
    // grouping with an italic font state.
    let events = parse(r"\textit{italic}");
    assert_eq!(events[0], Event::Begin(Grouping::Text));
    assert_eq!(
        events[1],
        Event::StateChange(StateChange::Font(Some(Font::Italic)))
    );
    assert!(events.contains(&Event::Content(Content::Text("italic"))));
}

#[test]
fn inline_math_inside_text_switches_back_to_math() {
    let events = parse(r"\text{value is $x^2$ here}");
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
    let events = parse(r"\text{a~b}");
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
    let events = parse(r"\text{\dag and \ddag}");
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
    let events = parse(r"\text{}");
    assert_eq!(events, vec![Event::Begin(Grouping::Text), Event::End]);
}

#[test]
fn mbox_and_hbox_aliases_work() {
    for src in [r"\mbox{x}", r"\hbox{x}"] {
        let events = parse(src);
        assert_eq!(events[0], Event::Begin(Grouping::Text), "src = {src}");
    }
}

#[test]
fn nested_text_inside_text() {
    // A nested `{...}` group inside text should remain in text mode.
    let events = parse(r"\text{a {b} c}");
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::Content(Content::Text(s)) => Some(*s),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.contains('b')));
}
