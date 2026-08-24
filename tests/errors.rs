use pulldown_latex::{push_mathml, Parser, Storage};

macro_rules! should_error {
    ($name:ident, $($input:literal),+ $(,)?) => {
        #[test]
        pub fn $name() {
            let inputs = &[$($input),*];
            let mut storage = pulldown_latex::Storage::new();
            for input in inputs {
                let parser = pulldown_latex::parser::Parser::new(input, &storage);
                let result = parser.collect::<Result<Vec<_>, _>>();
                assert!(result.is_err(), "expected error for input: {}", input);
                storage.reset();
            }
        }
    };
}

#[test]
fn error_rendering() {
    let storage = pulldown_latex::Storage::new();
    let mut out = String::new();
    let parser = pulldown_latex::parser::Parser::new(r"\errors \should \render", &storage);
    push_mathml(&mut out, parser, Default::default()).unwrap();
}

#[test]
fn error_rendering_unclosed_environment() {
    let storage = pulldown_latex::Storage::new();
    let mut out = String::new();
    let parser = pulldown_latex::parser::Parser::new("\\symit\\\0D", &storage);
    push_mathml(&mut out, parser, Default::default()).unwrap();
}

should_error! {
    double_scripts,
    r"a^b^c",
    r"a_b_c",
    r"a^b_c^d",
    r"a_b^c_d",
    r"a^b_c_d",
    r"a_b^c_d^e",
}

should_error! {
    invalid_escape_sequence,
    "5\\\u{6eb}%"
}

#[test]
fn comments() {
    let s = r#"{%"#;
    let storage = Storage::new();
    let parser = Parser::new(s, &storage);
    let mut mathml = String::new();
    let config = Default::default();

    push_mathml(&mut mathml, parser, config).unwrap();
}

should_error! {
    space_missing_groups,
    r"\Space 1em{2ex}{0pt}",
    r"\Space{1em} 2ex{0pt}",
    r"\Space{1em}{2ex} 0pt",
}

should_error! {
    braced_dimension_trailing_garbage,
    r"\kern{1em foo}",
    r"\hskip{1em garbage}",
}

should_error! {
    mkern_mskip_non_mu_units,
    r"\mkern{1em}",
    r"\mskip{1em plus 1em}",
}

should_error! {
    // Issue #56: `\char` followed by a multi-byte non-ASCII character sliced the
    // input at byte index 1, which is not a char boundary, and panicked (in
    // release too, so a DoS on untrusted input). It must return an error instead.
    char_number_non_ascii,
    "\\char\u{a7}",    // 2-byte: §
    "\\char\u{27e8}",  // 3-byte: ⟨
    "\\char\u{1f600}", // 4-byte: 😀
}

#[test]
fn macro_param_overflow() {
    // Issue #44: `then_some` eagerly evaluates `c as u8 - b'0'` causing overflow
    // when the character after '#' is not an ASCII digit.
    let storage = Storage::new();
    let parser = Parser::new("\\def\\]#\x1d{}", &storage);
    for _ in parser {}
}

#[test]
fn newline_in_unexpected_env() {
    // Newline event in an environment that doesn't support it should not panic.
    let storage = Storage::new();
    let parser = Parser::new(
        "\\begin{matrix} \\\x1d\\\\\\frac}1\\\\]\\\\\\\\\\]\\end{matrix}",
        &storage,
    );
    let mut out = String::new();
    push_mathml(&mut out, parser, Default::default()).unwrap();
}

#[test]
fn error_context_char_boundary() {
    // Error context slicing must respect char boundaries in multi-byte input
    // with macro expansions.
    let storage = Storage::new();
    let parser = Parser::new(
        "\\newcommand{\\foo}[1]{#1}\\foo{x}\\foo}[1]{#1}\\foo{x}^]_\u{8df7}:",
        &storage,
    );
    for _ in parser {}
}

#[test]
fn macro_recursion_limit() {
    // Recursive macro expansion should hit depth limit and error.
    let storage = Storage::new();
    let parser = Parser::new(
        "~zU\\newcommand{\\foo}[2]{#1]\\foo{x}\0#1}\\foo{}}",
        &storage,
    );
    let mut out = String::new();
    push_mathml(&mut out, parser, Default::default()).unwrap();
}

#[test]
fn suffix_bounds_check() {
    // content_with_suffix must check bounds before accessing the slice.
    let storage = Storage::new();
    let parser = Parser::new("\0\\def\\]a#1  {}f\\]ar3c%\\", &storage);
    for _ in parser {}
}

#[test]
fn escapes_html_special_chars() {
    for (input, expected) in [
        (r"\text{a < b > c & d}", "a &lt; b &gt; c &amp; d"),
        (r"\text{<>&}", "&lt;&gt;&amp;"),
        (r"\text{<<&&>>}", "&lt;&lt;&amp;&amp;&gt;&gt;"),
        (r"\text{plain}", "plain"),
    ] {
        let storage = Storage::new();
        let parser = Parser::new(input, &storage);
        let mut out = String::new();
        push_mathml(&mut out, parser, Default::default()).unwrap();
        assert!(
            out.contains(expected),
            "expected {expected:?} in output for {input:?}, got {out}"
        );
    }
}

// The `[n]` parameter count of `\newcommand` is optional, but what stands in
// its place must still be a parameter count.
should_error! {
    newcommand_malformed_parameter_count,
    // not a number at all
    r"\newcommand{\bad}[x]{y}",
    // a default for `#1` cannot stand in for the count
    r"\newcommand{\bad}[d]{y}",
    // out of range for a `u8`
    r"\newcommand{\bad}[300]{y}",
}

should_error! {
    newcommand_too_many_parameters,
    r"\newcommand{\bad}[10]{y}",
    r"\newcommand{\bad}[10][d]{y}",
}

should_error! {
    newcommand_without_a_count_takes_no_parameters,
    // no count means zero parameters, so `#1` in the replacement has nothing
    // to refer to
    r"\newcommand{\bad}{#1}",
}

should_error! {
    newcommand_still_needs_a_replacement_text,
    // dropping the count must not make the replacement optional too
    r"\newcommand{\bad}",
    r"\newcommand{\bad}[1]",
}
