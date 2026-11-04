//! Image-to-character mapping adapted from RASCII, MIT, KoBruh and contributors.
//! The resize, weighted luminance and ramp-index approach is retained.
//! ASCII validation, alpha compositing, bounds, dithering and GIF import are new.
use crate::{
    Animation, Frame, MAX_FRAMES, MAX_INPUT_BYTES, MAX_PROJECT_CELLS, Result, TimedFrame,
    dimensions, printable,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use std::f64::consts::{FRAC_PI_4, PI};
use std::io::Cursor;

pub const RAMPS: &[(&str, &str)] = &[
    ("standard", " .:-=+*#%@"),
    (
        "fine",
        " .'^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    ),
    ("simple", " .oO#"),
    ("binary", " #"),
];
/// Edge strength, as a share of a full black-to-white step, that earns a direction character.
const STRONG_EDGE: f64 = 0.25;
#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub width: usize,
    pub height: Option<usize>,
    pub ramp: String,
    pub invert: bool,
    pub cell_aspect: f64,
    pub gamma: f64,
    pub contrast: f64,
    pub background: u8,
    pub dither: bool,
    pub edges: bool,
    /// Trace edges as `edges` does, drawing strong ones with `|`, `/`, `-` or `\` along their
    /// direction instead of a ramp character.
    pub edge_glyphs: bool,
}
impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 80,
            height: None,
            ramp: RAMPS[0].1.into(),
            invert: false,
            cell_aspect: 0.5,
            gamma: 1.0,
            contrast: 1.0,
            background: 0,
            dither: false,
            edges: false,
            edge_glyphs: false,
        }
    }
}
impl RenderOptions {
    pub fn validate(&self) -> Result<()> {
        dimensions(self.width, self.height.unwrap_or(1))?;
        printable(self.ramp.as_bytes())?;
        if !(2..=256).contains(&self.ramp.len()) {
            return Err("Ramp needs 2..=256 printable ASCII characters".into());
        }
        if !self.cell_aspect.is_finite() || !(0.1..=2.0).contains(&self.cell_aspect) {
            return Err("Cell aspect must be a finite number in 0.1..=2.0".into());
        }
        if !self.gamma.is_finite()
            || !(0.1..=5.0).contains(&self.gamma)
            || !self.contrast.is_finite()
            || !(0.1..=5.0).contains(&self.contrast)
        {
            return Err("Gamma and contrast must be finite numbers in 0.1..=5.0".into());
        }
        Ok(())
    }
    /// Output rows for a source of this pixel size, from the fixed height or the aspect ratio.
    pub fn output_height(&self, source_width: u32, source_height: u32) -> usize {
        self.height.unwrap_or_else(|| {
            ((self.width as f64 * f64::from(source_height) / f64::from(source_width)
                * self.cell_aspect)
                .ceil() as usize)
                .max(1)
        })
    }
}
pub fn render_image(resource: &DynamicImage, options: &RenderOptions) -> Result<Frame> {
    options.validate()?;
    let (source_w, source_h) = (resource.width(), resource.height());
    check_pixels(source_w, source_h)?;
    let height = options.output_height(source_w, source_h);
    let count = dimensions(options.width, height)?;
    let resized = resource
        .resize_exact(
            options.width as u32,
            height as u32,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8();
    let mut values: Vec<f64> = resized
        .pixels()
        .map(|p| {
            let alpha = p[3] as f64 / 255.0;
            let gray = (p[0] as f64 * 0.299 + p[1] as f64 * 0.587 + p[2] as f64 * 0.114) / 255.0;
            gray * alpha + options.background as f64 / 255.0 * (1.0 - alpha)
        })
        .collect();
    let mut glyphs = vec![None; count];
    if options.edges || options.edge_glyphs {
        let original = values.clone();
        for y in 0..height {
            for x in 0..options.width {
                let sample = |dx: isize, dy: isize| -> f64 {
                    let px = (x as isize + dx).clamp(0, options.width as isize - 1) as usize;
                    let py = (y as isize + dy).clamp(0, height as isize - 1) as usize;
                    original[py * options.width + px]
                };
                let gx = -sample(-1, -1) + sample(1, -1) - 2.0 * sample(-1, 0) + 2.0 * sample(1, 0)
                    - sample(-1, 1)
                    + sample(1, 1);
                let gy = -sample(-1, -1) - 2.0 * sample(0, -1) - sample(1, -1)
                    + sample(-1, 1)
                    + 2.0 * sample(0, 1)
                    + sample(1, 1);
                let strength = (gx.hypot(gy) / 4.0).min(1.0);
                values[y * options.width + x] = strength;
                if options.edge_glyphs && strength >= STRONG_EDGE {
                    glyphs[y * options.width + x] = Some(edge_glyph(gx, gy));
                }
            }
        }
    }
    for value in &mut values {
        *value = ((*value - 0.5) * options.contrast + 0.5)
            .clamp(0.0, 1.0)
            .powf(1.0 / options.gamma);
        if options.invert {
            *value = 1.0 - *value;
        }
    }
    let ramp = options.ramp.as_bytes();
    let mut cells = Vec::with_capacity(count);
    for index in 0..count {
        let value = values[index].clamp(0.0, 1.0);
        let selected = (value * (ramp.len() - 1) as f64).round() as usize;
        cells.push(glyphs[index].unwrap_or(ramp[selected]));
        if options.dither {
            let error = value - selected as f64 / (ramp.len() - 1) as f64;
            let x = index % options.width;
            let y = index / options.width;
            if x + 1 < options.width {
                values[index + 1] += error * 7.0 / 16.0;
            }
            if y + 1 < height {
                if x > 0 {
                    values[index + options.width - 1] += error * 3.0 / 16.0;
                }
                values[index + options.width] += error * 5.0 / 16.0;
                if x + 1 < options.width {
                    values[index + options.width + 1] += error / 16.0;
                }
            }
        }
    }
    Frame::new(options.width, height, cells)
}
/// The character running along an edge, which lies across its brightness gradient.
/// Angles stay in cells rather than pixels because `/` and `\` are the diagonals of a cell.
fn edge_glyph(gx: f64, gy: f64) -> u8 {
    // A gradient and its opposite mark the same edge, so half a turn covers every direction.
    let eighths = (gy.atan2(gx).rem_euclid(PI) / FRAC_PI_4).round() as usize;
    b"|/-\\"[eighths % 4]
}
pub(crate) fn check_pixels(width: u32, height: u32) -> Result<()> {
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || u64::from(width) * u64::from(height) > 16_777_216
    {
        return Err("Decoded image must be nonempty and at most 4096 x 4096 pixels".into());
    }
    Ok(())
}
fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits
}
/// GIFs become timed text frames. Zero GIF delays use 100 ms.
pub fn decode_media(bytes: &[u8], options: &RenderOptions) -> Result<Animation> {
    options.validate()?;
    if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
        return Err("Media must be 1 byte to 16 MiB".into());
    }
    let format = image::guess_format(bytes).map_err(|e| format!("Unrecognised image: {e}"))?;
    if format == ImageFormat::Gif {
        let (width, height, count) = inspect_gif(bytes)?;
        check_pixels(width, height)?;
        let output_h = options.output_height(width, height);
        let output_cells = dimensions(options.width, output_h)?;
        if count == 0
            || count > MAX_FRAMES
            || output_cells
                .checked_mul(count)
                .is_none_or(|n| n > MAX_PROJECT_CELLS)
            || u64::from(width) * u64::from(height) * count as u64 > 64_000_000
        {
            return Err("GIF exceeds frame, output cell or 64 million decoded pixel budget".into());
        }
        let mut frames = Vec::with_capacity(count);
        crate::gif::for_each_frame(bytes, |screen, delay_cs| {
            if frames.len() >= count {
                return Err("GIF frame count changed during decoding".into());
            }
            let delay = if delay_cs == 0 {
                100
            } else {
                u32::from(delay_cs) * 10
            };
            if !(10..=60_000).contains(&delay) {
                return Err("GIF frame delay is outside 10..=60000 ms".into());
            }
            let frame = render_image(&DynamicImage::ImageRgba8(screen), options)?;
            frames.push(TimedFrame {
                frame,
                duration_ms: delay,
            });
            Ok(())
        })?;
        if frames.len() != count {
            return Err("GIF frame count does not match its container".into());
        }
        Animation::new(frames, true)
    } else {
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        reader.limits(limits());
        let decoder = reader
            .into_decoder()
            .map_err(|e| format!("Image header: {e}"))?;
        let (w, h) = decoder.dimensions();
        check_pixels(w, h)?;
        let image =
            DynamicImage::from_decoder(decoder).map_err(|e| format!("Image decode: {e}"))?;
        Ok(Animation::still(render_image(&image, options)?))
    }
}
/// Count GIF image descriptors before allocating decoded frames.
/// Compressed data is skipped by bounded sub-block lengths.
fn inspect_gif(bytes: &[u8]) -> Result<(u32, u32, usize)> {
    if bytes.len() < 13 || (&bytes[..6] != b"GIF89a" && &bytes[..6] != b"GIF87a") {
        return Err("Invalid GIF header".into());
    }
    let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
    check_pixels(width, height)?;
    let mut cursor = 13;
    if bytes[10] & 0x80 != 0 {
        advance(
            bytes,
            &mut cursor,
            3usize << (usize::from(bytes[10] & 7) + 1),
        )?;
    }
    let mut frames = 0;
    loop {
        let marker = *bytes.get(cursor).ok_or("GIF trailer is missing")?;
        cursor += 1;
        match marker {
            0x3b => return Ok((width, height, frames)),
            0x21 => {
                advance(bytes, &mut cursor, 1)?;
                sub_blocks(bytes, &mut cursor)?;
            }
            0x2c => {
                let descriptor = bytes
                    .get(cursor..cursor + 9)
                    .ok_or("Truncated GIF image descriptor")?;
                let left = u32::from(u16::from_le_bytes([descriptor[0], descriptor[1]]));
                let top = u32::from(u16::from_le_bytes([descriptor[2], descriptor[3]]));
                let frame_width = u32::from(u16::from_le_bytes([descriptor[4], descriptor[5]]));
                let frame_height = u32::from(u16::from_le_bytes([descriptor[6], descriptor[7]]));
                if frame_width == 0
                    || frame_height == 0
                    || left + frame_width > width
                    || top + frame_height > height
                {
                    return Err("GIF frame rectangle must fit the logical screen".into());
                }
                let flags = descriptor[8];
                advance(bytes, &mut cursor, 9)?;
                if flags & 0x80 != 0 {
                    advance(bytes, &mut cursor, 3usize << (usize::from(flags & 7) + 1))?;
                }
                advance(bytes, &mut cursor, 1)?;
                sub_blocks(bytes, &mut cursor)?;
                frames += 1;
                if frames > MAX_FRAMES {
                    return Err("Too many GIF frames".into());
                }
            }
            _ => return Err("Invalid GIF block marker".into()),
        }
    }
}
fn advance(bytes: &[u8], cursor: &mut usize, count: usize) -> Result<()> {
    *cursor = cursor
        .checked_add(count)
        .filter(|n| *n <= bytes.len())
        .ok_or("Truncated GIF block")?;
    Ok(())
}
fn sub_blocks(bytes: &[u8], cursor: &mut usize) -> Result<()> {
    loop {
        let size = *bytes.get(*cursor).ok_or("Truncated GIF sub-block")? as usize;
        *cursor += 1;
        if size == 0 {
            return Ok(());
        }
        advance(bytes, cursor, size)?;
    }
}
