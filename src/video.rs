// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Sampling and timing for video and camera frames.
//! The browser decodes media and supplies RGBA pixels; this module never sees a decoder or device.
use crate::{
    Frame, MAX_FRAMES, MAX_PROJECT_CELLS, RenderOptions, Result, dimensions,
    image_renderer::check_pixels, render_image,
};
use image::{DynamicImage, RgbaImage};

/// The shortest frame duration a project accepts, in milliseconds.
const MIN_FRAME_MS: f64 = 10.0;
pub const MAX_FPS: u32 = 60;

/// Sample times in milliseconds from `start_ms` up to `end_ms` at `fps`.
/// A final sample that would last under 10 ms is omitted.
pub fn sample_times(duration_ms: f64, start_ms: f64, end_ms: f64, fps: u32) -> Result<Vec<f64>> {
    if !(1..=MAX_FPS).contains(&fps) {
        return Err(format!("Frames per second must be 1..={MAX_FPS}"));
    }
    if !duration_ms.is_finite() || duration_ms <= 0.0 {
        return Err("The video does not report a finite duration".into());
    }
    if !start_ms.is_finite()
        || !end_ms.is_finite()
        || start_ms < 0.0
        || end_ms > duration_ms
        || end_ms.round() - start_ms.round() < MIN_FRAME_MS
    {
        return Err(format!(
            "Start and end times must lie within the video's {:.2} seconds, with start before end",
            duration_ms / 1000.0
        ));
    }
    let rate = f64::from(fps);
    // One candidate beyond the limit shows whether the clip needs too many frames.
    let times: Vec<f64> = (0..=MAX_FRAMES)
        .map(|i| start_ms + i as f64 * 1000.0 / rate)
        .take_while(|t| end_ms.round() - t.round() >= MIN_FRAME_MS)
        .collect();
    if times.len() > MAX_FRAMES {
        let needed = ((end_ms - start_ms) * rate / 1000.0).ceil();
        return Err(format!(
            "The clip needs about {needed} frames; the limit is {MAX_FRAMES}. Shorten it or lower the frame rate."
        ));
    }
    Ok(times)
}

/// Whether a camera frame captured at `now_ms` keeps every duration at 10 ms or more.
pub fn capture_due(previous_ms: Option<f64>, now_ms: f64, end_ms: f64) -> bool {
    end_ms.round() - now_ms.round() >= MIN_FRAME_MS
        && previous_ms.is_none_or(|p| now_ms.round() - p.round() >= MIN_FRAME_MS)
}

/// Frame durations from capture times; they sum to the rounded span from the first time to `end_ms`.
pub fn frame_durations(times_ms: &[f64], end_ms: f64) -> Result<Vec<u32>> {
    if times_ms.is_empty() {
        return Err("No frames were captured".into());
    }
    times_ms
        .iter()
        .enumerate()
        .map(|(i, time)| {
            let next = times_ms.get(i + 1).copied().unwrap_or(end_ms);
            let duration = next.round() - time.round();
            if !duration.is_finite() || !(MIN_FRAME_MS..=60_000.0).contains(&duration) {
                return Err("Frame times must be 10..=60000 ms apart".into());
            }
            Ok(duration as u32)
        })
        .collect()
}

/// Pixel size for reading one frame: the source aspect at no more than two pixels per column.
pub fn sample_size(source_width: u32, source_height: u32, columns: usize) -> Result<(u32, u32)> {
    check_pixels(source_width, source_height)?;
    dimensions(columns, 1)?;
    let width = source_width.min(columns as u32 * 2);
    let height = (u64::from(source_height) * u64::from(width) + u64::from(source_width) / 2)
        / u64::from(source_width);
    Ok((width, (height as u32).max(1)))
}

/// Refuse a capture before decoding when its frames would exceed the project budgets.
pub fn check_capture(columns: usize, rows: usize, frames: usize) -> Result<()> {
    let cells = dimensions(columns, rows)?;
    if frames == 0 || frames > MAX_FRAMES {
        return Err(format!(
            "A capture needs 1..={MAX_FRAMES} frames; this one needs {frames}"
        ));
    }
    if cells * frames > MAX_PROJECT_CELLS {
        return Err(format!(
            "{frames} frames of {columns} x {rows} exceed the 8,000,000 cell project limit. Use fewer columns, a lower frame rate or a shorter clip."
        ));
    }
    Ok(())
}

/// Convert one frame of RGBA pixels, as read from a browser canvas, to text.
pub fn render_rgba(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    options: &RenderOptions,
) -> Result<Frame> {
    let image = RgbaImage::from_raw(width, height, rgba)
        .ok_or("Pixel data does not match the frame size")?;
    render_image(&DynamicImage::ImageRgba8(image), options)
}
