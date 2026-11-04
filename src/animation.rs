use crate::{Canvas, Frame, MAX_FRAMES, MAX_INPUT_BYTES, MAX_PROJECT_CELLS, Result, dimensions};
use serde_json::json;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedFrame {
    pub frame: Frame,
    pub duration_ms: u32,
}

/// Frames share dimensions. The project owns timing, not a renderer or clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Animation {
    frames: Vec<TimedFrame>,
    looping: bool,
}

impl Animation {
    pub fn new(frames: Vec<TimedFrame>, looping: bool) -> Result<Self> {
        if frames.is_empty() || frames.len() > MAX_FRAMES {
            return Err(format!("A project needs 1..={MAX_FRAMES} frames"));
        }
        let first = &frames[0].frame;
        if frames
            .iter()
            .any(|f| f.frame.width() != first.width() || f.frame.height() != first.height())
        {
            return Err("Every frame must have the same dimensions".into());
        }
        if first
            .cells()
            .len()
            .checked_mul(frames.len())
            .is_none_or(|n| n > MAX_PROJECT_CELLS)
        {
            return Err("Project exceeds 8,000,000 cells".into());
        }
        let mut total = 0u64;
        for frame in &frames {
            if !(10..=60_000).contains(&frame.duration_ms) {
                return Err("Frame duration must be 10..=60000 milliseconds".into());
            }
            total += u64::from(frame.duration_ms);
        }
        if total > 3_600_000 {
            return Err("Project duration exceeds one hour".into());
        }
        Ok(Self { frames, looping })
    }
    pub fn still(frame: Frame) -> Self {
        Self {
            frames: vec![TimedFrame {
                frame,
                duration_ms: 100,
            }],
            looping: false,
        }
    }
    pub fn frames(&self) -> &[TimedFrame] {
        &self.frames
    }
    pub fn width(&self) -> usize {
        self.frames[0].frame.width()
    }
    pub fn height(&self) -> usize {
        self.frames[0].frame.height()
    }
    pub fn looping(&self) -> bool {
        self.looping
    }
    pub fn duration_ms(&self) -> u64 {
        self.frames.iter().map(|f| u64::from(f.duration_ms)).sum()
    }
    pub fn frame_at(&self, elapsed_ms: u64) -> usize {
        let total = self.duration_ms();
        let mut remaining = if self.looping {
            elapsed_ms % total
        } else {
            elapsed_ms.min(total - 1)
        };
        for (i, frame) in self.frames.iter().enumerate() {
            if remaining < u64::from(frame.duration_ms) {
                return i;
            }
            remaining -= u64::from(frame.duration_ms);
        }
        self.frames.len() - 1
    }
    pub fn replace_frame(&mut self, index: usize, frame: TimedFrame) -> Result<()> {
        if index >= self.frames.len() {
            return Err("Frame index is outside the project".into());
        }
        let mut frames = self.frames.clone();
        frames[index] = frame;
        *self = Self::new(frames, self.looping)?;
        Ok(())
    }
    pub fn insert_frame(&mut self, index: usize, frame: TimedFrame) -> Result<()> {
        if index > self.frames.len() {
            return Err("Frame index is outside the project".into());
        }
        let mut frames = self.frames.clone();
        frames.insert(index, frame);
        *self = Self::new(frames, self.looping)?;
        Ok(())
    }
    pub fn remove_frame(&mut self, index: usize) -> Result<()> {
        if self.frames.len() == 1 {
            return Err("Keep at least one frame".into());
        }
        if index >= self.frames.len() {
            return Err("Frame index is outside the project".into());
        }
        self.frames.remove(index);
        Ok(())
    }
    pub fn swap_frames(&mut self, a: usize, b: usize) -> Result<()> {
        if a >= self.frames.len() || b >= self.frames.len() {
            return Err("Frame index is outside the project".into());
        }
        self.frames.swap(a, b);
        Ok(())
    }
    fn check_range(&self, range: &Range<usize>) -> Result<()> {
        if range.start >= range.end || range.end > self.frames.len() {
            return Err("Choose a non-empty frame range inside the project".into());
        }
        Ok(())
    }
    /// Insert ordered copies immediately after a non-empty, end-exclusive range.
    /// Validation failures leave the project unchanged.
    pub fn duplicate_frames(&mut self, range: Range<usize>) -> Result<()> {
        self.check_range(&range)?;
        let mut frames = self.frames.clone();
        frames.splice(range.end..range.end, self.frames[range].iter().cloned());
        *self = Self::new(frames, self.looping)?;
        Ok(())
    }
    /// Remove a non-empty, end-exclusive range, keeping at least one frame.
    pub fn remove_frames(&mut self, range: Range<usize>) -> Result<()> {
        self.check_range(&range)?;
        if range.len() == self.frames.len() {
            return Err("Keep at least one frame".into());
        }
        self.frames.drain(range);
        Ok(())
    }
    /// Move a non-empty, end-exclusive range to `destination` in the resulting
    /// project. Both the moved range and the other frames retain their order.
    pub fn move_frames(&mut self, range: Range<usize>, destination: usize) -> Result<()> {
        self.check_range(&range)?;
        let count = range.len();
        if destination > self.frames.len() - count {
            return Err("Frame destination is outside the project".into());
        }
        if destination < range.start {
            self.frames[destination..range.end].rotate_right(count);
        } else if destination > range.start {
            self.frames[range.start..destination + count].rotate_left(count);
        }
        Ok(())
    }
    /// Set the duration of every frame in a non-empty, end-exclusive range.
    /// Per-frame and total-duration limits are checked before any change.
    pub fn set_frame_durations(&mut self, range: Range<usize>, duration_ms: u32) -> Result<()> {
        self.check_range(&range)?;
        let mut frames = self.frames.clone();
        for entry in &mut frames[range] {
            entry.duration_ms = duration_ms;
        }
        *self = Self::new(frames, self.looping)?;
        Ok(())
    }
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }
    pub fn to_json(&self) -> String {
        json!({"format":"asciiforge","version":1,"width":self.width(),"height":self.height(),
            "loop":self.looping,"frames":self.frames.iter().map(|f|
                json!({"text":f.frame.text(),"duration_ms":f.duration_ms})).collect::<Vec<_>>()
        })
        .to_string()
    }
    pub fn from_json(input: &str) -> Result<Self> {
        if input.len() > MAX_INPUT_BYTES {
            return Err("Project exceeds the 16 MiB input limit".into());
        }
        crate::project_json::parse(input)
    }
}

pub const SCENES: &[&str] = &[
    "ghost",
    "donut",
    "wave",
    "starfield",
    "rain",
    "fire",
    "reveal",
    "scroll",
];

#[derive(Clone, Debug)]
pub struct SceneOptions {
    pub width: usize,
    pub height: usize,
    pub frames: usize,
    pub fps: u32,
    pub seed: u32,
    pub text: String,
}
impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            width: 72,
            height: 24,
            frames: 80,
            fps: 20,
            seed: 42,
            text: "HELLO, FRIEND".into(),
        }
    }
}

pub fn generate(scene: &str, options: &SceneOptions) -> Result<Animation> {
    let cells = dimensions(options.width, options.height)?;
    if !(1..=60).contains(&options.fps) {
        return Err("FPS must be 1..=60".into());
    }
    if options.frames == 0
        || options.frames > MAX_FRAMES
        || cells
            .checked_mul(options.frames)
            .is_none_or(|n| n > MAX_PROJECT_CELLS)
    {
        return Err("Animation exceeds frame or cell limits".into());
    }
    if !SCENES.contains(&scene) {
        return Err(format!(
            "Unknown scene {scene}; choose {}",
            SCENES.join(", ")
        ));
    }
    crate::printable(options.text.as_bytes())?;
    if options.text.len() > 512 {
        return Err("Scene text must be at most 512 characters".into());
    }
    let mut frames = Vec::with_capacity(options.frames);
    for index in 0..options.frames {
        let t = index as f64 / options.fps as f64;
        let phase = index as f64 / options.frames as f64 * std::f64::consts::TAU;
        let mut canvas = Canvas::new(options.width, options.height)?;
        match scene {
            "ghost" => ghost(&mut canvas, options, phase)?,
            "donut" => donut(&mut canvas, options, t)?,
            "wave" | "fire" => field(&mut canvas, options, phase, scene == "fire")?,
            "starfield" => stars(&mut canvas, options, t)?,
            "rain" => rain(&mut canvas, options, index)?,
            "reveal" | "scroll" => moving_text(&mut canvas, options, index, scene == "scroll")?,
            _ => unreachable!("Scene was validated"),
        }
        frames.push(TimedFrame {
            frame: canvas.finish(),
            duration_ms: 1000 / options.fps,
        });
    }
    Animation::new(frames, scene != "reveal")
}

fn ghost(c: &mut Canvas, o: &SceneOptions, p: f64) -> Result<()> {
    let scale = (o.height as f64 / 14.0)
        .min(o.width as f64 / 30.0)
        .max(0.25);
    let cx = o.width as f64 / 2.0 + p.sin() * 2.0 * scale;
    let cy = o.height as f64 / 2.0 + p.cos() * 0.8 * scale;
    for y in 0..o.height {
        for x in 0..o.width {
            let u = (x as f64 - cx) / (9.0 * scale);
            let v = (y as f64 - cy) / (5.0 * scale);
            let head = u * u + (v + 0.25).powi(2) < 1.0 && v < 0.0;
            let skirt = u.abs() < 1.0 && v >= 0.0 && v < 0.8 + 0.16 * (u * 8.0 + p).sin();
            if head || skirt {
                let eye = (u.abs() - 0.38).abs() < 0.14 && (v + 0.23).abs() < 0.19;
                let mouth = u.abs() < 0.16 && (v - 0.32).abs() < 0.08;
                let ch = if eye || mouth {
                    b' '
                } else if u.abs() > 0.8 {
                    b':'
                } else {
                    b'#'
                };
                c.set(x as i32, y as i32, ch)?;
            }
        }
    }
    Ok(())
}

fn donut(c: &mut Canvas, o: &SceneOptions, t: f64) -> Result<()> {
    let mut depth = vec![f64::NEG_INFINITY; o.width * o.height];
    let ramp = b".,-~:;=!*#$@";
    let a = t * 0.8;
    let b = t * 0.45;
    let scale = (o.width as f64 / 5.2).min(o.height as f64 / 2.6);
    for ring in 0..96 {
        for tube in 0..36 {
            let u = ring as f64 / 96.0 * std::f64::consts::TAU;
            let v = tube as f64 / 36.0 * std::f64::consts::TAU;
            let radius = 1.0 + 0.4 * v.cos();
            let x = radius * u.cos();
            let y = radius * u.sin();
            let z = 0.4 * v.sin();
            let ry = y * a.cos() - z * a.sin();
            let rz = y * a.sin() + z * a.cos();
            let rx = x * b.cos() - ry * b.sin();
            let ry = x * b.sin() + ry * b.cos();
            let distance = 4.0 + rz;
            let px = (o.width as f64 / 2.0 + scale * rx * 3.0 / distance).round() as i32;
            let py = (o.height as f64 / 2.0 - scale * ry * 1.5 / distance).round() as i32;
            if px < 0 || py < 0 || px as usize >= o.width || py as usize >= o.height {
                continue;
            }
            let cell = py as usize * o.width + px as usize;
            if -distance <= depth[cell] {
                continue;
            }
            depth[cell] = -distance;
            let ny = u.sin() * v.cos() * a.cos() - v.sin() * a.sin();
            let nz = u.sin() * v.cos() * a.sin() + v.sin() * a.cos();
            let nx = u.cos() * v.cos() * b.cos() - ny * b.sin();
            let light = (0.35 + 0.65 * (nx * 0.3 + ny * 0.5 - nz * 0.8)).clamp(0.0, 1.0);
            c.set(px, py, ramp[(light * (ramp.len() - 1) as f64) as usize])?;
        }
    }
    Ok(())
}

fn field(c: &mut Canvas, o: &SceneOptions, p: f64, fire: bool) -> Result<()> {
    let ramp = if fire {
        b" .,:;*#%@".as_slice()
    } else {
        b" .-~+=*#".as_slice()
    };
    for y in 0..o.height {
        for x in 0..o.width {
            let nx = x as f64 / o.width as f64;
            let ny = y as f64 / o.height as f64;
            let value = if fire {
                (ny * 1.5 - 0.45
                    + 0.25 * (nx * 19.0 + p * 2.0).sin()
                    + 0.15 * (nx * 37.0 - ny * 11.0 - p * 3.0).sin())
                .clamp(0.0, 1.0)
            } else {
                let line = 0.5 + 0.22 * (nx * 12.0 + p).sin() + 0.12 * (nx * 23.0 - p * 2.0).sin();
                (1.0 - (ny - line).abs() * o.height as f64 / 3.0).clamp(0.0, 1.0)
            };
            c.set(
                x as i32,
                y as i32,
                ramp[(value * (ramp.len() - 1) as f64) as usize],
            )?;
        }
    }
    Ok(())
}
pub(crate) fn random(seed: u32, index: u32) -> f64 {
    let mut v = seed.wrapping_add(index.wrapping_mul(0x9e3779b9));
    v = (v ^ (v >> 16)).wrapping_mul(0x85ebca6b);
    v = (v ^ (v >> 13)).wrapping_mul(0xc2b2ae35);
    (v ^ (v >> 16)) as f64 / u32::MAX as f64
}
fn stars(c: &mut Canvas, o: &SceneOptions, t: f64) -> Result<()> {
    for i in 0..100 {
        let angle = random(o.seed, i * 3) * std::f64::consts::TAU;
        let distance =
            (random(o.seed, i * 3 + 1) + t * (0.08 + random(o.seed, i * 3 + 2) * 0.15)).fract();
        let x = o.width as f64 / 2.0 + angle.cos() * distance * o.width as f64 * 0.65;
        let y = o.height as f64 / 2.0 + angle.sin() * distance * o.height as f64 * 0.65;
        c.set(
            x as i32,
            y as i32,
            if distance > 0.75 {
                b'+'
            } else if distance > 0.4 {
                b'*'
            } else {
                b'.'
            },
        )?;
    }
    Ok(())
}
fn rain(c: &mut Canvas, o: &SceneOptions, index: usize) -> Result<()> {
    let chars = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for x in (0..o.width).step_by(2) {
        let offset = (random(o.seed, x as u32) * o.height as f64) as usize;
        let head = (offset + index * (1 + x % 3)) % (o.height + 8);
        for tail in 0..8 {
            let y = head as i32 - tail;
            let ch = if tail == 7 {
                b'.'
            } else {
                chars[(x * 7 + index / 3 + tail as usize) % chars.len()]
            };
            c.set(x as i32, y, ch)?;
        }
    }
    Ok(())
}
fn moving_text(c: &mut Canvas, o: &SceneOptions, index: usize, scroll: bool) -> Result<()> {
    let text = if o.text.is_empty() {
        " "
    } else {
        o.text.as_str()
    };
    let x = if scroll {
        o.width as i32 - (index * (o.width + text.len()) / o.frames.max(1)) as i32
    } else {
        (o.width as i32 - text.len() as i32) / 2
    };
    let count = if scroll {
        text.len()
    } else {
        ((index + 1) * text.len())
            .div_ceil(o.frames)
            .min(text.len())
    };
    c.text(x, o.height as i32 / 2, &text[..count])
}
