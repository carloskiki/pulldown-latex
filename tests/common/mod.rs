use std::{io::Write, sync::Mutex};

use heck::ToTitleCase;
use inventory::collect;
use libtest_mimic::{Arguments, Failed, Trial};
use pulldown_latex::{mathml::push_mathml, parser::Parser};

static RENDERED: Mutex<Vec<(&str, Vec<(&str, String)>)>> = Mutex::new(Vec::new());
const OUTPUT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/test-output/");

pub struct TestCase {
    pub name: &'static str,
    pub test: fn() -> Result<(), Failed>,
}
collect!(TestCase);

pub fn test() {
    let args = Arguments::from_args();
    let tests = inventory::iter::<TestCase>
        .into_iter()
        .map(|TestCase { name, test }| Trial::test(*name, test))
        .collect::<Vec<_>>();
    let concl = libtest_mimic::run(&args, tests);
    output("wikipedia.html").unwrap();

    concl.exit();
}

pub fn round_trip(fn_name: &'static str, inputs: &[&'static str]) -> Result<(), Failed> {
    let rendered: Vec<_> = inputs.iter().map(|input| -> Result<_, Failed> {
        let _ = show_errors(Parser::new(input))?;
        let parser = Parser::new(input);
        let mut output = String::new();
        push_mathml(&mut output, parser, Default::default())?;
        Ok((*input, output))
    }).collect::<Result<_, Failed>>()?;

    RENDERED.lock().unwrap().push((fn_name, rendered));

    Ok(())
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

pub fn output(file_name: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(format!("{}{}", OUTPUT_DIR, file_name))?;

    file.write_fmt(format_args!(
        r#"<!DOCTYPE html>
<html>
<head>
<title>{title}</title>
<link rel="stylesheet" type="text/css" href="../LatinModern/mathfonts.css">
</head>
<body>
<table style="max-width: 60vw; margin: auto;">"#,
        title = file_name.split_once('.').unwrap().0.to_title_case()
    ))?;

    let mut rendered = RENDERED.lock().unwrap();
    rendered.sort();
    rendered
        .iter()
        .try_for_each(|(table_name, rows)| -> std::io::Result<()> {
            file.write_fmt(format_args!(
                r#"<tr><th colspan="2">{table_name}</th></tr>"#,
                table_name = table_name.to_title_case()
            ))?;

            rows.iter().try_for_each(|(input, output)| {
                file.write_fmt(format_args!(
                    r#"<tr><td>{input}</td><td>{output}</td></tr>"#,
                    input = input,
                    output = output
                ))
            })
        })?;

    file.write_fmt(format_args!(
        r#"</table>
</body>
</html>"#
    ))
}

#[macro_export]
macro_rules! round_trip {
    (should_panic, $name:ident, $($input:literal),*) => {
        pub fn $name() -> Result<(), libtest_mimic::Failed> {
            let inputs = &[$($input),*];
            for input in inputs {
                let mut parser = pulldown_latex::parser::Parser::new(input);
                if !parser.all(|event| event.is_err()) {
                    return Err(libtest_mimic::Failed::without_message());
                };
            }
            Ok(())
        }

        inventory::submit! {
            $crate::common::TestCase {
                name: stringify!($name),
                test: $name
            }
        }
    };
    ($name:ident, $($input:literal),*) => {
        pub fn $name() -> Result<(), libtest_mimic::Failed> {
            let inputs = &[$($input),*];
            $crate::common::round_trip(stringify!($name), inputs)
        }

        inventory::submit! {
            $crate::common::TestCase {
                name: stringify!($name),
                test: $name
            }
        }
    };
}
