//! File transport without paths, pickers or a second document lifecycle.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

use super::MAX_FILE_BYTES;
use crate::WordApp;

const MAX_FILE_NAME_BYTES: usize = 255;

pub(super) fn document_bytes(app: &WordApp, params: &Value) -> Result<Value, String> {
    if !app.has_document() {
        return Err("no document is open".into());
    }
    let format = match params.get("format") {
        None => "docx",
        Some(v) => ["docx", "pdf"].into_iter().find(|f| v.as_str() == Some(f)).ok_or("format must be docx or pdf")?,
    };
    let name = format!("{}.{format}", app.title_stem());
    let password = app.session.password.as_ref().map(wordcraft_engine::Password::as_str);
    let opts = wordcraft_engine::cmd::file::export_options(&app.session, &json!({}));
    let bytes = wordcraft_engine::io::save_bytes_with(&name, &app.session.doc, password, &opts)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("document exceeds the host file limit".into());
    }
    Ok(json!({"name": name, "format": format, "bytes": STANDARD.encode(bytes), "revision": app.session.rev()}))
}

pub(super) fn open(app: &mut WordApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).ok_or("name is required")?;
    // A browser File has a basename. Do not let host input become a later native save path.
    if name.len() > MAX_FILE_NAME_BYTES || name.contains(['/', '\\', '\0']) || matches!(name, "." | "..") {
        return Err(format!("name must be a file basename of at most {MAX_FILE_NAME_BYTES} bytes"));
    }
    let text = params.get("bytes").and_then(Value::as_str).ok_or("bytes must be base64 text")?;
    if text.len() > MAX_FILE_BYTES.div_ceil(3).saturating_mul(4) {
        return Err("file exceeds the host file limit".into());
    }
    // Validate before asking to replace anything; use the engine's own decoder/importer afterwards.
    let bytes = STANDARD.decode(text).map_err(|_| "bytes must be valid standard base64")?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("file exceeds the host file limit".into());
    }
    // Reject unreadable/encrypted imports before the user is asked to discard work. The app's
    // user path would otherwise open its Password dialog; this API only returns an error.
    if !bytes.is_empty() {
        wordcraft_engine::io::open_bytes_in(name, &bytes, None, &app.session.region())?;
    }
    // This answer supersedes a picker request, including a late native chooser result.
    let _ = cancel(app, ctx);
    if let Some(pending) = save_changes_first(app, "file.open", json!({"path": name, "data": text})) {
        return Ok(pending);
    }
    let result = app.run("file.open", json!({"path": name, "data": text}))?;
    app.ui.backstage = false;
    Ok(result)
}

/// Upstream opens a file or a new document beside unsaved work rather than asking. The host's
/// contract is to ask Save / Don't Save / Cancel first and run `then` once answered, so the prompt
/// `run` would have raised for a discarding command is raised here.
fn save_changes_first(app: &mut WordApp, then: &str, params: Value) -> Option<Value> {
    if !(app.session.dirty || app.save_unconfirmed()) {
        return None;
    }
    let name = app.session.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| app.title_stem());
    app.dialog = Some(crate::dialogs::Dialog::SaveChanges { name, then: then.into(), params, document: app.session.document_id(), message: String::new() });
    Some(json!({"pending": "saveChanges"}))
}

pub(super) fn new_document(app: &mut WordApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    if params.get("template").is_some_and(|v| !v.is_string()) {
        return Err("template must be a string".into());
    }
    // The existing command owns template names and defaults, including future additions.
    let _ = cancel(app, ctx);
    if let Some(pending) = save_changes_first(app, "file.new", params.clone()) {
        return Ok(pending);
    }
    let result = app.run("file.new", params.clone())?;
    app.ui.backstage = false;
    Ok(result)
}

pub(super) fn cancel(app: &mut WordApp, ctx: &egui::Context) -> Value {
    let cancelled = app.file_dialog.take().is_some();
    #[cfg(not(target_arch = "wasm32"))]
    let cancelled = app.file_browser.open.take().is_some() || cancelled;
    let cancelled = cancel_picker(ctx) || cancelled;
    if cancelled {
        app.recipient_list_pending = false;
    }
    json!({"cancelled": cancelled})
}

/// The page picker owns the flag too: cancellation makes a later browser answer inert.
pub fn begin_picker(ctx: &egui::Context) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    cancel_picker(ctx);
    let pending = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    ctx.data_mut(|d| d.insert_temp(picker_id(), pending.clone()));
    pending
}

fn picker_id() -> egui::Id {
    egui::Id::unique("wordcraft-host-file-picker")
}

pub(super) fn picker_pending(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<std::sync::Arc<std::sync::atomic::AtomicBool>>(picker_id()))
        .is_some_and(|pending| pending.load(std::sync::atomic::Ordering::Relaxed))
}

fn cancel_picker(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<std::sync::Arc<std::sync::atomic::AtomicBool>>(picker_id()))
        .is_some_and(|pending| pending.swap(false, std::sync::atomic::Ordering::Relaxed))
}
