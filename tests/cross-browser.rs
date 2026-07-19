use std::{io::Write, path::Path, process::Stdio, time::Duration};

use common::{html_template, OUTPUT_DIR};
use fantoccini::{ClientBuilder, Locator};
use heck::ToTitleCase;
use pulldown_latex::config::DisplayMode;
use tokio::process::Command;

use crate::common::RENDERED;

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(cross_browser()).unwrap();

    let mut file = std::fs::File::create(Path::new(OUTPUT_DIR).join("cross-browser.html")).unwrap();
    html_template(
        &mut file,
        "Cross Browser Tests",
        Some("cross-browser.css"),
        cross_browser_tabled,
    )
    .unwrap();
    concl.exit()
}

pub async fn cross_browser() -> anyhow::Result<()> {
    let chromedriver = driver_command("chromedriver", "CHROMEDRIVER");
    browser_tests(&chromedriver, "chrome").await?;
    let geckodriver = driver_command("geckodriver", "GECKODRIVER");
    browser_tests(&geckodriver, "firefox").await?;

    #[cfg(target_os = "macos")]
    {
        let safaridriver = driver_command("safaridriver", "SAFARIDRIVER");
        browser_tests(&safaridriver, "safari").await?
    }

    Ok(())
}

fn driver_command(default_command: &str, env_var: &str) -> String {
    std::env::var(env_var)
        .or_else(|_| std::env::var(format!("{env_var}_CMD")))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_command.to_string())
}

async fn browser_tests(command: &str, name: &str) -> anyhow::Result<()> {
    let mut tmp = tempfile::Builder::new().suffix(".html").tempfile()?;
    let path = format!("file://{}", tmp.path().to_str().unwrap());

    let mut process = Command::new(command)
        .arg("--port=4444")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?;

    let rendered = &*RENDERED.lock().unwrap();
    for (table_name, rows) in rendered {
        for (_input, output) in rows {
            html_template(
                tmp.as_file_mut(),
                "",
                Some("cross-browser-render.css"),
                |file: &mut std::fs::File| -> anyhow::Result<()> {
                    file.write_all(output.as_bytes())?;
                    Ok(())
                },
            )?;

            client.goto(&path).await?;
            let elem = client
                .wait()
                .at_most(Duration::from_secs(10))
                .for_element(Locator::XPath("/html/body"))
                .await?;

            let screenshot = elem.screenshot().await?;

            tokio::fs::write(
                format!("{OUTPUT_DIR}/screenshots/{name}/{table_name}.png"),
                screenshot,
            )
            .await?;
            tmp.as_file().set_len(0)?;
        }
    }
    tmp.close()?;
    client.close().await?;
    process.kill().await?;

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
                    format_args!(r#"<td class="image-container"><img class="{browser}-img" src="{OUTPUT_DIR}/screenshots/{browser}/{table_name}.png"></td>"#)
                )?;
            }
            file.write_all(b"</tr>")?;
            Ok(())
        })?;
    file.write_all(b"</table>")?;
    Ok(())
}

round_trip! {basic, "5 + 5 = 10", display_mode = DisplayMode::Block}

round_trip!(
    complex_array,
    r"\begin{array}{||c|c::c|c||}
            \hline
            A & B & C & D \\ \hdashline
            1 & 2 & 3 & 4 \\ \hline
            5 & 6 & 7 & 8 \\
            9 & 10 & 11 & 12
            \end{array}",
    display_mode = DisplayMode::Block
);

round_trip! {
    cases,
r#"\begin{cases}
    1 & \text{if } x \ge 0 \\
    0 & \text{if } x < 0
\end{cases}"#,
r#"\begin{rcases}
    a & \text{if } x \ge 0 \\
    b & \text{if } x < 0
\end{rcases}"#
}

round_trip! {
    align,
r#"\begin{align}
    a &= b + c \\
    d &= e + f
\end{align}"#,
r#"\begin{align*}
    a &= b + c \\
    d &= e + f
\end{align*}"#,
}
round_trip! {
    aligned,
r#"\begin{aligned}
    a &= b + c \\
    d &= e + f
\end{aligned}"#,
}
round_trip! {
    subarray,
r#"\begin{subarray}{c}
    a + b \\
    c + d
\end{subarray}"#, 
r#"\begin{subarray}{l}
    a = b \\
    c = d
\end{subarray}"#
}
round_trip! {
    alignat,
    r#"\begin{alignat}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignat}"#,
r#"\begin{alignat*}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignat*}"#
}
round_trip! {
    alignedat,
    r#"\begin{alignedat}{2}
    a &= b + c & d &= e + f \\
    g &= h + i & j &= k + l
\end{alignedat}"#,
}
round_trip! {
    gather,
    r#"\begin{gather}
    a = b + c \\
    d = e + f
\end{gather}"#,
r#"\begin{gather*}
    a = b + c \\
    d = e + f
\end{gather*}"#,

}
round_trip! {
    gathered,
    r#"\begin{gathered}
    a = b + c \\
    d = e + f
\end{gathered}"#,
}
round_trip! {
    multline,
    r#"\begin{multline}
    a + b + c \\
    d + e + f
\end{multline}"#,
}
round_trip! {
    split,
    r#"\begin{split}
    a + b + c \\
    d + e + f
\end{split}"#,
}
round_trip! {
    equation,
r#"\begin{equation}
    a = b + c
\end{equation}"#,
r#"\begin{equation*}
    a = b + c
\end{equation*}"#,
}
round_trip_display! {
    colors,
    r"\fcolorbox{red}{blue}{\textcolor{white}{a + b = c}}"
}
