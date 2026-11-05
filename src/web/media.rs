// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Video files and the camera become timed text frames. Pixels are read through an
//! offscreen canvas that is never shown or exported; only characters reach the page.
use super::{
    App, document, input, listen, node, now, number, render_options, report, state, status,
};
use crate::{Animation, Frame, RenderOptions, Result, TimedFrame, video};
use js_sys::{Function, Object, Promise, Reflect};
use std::cell::{Cell, RefCell};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    AbortController, AddEventListenerOptions, DomException, Element, Event, EventTarget,
    HtmlProgressElement, HtmlVideoElement, MediaDevices, MediaStream, MediaStreamConstraints,
    MediaStreamTrack, OffscreenCanvas, OffscreenCanvasRenderingContext2d, Url, Window,
};

/// How long one media event may take before the read is abandoned.
const WAIT_MS: i32 = 15_000;

thread_local! {
    // One handler serves every camera session and does nothing while the camera is off.
    static TICK: Function = Closure::<dyn FnMut()>::new(|| report(tick()))
        .into_js_value()
        .unchecked_into();
    // Rejects the media wait in progress, so Cancel releases the video at once.
    static WAITING: RefCell<Option<Function>> = const { RefCell::new(None) };
    // The ticket of a camera permission request the browser has not answered yet.
    static REQUEST: Cell<Option<u32>> = const { Cell::new(None) };
}

pub(super) struct Camera {
    id: u32,
    stream: MediaStream,
    video: HtmlVideoElement,
    /// Created once the camera reports its picture size.
    reader: Option<Reader>,
    options: RenderOptions,
    fps: u32,
    timer: i32,
    recording: Option<Recording>,
}
struct Recording {
    seconds: u32,
    limit: usize,
    times: Vec<f64>,
    frames: Vec<Frame>,
}

/// Reads one video frame into a fixed pixel size for the converter.
struct Reader {
    context: OffscreenCanvasRenderingContext2d,
    width: u32,
    height: u32,
}
impl Reader {
    fn new(video: &HtmlVideoElement, columns: usize) -> Result<Self> {
        let (width, height) =
            video::sample_size(video.video_width(), video.video_height(), columns)?;
        let unsupported = "This browser cannot read video pixels";
        let canvas = OffscreenCanvas::new(width, height).map_err(|_| unsupported)?;
        let options = Object::new();
        Reflect::set(&options, &"willReadFrequently".into(), &JsValue::TRUE)
            .map_err(|_| unsupported)?;
        let context = canvas
            .get_context_with_context_options("2d", &options)
            .ok()
            .flatten()
            .ok_or(unsupported)?
            .dyn_into()
            .map_err(|_| unsupported)?;
        Ok(Self {
            context,
            width,
            height,
        })
    }
    fn rows(&self, options: &RenderOptions) -> usize {
        options.output_height(self.width, self.height)
    }
    fn read(&self, video: &HtmlVideoElement, options: &RenderOptions) -> Result<Frame> {
        let (width, height) = (f64::from(self.width), f64::from(self.height));
        self.context
            .draw_image_with_html_video_element_and_dw_and_dh(video, 0.0, 0.0, width, height)
            .map_err(|_| "The video frame could not be read")?;
        let pixels = self
            .context
            .get_image_data(0.0, 0.0, width, height)
            .map_err(|_| "The video frame could not be read")?;
        video::render_rgba(self.width, self.height, pixels.data().0, options)
    }
}

fn window() -> Window {
    web_sys::window().expect("Browser window")
}
fn video_element() -> Result<HtmlVideoElement> {
    let video: HtmlVideoElement = document()
        .create_element("video")
        .map_err(|_| "Cannot create a video reader")?
        .dyn_into()
        .map_err(|_| "Cannot create a video reader")?;
    video.set_muted(true);
    video
        .set_attribute("playsinline", "")
        .map_err(|_| "Cannot create a video reader")?;
    Ok(video)
}
/// Resolves on `event`; fails on a media error or after `WAIT_MS`.
async fn wait(target: &EventTarget, event: &str, failure: &str) -> Result<()> {
    let abort = AbortController::new().map_err(|_| "This browser cannot track media events")?;
    let mut timer = None;
    let mut own = None;
    let promise = Promise::new(&mut |resolve, reject| {
        let options = AddEventListenerOptions::new();
        options.set_once(true);
        options.set_signal(&abort.signal());
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            event, &resolve, &options,
        );
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            "error", &reject, &options,
        );
        timer = window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&reject, WAIT_MS)
            .ok();
        // Only the current conversion waits; an older wait belongs to a superseded one.
        if let Some(stale) = WAITING.with(|waiting| waiting.replace(Some(reject.clone()))) {
            let _ = stale.call0(&JsValue::NULL);
        }
        own = Some(reject);
    });
    let outcome = JsFuture::from(promise).await;
    WAITING.with(|waiting| {
        let mut waiting = waiting.borrow_mut();
        if waiting.as_ref() == own.as_ref() {
            *waiting = None;
        }
    });
    abort.abort();
    if let Some(timer) = timer {
        window().clear_timeout_with_handle(timer);
    }
    outcome.map(|_| ()).map_err(|_| failure.to_string())
}
fn fps() -> Result<u32> {
    u32::try_from(number("media-fps")?)
        .ok()
        .filter(|fps| (1..=video::MAX_FPS).contains(fps))
        .ok_or_else(|| format!("Frames per second must be 1..={}", video::MAX_FPS))
}
/// An optional time field in seconds, returned in milliseconds.
fn milliseconds(id: &str) -> Result<Option<f64>> {
    let value = input(id)?.value();
    if value.trim().is_empty() {
        return Ok(None);
    }
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(|s| Some(s * 1000.0))
        .ok_or_else(|| "Start and end must be numbers of seconds".into())
}
fn progress(done: Option<(f64, f64)>) {
    if let (Ok(row), Ok(bar)) = (
        node::<Element>("media-busy"),
        node::<HtmlProgressElement>("media-progress"),
    ) {
        if let Some((value, max)) = done {
            bar.set_max(max);
            bar.set_value(value);
            let _ = row.remove_attribute("hidden");
        } else {
            let _ = row.set_attribute("hidden", "");
        }
    }
}
fn set_disabled(id: &str, disabled: bool) -> Result<()> {
    let element = node::<Element>(id)?;
    if disabled {
        element.set_attribute("disabled", "disabled")
    } else {
        element.remove_attribute("disabled")
    }
    .map_err(|_| "Cannot update controls".into())
}
/// Enables the media controls for the editor's current state.
pub(super) fn controls(app: &App) -> Result<()> {
    let busy = app.loading || app.camera.is_some() || app.recovery.locked;
    let ready = app
        .camera
        .as_ref()
        .is_some_and(|c| c.reader.is_some() && c.recording.is_none());
    set_disabled("video-convert", busy)?;
    set_disabled("media-cancel", !app.loading)?;
    set_disabled("camera-start", busy)?;
    set_disabled("camera-record", !ready)?;
    set_disabled("camera-stop", app.camera.is_none())
}
fn begin(app: &mut App, message: &str) -> Result<u32> {
    app.pause();
    app.loading = true;
    app.ticket = app.ticket.wrapping_add(1);
    app.display(false)?;
    status(message, false);
    Ok(app.ticket)
}
fn current(ticket: u32) -> bool {
    state(|app| Ok(app.ticket == ticket)).unwrap_or(false)
}
/// Commits a finished capture unless it was cancelled or superseded.
fn finish(app: &mut App, project: Animation, message: &str) -> Result<()> {
    app.commit(project);
    app.seek(0);
    input("width")?.set_value(&app.project.width().to_string());
    input("height")?.set_value(&app.project.height().to_string());
    app.display(true)?;
    status(message, false);
    Ok(())
}

pub(super) fn start() -> std::result::Result<(), JsValue> {
    listen("video-convert", "click", |_| convert_video())?;
    listen("camera-start", "click", |_| start_camera())?;
    listen("camera-record", "click", |_| report(record()))?;
    listen("camera-stop", "click", |_| {
        report(state(|app| {
            let message = if app.camera.as_ref().is_some_and(|c| c.recording.is_some()) {
                "Camera off. The unfinished recording was discarded."
            } else {
                "Camera off."
            };
            stop(app);
            app.display(true)?;
            status(message, false);
            Ok(())
        }))
    })?;
    listen("media-cancel", "click", |_| {
        report(state(|app| {
            if !app.loading {
                return Ok(());
            }
            // A new ticket makes the pending task discard its result and release its media.
            app.ticket = app.ticket.wrapping_add(1);
            app.loading = false;
            REQUEST.set(None);
            if let Some(reject) = WAITING.with(|waiting| waiting.borrow_mut().take()) {
                let _ = reject.call0(&JsValue::NULL);
            }
            progress(None);
            app.display(false)?;
            status("Cancelled. The artwork is unchanged.", false);
            Ok(())
        }))
    })?;
    let hidden = Closure::<dyn FnMut(Event)>::new(|event: Event| {
        if event.type_() == "pagehide" || document().hidden() {
            report(state(|app| {
                if REQUEST.get() == Some(app.ticket) && app.loading {
                    // A permission answer arriving later finds a stale ticket and stops its tracks.
                    REQUEST.set(None);
                    app.ticket = app.ticket.wrapping_add(1);
                    app.loading = false;
                    app.display(false)?;
                    status(
                        "The camera request was cancelled because the page was hidden.",
                        false,
                    );
                }
                if stop(app) {
                    app.display(true)?;
                    status(
                        "The camera turned off because the page was hidden. Any unfinished recording was discarded.",
                        false,
                    );
                }
                Ok(())
            }));
        }
    });
    let callback: &Function = hidden.as_ref().unchecked_ref();
    document().add_event_listener_with_callback("visibilitychange", callback)?;
    window().add_event_listener_with_callback("pagehide", callback)?;
    // Lives for the lifetime of this single-page app.
    hidden.forget();
    Ok(())
}

fn convert_video() {
    let prepare = state(|app| {
        app.clean()?;
        let file = input("video-file")?
            .files()
            .and_then(|files| files.get(0))
            .ok_or("Choose a video file first")?;
        let options = render_options()?;
        let fps = fps()?;
        let start = milliseconds("video-start")?.unwrap_or(0.0);
        let end = milliseconds("video-end")?;
        let ticket = begin(app, "Reading the video in this browser...")?;
        progress(Some((0.0, 1.0)));
        Ok((file, ticket, options, fps, start, end))
    });
    let (file, ticket, options, fps, start, end) = match prepare {
        Ok(value) => value,
        Err(error) => return status(&error, true),
    };
    spawn_local(async move {
        let result = match (Url::create_object_url_with_blob(&file), video_element()) {
            (Ok(url), Ok(video)) => {
                let result = sample(&video, &url, ticket, &options, fps, start, end).await;
                // Release the decoder and the file reference whatever the outcome.
                let _ = video.remove_attribute("src");
                video.load();
                let _ = Url::revoke_object_url(&url);
                result
            }
            _ => Err("The video could not be opened".into()),
        };
        let outcome = state(|app| {
            if ticket != app.ticket {
                return Ok(());
            }
            app.loading = false;
            progress(None);
            let project = result?;
            let message = format!(
                "Video converted to {} text frames. Save JSON to keep every frame and its timing.",
                project.frames().len()
            );
            finish(app, project, &message)
        });
        if let Err(error) = outcome {
            status(&error, true);
            report(state(|app| app.display(false)));
        }
    });
}
async fn sample(
    video: &HtmlVideoElement,
    url: &str,
    ticket: u32,
    options: &RenderOptions,
    fps: u32,
    start_ms: f64,
    end_ms: Option<f64>,
) -> Result<Animation> {
    video.set_preload("auto");
    video.set_src(url);
    wait(
        video,
        "loadedmetadata",
        "This browser cannot play the selected video",
    )
    .await?;
    let duration = video.duration() * 1000.0;
    let end = end_ms.unwrap_or(duration);
    let times = video::sample_times(duration, start_ms, end, fps)?;
    let durations = video::frame_durations(&times, end)?;
    let reader = Reader::new(video, options.width)?;
    video::check_capture(options.width, reader.rows(options), times.len())?;
    let mut frames = Vec::with_capacity(times.len());
    for (index, (time, duration_ms)) in times.iter().zip(durations).enumerate() {
        if !current(ticket) {
            return Err("Cancelled".into());
        }
        progress(Some((index as f64, times.len() as f64)));
        video.set_current_time(time / 1000.0);
        let failure = format!("The video could not be read at {:.2} s", time / 1000.0);
        wait(video, "seeked", &failure).await?;
        if !current(ticket) {
            return Err("Cancelled".into());
        }
        frames.push(TimedFrame {
            frame: reader.read(video, options)?,
            duration_ms,
        });
    }
    Animation::new(frames, true)
}

fn start_camera() {
    let prepare = state(|app| {
        app.clean()?;
        let options = render_options()?;
        let fps = fps()?;
        let devices: MediaDevices = Reflect::get(&window().navigator(), &"mediaDevices".into())
            .ok()
            .filter(|devices| !devices.is_undefined())
            .ok_or("Camera access needs a secure (HTTPS) page in a browser that supports it")?
            .unchecked_into();
        let constraints = MediaStreamConstraints::new();
        constraints.set_video_bool(true);
        constraints.set_audio_bool(false);
        // The request is owned by its ticket before the browser is asked.
        let ticket = begin(
            app,
            "Waiting for camera permission. Frames stay in this browser.",
        )?;
        REQUEST.set(Some(ticket));
        match devices.get_user_media_with_constraints(&constraints) {
            Ok(request) => Ok((request, ticket, options, fps)),
            Err(_) => {
                REQUEST.set(None);
                app.loading = false;
                app.display(false)?;
                Err("The camera could not be requested".into())
            }
        }
    });
    let (request, ticket, options, fps) = match prepare {
        Ok(value) => value,
        Err(error) => return status(&error, true),
    };
    spawn_local(async move {
        let answer = JsFuture::from(request).await;
        report(state(|app| {
            let stream: MediaStream = match answer {
                Ok(stream) => stream.unchecked_into(),
                Err(error) => {
                    if ticket != app.ticket {
                        return Ok(());
                    }
                    REQUEST.set(None);
                    app.loading = false;
                    app.display(false)?;
                    let name = error.dyn_ref::<DomException>().map(|e| e.name());
                    return Err(match name.as_deref() {
                        Some("NotAllowedError") => {
                            "Camera permission was refused. Allow it in the browser's site settings to try again."
                        }
                        Some("NotFoundError" | "OverconstrainedError") => "No camera was found.",
                        Some("NotReadableError") => "The camera is in use by another application.",
                        _ => "The camera could not start.",
                    }
                    .into());
                }
            };
            if ticket != app.ticket {
                stop_tracks(&stream);
                return Ok(());
            }
            REQUEST.set(None);
            app.loading = false;
            if document().hidden() {
                stop_tracks(&stream);
                app.display(false)?;
                status("The camera stayed off because the page was hidden.", false);
                return Ok(());
            }
            let opened = video_element().and_then(|video| {
                video.set_src_object(Some(&stream));
                let play = video
                    .play()
                    .map_err(|_| "The camera stream could not start")?;
                let timer = TICK
                    .with(|tick| {
                        window().set_interval_with_callback_and_timeout_and_arguments_0(
                            tick,
                            (1000 / fps) as i32,
                        )
                    })
                    .map_err(|_| "Cannot schedule camera frames")?;
                Ok((video, play, timer))
            });
            let (video, play, timer) = match opened {
                Ok(value) => value,
                Err(error) => {
                    stop_tracks(&stream);
                    app.display(false)?;
                    return Err(error);
                }
            };
            // The camera is owned, and so stoppable, before playback starts.
            app.camera = Some(Camera {
                id: ticket,
                stream,
                video,
                reader: None,
                options,
                fps,
                timer,
                recording: None,
            });
            spawn_local(async move {
                if JsFuture::from(play).await.is_err() {
                    report(state(|app| {
                        if app.camera.as_ref().is_some_and(|c| c.id == ticket) {
                            stop(app);
                            app.display(true)?;
                            return Err("The camera stream could not start".into());
                        }
                        Ok(())
                    }));
                }
            });
            app.display(false)?;
            status(
                "Camera on. The preview is text; frames stay in this browser. Choose Record to capture them.",
                false,
            );
            Ok(())
        }));
    });
}
fn stop_tracks(stream: &MediaStream) {
    for track in stream.get_tracks().iter() {
        track.unchecked_into::<MediaStreamTrack>().stop();
    }
}
/// Stops every camera track and the frame timer. Returns whether a camera was on.
fn stop(app: &mut App) -> bool {
    let Some(camera) = app.camera.take() else {
        return false;
    };
    window().clear_interval_with_handle(camera.timer);
    stop_tracks(&camera.stream);
    camera.video.set_src_object(None);
    progress(None);
    true
}
fn record() -> Result<()> {
    state(|app| {
        let camera = app.camera.as_mut().ok_or("Turn the camera on first")?;
        if camera.recording.is_some() {
            return Err("A recording is already in progress.".into());
        }
        let seconds = u32::try_from(number("camera-seconds")?)
            .ok()
            .filter(|s| (1..=60).contains(s))
            .ok_or("Recording length must be 1..=60 seconds")?;
        let limit = (seconds * camera.fps) as usize;
        let reader = camera
            .reader
            .as_ref()
            .ok_or("Wait for the camera picture to appear")?;
        video::check_capture(camera.options.width, reader.rows(&camera.options), limit)?;
        camera.recording = Some(Recording {
            seconds,
            limit,
            times: Vec::with_capacity(limit),
            frames: Vec::with_capacity(limit),
        });
        progress(Some((0.0, f64::from(seconds) * 1000.0)));
        controls(app)?;
        status(
            &format!("Recording {seconds} seconds from the camera."),
            false,
        );
        Ok(())
    })
}
fn tick() -> Result<()> {
    state(|app| {
        let Some(camera) = app.camera.as_mut() else {
            return Ok(());
        };
        if camera.reader.is_none() && camera.stream.active() {
            if camera.video.video_width() == 0 {
                return Ok(());
            }
            match Reader::new(&camera.video, camera.options.width) {
                Ok(reader) => {
                    node::<Element>("dimensions")?.set_text_content(Some(&format!(
                        "{} columns x {} rows, live camera",
                        camera.options.width,
                        reader.rows(&camera.options)
                    )));
                    camera.reader = Some(reader);
                    return controls(app);
                }
                Err(error) => {
                    stop(app);
                    app.display(true)?;
                    return Err(error);
                }
            }
        }
        let frame = match (&camera.reader, camera.stream.active()) {
            (Some(reader), true) => reader.read(&camera.video, &camera.options),
            _ => Err("The camera stopped.".into()),
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(error) => {
                stop(app);
                app.display(true)?;
                return Err(error);
            }
        };
        node::<Element>("art")?.set_text_content(Some(&frame.text()));
        let Some(recording) = camera.recording.as_mut() else {
            return Ok(());
        };
        let time = now();
        let span = f64::from(recording.seconds) * 1000.0;
        let end = recording
            .times
            .first()
            .map_or(time + span, |first| first + span);
        if recording.frames.len() < recording.limit
            && video::capture_due(recording.times.last().copied(), time, end)
        {
            recording.times.push(time);
            recording.frames.push(frame);
        }
        progress(Some((span - (end - time), span)));
        if time < end {
            return Ok(());
        }
        let frames = std::mem::take(&mut recording.frames);
        let project = video::frame_durations(&recording.times, end).and_then(|durations| {
            Animation::new(
                frames
                    .into_iter()
                    .zip(durations)
                    .map(|(frame, duration_ms)| TimedFrame { frame, duration_ms })
                    .collect(),
                true,
            )
        });
        stop(app);
        let project = match project {
            Ok(project) => project,
            Err(error) => {
                app.display(true)?;
                return Err(error);
            }
        };
        let message = format!(
            "Recorded {} camera frames and turned the camera off. Save JSON to keep them.",
            project.frames().len()
        );
        finish(app, project, &message)
    })
}
