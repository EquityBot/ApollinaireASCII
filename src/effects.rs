// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Deterministic text effects that animate any frame of ASCII art or lettering.
//!
//! Every effect keeps the source dimensions and emits printable ASCII only. Spaces in the
//! source are background: effects move, reveal or burn the other characters. All effects
//! except `burn` finish on the unchanged source frame; `burn` finishes blank. With
//! `reverse`, the frames play backwards, so the art leaves instead of arriving.

use crate::animation::random;
use crate::{Animation, Frame, MAX_FRAMES, MAX_PROJECT_CELLS, Result, TimedFrame};

pub const EFFECTS: &[&str] = &[
    "reveal",
    "typewriter",
    "decrypt",
    "scatter",
    "wipe-left",
    "wipe-right",
    "wipe-up",
    "wipe-down",
    "rain",
    "wave",
    "burn",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectOptions {
    /// Total length, shared between the frames; the frame count is `duration_ms * fps / 1000`,
    /// rounded up, and at least 2.
    pub duration_ms: u32,
    pub fps: u32,
    pub seed: u32,
    /// Play the frames backwards: the art starts whole and leaves.
    pub reverse: bool,
}

impl Default for EffectOptions {
    fn default() -> Self {
        Self {
            duration_ms: 2000,
            fps: 20,
            seed: 42,
            reverse: false,
        }
    }
}

const BURN: &[u8] = b"@%#*+=:-.";

pub fn apply(effect: &str, source: &Frame, options: &EffectOptions) -> Result<Animation> {
    if !EFFECTS.contains(&effect) {
        return Err(format!(
            "Unknown effect {effect}; choose {}",
            EFFECTS.join(", ")
        ));
    }
    if !(1..=60).contains(&options.fps) {
        return Err("FPS must be 1..=60".into());
    }
    if !(100..=60_000).contains(&options.duration_ms) {
        return Err("Effect duration must be 100..=60000 milliseconds".into());
    }
    let count = (u64::from(options.duration_ms) * u64::from(options.fps))
        .div_ceil(1000)
        .max(2) as usize;
    if count > MAX_FRAMES || source.cells().len() * count > MAX_PROJECT_CELLS {
        return Err(format!(
            "The effect needs {count} frames; projects allow {MAX_FRAMES} frames and \
             8,000,000 cells. Shorten the duration, lower the FPS or use smaller art."
        ));
    }
    let width = source.width();
    let height = source.height();
    // Indices of the characters an effect animates, in reading order.
    let marks: Vec<usize> = (0..source.cells().len())
        .filter(|&i| source.cells()[i] != b' ')
        .collect();
    let seed = options.seed;
    let mut frames = Vec::with_capacity(count);
    for index in 0..count {
        let p = index as f64 / (count - 1) as f64;
        let mut cells = vec![b' '; width * height];
        let mut put = |x: i64, y: i64, value: u8| {
            if (0..width as i64).contains(&x) && (0..height as i64).contains(&y) {
                cells[y as usize * width + x as usize] = value;
            }
        };
        for (order, &cell) in marks.iter().enumerate() {
            let value = source.cells()[cell];
            let (x, y) = ((cell % width) as i64, (cell / width) as i64);
            let chance = |salt: u32| random(seed ^ salt, cell as u32);
            match effect {
                "reveal" => {
                    if chance(0x5245) <= p {
                        put(x, y, value);
                    }
                }
                "typewriter" => {
                    let shown = (p * marks.len() as f64).ceil() as usize;
                    if order < shown {
                        put(x, y, value);
                    } else if order == shown {
                        put(x, y, b'_');
                    }
                }
                "decrypt" => {
                    if p >= 0.3 + 0.7 * chance(0x4445) {
                        put(x, y, value);
                    } else {
                        let noise = random(seed.wrapping_add(index as u32 * 7919), cell as u32);
                        put(x, y, 33 + (noise * 93.0).round() as u8);
                    }
                }
                "scatter" => {
                    let eased = 1.0 - (1.0 - p).powi(3);
                    let sx = (chance(0x5358) * (width - 1) as f64).round();
                    let sy = (chance(0x5359) * (height - 1) as f64).round();
                    put(
                        (sx + (x as f64 - sx) * eased).round() as i64,
                        (sy + (y as f64 - sy) * eased).round() as i64,
                        value,
                    );
                }
                "wipe-left" | "wipe-right" | "wipe-up" | "wipe-down" => {
                    let (position, span) = match effect {
                        "wipe-right" => (x, width),
                        "wipe-left" => (width as i64 - 1 - x, width),
                        "wipe-down" => (y, height),
                        _ => (height as i64 - 1 - y, height),
                    };
                    if (position as f64) < p * span as f64 {
                        put(x, y, value);
                    }
                }
                "rain" => {
                    // Each character starts above the top row and falls under gravity.
                    let fall = ((p - 0.6 * chance(0x5241)) / 0.35).clamp(0.0, 1.0);
                    let drop = ((1.0 - fall * fall) * (y + 1) as f64).round() as i64;
                    put(x, y - drop, value);
                }
                "wave" => {
                    let swell = (std::f64::consts::PI * p).sin();
                    let phase = std::f64::consts::TAU * 2.0 * p + y as f64 * 0.7;
                    let amplitude = (width as f64 / 8.0).clamp(1.0, 3.0);
                    put(
                        x + (amplitude * swell * phase.sin()).round() as i64,
                        y,
                        value,
                    );
                }
                _ => {
                    // burn: rows ignite from the bottom with some randomness, then cool to ash.
                    let rise = (height - 1 - y as usize) as f64 / (height - 1).max(1) as f64;
                    let stage = (p - (0.55 * rise + 0.15 * chance(0x4255))) / 0.28;
                    if stage < 0.0 {
                        put(x, y, value);
                    } else if stage < 1.0 {
                        put(x, y, BURN[(stage * BURN.len() as f64) as usize]);
                    }
                }
            }
        }
        // Spread the remainder over the first frames so the durations sum to duration_ms.
        let share = options.duration_ms / count as u32;
        let extra = u32::from(index < options.duration_ms as usize % count);
        frames.push(TimedFrame {
            frame: Frame::new(width, height, cells)?,
            duration_ms: share + extra,
        });
    }
    if options.reverse {
        frames.reverse();
    }
    Animation::new(frames, false)
}
