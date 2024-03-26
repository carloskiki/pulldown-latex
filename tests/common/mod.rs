#[macro_export]
macro_rules! round_trip {
    ($name:ident, $input:literal) => {
        #[test]
        fn $name() {
            use pulldown_latex::{parser::Parser, mathml::write_mathml};
            let _ = Parser::new($input).collect::<Result<Vec<_>, _>>().unwrap();
            let parser = Parser::new($input);
            write_mathml(std::io::sink(), parser, Default::default()).unwrap();
        }
    };
}
pub use round_trip;
