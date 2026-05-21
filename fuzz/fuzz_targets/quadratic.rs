#![no_main]

use libfuzzer_sys::fuzz_target;
use pulldown_latex::{push_mathml, Parser, RenderConfig, Storage};
use std::time::Instant;

/// Maximum number of doubling steps.
const MAX_STEPS: u32 = 3;
/// If duration-per-byte ratio between doublings consistently exceeds this,
/// it indicates quadratic (or worse) behavior.
const MIN_RATIO: f64 = 2.5;
/// Starting input size in bytes.
const BASE_NUM_BYTES: usize = 1024;

fuzz_target!(|data: &str| {
    if data.is_empty() {
        return;
    }

    let mut prev_duration_per_byte: Option<f64> = None;
    let mut quadratic_steps = 0u32;

    for step in 0..=MAX_STEPS {
        let num_bytes = BASE_NUM_BYTES << step;
        let input = build_input(data, num_bytes);

        let start = Instant::now();

        let storage = Storage::new();
        let parser = Parser::new(&input, &storage);
        let config = RenderConfig::default();
        let mut output = String::new();
        let _ = push_mathml(&mut output, parser, config);

        let elapsed = start.elapsed().as_secs_f64();
        let duration_per_byte = elapsed / input.len() as f64;

        if let Some(prev) = prev_duration_per_byte {
            if prev > 0.0 {
                let ratio = duration_per_byte / prev;
                if ratio > MIN_RATIO {
                    quadratic_steps += 1;
                } else {
                    quadratic_steps = 0;
                }
            }
        }
        prev_duration_per_byte = Some(duration_per_byte);

        if quadratic_steps >= 2 {
            panic!(
                "Potential quadratic behavior detected! \
                 Input pattern (first 80 bytes): {:?}, \
                 ratio exceeded {MIN_RATIO} for {quadratic_steps} consecutive doublings",
                &data[..data.len().min(80)]
            );
        }
    }
});

/// Build a repeated input of at least `target_len` bytes from the seed `data`.
fn build_input(data: &str, target_len: usize) -> String {
    let repeats = (target_len / data.len()).max(1);
    data.repeat(repeats)
}
