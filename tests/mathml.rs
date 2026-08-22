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

#[test]
fn radical_index_is_a_group() {
    // The index of `\sqrt[..]{..}` used to be emitted as bare events, so an
    // index of more than one token spilled out of the radical: `\sqrt[n+1]{x}`
    // rendered as the n-th root of x followed by a stray `+ 1`. The index must
    // be one group, exactly like the radicand and like the optional argument
    // of `\xrightarrow`.
    for (input, expected) in [
        (
            r"\sqrt[n+1]{x}",
            "<mroot><mrow><mi>x</mi></mrow><mrow><mi>n</mi><mo>+</mo><mn>1</mn></mrow></mroot>",
        ),
        (
            r"\sqrt[n]{x}",
            "<mroot><mrow><mi>x</mi></mrow><mrow><mi>n</mi></mrow></mroot>",
        ),
        // an index made of several pieces, one of them a group
        (
            r"\sqrt[a{bc}d]{x}",
            "<mroot><mrow><mi>x</mi></mrow><mrow><mi>a</mi><mrow><mi>b</mi><mi>c</mi></mrow>\
             <mi>d</mi></mrow></mroot>",
        ),
        // `mroot` takes exactly two children, so an empty index must still
        // produce one
        (
            r"\sqrt[]{x}",
            "<mroot><mrow><mi>x</mi></mrow><mrow></mrow></mroot>",
        ),
    ] {
        let out = render(input);
        assert!(
            out.contains(expected),
            "expected {expected} in output for {input}, got {out}"
        );
    }
}

#[test]
fn radical_index_does_not_escape_the_radical() {
    // The symptom: part of the index was rendered as a sibling of the radical
    // rather than inside it.
    for input in [
        r"\sqrt[n+1]{x}",
        r"\sqrt[a{bc}d]{x}",
        r"\sqrt[\frac{1}{2}]{x}",
        r"\sqrt[n^2]{x}",
    ] {
        let out = render(input);
        assert!(
            out.ends_with("</mroot></math>"),
            "index escaped the radical for {input}, got {out}"
        );
    }
}
