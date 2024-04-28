use std::path::Path;

use common::{cross_browser, cross_browser_tabled, html_template, OUTPUT_DIR};
use pulldown_latex::config::DisplayMode;

mod common;

fn main() {
    let concl = common::test();

    if std::env::var("BROWSER_RENDER") != Ok("false".to_string()) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(cross_browser()).unwrap();
    }

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

round_trip!(basic, "5 + 5 = 10", display_mode = DisplayMode::Block);
