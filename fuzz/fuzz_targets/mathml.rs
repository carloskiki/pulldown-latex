#![no_main]

use libfuzzer_sys::fuzz_target;
use pulldown_latex::config::{DisplayMode, MathStyle};
use pulldown_latex::{push_mathml, Parser, RenderConfig, Storage};

fuzz_target!(|data: &str| {
    for &display_mode in &[DisplayMode::Inline, DisplayMode::Block] {
        for &math_style in &[
            MathStyle::TeX,
            MathStyle::ISO,
            MathStyle::French,
            MathStyle::Upright,
        ] {
            let storage = Storage::new();
            let parser = Parser::new(data, &storage);
            let config = RenderConfig {
                display_mode,
                math_style,
                ..Default::default()
            };
            let mut output = String::new();
            let _ = push_mathml(&mut output, parser, config);
        }
    }
});
