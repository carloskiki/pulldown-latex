use std::{fs::File, path::Path};

use common::{html_template, tabled, OUTPUT_DIR};

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let mut file = File::create(Path::new(OUTPUT_DIR).join("environments.html")).unwrap();
    html_template(&mut file, "Mathematical Environments Tests", None, tabled).unwrap();

    concl.exit();
}

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

round_trip_display!(
    arrays,
    r#"\begin{array}{||c|r|l||}
    a + b \\[2em]
    a + b & c & d \\[2em] \hline
    a + b
    \end{array}"#,
    r#"\begin{array}{c:c:c}
       a & b & c \\ \hline
       d & e & f \\
       \hdashline
       g & h & i
    \end{array}"#,
);
