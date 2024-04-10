#[macro_export]
macro_rules! round_trip {
    ($name:ident, $input:literal) => {
        #[test]
        fn $name() {
            use pulldown_latex::{mathml::write_mathml, parser::Parser};
            let _ = crate::common::show_errors(Parser::new($input)).unwrap();
            let parser = Parser::new($input);
            let events = parser.collect::<Result<Vec<_>, _>>().unwrap();
            println!("{:#?}", events);
            let parser = Parser::new($input);
            write_mathml(std::io::sink(), parser, Default::default()).unwrap();
        }
    };
}
pub use round_trip;

use pulldown_latex::parser::Parser;

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
