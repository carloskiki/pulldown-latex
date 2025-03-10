#![no_main]

use fantoccini::{ClientBuilder, Locator};
use libfuzzer_sys::fuzz_target;
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;

fuzz_target!(|data: &str| {
    // Parse with our parser
    let pulldown = {
        let mut pulldown = String::new();
        let storage = pulldown_latex::Storage::new();
        let parser = pulldown_latex::Parser::new(data, &storage);
        let mut config = pulldown_latex::RenderConfig::default();
        config.display_mode = pulldown_latex::config::DisplayMode::Block;
        config.xml = true;
        config.annotation = Some(data);
        match pulldown_latex::push_mathml(&mut pulldown, parser, config) {
            Ok(()) => (),
            // we don't implement everything katex implements, so this is fine
            // if it's a problem, we'll work with it
            Err(_) => return,
        }
        pulldown
            // work around minor display differences
            // we might want to fix these; I dunno
            .replace(r##" mathvariant="normal""##, "")
            .to_owned()
    };

    // Uncomment this to just hammer the parser without comparing it against anything.
    //return;

    // Parse with katex
    let katex = if let Ok(katex) = 
        katex::render_with_opts(
            data,
            &katex::Opts::builder()
                .output_type(katex::OutputType::Mathml)
                .display_mode(true)
                .build()
                .unwrap()
        )
    {
        katex
            // remove pointless wrappers
            .strip_prefix("<span class=\"katex\">")
            .unwrap_or(&katex)
            .strip_suffix("</span>")
            .unwrap_or(&katex)
            // work around minor display differences
            // we might want to fix these; I dunno
            .replace(r##" mathvariant="normal""##, "")
            .to_owned()
    } else {
        return;
    };

    // Performance hack: we compare the katex and pulldown mathml with the root tag
    // removed, because they put some of the attributes in the opposite order.
    // If they're identical, then none of this matters.
    {
        let pulldown_trimmed = pulldown.strip_prefix(r#"<math display="block" xmlns="http://www.w3.org/1998/Math/MathML">"#).unwrap_or(&pulldown);
        let katex_trimmed = katex.strip_prefix(r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">"#).unwrap_or(&katex);
        if pulldown_trimmed == katex_trimmed {
            return;
        } else {
            println!("== need to check in a browser, because the mathml isn't identical");
            println!("data:             {data:?}");
            println!("trimmed pulldown: {pulldown_trimmed:?} {len}", len = pulldown_trimmed.len());
            println!("trimmed katex:    {katex_trimmed:?} {len}", len = katex_trimmed.len());
        }
    }

    // Now pipe both of these out to Firefox, and see if they produce identical results.
    // For speed, the fuzz tester only checks with one browser.
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let port = 4445;
        let mut process = tokio::process::Command::new("geckodriver")
            .args(["--port", port.to_string().as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let client = ClientBuilder::native()
            .connect(&format!("http://localhost:{}", port))
            .await?;

        // Wait for Firefox to start.
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        // Write pulldown mathml to HTML template
        let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
        let path = format!("file://{}", tmp.path().to_str().unwrap());
        html_template(
            tmp.as_file_mut(),
            "pulldown",
            |file: &mut std::fs::File| {
                file.write_all(pulldown.as_bytes())?;
                Ok(())
            },
        )?;

        // Get pulldown screenshot
        client.goto(&path).await?;
        let elem = client
            .wait()
            .at_most(Duration::from_secs(10))
            .for_element(Locator::XPath("/html/body"))
            .await?;
        let screenshot_pulldown = elem.screenshot().await.ok().and_then(|screenshot| {
            ImageReader::new(Cursor::new(screenshot)).with_guessed_format().ok()?.decode().ok()
        });

        // Write katex mathml to HTML template
        let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
        let path = format!("file://{}", tmp.path().to_str().unwrap());
        html_template(
            tmp.as_file_mut(),
            "katex",
            |file: &mut std::fs::File| {
                file.write_all(katex.as_bytes())?;
                Ok(())
            },
        )?;

        // Get katex screenshot
        client.goto(&path).await?;
        let elem = client
            .wait()
            .at_most(Duration::from_secs(10))
            .for_element(Locator::XPath("/html/body"))
            .await?;
        let screenshot_katex = elem.screenshot().await.ok().and_then(|screenshot| {
            ImageReader::new(Cursor::new(screenshot)).with_guessed_format().ok()?.decode().ok()
        });

        client.close().await?;
        process.kill().await?;

        if screenshot_pulldown != screenshot_katex {
            if let Some(screenshot_katex) = screenshot_katex {
                screenshot_katex.save_with_format("katex.png", ImageFormat::Png)?;
            }
            if let Some(screenshot_pulldown) = screenshot_pulldown {
                screenshot_pulldown.save_with_format("pulldown.png", ImageFormat::Png)?;
            }
            panic!();
        }

        Result::<(), Box<dyn std::error::Error>>::Ok(())
    }).unwrap();
});

fn html_template(
    file: &mut std::fs::File,
    title: &str,
    render: impl FnOnce(&mut std::fs::File) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    file.write_fmt(format_args!(
        r#"<!DOCTYPE html>
<html>
<head>
<title>{title}</title>
<meta charset="UTF-8">
<link rel="stylesheet" href="{source}/../tests/out/cross-browser-render.css">
</head>
<body>
"#,
        source = env!("CARGO_MANIFEST_DIR"),
    ))?;

    render(file)?;

    file.write_fmt(format_args!(
        r#"</body>
</html>"#
    ))?;
    Ok(())
}