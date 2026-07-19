//! Tests for amsmath atom-type commands: `\mathord`, `\mathrel`, `\mathbin`,
//! `\mathop`, `\mathopen`, `\mathclose`, `\mathpunct`, `\mathinner`.

use pulldown_latex::{push_mathml, Parser, Storage};

/// Parse the input and render to MathML, asserting both succeed.
fn render(input: &str) -> String {
    let storage = Storage::new();
    let parser = Parser::new(input, &storage);
    // First, ensure the parser produces no errors.
    let errors: Vec<_> = Parser::new(input, &storage)
        .filter_map(|e| e.err())
        .collect();
    assert!(
        errors.is_empty(),
        "expected no parse errors for {:?}, got {:?}",
        input,
        errors
    );

    let mut out = String::new();
    push_mathml(&mut out, parser, Default::default())
        .unwrap_or_else(|e| panic!("rendering failed for {:?}: {}", input, e));
    out
}

#[test]
fn mathord_single_char() {
    let out = render(r"\mathord{+}");
    // A single `+` wrapped in \mathord should render as <mi>+</mi>
    // (Ordinary class), not as a <mo> (BinaryOp class).
    assert!(
        out.contains("<mi>+</mi>") || out.contains("<mi mathvariant=\"normal\">+</mi>"),
        "expected <mi>+</mi> in {out}"
    );
}

#[test]
fn mathrel_single_char() {
    let out = render(r"a \mathrel{R} b");
    // The `R` should be wrapped in <mo> with relation-style spacing.
    assert!(out.contains("<mo>R</mo>"), "expected <mo>R</mo> in {out}");
}

#[test]
fn mathbin_single_char() {
    let out = render(r"a \mathbin{*} b");
    // `*` is a binary operator by default; wrapping in \mathbin should still
    // give us a binary-class atom.
    assert!(
        out.contains("<mo>*</mo>") || out.contains("<mo>*</mo>"),
        "expected <mo>*</mo> in {out}"
    );
}

#[test]
fn mathop_single_char() {
    let out = render(r"\mathop{X}_{i=1}");
    // \mathop on a single char should produce an operator with movable scripts.
    assert!(out.contains("X"), "expected operator X in {out}");
    // The script should be a subscript via munder/munderover when used in
    // display mode, but at least the structure should render without error.
}

#[test]
fn mathop_multi_char_renders_as_function() {
    let out = render(r"\mathop{foo}(x)");
    // Multi-letter \mathop should render like \operatorname{foo} → <mi>foo</mi>.
    assert!(out.contains("foo"), "expected 'foo' in {out}");
}

#[test]
fn mathopen_mathclose() {
    let out = render(r"\mathopen{[}x\mathclose{]}");
    // Both delimiters should appear as <mo> with open/close semantics.
    assert!(out.contains("["), "expected '[' in {out}");
    assert!(out.contains("]"), "expected ']' in {out}");
}

#[test]
fn mathpunct_single_char() {
    let out = render(r"a\mathpunct{;}b");
    assert!(out.contains("<mo>;</mo>"), "expected <mo>;</mo> in {out}");
}

#[test]
fn mathinner_renders_without_error() {
    // \mathinner with a group should wrap the contents in an mrow-equivalent
    // and not error.
    let out = render(r"\mathinner{abc}");
    assert!(out.contains("a"), "expected 'a' in {out}");
    assert!(out.contains("b"), "expected 'b' in {out}");
    assert!(out.contains("c"), "expected 'c' in {out}");
}

#[test]
fn all_atom_commands_parse_in_one_expression() {
    // Exercise every command in a single string to make sure dispatch and
    // surrounding-spacing logic don't choke on combinations.
    let out = render(
        r"\mathord{a} \mathop{f} \mathbin{+} \mathrel{=} \mathopen{(} x \mathclose{)} \mathpunct{,} \mathinner{y}",
    );
    assert!(!out.is_empty());
}
