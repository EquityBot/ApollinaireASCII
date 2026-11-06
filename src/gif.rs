// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! GIF decoding into composited RGBA frames, using weezl 0.2 for LZW.
//!
//! Released `gif` crates still use weezl 0.1 (see docs/DEPENDENCIES.md), so this module
//! replaces the `image` crate's GIF decoder. It keeps that decoder's results: the canvas
//! starts transparent, a transparent pixel shows the canvas beneath it, palette indices
//! beyond the table are transparent, a frame's disposal method sets what the next frame
//! draws over, and short image data is an error.

use crate::Result;
use image::{Rgba, RgbaImage};
use weezl::{BitOrder, LzwStatus, decode::Decoder};

/// Decode every frame in order, passing the composited logical screen and its delay in
/// centiseconds to `each`, so the caller can convert and drop each frame. The canvas is
/// allocated from the header, so callers check the screen size first, as `decode_media` does.
pub(crate) fn for_each_frame(
    bytes: &[u8],
    mut each: impl FnMut(RgbaImage, u16) -> Result<()>,
) -> Result<()> {
    let mut input = Input { bytes, cursor: 0 };
    let header = input.take(13).ok_or("Invalid GIF header")?;
    if &header[..6] != b"GIF89a" && &header[..6] != b"GIF87a" {
        return Err("Invalid GIF header".into());
    }
    let width = u32::from(u16::from_le_bytes([header[6], header[7]]));
    let height = u32::from(u16::from_le_bytes([header[8], header[9]]));
    let global = colour_table(&mut input, header[10])?;
    // The canvas the next frame draws over, after the previous frames' disposal.
    let mut canvas = RgbaImage::new(width, height);
    let mut control = Control::default();
    loop {
        match input.byte().ok_or("GIF trailer is missing")? {
            0x3b => return Ok(()),
            0x21 => {
                let label = input.byte().ok_or("Truncated GIF extension")?;
                // Plain text, graphic control, comment and application, as `gif` accepts.
                if ![0x01, 0xf9, 0xfe, 0xff].contains(&label) {
                    return Err("Unknown GIF extension".into());
                }
                // Like `gif`, a graphic control extension is read from its last sub-block.
                let mut last: &[u8] = &[];
                while let Some(block) = input.sub_block()? {
                    last = block;
                }
                if label == 0xf9 {
                    let [flags, delay_low, delay_high, transparent] = *last else {
                        return Err("Invalid GIF graphic control extension".into());
                    };
                    control = Control {
                        disposal: (flags >> 2) & 7,
                        delay_cs: u16::from_le_bytes([delay_low, delay_high]),
                        transparent: (flags & 1 != 0).then_some(transparent),
                    };
                }
            }
            0x2c => {
                let d = input.take(9).ok_or("Truncated GIF image descriptor")?;
                let left = u32::from(u16::from_le_bytes([d[0], d[1]]));
                let top = u32::from(u16::from_le_bytes([d[2], d[3]]));
                let w = u32::from(u16::from_le_bytes([d[4], d[5]]));
                let h = u32::from(u16::from_le_bytes([d[6], d[7]]));
                if w == 0 || h == 0 || left + w > width || top + h > height {
                    return Err("GIF frame rectangle must fit the logical screen".into());
                }
                let local = colour_table(&mut input, d[8])?;
                let palette = local.or(global).ok_or("GIF frame has no colour table")?;
                let indices = image_data(&mut input, w as usize * h as usize)?;
                let mut screen = canvas.clone();
                for (row, y) in rows(h, d[8] & 0x40 != 0).enumerate() {
                    for x in 0..w {
                        let index = indices[row * w as usize + x as usize];
                        let (sx, sy) = (left + x, top + y);
                        let start = usize::from(index) * 3;
                        let pixel = match palette.get(start..start + 3) {
                            Some(&[r, g, b]) if control.transparent != Some(index) => {
                                Rgba([r, g, b, 255])
                            }
                            _ => *canvas.get_pixel(sx, sy),
                        };
                        screen.put_pixel(sx, sy, pixel);
                        // 2 restores the background, which `image` treated as transparent;
                        // 3 restores the previous canvas; 0, 1 and undefined values keep.
                        match control.disposal {
                            2 => canvas.put_pixel(sx, sy, Rgba([0, 0, 0, 0])),
                            3 => {}
                            _ => canvas.put_pixel(sx, sy, pixel),
                        }
                    }
                }
                each(screen, control.delay_cs)?;
                control = Control::default();
            }
            _ => return Err("Invalid GIF block marker".into()),
        }
    }
}

/// The graphic control extension that applies to the next image only.
#[derive(Default)]
struct Control {
    disposal: u8,
    delay_cs: u16,
    transparent: Option<u8>,
}

struct Input<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Input<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.cursor.checked_add(count)?;
        let slice = self.bytes.get(self.cursor..end)?;
        self.cursor = end;
        Some(slice)
    }
    fn byte(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    /// The next data sub-block, or `None` at the zero-length terminator.
    fn sub_block(&mut self) -> Result<Option<&'a [u8]>> {
        let size = self.byte().ok_or("Truncated GIF sub-block")?;
        if size == 0 {
            return Ok(None);
        }
        Ok(Some(
            self.take(usize::from(size))
                .ok_or("Truncated GIF sub-block")?,
        ))
    }
}

fn colour_table<'a>(input: &mut Input<'a>, flags: u8) -> Result<Option<&'a [u8]>> {
    if flags & 0x80 == 0 {
        return Ok(None);
    }
    let size = 3usize << (usize::from(flags & 7) + 1);
    Ok(Some(input.take(size).ok_or("Truncated GIF colour table")?))
}

/// Decompress one image's indices. Data beyond `count` pixels is ignored; less is an error.
fn image_data(input: &mut Input, count: usize) -> Result<Vec<u8>> {
    let min_code_size = input.byte().ok_or("Truncated GIF image data")?;
    if !(1..=11).contains(&min_code_size) {
        return Err("Invalid GIF LZW code size".into());
    }
    let mut data = Vec::new();
    while let Some(block) = input.sub_block()? {
        data.extend_from_slice(block);
    }
    let mut decoder = Decoder::new(BitOrder::Lsb, min_code_size);
    let mut indices = vec![0; count];
    let (mut read, mut written) = (0, 0);
    let invalid = |e| format!("Invalid GIF image data: {e}");
    while written < count {
        let result = decoder.decode_bytes(&data[read..], &mut indices[written..]);
        read += result.consumed_in;
        written += result.consumed_out;
        // weezl returns `Ok` whenever its state advanced, even with no bytes read or written,
        // and `NoProgress` once nothing can advance.
        match result.status.map_err(invalid)? {
            LzwStatus::Ok => {}
            LzwStatus::NoProgress | LzwStatus::Done => break,
        }
    }
    if written < count {
        return Err("GIF image data is truncated".into());
    }
    // Like `gif`, refuse an invalid code that weezl has already read past the last pixel.
    decoder.decode_bytes(&[], &mut []).status.map_err(invalid)?;
    Ok(indices)
}

/// Screen rows in the order the image data stores them.
fn rows(height: u32, interlaced: bool) -> Box<dyn Iterator<Item = u32>> {
    if interlaced {
        Box::new(
            (0..height)
                .step_by(8)
                .chain((4..height).step_by(8))
                .chain((2..height).step_by(4))
                .chain((1..height).step_by(2)),
        )
    } else {
        Box::new(0..height)
    }
}

// Declared here, not inside `tests`, so the path resolves from `src/` on every platform.
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::{GifFrame, gif};
    use super::for_each_frame;
    use image::Rgba;

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];
    const CLEAR: Rgba<u8> = Rgba([0, 0, 0, 0]);

    fn frames(bytes: &[u8]) -> Vec<(Vec<Rgba<u8>>, u16)> {
        let mut out = Vec::new();
        for_each_frame(bytes, |screen, delay| {
            out.push((screen.pixels().copied().collect(), delay));
            Ok(())
        })
        .unwrap();
        out
    }
    fn rgba([r, g, b]: [u8; 3]) -> Rgba<u8> {
        Rgba([r, g, b, 255])
    }
    fn part(rect: (u16, u16, u16, u16), indices: Vec<u8>, disposal: u8) -> GifFrame {
        GifFrame {
            rect,
            disposal,
            ..GifFrame::full(rect.2, rect.3, indices, 5)
        }
    }

    #[test]
    fn transparent_pixels_show_the_canvas_and_keep_leaves_the_frame() {
        let mut second = part((0, 0, 2, 1), vec![1, 3], 1);
        second.transparent = Some(3);
        let bytes = gif(
            2,
            1,
            Some(&[RED, GREEN, BLUE]),
            &[part((0, 0, 2, 1), vec![0, 0], 1), second],
        );
        let out = frames(&bytes);
        assert_eq!(out[0], (vec![rgba(RED), rgba(RED)], 5));
        assert_eq!(out[1].0, vec![rgba(GREEN), rgba(RED)]);
    }

    #[test]
    fn background_disposal_clears_only_the_frame_rectangle() {
        let bytes = gif(
            3,
            1,
            Some(&[RED, GREEN]),
            &[
                part((0, 0, 3, 1), vec![0, 0, 0], 1),
                part((1, 0, 2, 1), vec![1, 1], 2),
                part((0, 0, 1, 1), vec![1], 1),
            ],
        );
        let out = frames(&bytes);
        assert_eq!(out[1].0, vec![rgba(RED), rgba(GREEN), rgba(GREEN)]);
        assert_eq!(out[2].0, vec![rgba(GREEN), CLEAR, CLEAR]);
    }

    #[test]
    fn previous_disposal_restores_the_canvas_before_the_frame() {
        let bytes = gif(
            2,
            1,
            Some(&[RED, GREEN, BLUE]),
            &[
                part((0, 0, 2, 1), vec![0, 0], 1),
                part((0, 0, 2, 1), vec![1, 1], 3),
                part((0, 0, 1, 1), vec![2], 1),
            ],
        );
        let out = frames(&bytes);
        assert_eq!(out[1].0, vec![rgba(GREEN), rgba(GREEN)]);
        assert_eq!(out[2].0, vec![rgba(BLUE), rgba(RED)]);
    }

    #[test]
    fn interlaced_rows_land_in_screen_order() {
        let indices: Vec<u8> = (0..10).collect();
        let palette: Vec<[u8; 3]> = (0..10).map(|i| [i * 20, 0, 0]).collect();
        let mut frame = GifFrame::full(1, 10, indices, 0);
        frame.interlaced = true;
        let out = frames(&gif(1, 10, Some(&palette), &[frame]));
        let expected: Vec<_> = palette.iter().map(|&c| rgba(c)).collect();
        assert_eq!(out[0], (expected, 0));
    }

    #[test]
    fn local_tables_override_and_unknown_indices_are_transparent() {
        let mut frame = GifFrame::full(2, 1, vec![0, 5], 0);
        frame.local = Some(vec![BLUE, GREEN]);
        let out = frames(&gif(2, 1, Some(&[RED]), &[frame]));
        assert_eq!(out[0].0, vec![rgba(BLUE), CLEAR]);
    }

    #[test]
    fn malformed_structure_is_refused() {
        let valid = gif(
            2,
            2,
            Some(&[RED, GREEN]),
            &[GifFrame::full(2, 2, vec![0, 1, 1, 0], 3)],
        );
        let refused = |bytes: &[u8]| for_each_frame(bytes, |_, _| Ok(())).is_err();
        assert!(!refused(&valid));
        // No colour table anywhere.
        assert!(refused(&gif(
            2,
            2,
            None,
            &[GifFrame::full(2, 2, vec![0; 4], 3)]
        )));
        // Header 13 + table 6 + control 8 puts the descriptor at 28 and the code size at 37.
        // 20 is the control label: 0x02 is an extension `gif` does not know.
        for (offset, value) in [(37, 0), (37, 12), (20, 2), (21, 3), (34, 3)] {
            let mut bytes = valid.clone();
            bytes[offset] = value;
            assert!(refused(&bytes), "byte {offset} = {value}");
        }
        for length in 0..valid.len() {
            assert!(refused(&valid[..length]), "truncation {length}");
        }
    }

    #[test]
    fn a_control_extension_uses_its_last_sub_block() {
        let valid = gif(1, 1, Some(&[RED]), &[GifFrame::full(1, 1, vec![0], 3)]);
        // Header 13 + table 6: the control block size is at 21. Put a 2-byte block first.
        let mut split = valid[..21].to_vec();
        split.extend([2, 0xaa, 0xbb]);
        split.extend(&valid[21..]);
        assert_eq!(frames(&split)[0].1, 3);
    }

    /// A 2 x 1 GIF with a 4-colour table (red, green, black, black) and raw LZW data.
    fn raw(code_size: u8, lzw: [u8; 2]) -> Vec<u8> {
        let mut bytes = b"GIF89a\x02\x00\x01\x00\x81\x00\x00".to_vec();
        bytes.extend([255, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend([0x2c, 0, 0, 0, 0, 2, 0, 1, 0, 0, code_size, 2]);
        bytes.extend(lzw);
        bytes.extend([0, 0x3b]);
        bytes
    }

    #[test]
    fn repeated_clear_codes_decode() {
        // 3-bit codes clear, clear, 1, 0, end. weezl returns once without visible progress.
        let out = frames(&raw(2, [0x64, 0x50]));
        assert_eq!(out[0].0, vec![rgba(GREEN), rgba(RED)]);
    }

    #[test]
    fn an_invalid_code_after_the_last_pixel_is_refused() {
        // 3-bit codes 0, 0, clear, then 6, which needs a predecessor. weezl reads it with
        // the pixels but reports it on the next call.
        assert!(for_each_frame(&raw(2, [0x00, 0x0d]), |_, _| Ok(())).is_err());
    }

    #[test]
    fn corrupt_bytes_never_panic() {
        let valid = gif(
            4,
            3,
            Some(&[RED, GREEN, BLUE]),
            &[
                GifFrame::full(4, 3, (0..12).map(|i| i % 3).collect(), 2),
                part((1, 1, 2, 2), vec![2, 1, 0, 2], 3),
            ],
        );
        for offset in 0..valid.len() {
            for value in [0, 1, 2, 0x7f, 0x80, 0xfe, 0xff] {
                let mut bytes = valid.clone();
                bytes[offset] = value;
                let _ = for_each_frame(&bytes, |_, _| Ok(()));
            }
        }
    }
}
