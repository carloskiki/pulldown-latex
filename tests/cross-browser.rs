use std::path::Path;

use common::{cross_browser, cross_browser_tabled, html_template, OUTPUT_DIR};
use pulldown_latex::config::DisplayMode;

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

round_trip! {basic, "5 + 5 = 10", display_mode = DisplayMode::Block}

round_trip!(
    complex_array,
    r"\begin{array}{||c|c::c|c||}
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
round_trip_display! {
    colors,
    r"\fcolorbox{red}{blue}{\textcolor{white}{a + b = c}}"
}
