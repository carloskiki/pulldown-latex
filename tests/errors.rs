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
