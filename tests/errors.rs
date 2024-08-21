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

should_error!{
    double_scripts,
    r"a^b^c",
    r"a_b_c",
    r"a^b_c^d",
    r"a_b^c_d",
    r"a^b_c_d",
    r"a_b^c_d^e",
}
