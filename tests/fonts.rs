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

// Text-mode font selectors usable inside math mode (KaTeX/MathJax compatibility).
round_trip_display!(
    text_font_selectors,
    r"a + \textrm{plain} + b",
    r"a + \textbf{bold} + b",
    r"a + \textit{italic} + b",
    r"a + \textsf{sans} + b",
    r"a + \texttt{mono} + b",
);

// `\text{...}` inheriting math-mode font state covers the remaining `mathvariant`
// branches in the MathML renderer.
round_trip_display!(
    text_under_math_fonts,
    r"\mathbb{\text{N}}",
    r"\mathfrak{\text{g}}",
    r"\mathbfcal{\text{S}}",
    r"\mathbfit{\text{v}}",
    r"\mathsfit{\text{x}}",
    r"\mathbffrak{\text{F}}",
    r"\mathbfsfup{\text{B}}",
    r"\mathbfsfit{\text{X}}",
    r"{\cal \text{C}}",
);

// `\colorbox`/`\fcolorbox` still parse a text argument after the refactor.
round_trip_display!(
    colorbox_text_argument,
    r"\colorbox{yellow}{hi}",
    r"\fcolorbox{red}{blue}{ok}",
);

// Unbraced single-character argument to `\text` exercises the `Token::Character`
// arm of `text_argument`.
round_trip_display!(text_single_char_argument, r"\text x");

// A control sequence as the argument to `\text` must error out
// (`ControlSequenceAsArgument`), covering the fallback arm of `text_argument`.
round_trip_display!(should_panic, text_control_sequence_argument, r"\text\alpha");
