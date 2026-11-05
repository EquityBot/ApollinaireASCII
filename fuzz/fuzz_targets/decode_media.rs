// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

#![no_main]
//! Image and GIF bytes from a user's file or URL, through the format sniff, the
//! in-house GIF decoder and the renderer.

use apollinaire_ascii::{RenderOptions, decode_media};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let options = RenderOptions {
        width: 40,
        ..RenderOptions::default()
    };
    let _ = decode_media(data, &options);
});
