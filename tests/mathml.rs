use pulldown_latex::{
    config::{MathStyle, RenderConfig},
    push_mathml, Parser, Storage,
};

fn render(input: &str) -> String {
    render_with(input, RenderConfig::default())
}

fn render_with(input: &str, config: RenderConfig) -> String {
    let storage = Storage::new();
    let parser = Parser::new(input, &storage);
    let mut out = String::new();
    push_mathml(&mut out, parser, config).unwrap();
    out
}

fn assert_chars(out: &str, expected: impl IntoIterator<Item = char>, label: &str) {
    for ch in expected {
        assert!(
            out.contains(ch),
            "expected {label} U+{:04X} in output: {out}",
            ch as u32,
        );
    }
}

fn assert_no_chars(out: &str, forbidden: impl IntoIterator<Item = char>, label: &str) {
    for ch in forbidden {
        assert!(
            !out.contains(ch),
            "{label} U+{:04X} should not appear in output: {out}",
            ch as u32,
        );
    }
}

#[test]
fn boldsymbol_digits_render_as_bold_upright() {
    // Digits have no italic Unicode math variant, so `\boldsymbol{0..9}` must
    // resolve to MATHEMATICAL BOLD DIGIT (U+1D7CE..U+1D7D7)
    let out = render(r"\boldsymbol{0123456789}");
    assert_chars(
        &out,
        ('0'..='9').map(|c| char::from_u32(c as u32 + 0x1D79E).unwrap()),
        "bold digit",
    );
    assert_no_chars(&out, '0'..='9', "plain ASCII digit");
}

#[test]
fn boldsymbol_latin_lowercase_is_bold_italic() {
    let out = render(r"\boldsymbol{abcdefghijklmnopqrstuvwxyz}");
    assert_chars(
        &out,
        ('a'..='z').map(|c| char::from_u32(c as u32 + 0x1D421).unwrap()),
        "bold italic lowercase",
    );
}

#[test]
fn boldsymbol_latin_uppercase_is_bold_italic() {
    let out = render(r"\boldsymbol{ABCDEFGHIJKLMNOPQRSTUVWXYZ}");
    assert_chars(
        &out,
        ('A'..='Z').map(|c| char::from_u32(c as u32 + 0x1D427).unwrap()),
        "bold italic uppercase",
    );
}

#[test]
fn boldsymbol_capital_greek_is_bold_upright() {
    // Capital Greek is upright-by-default in TeX style → BOLD CAPITAL (U+1D6A8..).
    let out = render(r"\boldsymbol{\Alpha \Gamma \Delta \Omega}");
    let expected: [char; 4] = ['\u{0391}', '\u{0393}', '\u{0394}', '\u{03A9}']
        .map(|c| char::from_u32(c as u32 + 0x1D317).unwrap());
    assert_chars(&out, expected, "bold upright capital Greek");
    assert_no_chars(
        &out,
        expected
            .iter()
            .map(|c| char::from_u32(*c as u32 - 0x1D317).unwrap()),
        "plain capital Greek",
    );
}

#[test]
fn boldsymbol_lowercase_greek_is_bold_italic() {
    // Lowercase Greek is italic-by-default → BOLD ITALIC SMALL (U+1D736..).
    let out = render(r"\boldsymbol{\alpha \beta \gamma \omega}");
    let expected: [char; 4] = ['\u{03B1}', '\u{03B2}', '\u{03B3}', '\u{03C9}']
        .map(|c| char::from_u32(c as u32 + 0x1D385).unwrap());
    assert_chars(&out, expected, "bold italic lowercase Greek");
}

#[test]
fn boldsymbol_nabla_and_partial_are_bold_italic() {
    // \nabla and \partial are italic-by-default symbols.
    let out = render(r"\boldsymbol{\nabla \partial}");
    let nabla_bold_italic = char::from_u32(0x2207 + 0x1B52E).unwrap();
    let partial_bold_italic = char::from_u32(0x2202 + 0x1B54D).unwrap();
    assert_chars(
        &out,
        [nabla_bold_italic, partial_bold_italic],
        "bold italic operator",
    );
}

#[test]
fn boldsymbol_with_upright_style_forces_bold_upright() {
    let out = render_with(
        r"\boldsymbol{abc}",
        RenderConfig {
            math_style: MathStyle::Upright,
            ..Default::default()
        },
    );
    assert_chars(
        &out,
        ('a'..='c').map(|c| char::from_u32(c as u32 + 0x1D3B9).unwrap()),
        "bold upright lowercase",
    );
}

/// Assert that `input` parsed without error and rendered `expected`.
///
/// `push_mathml` turns a parse error into an inline `<merror>` and carries on,
/// so a failed definition still leaves its replacement text in the output as
/// ordinary content. Checking for the content alone would not see the failure.
fn assert_renders(input: &str, expected: &str) {
    let out = render(input);
    assert!(
        !out.contains("<merror"),
        "expected {input} to parse, got {out}"
    );
    assert!(
        out.contains(expected),
        "expected {expected} in output for {input}, got {out}"
    );
}

#[test]
fn newcommand_parameter_count_is_optional() {
    // `\newcommand{\R}{\mathbb{R}}` is valid LaTeX: the `[n]` parameter count
    // may be left out, and the macro then takes no arguments. It used to be
    // mandatory here, so the definition failed with "expected an argument".
    for (input, expected) in [
        // no count given
        (r"\newcommand{\R}{\mathbb{R}}\R", "<mi>\u{211D}</mi>"),
        // an explicit zero means the same thing
        (r"\newcommand{\R}[0]{\mathbb{R}}\R", "<mi>\u{211D}</mi>"),
        // `\renewcommand` and `\providecommand` share the same code path
        (
            r"\newcommand{\R}{\mathbb{R}}\renewcommand{\R}{\mathbb{C}}\R",
            "<mi>\u{2102}</mi>",
        ),
        (r"\providecommand{\Q}{\mathbb{Q}}\Q", "<mi>\u{211A}</mi>"),
        // a count that is given is still honoured
        (r"\newcommand{\f}[1]{f(#1)}\f{x}", "<mi>x</mi>"),
        // as is a default for the first argument
        (r"\newcommand{\g}[1][d]{g(#1)}\g", "<mi>d</mi>"),
        (r"\newcommand{\g}[1][d]{g(#1)}\g[z]", "<mi>z</mi>"),
    ] {
        assert_renders(input, expected);
    }
}

#[test]
fn newcommand_without_a_count_does_not_swallow_a_later_bracket() {
    // Two optional arguments are read in a row: the parameter count and the
    // default for `#1`. With no count present the parser sits on the opening
    // brace of the replacement text, so the second read comes up empty as
    // well; a `[..]` that follows the definition stays in the document.
    assert_renders(
        r"\newcommand{\R}{\mathbb{R}}[z]\R",
        "<mo symmetric=\"false\" stretchy=\"false\">[</mo><mi>z</mi>",
    );
    assert_renders(r"\newcommand{\R}{\mathbb{R}}[z]\R", "<mi>\u{211D}</mi>");
}
