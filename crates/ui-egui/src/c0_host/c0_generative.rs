//! `c0.selection` and `c0.place`: what the AI actions read from, and write into, the document.
//! Every change is made with WordCraft's own commands and lands as one undo step.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use wordcraft_doc::{Pos, StoryRef};

use crate::WordApp;

/// Largest picture accepted by `c0.place`, decoded.
pub const MAX_IMAGE_BYTES: usize = 40 * 1024 * 1024;
/// Largest text accepted by `c0.place`.
const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;

fn need_document(app: &WordApp) -> Result<(), String> {
    if app.has_document() { Ok(()) } else { Err("no document is open".into()) }
}

/// Character offset of `pos` in the body's plain text (paragraphs joined by one line break).
fn body_offset(app: &WordApp, pos: &Pos) -> Option<usize> {
    if pos.story != StoryRef::Body {
        return None;
    }
    let mut before = 0usize;
    for path in app.session.doc.para_paths(StoryRef::Body) {
        let text = &app.session.doc.para(StoryRef::Body, &path)?.text;
        if path == pos.path {
            let mut off = pos.off.min(text.len());
            while !text.is_char_boundary(off) {
                off -= 1;
            }
            return Some(before + text[..off].chars().count());
        }
        before += text.chars().count() + 1;
    }
    None
}

pub(super) fn selection(app: &mut WordApp) -> Result<Value, String> {
    need_document(app)?;
    let sel = app.session.sel.clone();
    let paragraph = app.session.doc.para_at(&sel.focus).map(|p| p.plain_text()).unwrap_or_default();
    let title = app.title_stem();
    let words = app.session.word_count();
    if sel.is_collapsed() {
        return Ok(json!({"kind": "caret", "paragraph": paragraph, "documentTitle": title, "wordCount": words}));
    }
    let (a, b) = sel.ordered();
    Ok(json!({
        "kind": "text", "text": app.session.selected_text(), "start": body_offset(app, &a), "end": body_offset(app, &b),
        "paragraph": paragraph, "documentTitle": title, "wordCount": words,
    }))
}

enum Mode {
    Replace,
    Insert,
    After,
}

impl Mode {
    fn name(&self) -> &'static str {
        match self {
            Mode::Replace => "replace",
            Mode::Insert => "insert",
            Mode::After => "after",
        }
    }
}

pub(super) fn place(app: &mut WordApp, params: &Value) -> Result<Value, String> {
    need_document(app)?;
    let image = ["png", "jpeg"].into_iter().find(|k| params.get(*k).is_some());
    if image.is_some() && params.get("text").is_some() {
        return Err("send either text or a picture, not both".into());
    }
    // Validate everything before the document is touched.
    enum Job {
        Text(String, Mode),
        Picture(Vec<u8>, String, Option<f64>),
    }
    let job = match image {
        Some(kind) => {
            let b64 = params.get(kind).and_then(Value::as_str).ok_or_else(|| format!("{kind} must be base64 text"))?;
            if b64.len() / 4 * 3 > MAX_IMAGE_BYTES + 3 {
                return Err("picture is larger than 40 MB".into());
            }
            let bytes = STANDARD.decode(b64).map_err(|_| format!("{kind} must be valid standard base64"))?;
            if bytes.len() > MAX_IMAGE_BYTES {
                return Err("picture is larger than 40 MB".into());
            }
            let magic_ok = if kind == "png" { bytes.starts_with(&[0x89, b'P', b'N', b'G']) } else { bytes.starts_with(&[0xFF, 0xD8]) };
            if !magic_ok {
                return Err(format!("the data is not a {} picture", kind.to_uppercase()));
            }
            let width = match params.get("widthPx") {
                None | Some(Value::Null) => None,
                Some(v) => Some(v.as_f64().filter(|w| w.is_finite() && *w >= 1.0 && *w <= 100_000.0).ok_or("widthPx must be a number from 1 to 100000")?),
            };
            let name = match params.get("name") {
                None | Some(Value::Null) => String::new(),
                Some(v) => v.as_str().ok_or("name must be text")?.chars().take(200).collect(),
            };
            Job::Picture(bytes, name, width)
        }
        None => {
            let text = params.get("text").and_then(Value::as_str).ok_or("send `text`, or a `png` or `jpeg` picture")?;
            if text.trim().is_empty() {
                return Err("text is empty".into());
            }
            if text.len() > MAX_TEXT_BYTES {
                return Err("text is larger than 4 MB".into());
            }
            let mode = match params.get("mode").and_then(Value::as_str) {
                None | Some("replace") => Mode::Replace,
                Some("insert") => Mode::Insert,
                Some("after") => Mode::After,
                Some(_) => return Err("mode must be replace, insert or after".into()),
            };
            Job::Text(text.replace("\r\n", "\n").replace('\r', "\n"), mode)
        }
    };

    let mark = app.session.undo_mark();
    app.session.close_typing();
    let result = apply(app, &job);
    app.session.close_typing();
    // One Ctrl+Z removes everything this call did.
    app.session.merge_undo_since(mark);
    if result.is_err() && app.session.undo_mark() > mark {
        app.session.undo();
    }
    return result;

    fn apply(app: &mut WordApp, job: &Job) -> Result<Value, String> {
        match job {
            Job::Text(text, mode) => {
                let (a, b) = app.session.sel.ordered();
                let mode_used = match mode {
                    Mode::Replace if app.session.sel.is_collapsed() => Mode::Insert,
                    Mode::Replace => Mode::Replace,
                    Mode::Insert => Mode::Insert,
                    Mode::After => Mode::After,
                };
                let body = match mode_used {
                    Mode::Replace => text.clone(),
                    Mode::Insert => {
                        if a != b {
                            app.execute("caret.set", json!({"pos": b}))?;
                        }
                        text.clone()
                    }
                    Mode::After => {
                        let end = Pos { off: app.session.doc.para_at(&b).map_or(b.off, |p| p.text.len()), ..b };
                        app.execute("caret.set", json!({"pos": end}))?;
                        // Blank lines split paragraphs; every kept line is its own paragraph.
                        let lines: Vec<&str> = text.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).collect();
                        format!("\n{}", lines.join("\n"))
                    }
                };
                app.execute("text.insert", json!({"text": body, "raw": true}))?;
                let paragraphs = text.split("\n\n").filter(|c| !c.trim().is_empty()).count().max(1);
                Ok(json!({"placed": "text", "mode": mode_used.name(), "requestedMode": mode.name(), "chars": text.chars().count(), "paragraphs": paragraphs,
                          "revision": app.session.rev()}))
            }
            Job::Picture(bytes, name, width) => {
                let (_, b) = app.session.sel.ordered();
                if !app.session.sel.is_collapsed() {
                    app.execute("caret.set", json!({"pos": b}))?;
                }
                let mut p = json!({"data": STANDARD.encode(bytes), "alt": name});
                if let Some(w) = width {
                    p["width"] = json!(w * 0.75);
                }
                let r = app.execute("insert.picture", p)?;
                Ok(json!({"placed": "image", "name": name, "bytes": bytes.len(), "widthPx": width, "revision": app.session.rev(), "engine": r.get("media")}))
            }
        }
    }
}
