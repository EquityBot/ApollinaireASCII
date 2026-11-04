use crate::{
    Animation, Canvas, EffectOptions, Frame, MAX_INPUT_BYTES, RenderOptions, Result, SceneOptions,
    TimedFrame, animation, art, decode_media, effects, export, image_renderer::RAMPS, lettering,
};
use js_sys::{Array, Uint8Array};
use std::cell::RefCell;
use std::ops::Range;
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Document, Element, Event, HtmlAnchorElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement,
};

mod draw;
mod media;
mod recovery;
#[cfg(test)]
mod tests;

thread_local! {static APP:RefCell<Option<App>>=const {RefCell::new(None)};}
struct App {
    project: Animation,
    selected: usize,
    frame_range: Range<usize>,
    playing: bool,
    elapsed: f64,
    last: f64,
    dirty: bool,
    modified: bool,
    loading: bool,
    ticket: u32,
    font: Option<String>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    drawing: draw::Drawing,
    camera: Option<media::Camera>,
    recovery: recovery::Recovery,
}
struct Snapshot {
    project: Animation,
    selected: usize,
    frame_range: Range<usize>,
}
fn document() -> Document {
    web_sys::window()
        .expect("Browser window")
        .document()
        .expect("Browser document")
}
fn node<T: JsCast>(id: &str) -> Result<T> {
    document()
        .get_element_by_id(id)
        .ok_or_else(|| format!("Missing interface element {id}"))?
        .dyn_into()
        .map_err(|_| format!("Wrong interface element type for {id}"))
}
fn input(id: &str) -> Result<HtmlInputElement> {
    node(id)
}
fn select(id: &str) -> Result<String> {
    Ok(node::<HtmlSelectElement>(id)?.value())
}
fn text(id: &str) -> Result<String> {
    Ok(node::<HtmlTextAreaElement>(id)?.value())
}
fn number(id: &str) -> Result<usize> {
    input(id)?
        .value()
        .parse()
        .map_err(|_| format!("{id} must be a non-negative integer"))
}
fn decimal(id: &str) -> Result<f64> {
    input(id)?
        .value()
        .parse()
        .map_err(|_| format!("{id} must be a number"))
}
fn checked(id: &str) -> Result<bool> {
    Ok(input(id)?.checked())
}
fn now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}
fn state<T>(f: impl FnOnce(&mut App) -> Result<T>) -> Result<T> {
    APP.with(|cell| f(cell.borrow_mut().as_mut().ok_or("Editor is not ready")?))
}
fn status(message: &str, error: bool) {
    if let Ok(element) = node::<Element>("status") {
        element.set_text_content(Some(message));
        let _ = element.set_attribute("data-kind", if error { "error" } else { "info" });
    }
}
fn report(result: Result<()>) {
    if let Err(error) = result {
        status(&error, true);
    }
}
impl App {
    fn clean(&self) -> Result<()> {
        if self.recovery.locked {
            return Err("Wait for the recovery operation to finish.".into());
        }
        if self.loading {
            return Err("A file is loading. Wait or choose another file.".into());
        }
        if self.camera.is_some() {
            return Err("Turn the camera off before continuing.".into());
        }
        if self.dirty {
            return Err("Apply or discard the frame edits before continuing.".into());
        }
        Ok(())
    }
    fn pause(&mut self) {
        self.playing = false;
        self.last = 0.0;
    }
    fn seek(&mut self, index: usize) {
        self.pause();
        self.selected = index;
        self.frame_range = index..index + 1;
        self.reset_drawing();
        self.elapsed = self.project.frames()[..index]
            .iter()
            .map(|frame| f64::from(frame.duration_ms))
            .sum();
    }
    fn commit(&mut self, project: Animation) {
        self.pause();
        self.undo.push(Snapshot {
            project: std::mem::replace(&mut self.project, project),
            selected: self.selected,
            frame_range: self.frame_range.clone(),
        });
        self.redo.clear();
        self.limit_history();
        let end = self.frame_range.end.min(self.project.frames().len());
        let range = self.frame_range.start.min(end - 1)..end;
        self.seek(self.selected.min(self.project.frames().len() - 1));
        self.frame_range = range;
        self.dirty = false;
        self.modified = true;
        self.recovery.changed();
    }
    fn limit_history(&mut self) {
        while self.undo.len() + self.redo.len() > 100
            || self
                .undo
                .iter()
                .chain(self.redo.iter())
                .map(|s| s.project.width() * s.project.height() * s.project.frames().len())
                .sum::<usize>()
                > 16_000_000
        {
            if !self.undo.is_empty() {
                self.undo.remove(0);
            } else {
                self.redo.remove(0);
            }
        }
    }
    fn display(&self, editor: bool) -> Result<()> {
        let entry = &self.project.frames()[self.selected];
        self.render()?;
        node::<Element>("dimensions")?.set_text_content(Some(&format!(
            "{} columns x {} rows",
            self.project.width(),
            self.project.height()
        )));
        let frame_label = format!(
            "Frame {} / {}",
            self.selected + 1,
            self.project.frames().len()
        );
        node::<Element>("frame-count")?.set_text_content(Some(&frame_label));
        let timeline = input("timeline")?;
        timeline.set_max(&(self.project.frames().len() - 1).to_string());
        timeline.set_value(&self.selected.to_string());
        timeline
            .set_attribute("aria-valuetext", &frame_label)
            .map_err(|_| "Cannot update timeline")?;
        input("loop")?.set_checked(self.project.looping());
        if editor {
            node::<HtmlTextAreaElement>("editor")?.set_value(&entry.frame.text());
            input("duration")?.set_value(&entry.duration_ms.to_string());
            input("range-first")?.set_value(&(self.frame_range.start + 1).to_string());
            input("range-last")?.set_value(&self.frame_range.end.to_string());
        }
        for id in ["range-first", "range-last"] {
            input(id)?.set_max(&self.project.frames().len().to_string());
        }
        let range_label = format!(
            "Selected frames {} to {} ({} frame{})",
            self.frame_range.start + 1,
            self.frame_range.end,
            self.frame_range.len(),
            if self.frame_range.len() == 1 { "" } else { "s" },
        );
        let range_count = node::<Element>("range-count")?;
        if range_count.text_content().as_deref() != Some(range_label.as_str()) {
            range_count.set_text_content(Some(&range_label));
        }
        let busy = self.loading || self.camera.is_some() || self.recovery.locked;
        node::<Element>("play")?.set_text_content(Some(if self.playing {
            "Pause"
        } else {
            "Play"
        }));
        for id in [
            "generate",
            "convert",
            "copy",
            "download-text",
            "download-json",
            "download-html",
            "apply",
            "discard",
            "editor",
            "duration",
            "add-frame",
            "duplicate",
            "range-first",
            "range-last",
            "select-range",
            "range-duration",
            "apply-range-duration",
            "flip-horizontal",
            "flip-vertical",
            "apply-effect",
        ] {
            let element = node::<Element>(id)?;
            if busy {
                element
                    .set_attribute("disabled", "disabled")
                    .map_err(|_| "Cannot update controls")?;
            } else {
                element
                    .remove_attribute("disabled")
                    .map_err(|_| "Cannot update controls")?;
            }
        }
        for (id, disabled) in [
            ("play", busy || self.project.frames().len() == 1),
            ("previous", busy),
            ("next", busy),
            (
                "delete-frame",
                self.frame_range.len() == self.project.frames().len() || busy,
            ),
            ("move-left", self.frame_range.start == 0 || busy),
            (
                "move-right",
                self.frame_range.end == self.project.frames().len() || busy,
            ),
            ("undo", self.undo.is_empty() || busy),
            ("redo", self.redo.is_empty() || busy),
        ] {
            let element = node::<Element>(id)?;
            if disabled {
                element
                    .set_attribute("disabled", "disabled")
                    .map_err(|_| "Cannot update controls")?;
            } else {
                element
                    .remove_attribute("disabled")
                    .map_err(|_| "Cannot update controls")?;
            }
        }
        self.recovery.controls(busy || self.dirty)?;
        media::controls(self)
    }
    fn update_frame(&mut self, frame: Frame, duration_ms: u32) -> Result<()> {
        let mut project = self.project.clone();
        project.replace_frame(self.selected, TimedFrame { frame, duration_ms })?;
        self.commit(project);
        self.display(true)
    }
}

#[wasm_bindgen(start)]
pub fn start() -> std::result::Result<(), JsValue> {
    let project = animation::generate("ghost", &SceneOptions::default())
        .map_err(|e| JsValue::from_str(&e))?;
    let reduced = web_sys::window()
        .expect("Browser window")
        .match_media("(prefers-reduced-motion: reduce)")?
        .is_some_and(|q| q.matches());
    APP.with(|cell| {
        *cell.borrow_mut() = Some(App {
            project,
            selected: 0,
            frame_range: 0..1,
            playing: !reduced,
            elapsed: 0.0,
            last: now(),
            dirty: false,
            modified: false,
            loading: false,
            ticket: 0,
            font: None,
            undo: vec![],
            redo: vec![],
            drawing: draw::Drawing::default(),
            camera: None,
            recovery: recovery::Recovery::default(),
        })
    });
    for id in [
        "generate",
        "convert",
        "play",
        "previous",
        "next",
        "apply",
        "discard",
        "add-frame",
        "duplicate",
        "delete-frame",
        "move-left",
        "move-right",
        "select-range",
        "apply-range-duration",
        "flip-horizontal",
        "flip-vertical",
        "undo",
        "redo",
        "copy",
        "download-text",
        "download-json",
        "download-html",
        "apply-effect",
    ] {
        listen(id, "click", move |_| report(action(id)))?;
    }
    listen("mode", "change", move |_| report(fields()))?;
    for id in ["range-first", "range-last", "range-duration"] {
        listen(id, "focus", |_| {
            report(state(|app| {
                app.pause();
                app.display(false)
            }))
        })?;
    }
    draw::listen_all()?;
    fill_effects().map_err(|e| JsValue::from_str(&e))?;
    fill_art("").map_err(|e| JsValue::from_str(&e))?;
    listen("art-search", "input", move |_| {
        report(input("art-search").and_then(|search| fill_art(&search.value())))
    })?;
    listen("timeline", "input", move |_| {
        report(state(|app| {
            app.clean()?;
            app.seek(number("timeline")?.min(app.project.frames().len() - 1));
            app.display(true)
        }))
    })?;
    listen("loop", "change", move |_| {
        report(state(|app| {
            app.clean()?;
            let mut project = app.project.clone();
            project.set_looping(checked("loop")?);
            app.commit(project);
            app.display(false)
        }))
    })?;
    for id in ["editor", "duration"] {
        listen(id, "focus", move |_| {
            report(state(|app| {
                app.pause();
                app.display(false)
            }))
        })?;
        listen(id, "input", move |_| {
            report(state(|app| {
                app.pause();
                if app.recovery.locked {
                    return Err("Wait for the recovery operation to finish.".into());
                }
                if !app.dirty {
                    status("Frame edits are not applied yet.", false);
                }
                app.dirty = true;
                app.display(false)
            }))
        })?;
    }
    for id in ["open-file", "font-file"] {
        listen(id, "change", move |_| begin_file(id))?;
    }
    listen("preset", "change", move |_| report(apply_preset()))?;
    // Changing a setting by hand leaves the preset behind.
    for id in [
        "ramp",
        "dither",
        "edges",
        "edge-glyphs",
        "gamma",
        "contrast",
    ] {
        listen(id, "input", move |_| {
            report(node::<HtmlSelectElement>("preset").map(|preset| preset.set_value("custom")))
        })?;
    }
    media::start()?;
    let fonts = node::<Element>("font").map_err(|e| JsValue::from_str(&e))?;
    let uploaded = node::<Element>("custom-option").map_err(|e| JsValue::from_str(&e))?;
    for name in lettering::FONTS {
        let option = document().create_element("option")?;
        option.set_text_content(Some(name));
        fonts.insert_before(&option, Some(&uploaded))?;
    }
    let visibility = Closure::<dyn FnMut(Event)>::new(move |_| {
        if document().hidden() {
            report(state(|app| {
                app.pause();
                recovery::tick(app, true);
                app.display(false)
            }));
        }
    });
    document().add_event_listener_with_callback(
        "visibilitychange",
        visibility.as_ref().unchecked_ref(),
    )?;
    visibility.forget();
    if let Some(media) = web_sys::window()
        .expect("Browser window")
        .match_media("(prefers-reduced-motion: reduce)")?
    {
        let query = media.clone();
        let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
            if query.matches() {
                report(state(|app| {
                    app.pause();
                    app.display(false)
                }));
            }
        });
        media.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    let unload = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
        move |event: web_sys::BeforeUnloadEvent| {
            let unsaved = APP.with(|cell| {
                cell.borrow()
                    .as_ref()
                    .is_some_and(|a| a.modified || a.dirty)
            });
            if unsaved {
                event.prevent_default();
                event.set_return_value("");
            }
        },
    );
    web_sys::window()
        .expect("Browser window")
        .add_event_listener_with_callback("beforeunload", unload.as_ref().unchecked_ref())?;
    unload.forget();
    let timer = Closure::<dyn FnMut()>::new(move || report(tick()));
    web_sys::window()
        .expect("Browser window")
        .set_interval_with_callback_and_timeout_and_arguments_0(
            timer.as_ref().unchecked_ref(),
            16,
        )?;
    timer.forget();
    report(fields());
    report(state(|app| app.display(true)));
    status(
        if reduced {
            "Ready. Reduced motion is enabled, so playback starts paused."
        } else {
            "Ready. Select and copy the artwork, or make something of your own."
        },
        false,
    );
    recovery::start()?;
    Ok(())
}
fn listen(
    id: &str,
    event: &str,
    callback: impl FnMut(Event) + 'static + std::panic::UnwindSafe,
) -> std::result::Result<(), JsValue> {
    let closure = Closure::<dyn FnMut(Event)>::new(callback);
    node::<Element>(id)
        .map_err(|e| JsValue::from_str(&e))?
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
    // A fixed number of handlers live for the lifetime of this single-page app.
    closure.forget();
    Ok(())
}
fn fields() -> Result<()> {
    let mode = select("mode")?;
    for (id, visible) in [
        ("scene-fields", mode == "scene"),
        ("art-fields", mode == "art"),
        ("letter-fields", mode == "letters"),
        ("text-fields", mode == "letters" || mode == "scene"),
    ] {
        let element = node::<Element>(id)?;
        if visible {
            element
                .remove_attribute("hidden")
                .map_err(|_| "Cannot show settings")?;
        } else {
            element
                .set_attribute("hidden", "")
                .map_err(|_| "Cannot hide settings")?;
        }
    }
    Ok(())
}
fn generate(app: &mut App) -> Result<()> {
    app.clean()?;
    let width = number("width")?;
    let height = number("height")?;
    let mut project = match select("mode")?.as_str() {
        "scene" => animation::generate(
            &select("scene")?,
            &SceneOptions {
                width,
                height,
                frames: number("frames")?,
                fps: u32::try_from(number("fps")?).map_err(|_| "FPS is too large")?,
                seed: u32::try_from(number("seed")?).map_err(|_| "Seed is too large")?,
                text: text("message")?,
            },
        )?,
        "art" => {
            let name = select("art-name")?;
            if name.is_empty() {
                return Err("No artwork matches the search. Clear it to see every piece.".into());
            }
            Animation::still(art::template(&name)?)
        }
        "letters" => {
            let font = select("font")?;
            let custom = if font == "custom" {
                Some(app.font.as_deref().ok_or("Choose a font file first")?)
            } else {
                None
            };
            let layout = |id| -> Result<lettering::Layout> {
                Ok(match select(id)?.as_str() {
                    "full" => lettering::Layout::Full,
                    "fitted" => lettering::Layout::Fitted,
                    "smush" => lettering::Layout::Smush,
                    _ => lettering::Layout::Default,
                })
            };
            let wrap = input("wrap")?.value();
            let options = lettering::Options {
                alignment: match select("align")?.as_str() {
                    "left" => lettering::Alignment::Left,
                    "right" => lettering::Alignment::Right,
                    _ => lettering::Alignment::Center,
                },
                width: Some(width),
                wrap: if wrap.is_empty() {
                    None
                } else {
                    Some(
                        wrap.parse()
                            .map_err(|_| "Wrap width must be a whole number")?,
                    )
                },
                horizontal: layout("horizontal")?,
                vertical: layout("vertical")?,
            };
            Animation::still(lettering::render_with(
                &text("message")?,
                &font,
                custom,
                &options,
            )?)
        }
        "blank" => Animation::still(Frame::blank(width, height)?),
        _ => return Err("Choose a valid artwork source".into()),
    };
    if checked("border")? {
        project = Animation::new(
            project
                .frames()
                .iter()
                .map(|f| {
                    Ok(TimedFrame {
                        frame: f.frame.border()?,
                        duration_ms: f.duration_ms,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            project.looping(),
        )?;
    }
    app.commit(project);
    app.seek(0);
    app.display(true)?;
    status(
        "Artwork generated. Save the project to keep every frame.",
        false,
    );
    Ok(())
}
fn action(id: &str) -> Result<()> {
    if id == "convert" {
        begin_file("media-file");
        return Ok(());
    }
    if id == "copy" {
        let content = state(|app| {
            app.clean()?;
            Ok(app.project.frames()[app.selected].frame.text())
        })?;
        copy_text(content, "Frame copied as ASCII text.");
        return Ok(());
    }
    state(|app| {
        if id == "discard" {
            if app.recovery.locked {
                return Err("Wait for the recovery operation to finish.".into());
            }
            app.dirty = false;
            app.display(true)?;
            status("Frame edits discarded.", false);
            return Ok(());
        }
        if id == "apply" {
            if app.recovery.locked {
                return Err("Wait for the recovery operation to finish.".into());
            }
            if app.loading {
                return Err("Wait for the file to finish loading.".into());
            }
            let mut canvas = Canvas::new(app.project.width(), app.project.height())?;
            let draft = text("editor")?;
            if !draft.is_empty() {
                let edited = Frame::from_text_padded(&draft)?;
                if edited.width() > app.project.width() || edited.height() > app.project.height() {
                    return Err("Edited text extends beyond the project dimensions.".into());
                }
                canvas.overlay(&edited, 0, 0, false)?;
            }
            let duration =
                u32::try_from(number("duration")?).map_err(|_| "Duration is too large")?;
            app.update_frame(canvas.finish(), duration)?;
            status("Frame edits applied.", false);
            return Ok(());
        }
        app.clean()?;
        app.drawing_cancel();
        match id {
            "generate" => return generate(app),
            "play" => {
                if app.playing {
                    app.pause();
                } else {
                    if !app.project.looping() && app.elapsed >= app.project.duration_ms() as f64 {
                        let range = app.frame_range.clone();
                        app.seek(0);
                        app.frame_range = range;
                    }
                    app.last = now();
                    app.playing = true;
                }
            }
            "previous" | "next" => {
                let count = app.project.frames().len();
                let index = if id == "next" {
                    (app.selected + 1) % count
                } else {
                    (app.selected + count - 1) % count
                };
                app.seek(index);
            }
            "undo" | "redo" => {
                app.pause();
                let previous = if id == "undo" {
                    app.undo.pop()
                } else {
                    app.redo.pop()
                }
                .ok_or("No history is available")?;
                let current = Snapshot {
                    project: std::mem::replace(&mut app.project, previous.project),
                    selected: app.selected,
                    frame_range: app.frame_range.clone(),
                };
                if id == "undo" {
                    app.redo.push(current);
                } else {
                    app.undo.push(current);
                }
                app.limit_history();
                app.seek(previous.selected);
                app.frame_range = previous.frame_range;
                app.modified = true;
                app.recovery.changed();
            }
            "select-range" => {
                let first = number("range-first")?;
                let last = number("range-last")?;
                if first == 0 || first > last || last > app.project.frames().len() {
                    return Err(
                        "Choose a frame range with 1 <= first <= last <= frame count".into(),
                    );
                }
                let range = first - 1..last;
                app.seek(app.selected.clamp(range.start, range.end - 1));
                app.frame_range = range;
            }
            "apply-range-duration" => {
                let duration = u32::try_from(number("range-duration")?)
                    .map_err(|_| "Frame duration is too large")?;
                let mut project = app.project.clone();
                project.set_frame_durations(app.frame_range.clone(), duration)?;
                if project != app.project {
                    app.commit(project);
                }
                status("Selected frame durations applied.", false);
            }
            "add-frame" | "duplicate" | "delete-frame" | "move-left" | "move-right" => {
                let mut project = app.project.clone();
                let range = app.frame_range.clone();
                let offset = app
                    .selected
                    .saturating_sub(range.start)
                    .min(range.len() - 1);
                let (selected, new_range);
                match id {
                    "add-frame" => {
                        project.insert_frame(
                            app.selected + 1,
                            TimedFrame {
                                frame: Frame::blank(project.width(), project.height())?,
                                duration_ms: project.frames()[app.selected].duration_ms,
                            },
                        )?;
                        selected = app.selected + 1;
                        new_range = selected..selected + 1;
                    }
                    "duplicate" => {
                        project.duplicate_frames(range.clone())?;
                        selected = range.end + offset;
                        new_range = range.end..range.end + range.len();
                    }
                    "delete-frame" => {
                        project.remove_frames(range.clone())?;
                        selected = range.start.min(project.frames().len() - 1);
                        new_range = selected..selected + 1;
                    }
                    _ => {
                        let destination = if id == "move-left" {
                            range
                                .start
                                .checked_sub(1)
                                .ok_or("Already the first frame range")?
                        } else {
                            range.start + 1
                        };
                        project.move_frames(range.clone(), destination)?;
                        selected = destination + offset;
                        new_range = destination..destination + range.len();
                    }
                }
                app.commit(project);
                app.seek(selected);
                app.frame_range = new_range;
            }
            "flip-horizontal" | "flip-vertical" => {
                let entry = &app.project.frames()[app.selected];
                let frame = match (id, checked("mirror-glyphs")?) {
                    ("flip-horizontal", true) => entry.frame.mirror_horizontal(),
                    ("flip-horizontal", false) => entry.frame.flip_horizontal(),
                    (_, true) => entry.frame.mirror_vertical(),
                    (_, false) => entry.frame.flip_vertical(),
                };
                app.update_frame(frame, entry.duration_ms)?;
            }
            "download-text" => download(
                &(app.project.frames()[app.selected].frame.text() + "\n"),
                "frame.txt",
                "text/plain;charset=utf-8",
            )?,
            "download-json" => {
                download(
                    &app.project.to_json(),
                    "artwork.ascii.json",
                    "application/json",
                )?;
                status(
                    "Project download requested. Check the saved file before closing this page.",
                    false,
                );
            }
            "download-html" => download(
                &export::html(&app.project),
                "artwork.html",
                "text/html;charset=utf-8",
            )?,
            "apply-effect" => apply_effect(app)?,
            _ => return Err("Unknown editor action".into()),
        }
        app.display(true)
    })
}
fn copy_text(content: String, done: &'static str) {
    spawn_local(async move {
        let promise = web_sys::window()
            .expect("Browser window")
            .navigator()
            .clipboard()
            .write_text(&content);
        if JsFuture::from(promise).await.is_ok() {
            status(done, false);
        } else {
            status(
                "Copy was blocked. Select the artwork and copy it with your keyboard.",
                true,
            );
        }
    });
}
fn apply_effect(app: &mut App) -> Result<()> {
    let source = app.project.frames()[app.selected].frame.clone();
    let options = EffectOptions {
        duration_ms: u32::try_from(number("effect-duration")?)
            .map_err(|_| "Duration is too large")?,
        fps: u32::try_from(number("effect-fps")?).map_err(|_| "FPS is too large")?,
        seed: u32::try_from(number("effect-seed")?).map_err(|_| "Seed is too large")?,
        reverse: checked("effect-reverse")?,
    };
    let project = effects::apply(&select("effect")?, &source, &options)?;
    let count = project.frames().len();
    app.commit(project);
    app.seek(0);
    status(
        &format!("Effect applied: {count} frames from the selected frame. Press Play to watch."),
        false,
    );
    Ok(())
}
fn element(tag: &str, text: &str) -> Result<Element> {
    let element = document()
        .create_element(tag)
        .map_err(|_| "Cannot create a list entry")?;
    element.set_text_content(Some(text));
    Ok(element)
}
fn fill_effects() -> Result<()> {
    let list = node::<Element>("effect")?;
    for name in effects::EFFECTS {
        let option = element("option", name)?;
        list.append_child(&option)
            .map_err(|_| "Cannot list effects")?;
    }
    Ok(())
}
/// Rebuild the artwork list, grouped by category, from the pieces matching `query`.
fn fill_art(query: &str) -> Result<()> {
    let list = node::<HtmlSelectElement>("art-name")?;
    let previous = list.value();
    list.set_text_content(None);
    for category in art::CATEGORIES {
        let group = element("optgroup", "")?;
        let label = category[..1].to_ascii_uppercase() + &category[1..];
        group
            .set_attribute("label", &label)
            .map_err(|_| "Cannot group artwork")?;
        for piece in art::search(query).filter(|p| p.category == *category) {
            let option = element("option", piece.name)?;
            group
                .append_child(&option)
                .map_err(|_| "Cannot list artwork")?;
        }
        if group.has_child_nodes() {
            list.append_child(&group)
                .map_err(|_| "Cannot list artwork")?;
        }
    }
    if art::piece(&previous).is_some_and(|p| p.matches(query)) {
        list.set_value(&previous);
    }
    let count = art::search(query).count();
    node::<Element>("art-matches")?.set_text_content(Some(&if count == 0 {
        "No artwork matches. Try a tag such as bird, sea or frame.".to_string()
    } else {
        format!("{count} of {} pieces", art::CATALOGUE.len())
    }));
    Ok(())
}
fn tick() -> Result<()> {
    state(|app| {
        recovery::tick(app, false);
        if !app.playing || document().hidden() {
            return Ok(());
        }
        let time = now();
        app.elapsed += (time - app.last).max(0.0);
        app.last = time;
        let index = app.project.frame_at(app.elapsed as u64);
        let stopped = !app.project.looping() && app.elapsed >= app.project.duration_ms() as f64;
        if stopped {
            app.pause();
        }
        if index != app.selected || stopped {
            app.selected = index;
            app.display(true)?;
        }
        Ok(())
    })
}
fn begin_file(id: &'static str) {
    let prepare = state(|app| {
        if app.recovery.locked {
            return Err("Wait for the recovery operation to finish before opening a file.".into());
        }
        // A newly selected file supersedes any earlier asynchronous read.
        if app.dirty {
            return Err("Apply or discard frame edits before opening a file.".into());
        }
        if app.camera.is_some() {
            return Err("Turn the camera off before opening a file.".into());
        }
        let file = input(id)?
            .files()
            .and_then(|files| files.get(0))
            .ok_or("Choose a file first")?;
        let limit = if id == "font-file" {
            1024 * 1024
        } else {
            MAX_INPUT_BYTES
        };
        if file.size() > limit as f64 {
            return Err(format!("File exceeds the {} KiB input limit", limit / 1024));
        }
        let options = if id == "media-file" {
            Some(render_options()?)
        } else {
            None
        };
        app.pause();
        app.loading = true;
        app.ticket = app.ticket.wrapping_add(1);
        app.display(false)?;
        status("Reading the file locally...", false);
        Ok((file, app.ticket, options))
    });
    let (file, ticket, options) = match prepare {
        Ok(value) => value,
        Err(error) => {
            status(&error, true);
            return;
        }
    };
    spawn_local(async move {
        let read = JsFuture::from(file.array_buffer())
            .await
            .map_err(|_| "The selected file could not be read".to_string());
        let result = state(|app| {
            if ticket != app.ticket {
                return Ok(());
            }
            app.loading = false;
            let bytes = Uint8Array::new(&read?).to_vec();
            if bytes.len() > MAX_INPUT_BYTES {
                return Err("File exceeds 16 MiB".into());
            }
            if id == "font-file" {
                let contents = String::from_utf8(bytes)
                    .map_err(|_| "Font must be plain UTF-8 .flf or .tlf text")?;
                lettering::validate_font(&contents)?;
                app.font = Some(contents);
                node::<Element>("custom-option")?
                    .remove_attribute("disabled")
                    .map_err(|_| "Cannot enable font")?;
                node::<HtmlSelectElement>("font")?.set_value("custom");
                status("Font loaded. Generate lettering to use it.", false);
            } else {
                let project = if let Some(options) = options {
                    decode_media(&bytes, &options)?
                } else {
                    let text = String::from_utf8(bytes)
                        .map_err(|_| "Text must contain printable ASCII")?;
                    if file.name().to_ascii_lowercase().ends_with(".json") {
                        Animation::from_json(&text)?
                    } else {
                        Animation::still(Frame::from_text_padded(&text.replace("\r\n", "\n"))?)
                    }
                };
                app.commit(project);
                app.seek(0);
                input("width")?.set_value(&app.project.width().to_string());
                input("height")?.set_value(&app.project.height().to_string());
                status(
                    "File opened as ASCII text. Save JSON to retain every frame.",
                    false,
                );
            }
            app.display(true)
        });
        if let Err(error) = result {
            status(&error, true);
            report(state(|app| app.display(false)));
        }
    });
}
/// Fill the conversion settings from the chosen preset. Inversion stays as it is: that suits
/// the page the text will sit on, not the picture.
fn apply_preset() -> Result<()> {
    let (ramp, dither, lines, contrast) = match select("preset")?.as_str() {
        "standard" => (RAMPS[0].1, false, false, "1"),
        "photo" => (RAMPS[1].1, true, false, "1"),
        "line-art" => (RAMPS[0].1, false, true, "1"),
        "poster" => (RAMPS[2].1, false, false, "1.5"),
        "custom" => return Ok(()),
        _ => return Err("Unknown conversion preset".into()),
    };
    input("ramp")?.set_value(ramp);
    input("dither")?.set_checked(dither);
    input("edges")?.set_checked(lines);
    input("edge-glyphs")?.set_checked(lines);
    input("gamma")?.set_value("1");
    input("contrast")?.set_value(contrast);
    status(
        "Preset applied. Convert a file, video or camera to use it.",
        false,
    );
    Ok(())
}
/// Validated conversion settings shared by image, video and camera input.
fn render_options() -> Result<RenderOptions> {
    let options = RenderOptions {
        width: number("width")?,
        height: if checked("auto-height")? {
            None
        } else {
            Some(number("height")?)
        },
        ramp: input("ramp")?.value(),
        invert: checked("invert")?,
        dither: checked("dither")?,
        edges: checked("edges")?,
        edge_glyphs: checked("edge-glyphs")?,
        gamma: decimal("gamma")?,
        contrast: decimal("contrast")?,
        ..RenderOptions::default()
    };
    options.validate()?;
    Ok(options)
}
fn download(content: &str, name: &str, mime: &str) -> Result<()> {
    let parts = Array::new();
    parts.push(&JsValue::from_str(content));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
        .map_err(|_| "Cannot create download")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Cannot create download URL")?;
    let anchor: HtmlAnchorElement = document()
        .create_element("a")
        .map_err(|_| "Cannot create download link")?
        .dyn_into()
        .map_err(|_| "Cannot create download link")?;
    anchor.set_href(&url);
    anchor.set_download(name);
    document()
        .body()
        .ok_or("Document body is missing")?
        .append_child(&anchor)
        .map_err(|_| "Cannot attach download")?;
    anchor.click();
    anchor.remove();
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    web_sys::window()
        .expect("Browser window")
        .set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 1000)
        .map_err(|_| "Cannot schedule download cleanup")?;
    Ok(())
}
