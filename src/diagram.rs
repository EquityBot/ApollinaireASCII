// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Box-and-arrow diagrams drawn with `+ - | > < ^ v`.
//!
//! Strokes join where they meet: a horizontal stroke over a vertical one, or the reverse,
//! becomes `+`, so boxes, elbows and crossings stay connected. Arrowheads count as strokes in
//! their direction. Drawing is clipped to the grid.
//!
//! ```
//! use apollinaire_ascii::Canvas;
//! let mut chart = Canvas::new(21, 3)?;
//! chart.diagram_box((0, 0), (6, 2), "CLI")?;
//! chart.diagram_box((14, 0), (20, 2), "LIB")?;
//! chart.connector((6, 1), (13, 1), true)?;
//! assert_eq!(
//!     chart.finish().text(),
//!     "+-----+       +-----+\n| CLI +------>| LIB |\n+-----+       +-----+"
//! );
//! # Ok::<(), String>(())
//! ```

use crate::{Canvas, Result, canvas::bounds, printable};

fn join(existing: u8, stroke: u8) -> u8 {
    match (existing, stroke) {
        (b'|' | b'+' | b'^' | b'v', b'-') | (b'-' | b'+' | b'<' | b'>', b'|') => b'+',
        _ => stroke,
    }
}
fn coordinates(points: [(i32, i32); 2]) -> Result<()> {
    if points
        .iter()
        .flat_map(|(x, y)| [x, y])
        .any(|v| !(-4096..=4096).contains(v))
    {
        return Err("Diagram coordinates must be within -4096..=4096".into());
    }
    Ok(())
}

impl Canvas {
    fn stroke(&mut self, x: i32, y: i32, value: u8) -> Result<()> {
        match self.get(x, y) {
            Some(existing) => self.set(x, y, join(existing, value)),
            None => Ok(()),
        }
    }
    /// Draw a straight horizontal or vertical run of `-` or `|`, joining crossings.
    fn run(&mut self, a: (i32, i32), b: (i32, i32)) -> Result<()> {
        if a.1 == b.1 {
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                self.stroke(x, a.1, b'-')?;
            }
        } else {
            for y in a.1.min(b.1)..=a.1.max(b.1) {
                self.stroke(a.0, y, b'|')?;
            }
        }
        Ok(())
    }
    /// Draw a box with `+` corners between two opposite corner cells, with `label` centred
    /// on its middle interior row. A label wider than the interior is clipped.
    pub fn diagram_box(&mut self, a: (i32, i32), b: (i32, i32), label: &str) -> Result<()> {
        coordinates([a, b])?;
        printable(label.as_bytes())?;
        let (x, y, width, height) = bounds(a, b);
        let (right, bottom) = (x + width as i32 - 1, y + height as i32 - 1);
        for (from, to) in [
            ((x, y), (right, y)),
            ((x, bottom), (right, bottom)),
            ((x, y), (x, bottom)),
            ((right, y), (right, bottom)),
        ] {
            self.run(from, to)?;
        }
        for (cx, cy) in [(x, y), (right, y), (x, bottom), (right, bottom)] {
            self.set(cx, cy, b'+')?;
        }
        if width > 2 && height > 2 {
            let label = &label[..label.len().min(width - 2)];
            let column = x + 1 + ((width - 2 - label.len()) / 2) as i32;
            self.text(column, y + (height as i32 - 1) / 2, label)?;
        }
        Ok(())
    }
    /// Draw an orthogonal connector from `start` to `end`: along the longer axis first, then
    /// one `+` elbow. A single cell is `+`. With `arrow`, the end cell becomes `>`, `<`, `^` or
    /// `v` in the final direction of travel; an end on an existing `-`, `|` or `+` keeps that
    /// cell and the arrowhead stops just before it.
    pub fn connector(&mut self, start: (i32, i32), end: (i32, i32), arrow: bool) -> Result<()> {
        coordinates([start, end])?;
        let landing = self.get(end.0, end.1);
        let elbow = if start.0.abs_diff(end.0) >= start.1.abs_diff(end.1) {
            (end.0, start.1)
        } else {
            (start.0, end.1)
        };
        if start == end {
            return self.stroke(start.0, start.1, b'+');
        }
        self.run(start, elbow)?;
        if elbow != end {
            self.run(elbow, end)?;
        }
        if arrow {
            let from = if elbow == end { start } else { elbow };
            let step = ((end.0 - from.0).signum(), (end.1 - from.1).signum());
            let head = match step {
                (1, _) => b'>',
                (-1, _) => b'<',
                (_, 1) => b'v',
                _ => b'^',
            };
            let mut tip = end;
            if let Some(cell @ (b'-' | b'|' | b'+')) = landing {
                self.set(end.0, end.1, cell)?;
                // Back off past any junctions; with no plain cell left, draw no arrowhead.
                let plain = if step.0 == 0 { b'|' } else { b'-' };
                tip = (end.0 - step.0, end.1 - step.1);
                while self.get(tip.0, tip.1) != Some(plain) {
                    if tip == from {
                        return Ok(());
                    }
                    tip = (tip.0 - step.0, tip.1 - step.1);
                }
            }
            self.set(tip.0, tip.1, head)?;
        }
        Ok(())
    }
}
