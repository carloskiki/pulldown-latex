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

    match push_mathml(&mut mathml, parser, config) {
        Ok(()) => println!("{}", mathml),
        Err(e) => eprintln!("Error while rendering: {}", e),
    }
}

// KaTeX/MathJax compatibility: braced dimension arguments for spacing primitives.

#[test]
fn braced_dimension_arguments_accepted() {
    let storage = Storage::new();
    let inputs = [
        r"\hskip{1em}",
        r"\hskip 1em",
        r"\kern{1em}",
        r"\kern 1em",
        r"\mkern{3mu}",
        r"\mkern 3mu",
        r"\mskip{3mu plus 1mu}",
        r"\mskip 3mu plus 1mu",
        r"\Space{1em}{2ex}{0pt}",
    ];
    for input in inputs {
        let parser = Parser::new(input, &storage);
        let result: Result<Vec<_>, _> = parser.collect();
        assert!(
            result.is_ok(),
            "expected braced/bare spacing input to parse: {:?} (err: {:?})",
            input,
            result.err()
        );
    }
}

// Regression tests from fuzzing

#[test]
fn fuzz_macro_param_overflow() {
    // Issue #44: `then_some` eagerly evaluates `c as u8 - b'0'` causing overflow
    // when the character after '#' is not an ASCII digit.
    let storage = Storage::new();
    let parser = Parser::new("\\def\\]#\x1d{}", &storage);
    for _ in parser {}
}

#[test]
fn fuzz_newline_in_unexpected_env() {
    // Newline event in an environment that doesn't support it should not panic.
    let storage = Storage::new();
    let parser = Parser::new(
        "\\begin{matrix} \\\x1d\\\\\\frac}1\\\\]\\\\\\\\\\]\\end{matrix}",
        &storage,
    );
    let mut out = String::new();
    let _ = push_mathml(&mut out, parser, Default::default());
}

#[test]
fn fuzz_error_context_char_boundary() {
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
fn fuzz_macro_recursion_limit() {
    // Recursive macro expansion should hit depth limit and error.
    let storage = Storage::new();
    let parser = Parser::new(
        "~zU\\newcommand{\\foo}[2]{#1]\\foo{x}\0#1}\\foo{}}",
        &storage,
    );
    let mut out = String::new();
    let _ = push_mathml(&mut out, parser, Default::default());
}

#[test]
fn fuzz_suffix_bounds_check() {
    // content_with_suffix must check bounds before accessing the slice.
    let storage = Storage::new();
    let parser = Parser::new("\0\\def\\]a#1  {}f\\]ar3c%\\", &storage);
    for _ in parser {}
}
