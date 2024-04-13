#[macro_export]
macro_rules! round_trip {
    (should_panic, $name:ident, $($input:literal),*) => {
        #[test]
        fn $name() {
            let inputs = &[$($input),*];
            for input in inputs {
                let mut parser = pulldown_latex::parser::Parser::new(input);
                assert!(parser.all(|event| event.is_err()))
            }
        }
    };
    ($name:ident, $($input:literal),*) => {
        #[test]
        fn $name() {
            let inputs = &[$($input),*];
            $crate::common::test(inputs);
        }
    };
}

use pulldown_latex::{mathml::write_mathml, parser::Parser};

pub fn test(inputs: &[&str]) {
    for input in inputs {
        let _ = show_errors(Parser::new(input)).unwrap();
        let parser = Parser::new(input);
        write_mathml(std::io::sink(), parser, Default::default()).unwrap();
    }
}

pub fn show_errors(parser: Parser) -> Result<(), usize> {
    let mut error_count = 0;

    parser.for_each(|event| {
        if let Err(e) = event {
            eprintln!("{}", e);
            error_count += 1;
        }
    });

    if error_count > 0 {
        Err(error_count)
    } else {
        Ok(())
    }
}
