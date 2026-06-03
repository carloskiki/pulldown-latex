use heck::ToTitleCase;
use inventory::collect;
use libtest_mimic::{Arguments, Conclusion, Failed, Trial};
use pulldown_latex::{config::RenderConfig, mathml::push_mathml, Parser, Storage};
use std::{io::Write, sync::Mutex};

#[allow(clippy::type_complexity)]
pub static RENDERED: Mutex<Vec<(&str, Vec<(&str, String)>)>> = Mutex::new(Vec::new());
pub const OUTPUT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/out");

pub struct TestCase {
    pub name: &'static str,
    pub test: fn() -> Result<(), Failed>,
}
collect!(TestCase);

pub fn test() -> Conclusion {
    let args = Arguments::from_args();
    let tests = inventory::iter::<TestCase>
        .into_iter()
        .map(|TestCase { name, test }| Trial::test(*name, test))
        .collect::<Vec<_>>();
    libtest_mimic::run(&args, tests)
}

pub fn round_trip(
    fn_name: &'static str,
    inputs: &[&'static str],
    config: RenderConfig,
) -> Result<(), Failed> {
    let mut storage = Storage::new();
    let rendered: Vec<_> = inputs
        .iter()
        .map(|input| -> Result<_, Failed> {
            show_errors(Parser::new(input, &storage))?;
            let parser = Parser::new(input, &storage);
            let mut output = String::new();
            push_mathml(&mut output, parser, config)?;
            storage.reset();
            Ok((*input, output))
        })
        .collect::<Result<_, Failed>>()?;

    RENDERED.lock()?.push((fn_name, rendered));

    Ok(())
}

pub fn show_errors(parser: Parser) -> Result<(), usize> {
    let mut error_count = 0;

    parser.for_each(|event| {
        if let Err(e) = event {
            eprintln!("{:?}", e);
            error_count += 1;
        }
    });

    if error_count > 0 {
        Err(error_count)
    } else {
        Ok(())
    }
}

pub fn tabled(file: &mut std::fs::File) -> anyhow::Result<()> {
    file.write_all(br#"<table style="max-width: 60vw; margin: auto;">"#)?;
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
                    r#"<tr><td>{input}</td><td style="position: relative">{output}</td></tr>"#,
                    input = input,
                    output = output
                ))
            })
        })?;
    file.write_all(b"</table>")?;
    Ok(())
}

pub fn html_template(
    file: &mut std::fs::File,
    title: &str,
    stylesheet: Option<&str>,
    render: impl FnOnce(&mut std::fs::File) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let styles = match stylesheet {
        Some(stylesheet) => {
            format!(r#"<link rel="stylesheet" type="text/css" href="{OUTPUT_DIR}/{stylesheet}">"#,)
        }
        None => "".to_string(),
    };

    file.write_fmt(format_args!(
        r#"<!DOCTYPE html>
<html>
<head>
<title>{title}</title>
<link rel="stylesheet" type="text/css" href="{}/styles.css">
<meta charset="UTF-8">
{styles}</head>
<body>
"#,
        env!("CARGO_MANIFEST_DIR")
    ))?;

    render(file)?;

    file.write_fmt(format_args!(
        r#"</body>
</html>"#
    ))?;
    Ok(())
}

#[macro_export]
macro_rules! round_trip_display {
    ($name:ident, $($input:literal),+ $(,)?) => {
        $crate::round_trip!(
            $name,
            $($input),+,
            display_mode = pulldown_latex::config::DisplayMode::Block
        );
    };
    (should_panic, $name:ident, $($input:literal),+ $(,)?) => {
        $crate::round_trip!(
            should_panic,
            $name,
            $($input),+
        );
    }
}

#[macro_export]
macro_rules! round_trip {
    (should_panic, $name:ident, $($input:literal),+ $(,)?) => {
        pub fn $name() -> Result<(), libtest_mimic::Failed> {
            let inputs = &[$($input),*];
            let mut storage = pulldown_latex::Storage::new();
            for input in inputs {
                let mut parser = pulldown_latex::parser::Parser::new(input, &storage);
                if !parser.all(|event| event.is_err()) {
                    return Err(libtest_mimic::Failed::without_message());
                };
                storage.reset();
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
    ($name:ident, $($input:literal),+ $(, $field:ident = $value:expr)* $(,)?) => {
        pub fn $name() -> Result<(), libtest_mimic::Failed> {
            let inputs = &[$($input),*];
            $crate::common::round_trip(stringify!($name), inputs, pulldown_latex::config::RenderConfig {
                $($field: $value,)*
                ..Default::default()
            })
        }

        inventory::submit! {
            $crate::common::TestCase {
                name: stringify!($name),
                test: $name
            }
        }
    };
}
