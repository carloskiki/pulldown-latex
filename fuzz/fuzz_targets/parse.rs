#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let storage = pulldown_latex::Storage::new();
    let parser = pulldown_latex::Parser::new(data, &storage);
    for _ in parser {}
});
