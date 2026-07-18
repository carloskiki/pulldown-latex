use std::{fs::File, path::Path};

use common::{html_template, tabled, OUTPUT_DIR};

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let mut file = File::create(Path::new(OUTPUT_DIR).join("fonts.html")).unwrap();
    html_template(&mut file, "Fonts Tests", None, tabled).unwrap();

    concl.exit();
}

round_trip_display!(
    boldsymbol_latin_letters,
    r"\boldsymbol{ABCDEFGHIJKLMNOPQRSTUVWXYZ}",
    r"\boldsymbol{abcdefghijklmnopqrstuvwxyz}"
);

round_trip_display!(boldsymbol_digits, r"\boldsymbol{0123456789}");

round_trip_display!(
    boldsymbol_nabla_and_partial,
    r"\boldsymbol{\nabla \partial}"
);

round_trip_display!(
    double_struck_italic,
    r"\symbbit{Ddeij}",
    r"\mathbbit{Ddeij}"
);
