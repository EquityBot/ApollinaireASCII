// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

#![no_main]
//! A user's FLF or TLF font file: parsed, and when valid, used to render text.

use apollinaire_ascii::lettering::{Options, render_with, validate_font};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(contents) = std::str::from_utf8(data) else {
        return;
    };
    if validate_font(contents).is_ok() {
        let _ = render_with("Fuzz", "standard", Some(contents), &Options::default());
    }
});
