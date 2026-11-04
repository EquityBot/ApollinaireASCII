//! Drawing tools on the artwork grid. Geometry lives in `Canvas`; this module maps pointer,
//! keyboard and clipboard events to it and keeps the overlays in step.

use super::{
    App, action, checked, copy_text, document, input, listen, node, report, state, status,
};
use crate::{Canvas, Frame, Result, canvas::bounds};
use wasm_bindgen::JsCast;
use web_sys::{
    ClipboardEvent, Element, Event, FocusOptions, HtmlElement, KeyboardEvent, PointerEvent,
};

type Area = (i32, i32, usize, usize);

#[derive(Clone, Copy, Default, PartialEq)]
enum Tool {
    #[default]
    SelectText,
    Pencil,
    Eraser,
    Line,
    Rectangle,
    Ellipse,
    Fill,
    Type,
    Area,
    Stamp,
}
impl Tool {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "select-text" => Self::SelectText,
            "pencil" => Self::Pencil,
            "eraser" => Self::Eraser,
            "line" => Self::Line,
            "rectangle" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "fill" => Self::Fill,
            "type" => Self::Type,
            "area" => Self::Area,
            "stamp-art" => Self::Stamp,
            _ => return Err("Unknown drawing tool".into()),
        })
    }
    fn help(self) -> &'static str {
        match self {
            Self::SelectText => "Select text: drag across the artwork, then copy it.",
            Self::Pencil => {
                "Pencil: drag to draw the character. Keyboard: arrow keys move; Space starts and stops drawing."
            }
            Self::Eraser => {
                "Eraser: drag to clear cells. Keyboard: arrow keys move; Space starts and stops erasing."
            }
            Self::Line => {
                "Line: drag from start to end. Keyboard: Space marks the start, arrow keys aim, Space draws, Escape cancels. Diagram mode draws a connector with one bend."
            }
            Self::Rectangle => {
                "Rectangle: drag between opposite corners. Keyboard: Space, arrow keys, then Space. Diagram mode draws a box labelled with the text field."
            }
            Self::Ellipse => {
                "Ellipse: drag between opposite corners of its box. Keyboard: Space, arrow keys, then Space."
            }
            Self::Fill => {
                "Fill: replace the touching run of one character. Keyboard: arrow keys move; Space fills."
            }
            Self::Type => {
                "Type: choose a cell, then type. Enter starts the next row and Backspace erases. Without a keyboard, use Stamp text."
            }
            Self::Area => {
                "Select area: drag to select, drag inside to move. Keyboard: Space, arrow keys, Space; Ctrl+C, Ctrl+X, Ctrl+V and Delete."
            }
            Self::Stamp => {
                "Stamp art: save an area as a stamp, then choose its top-left cell. Drag to preview, release to place. Keyboard: Space, arrow keys, Space; Escape cancels."
            }
        }
    }
}

struct Stroke {
    base: Frame,
    preview: Frame,
    start: (i32, i32),
    last: (i32, i32),
    /// The pointer drawing this stroke, or `None` for the keyboard.
    pointer: Option<i32>,
    /// Selected contents being moved, with their area before the move.
    lifted: Option<(Frame, Area)>,
}

/// Editor drawing state. A stroke previews against its base frame and becomes one undo step.
#[derive(Default)]
pub(super) struct Drawing {
    tool: Tool,
    cursor: (i32, i32),
    selection: Option<Area>,
    clipboard: Option<Frame>,
    stamps: Vec<Frame>,
    stamp: usize,
    stroke: Option<Stroke>,
    /// Consecutive typed characters share one undo step.
    typing: bool,
    type_column: i32,
}

fn brush() -> Result<u8> {
    match input("brush")?.value().as_bytes() {
        [value] => Ok(*value),
        _ => Err("Enter one printable ASCII character to draw with.".into()),
    }
}
fn label() -> Result<String> {
    Ok(input("stamp")?.value())
}
/// The corner a rectangle or ellipse reaches for a drag. With the constraint on, it is the
/// largest box inside the drag that looks square: cells are about twice as tall as wide, so it
/// spans 2 columns for each row.
fn shape_end(start: (i32, i32), end: (i32, i32)) -> Result<(i32, i32)> {
    if !checked("constrain")? {
        return Ok(end);
    }
    let (columns, rows) = (end.0 - start.0, end.1 - start.1);
    let span = rows.abs().min(columns.abs() / 2);
    Ok((
        start.0 + 2 * span * columns.signum(),
        start.1 + span * rows.signum(),
    ))
}
fn toggle(id: &str, attribute: &str, on: bool) -> Result<()> {
    let element = node::<Element>(id)?;
    if on {
        element.set_attribute(attribute, "")
    } else {
        element.remove_attribute(attribute)
    }
    .map_err(|_| format!("Cannot update {id}"))
}
fn place(id: &str, (x, y, width, height): Area, columns: usize, rows: usize) -> Result<()> {
    node::<Element>(id)?
        .set_attribute(
            "style",
            &format!(
                "left:{}%;top:{}%;width:{}%;height:{}%",
                f64::from(x) * 100.0 / columns as f64,
                f64::from(y) * 100.0 / rows as f64,
                width as f64 * 100.0 / columns as f64,
                height as f64 * 100.0 / rows as f64
            ),
        )
        .map_err(|_| format!("Cannot position {id}"))
}

impl App {
    fn frame(&self) -> &Frame {
        &self.project.frames()[self.selected].frame
    }
    fn duration(&self) -> u32 {
        self.project.frames()[self.selected].duration_ms
    }
    fn fits(&self, (x, y, width, height): Area) -> bool {
        x >= 0
            && y >= 0
            && x as usize + width <= self.project.width()
            && y as usize + height <= self.project.height()
    }
    /// Called whenever the selected frame or project changes.
    pub(super) fn reset_drawing(&mut self) {
        let fits = self.drawing.selection.is_none_or(|area| self.fits(area));
        let (columns, rows) = (self.project.width() as i32, self.project.height() as i32);
        let drawing = &mut self.drawing;
        drawing.stroke = None;
        drawing.typing = false;
        drawing.cursor = (
            drawing.cursor.0.min(columns - 1),
            drawing.cursor.1.min(rows - 1),
        );
        drawing.type_column = drawing.type_column.min(columns - 1);
        if !fits {
            drawing.selection = None;
        }
    }
    /// Abandon an unfinished stroke before another editor action.
    pub(super) fn drawing_cancel(&mut self) {
        self.drawing.stroke = None;
        self.drawing.typing = false;
    }
    /// Show the frame (or the stroke preview), onion skin, cursor and selection.
    pub(super) fn render(&self) -> Result<()> {
        let drawing = &self.drawing;
        let shown = drawing.stroke.as_ref().map_or(self.frame(), |s| &s.preview);
        // A live camera writes its own preview into the grid; the overlays wait until it stops.
        let live = self.camera.is_some();
        if !live {
            node::<Element>("art")?.set_text_content(Some(&shown.text()));
        }
        let (columns, rows) = (self.project.width(), self.project.height());
        let onion =
            (checked("onion-skin")? && self.selected > 0 && !self.playing && !live).then(|| {
                // The ghost fills only blank cells, so it never overlaps or joins the frame's text.
                let previous = &self.project.frames()[self.selected - 1].frame;
                let cells = previous
                    .cells()
                    .iter()
                    .zip(shown.cells())
                    .map(|(ghost, cell)| if *cell == b' ' { *ghost } else { b' ' })
                    .collect();
                Frame::new(columns, rows, cells)
            });
        let ghost = onion.transpose()?.map(|frame| frame.text());
        node::<Element>("onion")?.set_text_content(ghost.as_deref());
        toggle("onion", "hidden", ghost.is_none())?;
        let drawing_tool = drawing.tool != Tool::SelectText;
        toggle("grid", "data-drawing", drawing_tool)?;
        toggle("cursor", "hidden", !drawing_tool || live)?;
        place(
            "cursor",
            (drawing.cursor.0, drawing.cursor.1, 1, 1),
            columns,
            rows,
        )?;
        toggle("selection", "hidden", drawing.selection.is_none() || live)?;
        if let Some(area) = drawing.selection {
            place("selection", area, columns, rows)?;
        }
        node::<Element>("cursor-position")?.set_text_content(Some(&if drawing_tool {
            format!(
                "Column {}, row {}",
                drawing.cursor.0 + 1,
                drawing.cursor.1 + 1
            )
        } else {
            String::new()
        }));
        for (id, disabled) in [
            ("area-copy", drawing.selection.is_none()),
            ("area-cut", drawing.selection.is_none()),
            ("area-clear", drawing.selection.is_none()),
            ("area-paste", drawing.clipboard.is_none() || !drawing_tool),
            ("stamp-text", drawing.tool != Tool::Type),
            ("save-stamp", drawing.selection.is_none()),
            ("art-stamp", drawing.stamps.is_empty()),
            ("delete-stamp", drawing.stamps.is_empty()),
        ] {
            toggle(id, "disabled", disabled || self.loading || live)?;
        }
        Ok(())
    }
    fn refresh_stamps(&self) -> Result<()> {
        let picker = node::<web_sys::HtmlSelectElement>("art-stamp")?;
        picker.set_text_content(None);
        if self.drawing.stamps.is_empty() {
            let option = document()
                .create_element("option")
                .map_err(|_| "Cannot list stamps")?;
            option.set_text_content(Some("No stamps yet"));
            picker
                .append_child(&option)
                .map_err(|_| "Cannot list stamps")?;
        }
        for (index, frame) in self.drawing.stamps.iter().enumerate() {
            let option = document()
                .create_element("option")
                .map_err(|_| "Cannot list stamps")?;
            option
                .set_attribute("value", &index.to_string())
                .map_err(|_| "Cannot list stamps")?;
            option.set_text_content(Some(&format!(
                "{}: {} x {}",
                index + 1,
                frame.width(),
                frame.height()
            )));
            picker
                .append_child(&option)
                .map_err(|_| "Cannot list stamps")?;
        }
        if !self.drawing.stamps.is_empty() {
            picker.set_value(&self.drawing.stamp.to_string());
        }
        Ok(())
    }
    fn save_stamp(&mut self) -> Result<()> {
        self.clean()?;
        let (x, y, width, height) = self.drawing.selection.ok_or("Select an area first.")?;
        let stamp = self.frame().crop(x as usize, y as usize, width, height)?;
        if let Some(index) = self.drawing.stamps.iter().position(|saved| *saved == stamp) {
            self.drawing.stamp = index;
        } else {
            if self.drawing.stamps.len() >= 16
                || self
                    .drawing
                    .stamps
                    .iter()
                    .map(|f| f.cells().len())
                    .sum::<usize>()
                    + stamp.cells().len()
                    > 1_000_000
            {
                return Err(
                    "Stamps are limited to 16 entries and 1,000,000 cells. Remove a stamp first."
                        .into(),
                );
            }
            self.drawing.stamp = self.drawing.stamps.len();
            self.drawing.stamps.push(stamp);
        }
        self.refresh_stamps()?;
        status(
            "Area saved as a stamp for this session. Choose Stamp art to place it.",
            false,
        );
        Ok(())
    }
    fn set_frame(&mut self, frame: Frame, message: &str) -> Result<()> {
        if frame == *self.frame() {
            status("Nothing changed.", false);
        } else {
            self.update_frame(frame, self.duration())?;
            status(message, false);
        }
        Ok(())
    }
    fn begin(&mut self, point: (i32, i32), pointer: Option<i32>) -> Result<()> {
        self.clean()?;
        if self.drawing.tool == Tool::Stamp && self.drawing.stamps.is_empty() {
            return Err("Select an area and save it as a stamp first.".into());
        }
        self.pause();
        self.drawing.typing = false;
        self.drawing.cursor = point;
        let base = self.frame().clone();
        let lifted = match self.drawing.tool {
            Tool::SelectText => return Ok(()),
            Tool::Type => {
                self.drawing.type_column = point.0;
                return Ok(());
            }
            Tool::Fill => {
                let mut canvas = Canvas::from_frame(&base);
                canvas.flood_fill(point.0, point.1, brush()?)?;
                return self.set_frame(canvas.finish(), "Area filled.");
            }
            Tool::Area => match self.drawing.selection {
                Some(area @ (x, y, width, height))
                    if (x..x + width as i32).contains(&point.0)
                        && (y..y + height as i32).contains(&point.1) =>
                {
                    Some((base.crop(x as usize, y as usize, width, height)?, area))
                }
                _ => {
                    self.drawing.selection = Some(bounds(point, point));
                    None
                }
            },
            _ => None,
        };
        self.drawing.stroke = Some(Stroke {
            preview: base.clone(),
            base,
            start: point,
            last: point,
            pointer,
            lifted,
        });
        if pointer.is_none() {
            status(
                "Started. Move with the arrow keys, press Space to finish or Escape to cancel.",
                false,
            );
        }
        let result = self.extend(point);
        if result.is_err() {
            self.drawing.stroke = None;
        }
        result
    }
    fn extend(&mut self, point: (i32, i32)) -> Result<()> {
        self.drawing.cursor = point;
        let (columns, rows) = (self.project.width() as i32, self.project.height() as i32);
        let tool = self.drawing.tool;
        let Some(stroke) = self.drawing.stroke.as_mut() else {
            return Ok(());
        };
        let accumulate = matches!(tool, Tool::Pencil | Tool::Eraser);
        let mut canvas = Canvas::from_frame(if accumulate {
            &stroke.preview
        } else {
            &stroke.base
        });
        let start = stroke.start;
        match tool {
            Tool::Pencil => canvas.line(stroke.last, point, brush()?)?,
            Tool::Eraser => canvas.line(stroke.last, point, b' ')?,
            Tool::Line if checked("diagram")? => {
                canvas.connector(start, point, checked("arrow")?)?
            }
            Tool::Line => canvas.line(start, point, brush()?)?,
            Tool::Rectangle if checked("diagram")? => {
                canvas.diagram_box(start, shape_end(start, point)?, &label()?)?
            }
            Tool::Rectangle => {
                let (x, y, width, height) = bounds(start, shape_end(start, point)?);
                canvas.rectangle(x, y, width, height, brush()?)?
            }
            Tool::Ellipse => {
                let (x, y, width, height) = bounds(start, shape_end(start, point)?);
                canvas.ellipse(x, y, width, height, brush()?)?
            }
            Tool::Area => match &stroke.lifted {
                Some((contents, (x, y, width, height))) => {
                    // Keep the moved area whole by stopping it at the grid edges.
                    let dx = (point.0 - start.0).clamp(-x, columns - x - *width as i32);
                    let dy = (point.1 - start.1).clamp(-y, rows - y - *height as i32);
                    canvas.fill_area(*x, *y, *width, *height, b' ')?;
                    canvas.overlay(contents, x + dx, y + dy, false)?;
                    self.drawing.selection = Some((x + dx, y + dy, *width, *height));
                }
                None => self.drawing.selection = Some(bounds(start, point)),
            },
            Tool::Stamp => {
                let stamp = self
                    .drawing
                    .stamps
                    .get(self.drawing.stamp)
                    .ok_or("Choose a saved stamp first.")?;
                canvas.overlay(stamp, point.0, point.1, checked("stamp-transparent")?)?;
            }
            Tool::SelectText | Tool::Fill | Tool::Type => {}
        }
        stroke.preview = canvas.finish();
        stroke.last = point;
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        if self.recovery.locked {
            self.drawing_cancel();
            return Err("Wait for the recovery operation to finish.".into());
        }
        let Some(stroke) = self.drawing.stroke.take() else {
            return Ok(());
        };
        if self.drawing.tool == Tool::Area && stroke.lifted.is_none() {
            if let Some((_, _, width, height)) = self.drawing.selection {
                status(
                    &format!(
                        "Selected {width} x {height}. Copy, cut or clear it, or drag it to move."
                    ),
                    false,
                );
            }
            return Ok(());
        }
        let message = match self.drawing.tool {
            Tool::Area => "Area moved.",
            Tool::Rectangle if checked("diagram")? => {
                let end = shape_end(stroke.start, stroke.last)?;
                let (_, _, width, height) = bounds(stroke.start, end);
                let text = label()?;
                if !text.is_empty() && (text.len() + 2 > width || height < 3) {
                    "Box drawn. Make it larger to show the whole label."
                } else {
                    "Box drawn."
                }
            }
            _ => "Drawn.",
        };
        self.set_frame(stroke.preview, message)
    }
    fn cancel(&mut self) {
        if let Some(stroke) = self.drawing.stroke.take() {
            self.drawing.selection = stroke.lifted.map(|(_, area)| area);
            status("Cancelled.", false);
        } else if self.drawing.selection.take().is_some() {
            status("Selection cleared.", false);
        }
        self.drawing.typing = false;
    }
    fn type_key(&mut self, key: &str) -> Result<()> {
        self.clean()?;
        self.pause();
        let (columns, rows) = (self.project.width() as i32, self.project.height() as i32);
        let (x, y) = self.drawing.cursor;
        let mut canvas = Canvas::from_frame(self.frame());
        self.drawing.cursor = match key {
            "Enter" => (self.drawing.type_column, (y + 1).min(rows - 1)),
            "Backspace" => {
                canvas.set((x - 1).max(0), y, b' ')?;
                ((x - 1).max(0), y)
            }
            _ => {
                canvas.text(x, y, key)?;
                ((x + 1).min(columns - 1), y)
            }
        };
        let frame = canvas.finish();
        if frame == *self.frame() {
            return Ok(());
        }
        if self.drawing.typing {
            let duration = self.duration();
            self.project.replace_frame(
                self.selected,
                crate::TimedFrame {
                    frame,
                    duration_ms: duration,
                },
            )?;
            self.modified = true;
            self.recovery.changed();
            self.display(true)?;
        } else {
            self.update_frame(frame, self.duration())?;
            self.drawing.typing = true;
        }
        Ok(())
    }
    fn area_text(&mut self) -> Result<String> {
        self.clean()?;
        let (x, y, width, height) = self.drawing.selection.ok_or("Select an area first.")?;
        let contents = self.frame().crop(x as usize, y as usize, width, height)?;
        let text = contents.text();
        self.drawing.clipboard = Some(contents);
        Ok(text)
    }
    fn clear_area(&mut self, message: &str) -> Result<()> {
        self.clean()?;
        let (x, y, width, height) = self.drawing.selection.ok_or("Select an area first.")?;
        let mut canvas = Canvas::from_frame(self.frame());
        canvas.fill_area(x, y, width, height, b' ')?;
        self.set_frame(canvas.finish(), message)
    }
    fn paste(&mut self, contents: Frame) -> Result<()> {
        self.clean()?;
        self.pause();
        let (x, y) = self.drawing.cursor;
        let mut canvas = Canvas::from_frame(self.frame());
        canvas.overlay(&contents, x, y, false)?;
        let area = (
            x,
            y,
            contents.width().min(self.project.width() - x as usize),
            contents.height().min(self.project.height() - y as usize),
        );
        self.set_frame(canvas.finish(), "Pasted at the cursor.")?;
        if self.drawing.tool == Tool::Area {
            self.drawing.selection = Some(area);
        }
        Ok(())
    }
}

fn cell(app: &App, event: &PointerEvent) -> Result<(i32, i32)> {
    let rect = node::<Element>("art")?.get_bounding_client_rect();
    let axis = |offset: f64, size: f64, count: usize| {
        let index = (offset / size * count as f64).floor();
        index.clamp(0.0, count as f64 - 1.0) as i32
    };
    Ok((
        axis(
            f64::from(event.client_x()) - rect.left(),
            rect.width(),
            app.project.width(),
        ),
        axis(
            f64::from(event.client_y()) - rect.top(),
            rect.height(),
            app.project.height(),
        ),
    ))
}
fn pointer(event: Event) -> Result<()> {
    let event: PointerEvent = event.dyn_into().map_err(|_| "Expected a pointer event")?;
    state(|app| {
        if app.drawing.tool == Tool::SelectText {
            return Ok(());
        }
        // Other touches or pens can neither steer nor replace a stroke that one pointer started.
        let (ours, busy) = app.drawing.stroke.as_ref().map_or((false, false), |s| {
            (s.pointer == Some(event.pointer_id()), s.pointer.is_some())
        });
        match event.type_().as_str() {
            "pointerdown" if event.button() == 0 && event.is_primary() && (ours || !busy) => {
                event.prevent_default();
                let point = cell(app, &event)?;
                let _ = node::<Element>("grid")?.set_pointer_capture(event.pointer_id());
                // Scrolling the focused grid into view would move cells under the pointer.
                let options = FocusOptions::new();
                options.set_prevent_scroll(true);
                node::<HtmlElement>("art")?
                    .focus_with_options(&options)
                    .map_err(|_| "Cannot focus the artwork")?;
                app.begin(point, Some(event.pointer_id()))?;
            }
            "pointermove" if ours => {
                // Redraw only when the pointer enters another cell.
                let point = cell(app, &event)?;
                if app.drawing.stroke.as_ref().is_some_and(|s| s.last == point) {
                    return Ok(());
                }
                app.extend(point)?;
            }
            "pointerup" if ours => {
                app.extend(cell(app, &event)?)?;
                app.finish()?;
            }
            "pointercancel" | "lostpointercapture" if ours => app.cancel(),
            _ => return Ok(()),
        }
        app.display(false)
    })
}
fn key(event: Event) -> Result<()> {
    let event: KeyboardEvent = event.dyn_into().map_err(|_| "Expected a keyboard event")?;
    let key = event.key();
    // AltGr arrives as Ctrl+Alt on Windows; its characters are typing, not shortcuts.
    let altgr = event.get_modifier_state("AltGraph");
    if !altgr && (event.alt_key() || event.ctrl_key() || event.meta_key()) {
        if event.alt_key() {
            return Ok(());
        }
        let history = match key.to_ascii_lowercase().as_str() {
            "z" if event.shift_key() => "redo",
            "z" => "undo",
            "y" => "redo",
            _ => return Ok(()),
        };
        event.prevent_default();
        return action(history);
    }
    state(|app| {
        let tool = app.drawing.tool;
        if tool == Tool::SelectText {
            return Ok(());
        }
        let (x, y) = app.drawing.cursor;
        let step = match key.as_str() {
            "ArrowLeft" => Some((x - 1, y)),
            "ArrowRight" => Some((x + 1, y)),
            "ArrowUp" => Some((x, y - 1)),
            "ArrowDown" => Some((x, y + 1)),
            _ => None,
        };
        if let Some((x, y)) = step {
            let point = (
                x.clamp(0, app.project.width() as i32 - 1),
                y.clamp(0, app.project.height() as i32 - 1),
            );
            if app.drawing.stroke.is_some() {
                app.extend(point)?;
            } else {
                app.drawing.cursor = point;
                app.drawing.type_column = point.0;
                app.drawing.typing = false;
            }
        } else if key == "Escape" {
            app.cancel();
        } else if tool == Tool::Type
            && (key == "Enter" || key == "Backspace" || (key.len() == 1 && key.is_ascii()))
        {
            app.type_key(&key)?;
        } else if tool == Tool::Area
            && app.drawing.stroke.is_none()
            && (key == "Delete" || key == "Backspace")
        {
            app.clear_area("Area cleared.")?;
        } else if key == " " || key == "Enter" {
            if app.drawing.stroke.is_some() {
                app.finish()?;
            } else {
                app.begin(app.drawing.cursor, None)?;
            }
        } else {
            return Ok(());
        }
        event.prevent_default();
        app.display(false)
    })
}
fn clipboard(event: Event) -> Result<()> {
    let event: ClipboardEvent = event.dyn_into().map_err(|_| "Expected a clipboard event")?;
    let Some(data) = event.clipboard_data() else {
        return Ok(());
    };
    state(|app| {
        if app.drawing.tool == Tool::SelectText {
            return Ok(());
        }
        match event.type_().as_str() {
            "copy" | "cut" if app.drawing.selection.is_some() => {
                data.set_data("text/plain", &app.area_text()?)
                    .map_err(|_| "Cannot write to the clipboard")?;
                if event.type_() == "cut" {
                    app.clear_area("Area cut.")?;
                } else {
                    status("Area copied as ASCII text.", false);
                }
            }
            "paste" => {
                let text = data.get_data("text/plain").unwrap_or_default();
                app.paste(Frame::from_text_padded(&text.replace("\r\n", "\n"))?)?;
            }
            _ => return Ok(()),
        }
        event.prevent_default();
        app.display(false)
    })
}
fn button(id: &str) -> Result<()> {
    if id == "area-copy" {
        let text = state(|app| app.area_text())?;
        copy_text(text, "Area copied as ASCII text.");
        return state(|app| app.render());
    }
    state(|app| {
        match id {
            "area-cut" => {
                let text = app.area_text()?;
                copy_text(text, "Area cut and copied as ASCII text.");
                app.clear_area("Area cut.")?;
            }
            "area-clear" => app.clear_area("Area cleared.")?,
            "save-stamp" => app.save_stamp()?,
            "delete-stamp" => {
                app.drawing_cancel();
                if app.drawing.stamps.is_empty() {
                    return Err("There is no saved stamp to remove.".into());
                }
                app.drawing.stamps.remove(app.drawing.stamp);
                app.drawing.stamp = app
                    .drawing
                    .stamp
                    .min(app.drawing.stamps.len().saturating_sub(1));
                app.refresh_stamps()?;
                status("Stamp removed from this session.", false);
            }
            "area-paste" => {
                let contents = app
                    .drawing
                    .clipboard
                    .clone()
                    .ok_or("Copy or cut an area first.")?;
                app.paste(contents)?;
            }
            "stamp-text" => {
                let text = label()?;
                if text.is_empty() {
                    return Err("Enter text to stamp first.".into());
                }
                app.clean()?;
                app.pause();
                let (x, y) = app.drawing.cursor;
                let mut canvas = Canvas::from_frame(app.frame());
                canvas.text(x, y, &text)?;
                app.set_frame(canvas.finish(), "Text stamped at the cursor.")?;
            }
            _ => return Err("Unknown drawing action".into()),
        }
        app.display(false)
    })
}

pub(super) fn listen_all() -> std::result::Result<(), wasm_bindgen::JsValue> {
    // Small screens start with the options folded so the grid sits just below the tools.
    if web_sys::window()
        .ok_or("Browser window")?
        .match_media("(max-width: 760px)")?
        .is_some_and(|query| query.matches())
    {
        node::<Element>("tool-options")?.remove_attribute("open")?;
    }
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
    ] {
        listen("grid", name, move |event| report(pointer(event)))?;
    }
    listen("art", "keydown", move |event| report(key(event)))?;
    listen("art", "blur", move |_| {
        report(state(|app| {
            app.drawing.typing = false;
            if app
                .drawing
                .stroke
                .as_ref()
                .is_some_and(|s| s.pointer.is_none())
            {
                app.cancel();
            }
            app.render()
        }))
    })?;
    for name in ["copy", "cut", "paste"] {
        listen("art", name, move |event| report(clipboard(event)))?;
    }
    listen("tools", "change", move |_| {
        report((|| {
            let value = document()
                .query_selector("input[name=tool]:checked")
                .ok()
                .flatten()
                .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
                .ok_or("Choose a tool")?
                .value();
            let tool = Tool::parse(&value)?;
            state(|app| {
                app.drawing.stroke = None;
                app.drawing.typing = false;
                app.drawing.selection = None;
                app.drawing.tool = tool;
                status(tool.help(), false);
                node::<Element>("tool-help")?.set_text_content(Some(tool.help()));
                app.render()
            })
        })())
    })?;
    for id in ["onion-skin", "diagram"] {
        listen(id, "change", move |_| report(state(|app| app.render())))?;
    }
    listen("art-stamp", "change", move |_| {
        report(state(|app| {
            app.drawing_cancel();
            let index: usize = super::select("art-stamp")?
                .parse()
                .map_err(|_| "Choose a saved stamp")?;
            if index >= app.drawing.stamps.len() {
                return Err("Choose a saved stamp".into());
            }
            app.drawing.stamp = index;
            app.render()
        }))
    })?;
    listen("stamp-transparent", "change", move |_| {
        report(state(|app| {
            app.drawing_cancel();
            app.render()
        }))
    })?;
    for id in [
        "area-copy",
        "area-cut",
        "area-clear",
        "area-paste",
        "stamp-text",
        "save-stamp",
        "delete-stamp",
    ] {
        listen(id, "click", move |_| report(button(id)))?;
    }
    Ok(())
}
