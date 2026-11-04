use crate::{MAX_HEIGHT, MAX_INPUT_BYTES, Result, dimensions, printable};

/// A rectangular grid containing only bytes 32 through 126.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl Frame {
    pub fn new(width: usize, height: usize, cells: Vec<u8>) -> Result<Self> {
        let count = dimensions(width, height)?;
        if cells.len() != count {
            return Err(format!("Expected {count} cells, got {}", cells.len()));
        }
        printable(&cells)?;
        Ok(Self {
            width,
            height,
            cells,
        })
    }
    pub fn blank(width: usize, height: usize) -> Result<Self> {
        Self::new(width, height, vec![b' '; dimensions(width, height)?])
    }
    /// Parse equal-width LF-separated rows. A final LF is permitted.
    pub fn from_text(text: &str) -> Result<Self> {
        Self::parse_text(text, false)
    }
    /// Explicitly pad shorter rows on the right with spaces.
    pub fn from_text_padded(text: &str) -> Result<Self> {
        Self::parse_text(text, true)
    }
    fn parse_text(text: &str, pad: bool) -> Result<Self> {
        if text.len() > MAX_INPUT_BYTES {
            return Err("Text exceeds the 16 MiB input limit".into());
        }
        let text = text.strip_suffix('\n').unwrap_or(text);
        let rows: Vec<&str> = text.split('\n').take(MAX_HEIGHT + 1).collect();
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let count = dimensions(width, rows.len())?;
        let mut cells = Vec::with_capacity(count);
        for row in &rows {
            printable(row.as_bytes())?;
            if !pad && row.len() != width {
                return Err("Rows have different widths; use explicit right padding".into());
            }
            cells.extend_from_slice(row.as_bytes());
            cells.resize(cells.len() + width - row.len(), b' ');
        }
        Self::new(width, rows.len(), cells)
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }
    pub fn row(&self, index: usize) -> Option<&[u8]> {
        (index < self.height).then(|| &self.cells[index * self.width..(index + 1) * self.width])
    }
    /// Serialise without a final LF, preserving trailing spaces.
    pub fn text(&self) -> String {
        self.cells
            .chunks(self.width)
            .map(|row| std::str::from_utf8(row).expect("Validated ASCII"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn flip_horizontal(&self) -> Self {
        let mut result = self.clone();
        for row in result.cells.chunks_mut(self.width) {
            row.reverse();
        }
        result
    }
    pub fn flip_vertical(&self) -> Self {
        let mut cells = Vec::with_capacity(self.cells.len());
        for row in self.cells.chunks(self.width).rev() {
            cells.extend_from_slice(row);
        }
        Self {
            cells,
            ..self.clone()
        }
    }
    /// Flip columns and reflect `/\`, `()`, `[]`, `{}` and `<>` pairs.
    /// Other characters, including letters, keep their original glyphs.
    pub fn mirror_horizontal(&self) -> Self {
        let mut frame = self.flip_horizontal();
        for cell in &mut frame.cells {
            *cell = match *cell {
                b'/' => b'\\',
                b'\\' => b'/',
                b'(' => b')',
                b')' => b'(',
                b'[' => b']',
                b']' => b'[',
                b'{' => b'}',
                b'}' => b'{',
                b'<' => b'>',
                b'>' => b'<',
                other => other,
            };
        }
        frame
    }
    /// Flip rows and reflect `/\` and `^v` pairs. Other glyphs stay unchanged.
    /// Use `flip_vertical` when `v` is text rather than an arrowhead.
    pub fn mirror_vertical(&self) -> Self {
        let mut frame = self.flip_vertical();
        for cell in &mut frame.cells {
            *cell = match *cell {
                b'/' => b'\\',
                b'\\' => b'/',
                b'^' => b'v',
                b'v' => b'^',
                other => other,
            };
        }
        frame
    }
    pub fn crop(&self, x: usize, y: usize, width: usize, height: usize) -> Result<Self> {
        dimensions(width, height)?;
        if x.checked_add(width).is_none_or(|v| v > self.width)
            || y.checked_add(height).is_none_or(|v| v > self.height)
        {
            return Err("Crop extends outside the frame".into());
        }
        let cells = (y..y + height)
            .flat_map(|r| {
                self.cells[r * self.width + x..r * self.width + x + width]
                    .iter()
                    .copied()
            })
            .collect();
        Self::new(width, height, cells)
    }
    pub fn border(&self) -> Result<Self> {
        dimensions(self.width + 2, self.height + 2)?;
        let mut rows = vec![format!("+{}+", "-".repeat(self.width))];
        rows.extend(self.text().split('\n').map(|r| format!("|{r}|")));
        rows.push(format!("+{}+", "-".repeat(self.width)));
        Self::from_text(&rows.join("\n"))
    }
}
