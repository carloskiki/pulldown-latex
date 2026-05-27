use std::{fs::File, path::Path};

use crate::common::{OUTPUT_DIR, html_template, tabled};

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let mut file = File::create(Path::new(OUTPUT_DIR).join("miscellaneous.html")).unwrap();
    html_template(&mut file, "Miscellaneous Tests", None, tabled).unwrap();

    concl.exit();
}

round_trip_display!(
    stacking_amsmath,
    r"\overbrace{a+b+c}^{n}",
    r"\underbrace{a+b+c}_{m}",
    r"\overbracket{x+y}",
    r"\underbracket{p+q}",
    r"\sum_{\substack{i<n \\ j<m}} a_{ij}",
    r"\sideset{_a^b}{_c^d}\sum"
);

