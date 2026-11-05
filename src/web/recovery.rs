// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! One opt-in recovery slot. Conditional writes run entirely within an IndexedDB
//! transaction; a fresh revision also distinguishes identical artwork from another tab.
use super::*;
use js_sys::{JsString, Promise};
use std::{cell::Cell, rc::Rc};
use web_sys::{IdbDatabase, IdbRequest, IdbTransaction, IdbTransactionMode};

const DATABASE: &str = "apollinaire-recovery-v1";
const STORE: &str = "projects";
const KEY: &str = "applied-project";
const DEBOUNCE_MS: f64 = 750.0;
thread_local! { static GENERATION: Cell<u64> = const { Cell::new(0) }; }

pub(super) struct Recovery {
    ready: bool,
    busy: bool,
    pub(super) locked: bool,
    enabled: bool,
    owned: bool,
    record: JsValue,
    present: bool,
    restorable: bool,
    revision: u64,
    saved_revision: u64,
    changed_at: f64,
    generation: u64,
    message: String,
    phase: &'static str,
}
impl Default for Recovery {
    fn default() -> Self {
        Self {
            ready: false,
            busy: false,
            locked: false,
            enabled: false,
            owned: false,
            record: JsValue::UNDEFINED,
            present: false,
            restorable: false,
            revision: 0,
            saved_revision: 0,
            changed_at: 0.0,
            generation: GENERATION.with(|n| {
                n.set(n.get() + 1);
                n.get()
            }),
            message: "Checking for saved recovery...".into(),
            phase: "checking",
        }
    }
}
impl Recovery {
    pub(super) fn changed(&mut self) {
        self.revision += 1;
        self.changed_at = now();
        if self.enabled && !self.busy {
            self.message(
                "Applied project changes are waiting to be saved locally.",
                "pending",
            );
        }
    }
    fn message(&mut self, message: &str, phase: &'static str) {
        self.message = message.into();
        self.phase = phase;
    }
    fn observe(&mut self, record: JsValue) {
        let decoded = decode_record(&record);
        self.present = !matches!(decoded, Ok(None));
        self.restorable = matches!(decoded, Ok(Some(_)));
        self.record = record;
        self.owned = false;
        self.message(
            if self.restorable { "A saved project is available. Restore it or discard it before enabling recovery." }
            else if self.present { "Saved recovery is invalid or too large. Discard it to start again; the current project is unchanged." }
            else { "Recovery is off. No applied project is saved here." },
            if self.present { "available" } else { "off" },
        );
    }
    fn conflict(&mut self, current: JsValue) {
        self.enabled = false;
        self.observe(current);
        self.message("Recovery stopped: another tab changed the saved record, or browser data was cleared. Restore or discard any saved project before enabling recovery again.", "conflict");
    }
    pub(super) fn controls(&self, editor_busy: bool) -> Result<()> {
        input("recovery-enabled")?.set_checked(self.enabled);
        for (id, disabled) in [
            (
                "recovery-enabled",
                !self.ready || self.locked || (self.busy && !self.enabled),
            ),
            (
                "restore-recovery",
                !self.ready || self.busy || editor_busy || !self.restorable,
            ),
            (
                "discard-recovery",
                !self.ready || self.busy || editor_busy || !self.present,
            ),
        ] {
            let element = node::<Element>(id)?;
            if disabled {
                element.set_attribute("disabled", "disabled")
            } else {
                element.remove_attribute("disabled")
            }
            .map_err(|_| "Cannot update recovery controls")?;
        }
        let label = node::<Element>("recovery-status")?;
        if label.text_content().as_deref() != Some(self.message.as_str()) {
            label.set_text_content(Some(&self.message));
            // The panel starts collapsed, so a found project or stopped recovery also goes here.
            if matches!(self.phase, "available" | "conflict" | "error") {
                status(&self.message, self.phase != "available");
            }
        }
        label
            .set_attribute("data-state", self.phase)
            .map_err(|_| "Cannot update recovery status")?;
        Ok(())
    }
}

fn storage_error(value: JsValue) -> String {
    let name = value.dyn_ref::<web_sys::DomException>().map(|e| e.name());
    format!(
        "Local recovery storage failed ({}). Save JSON for a separate copy.",
        name.as_deref().unwrap_or("browser error")
    )
}
fn encode_record(project: Option<&Animation>) -> Result<JsValue> {
    let json = project.map(Animation::to_json).unwrap_or_default();
    if json.len() > MAX_INPUT_BYTES {
        return Err(
            "Recovery JSON exceeds 16 MiB. Recovery stopped; save the project separately.".into(),
        );
    }
    let mut bytes = [0; 16];
    web_sys::window()
        .ok_or("Browser window is unavailable")?
        .crypto()
        .map_err(storage_error)?
        .get_random_values_with_u8_array(&mut bytes)
        .map_err(storage_error)?;
    let revision: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    // An empty project is a revision-only discard marker, preventing stale empty-slot writers.
    Ok(format!("{revision}\n{json}").into())
}
fn decode_record(record: &JsValue) -> Result<Option<Animation>> {
    if record.is_undefined() {
        return Ok(None);
    }
    if !record.is_string()
        || JsString::from(record.clone()).length() as usize > MAX_INPUT_BYTES + 129
    {
        return Err("Saved recovery is not a bounded project record".into());
    }
    let record = record.as_string().ok_or("Saved recovery is not text")?;
    let (revision, json) = record
        .split_once('\n')
        .ok_or("Saved recovery has no revision")?;
    if revision.is_empty() || revision.len() > 128 || !revision.is_ascii() {
        return Err("Saved recovery has an invalid revision".into());
    }
    if json.is_empty() {
        return Ok(None);
    }
    Animation::from_json(json).map(Some)
}

/// Request listeners live only until the request settles, including error paths.
async fn request(request: &IdbRequest) -> Result<JsValue> {
    let mut success = None;
    let mut failure = None;
    let promise = Promise::new(&mut |resolve, reject| {
        let read = request.clone();
        success = Some(Closure::<dyn FnMut(Event)>::new(move |_| {
            if let Ok(value) = read.result() {
                let _ = resolve.call1(&JsValue::NULL, &value);
            }
        }));
        let read = request.clone();
        failure = Some(Closure::<dyn FnMut(Event)>::new(move |_| {
            let error: JsValue = read
                .error()
                .ok()
                .flatten()
                .map(Into::into)
                .unwrap_or(JsValue::NULL);
            let _ = reject.call1(&JsValue::NULL, &error);
        }));
        request.set_onsuccess(success.as_ref().map(|c| c.as_ref().unchecked_ref()));
        request.set_onerror(failure.as_ref().map(|c| c.as_ref().unchecked_ref()));
    });
    let result = JsFuture::from(promise).await.map_err(storage_error);
    request.set_onsuccess(None);
    request.set_onerror(None);
    result
}
struct Database {
    db: IdbDatabase,
    _version_change: Closure<dyn FnMut(Event)>,
}
impl Drop for Database {
    fn drop(&mut self) {
        self.db.set_onversionchange(None);
        self.db.close();
    }
}
async fn database() -> Result<Database> {
    let factory = web_sys::window()
        .ok_or("Browser window is unavailable")?
        .indexed_db()
        .map_err(storage_error)?
        .ok_or("This browser does not provide local recovery storage")?;
    let open = factory.open(DATABASE).map_err(storage_error)?;
    let upgrade_request = open.clone();
    let upgrade = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Ok(value) = upgrade_request.result()
            && let Some(db) = value.dyn_ref::<IdbDatabase>()
        {
            // The database has a versioned name; only its initial creation upgrades it.
            let _ = db.create_object_store(STORE);
        }
    });
    open.set_onupgradeneeded(Some(upgrade.as_ref().unchecked_ref()));
    let blocked = Closure::<dyn FnMut(Event)>::new(move |_| {
        report(state(|app| {
            app.recovery.message(
                "Recovery storage is blocked. Close other editor tabs and reload to try again.",
                "error",
            );
            app.display(false)
        }));
    });
    open.set_onblocked(Some(blocked.as_ref().unchecked_ref()));
    let result = request(&open).await;
    open.set_onupgradeneeded(None);
    open.set_onblocked(None);
    let db: IdbDatabase = result?
        .dyn_into()
        .map_err(|_| "Recovery database did not open")?;
    let connection = db.clone();
    let version_change = Closure::<dyn FnMut(Event)>::new(move |_| connection.close());
    db.set_onversionchange(Some(version_change.as_ref().unchecked_ref()));
    Ok(Database {
        db,
        _version_change: version_change,
    })
}
pub(super) async fn read_record() -> Result<JsValue> {
    let database = database().await?;
    let transaction = database
        .db
        .transaction_with_str(STORE)
        .map_err(storage_error)?;
    let store = transaction.object_store(STORE).map_err(storage_error)?;
    request(&store.get(&KEY.into()).map_err(storage_error)?).await
}
#[derive(Debug)]
pub(super) enum WriteError {
    Conflict(JsValue),
    Storage(String),
}
impl From<String> for WriteError {
    fn from(error: String) -> Self {
        Self::Storage(error)
    }
}
async fn transaction_done(transaction: &IdbTransaction) -> Result<()> {
    let mut success = None;
    let mut failure = None;
    let promise = Promise::new(&mut |resolve, reject| {
        success = Some(Closure::<dyn FnMut(Event)>::new(move |_| {
            let _ = resolve.call0(&JsValue::NULL);
        }));
        let tx = transaction.clone();
        failure = Some(Closure::<dyn FnMut(Event)>::new(move |_| {
            let error: JsValue = tx.error().map(Into::into).unwrap_or(JsValue::NULL);
            let _ = reject.call1(&JsValue::NULL, &error);
        }));
        transaction.set_oncomplete(success.as_ref().map(|c| c.as_ref().unchecked_ref()));
        transaction.set_onabort(failure.as_ref().map(|c| c.as_ref().unchecked_ref()));
    });
    let result = JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(storage_error);
    transaction.set_oncomplete(None);
    transaction.set_onabort(None);
    result
}
/// Text records and the empty slot compare by value. Each read of any other value is a fresh
/// structured clone, so all such invalid records count as one; replacing one never erases text.
fn same_record(current: &JsValue, expected: &JsValue) -> bool {
    let invalid = |value: &JsValue| !value.is_string() && !value.is_undefined();
    current == expected || (invalid(current) && invalid(expected))
}
pub(super) async fn replace_record(
    expected: JsValue,
    value: JsValue,
) -> std::result::Result<(), WriteError> {
    let database = database().await?;
    let transaction = database
        .db
        .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
        .map_err(storage_error)?;
    let store = transaction.object_store(STORE).map_err(storage_error)?;
    let get = store.get(&KEY.into()).map_err(storage_error)?;
    // Shared only with this asynchronous event handler; no App borrow crosses an await.
    let error = Rc::new(RefCell::new(None));
    let outcome = error.clone();
    let read = get.clone();
    let tx = transaction.clone();
    let compare = Closure::<dyn FnMut(Event)>::own_assert_unwind_safe(move |_| {
        match read.result() {
            Ok(current) if !same_record(&current, &expected) => {
                *outcome.borrow_mut() = Some(WriteError::Conflict(current));
                let _ = tx.abort();
            }
            Ok(_) => {
                // Stay in get-success: awaiting here could deactivate the transaction.
                if let Err(failure) = store.put_with_key(&value, &KEY.into()) {
                    *outcome.borrow_mut() = Some(WriteError::Storage(storage_error(failure)));
                    let _ = tx.abort();
                }
            }
            Err(failure) => {
                *outcome.borrow_mut() = Some(WriteError::Storage(storage_error(failure)));
                let _ = tx.abort();
            }
        }
    });
    get.set_onsuccess(Some(compare.as_ref().unchecked_ref()));
    let result = transaction_done(&transaction).await;
    get.set_onsuccess(None);
    if let Some(error) = error.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(Into::into)
}

enum Operation {
    Save,
    Restore(Animation),
    Discard,
}
fn launch(app: &mut App, operation: Operation) -> Result<()> {
    if !app.recovery.ready || app.recovery.busy {
        return Err("Wait for the recovery operation to finish".into());
    }
    if !matches!(operation, Operation::Save) {
        app.clean()?;
        app.pause();
        app.drawing_cancel();
    }
    let restoring = matches!(operation, Operation::Restore(_));
    let discarded = matches!(operation, Operation::Discard);
    let value = match &operation {
        Operation::Save => encode_record(Some(&app.project))?,
        Operation::Restore(project) => encode_record(Some(project))?,
        Operation::Discard => encode_record(None)?,
    };
    let r = &mut app.recovery;
    let expected = r.record.clone();
    let revision = r.revision;
    let generation = r.generation;
    r.busy = true;
    r.locked = !matches!(operation, Operation::Save);
    r.message("Updating local project recovery...", "saving");
    app.display(false)?;
    spawn_local(async move {
        let result = replace_record(expected, value.clone()).await;
        report(state(|app| {
            if app.recovery.generation != generation {
                return Ok(());
            }
            app.recovery.busy = false;
            app.recovery.locked = false;
            match result {
                Ok(()) => {
                    match operation {
                        Operation::Restore(project) => {
                            app.commit(project);
                            app.seek(0);
                            input("width")?.set_value(&app.project.width().to_string());
                            input("height")?.set_value(&app.project.height().to_string());
                            app.recovery.enabled = true;
                            app.recovery.saved_revision = app.recovery.revision;
                        }
                        Operation::Save => app.recovery.saved_revision = revision,
                        Operation::Discard => app.recovery.enabled = false,
                    }
                    let r = &mut app.recovery;
                    r.record = value;
                    r.present = !discarded;
                    r.restorable = !discarded;
                    r.owned = !discarded;
                    r.message(
                        if discarded { "Saved recovery discarded. Recovery is off." }
                        else if r.enabled && r.revision != r.saved_revision { "New applied changes are waiting to be saved locally." }
                        else if r.enabled { "Applied project saved locally. Unapplied edits and session stamps are excluded." }
                        else { "Recovery paused. The last saved project remains until you discard it." },
                        if discarded { "off" } else if r.enabled && r.revision != r.saved_revision { "pending" } else if r.enabled { "saved" } else { "paused" },
                    );
                }
                Err(WriteError::Conflict(current)) => app.recovery.conflict(current),
                Err(WriteError::Storage(error)) => {
                    app.recovery.enabled = false;
                    app.recovery.message(&error, "error");
                }
            }
            app.display(restoring)
        }));
    });
    Ok(())
}

pub(super) fn tick(app: &mut App, force: bool) {
    let r = &app.recovery;
    if r.enabled
        && !r.busy
        && r.revision != r.saved_revision
        && (force || now() - r.changed_at >= DEBOUNCE_MS)
        && let Err(error) = launch(app, Operation::Save)
    {
        app.recovery.enabled = false;
        app.recovery.message(&error, "error");
        report(app.display(false));
    }
}
/// Another tab or browser clearing can replace the slot whilst this tab has nothing to write,
/// so a tab keeping recovery rereads it when the user returns.
fn refresh(app: &mut App) -> Result<()> {
    let r = &app.recovery;
    if r.busy || !(r.enabled || r.owned) {
        return Ok(());
    }
    let expected = r.record.clone();
    spawn_local(async move {
        let result = read_record().await;
        report(state(|app| {
            let r = &mut app.recovery;
            // A write launched since the read checks the slot itself.
            if r.busy || r.record != expected {
                return Ok(());
            }
            match result {
                Ok(current) if current == expected => return Ok(()),
                Ok(current) => r.conflict(current),
                Err(error) => {
                    r.enabled = false;
                    r.message(&error, "error");
                }
            }
            app.display(false)
        }));
    });
    Ok(())
}
fn command(id: &str) -> Result<()> {
    state(|app| {
        let result = match id {
            "recovery-enabled" if !checked(id)? => {
                app.recovery.enabled = false;
                app.recovery.message(
                    if app.recovery.busy {
                        "Recovery paused. A write already in progress may still finish."
                    } else {
                        "Recovery paused. The last saved project remains until you discard it."
                    },
                    "paused",
                );
                Ok(())
            }
            "recovery-enabled" => {
                if app.recovery.present && !app.recovery.owned {
                    Err("Restore or discard the saved project before enabling recovery".into())
                } else {
                    app.recovery.enabled = true;
                    launch(app, Operation::Save)
                }
            }
            "restore-recovery" => {
                let project =
                    decode_record(&app.recovery.record)?.ok_or("No saved project is available")?;
                launch(app, Operation::Restore(project))
            }
            _ => launch(app, Operation::Discard),
        };
        if let Err(error) = result {
            app.recovery.enabled = false;
            app.recovery.message(&error, "error");
        }
        app.display(false)
    })
}
pub(super) fn start() -> std::result::Result<(), JsValue> {
    listen("recovery-enabled", "change", |_| {
        report(command("recovery-enabled"))
    })?;
    for id in ["restore-recovery", "discard-recovery"] {
        listen(id, "click", move |_| report(command(id)))?;
    }
    let returned = Closure::<dyn FnMut(Event)>::new(|_| {
        if !document().hidden() {
            report(state(refresh));
        }
    });
    let callback = returned.as_ref().unchecked_ref();
    document().add_event_listener_with_callback("visibilitychange", callback)?;
    web_sys::window()
        .expect("Browser window")
        .add_event_listener_with_callback("focus", callback)?;
    // Lives for the lifetime of this single-page app.
    returned.forget();
    let generation = state(|app| {
        app.recovery.busy = true;
        Ok(app.recovery.generation)
    })
    .map_err(|e| JsValue::from_str(&e))?;
    spawn_local(async move {
        let result = read_record().await;
        report(state(|app| {
            if app.recovery.generation != generation {
                return Ok(());
            }
            app.recovery.ready = true;
            app.recovery.busy = false;
            match result {
                Ok(record) => app.recovery.observe(record),
                Err(error) => app.recovery.message(&error, "error"),
            }
            app.display(false)
        }));
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn recovery_records_validate_projects_and_bound_both_parts() {
        let project = Animation::still(Frame::from_text("ASCII").unwrap());
        let encoded = encode_record(Some(&project)).unwrap();
        assert_eq!(decode_record(&encoded).unwrap(), Some(project.clone()));
        assert_eq!(decode_record(&encode_record(None).unwrap()).unwrap(), None);
        assert_eq!(decode_record(&JsValue::UNDEFINED).unwrap(), None);
        for invalid in [
            JsValue::NULL,
            "bad".into(),
            "\n{}".into(),
            format!("{}\n{}", "x".repeat(129), project.to_json()).into(),
            "revision\n{}".into(),
            format!("revision\n{}", project.to_json().replace("ASCII", "é")).into(),
            "x".repeat(MAX_INPUT_BYTES + 130).into(),
        ] {
            assert!(decode_record(&invalid).is_err());
        }
    }
}
