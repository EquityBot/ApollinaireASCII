// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! A minimal GIF writer for building test inputs.

/// One image in a test GIF. `indices` are in screen row order.
pub struct GifFrame {
    pub rect: (u16, u16, u16, u16),
    pub indices: Vec<u8>,
    pub delay_cs: u16,
    pub disposal: u8,
    pub transparent: Option<u8>,
    pub local: Option<Vec<[u8; 3]>>,
    pub interlaced: bool,
}

impl GifFrame {
    pub fn full(width: u16, height: u16, indices: Vec<u8>, delay_cs: u16) -> Self {
        GifFrame {
            rect: (0, 0, width, height),
            indices,
            delay_cs,
            disposal: 1,
            transparent: None,
            local: None,
            interlaced: false,
        }
    }
}

pub fn gif(width: u16, height: u16, global: Option<&[[u8; 3]]>, frames: &[GifFrame]) -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    out.extend([global.map_or(0, |t| 0x80 | table_bits(t.len())), 0, 0]);
    if let Some(table) = global {
        write_table(&mut out, table);
    }
    for frame in frames {
        let flags = frame.disposal << 2 | u8::from(frame.transparent.is_some());
        out.extend([0x21, 0xf9, 4, flags]);
        out.extend(frame.delay_cs.to_le_bytes());
        out.extend([frame.transparent.unwrap_or(0), 0]);
        let (left, top, w, h) = frame.rect;
        out.push(0x2c);
        for value in [left, top, w, h] {
            out.extend(value.to_le_bytes());
        }
        let local_flags = frame
            .local
            .as_ref()
            .map_or(0, |t| 0x80 | table_bits(t.len()));
        out.push(local_flags | if frame.interlaced { 0x40 } else { 0 });
        if let Some(table) = &frame.local {
            write_table(&mut out, table);
        }
        let rows: Vec<usize> = if frame.interlaced {
            let h = usize::from(h);
            (0..h)
                .step_by(8)
                .chain((4..h).step_by(8))
                .chain((2..h).step_by(4))
                .chain((1..h).step_by(2))
                .collect()
        } else {
            (0..usize::from(h)).collect()
        };
        let stored: Vec<u8> = rows
            .iter()
            .flat_map(|&row| &frame.indices[row * usize::from(w)..][..usize::from(w)])
            .copied()
            .collect();
        out.push(8);
        let data = weezl::encode::Encoder::new(weezl::BitOrder::Lsb, 8)
            .encode(&stored)
            .unwrap();
        for chunk in data.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend(chunk);
        }
        out.push(0);
    }
    out.push(0x3b);
    out
}

fn table_bits(entries: usize) -> u8 {
    (0..=7u8).find(|&bits| 2usize << bits >= entries).unwrap()
}

fn write_table(out: &mut Vec<u8>, table: &[[u8; 3]]) {
    let entries = 2usize << table_bits(table.len());
    for i in 0..entries {
        out.extend(table.get(i).copied().unwrap_or([0, 0, 0]));
    }
}
