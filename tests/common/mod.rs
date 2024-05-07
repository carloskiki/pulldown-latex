#![allow(clippy::await_holding_lock)]

use std::{io::Write, mem::MaybeUninit, process::Stdio, sync::Mutex, time::Duration};

use fantoccini::{Client, ClientBuilder, Locator};
use heck::ToTitleCase;
use inventory::collect;
use libtest_mimic::{Arguments, Conclusion, Failed, Trial};
use pulldown_latex::{config::RenderConfig, mathml::push_mathml, parser::Parser};
use tokio::process::Command;

#[allow(clippy::type_complexity)]
pub static RENDERED: Mutex<Vec<(&str, Vec<(&str, String)>)>> = Mutex::new(Vec::new());
pub const OUTPUT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs");

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
    let rendered: Vec<_> = inputs
        .iter()
        .map(|input| -> Result<_, Failed> {
            show_errors(Parser::new(input))?;
            let parser = Parser::new(input);
            let mut output = String::new();
            push_mathml(&mut output, parser, config)?;
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
            println!("{:?}", e);
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
                    r#"<tr><td>{input}</td><td>{output}</td></tr>"#,
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
<link rel="stylesheet" type="text/css" href="{OUTPUT_DIR}/LatinModern/mathfonts.css">
<meta charset="UTF-8">
{styles}</head>
<body>
"#,
    ))?;

    render(file)?;

    file.write_fmt(format_args!(
        r#"</body>
</html>"#
    ))?;
    Ok(())
}

pub async fn cross_browser() -> anyhow::Result<()> {
    let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
    let driver_processes = [
        {
            let port = 4444;
            let process = Command::new("chromedriver")
                .arg(format!("--port={port}"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()?;
            (process, "chrome", port)
        },
        {
            let port = 4445;
            let process = Command::new("geckodriver")
                .args(["--port", port.to_string().as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()?;
            (process, "firefox", 4445)
        },
        {
            let port = 4446;
            let process = Command::new("safaridriver")
                .args(["--port", port.to_string().as_str()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()?;
            (process, "safari", 4446)
        },
    ];
    // Wait for processes to start, otherwise the clients will fail to connect
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    // Safety: This is safe because all elements of the array do not need to be initialized.
    let mut clients: [MaybeUninit<Client>; 3] = unsafe { MaybeUninit::uninit().assume_init() };
    for (i, (_, _, port)) in driver_processes.iter().enumerate() {
        let client = ClientBuilder::native()
            .connect(&format!("http://localhost:{}", port))
            .await?;
        clients[i].write(client);
    }
    // Safety: The clients are initialized
    let clients = unsafe { std::mem::transmute::<_, [Client; 3]>(clients) };

    let rendered = &*RENDERED.lock().unwrap();
    for (table_name, rows) in rendered {
        html_template(
            tmp.as_file_mut(),
            "",
            Some("cross-browser-render.css"),
            |file: &mut std::fs::File| -> anyhow::Result<()> {
                for (_input, output) in rows.iter() {
                    file.write_all(output.as_bytes())?;
                }
                Ok(())
            },
        )?;

        for (name, client) in driver_processes.iter().map(|t| t.1).zip(&clients) {
            let path = format!("file://{}", tmp.path().to_str().unwrap());

            client.goto(&path).await?;
            let elem = client
                .wait()
                .at_most(Duration::from_secs(10))
                .for_element(Locator::XPath("/html/body"))
                .await?;

            let screenshot = elem.screenshot().await?;

            tokio::fs::write(
                format!("{OUTPUT_DIR}/test-output/{name}/{table_name}.png"),
                screenshot,
            )
            .await?;
        }
    }

    for (mut process, _, _) in driver_processes {
        process.kill().await?;
    }
    for client in clients {
        client.close().await?;
    }

    Ok(())
}

pub fn cross_browser_tabled(file: &mut std::fs::File) -> anyhow::Result<()> {
    file.write_all(br#"<table style="margin: auto;">"#)?;
    file.write_all(br#"<tr><th>Input</th><th>Chrome</th><th>Firefox</th><th>Safari</th></tr>"#)?;
    let mut rendered = RENDERED.lock().unwrap();
    rendered.sort();
    rendered
        .iter()
        .try_for_each(|(table_name, rows)| -> std::io::Result<()> {
            file.write_fmt(format_args!(
                r#"<tr><th colspan="4">{table_name}</th></tr>"#,
                table_name = table_name.to_title_case()
            ))?;

            file.write_all(br#"<tr><td class="input">"#)?;
            rows.iter()
                .try_for_each(|(input, _)| -> std::io::Result<()> {
                    file.write_all(input.as_bytes())?;
                    file.write_all(b"\n")?;
                    Ok(())
                })?;
            file.write_all(b"</td>")?;
            for browser in ["chrome", "firefox", "safari"] {
                file.write_fmt(
                    format_args!(r#"<td class="image-container"><img class="{browser}-img" src="{OUTPUT_DIR}/test-output/{browser}/{table_name}.png"></td>"#)
                )?;
            }
            file.write_all(b"</tr>")?;
            Ok(())
        })?;
    file.write_all(b"</table>")?;
    Ok(())
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
    ($name:ident, $($input:literal),* $(, $field:ident = $value:expr)*) => {
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
