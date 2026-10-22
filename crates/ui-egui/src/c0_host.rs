//! C0VIBE's WordCraft guest contract. All document changes use WordCraft commands.

pub(crate) mod c0_chrome;
mod c0_files;
mod c0_panels;
pub use c0_files::begin_picker;
pub(crate) use c0_chrome as chrome;

use serde_json::{Value, json};
use wordcraft_doc::{Block, Pos, StoryRef};

use crate::{WordApp, control::Outcome};

/// Maximum decoded file size, also bounded by the page's total JSON request limit.
pub const MAX_FILE_BYTES: usize = 72 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 96 * 1024 * 1024;
pub const METHODS: &[&str] = &["c0.inspect", "c0.goToBlock", "c0.documentBytes", "c0.openFile", "c0.newDocument", "c0.cancelFileDialog", "c0.set"];

pub fn default_on() -> bool {
    true
}

/// Keep the upstream About content and spacing behind one host chrome seam.
pub(crate) fn about_community(ui: &mut egui::Ui, enabled: bool) {
    if enabled {
        ui.label(crate::tl!("A free, open-source word processor written from scratch in Rust.\nPart of the Crafting Apps from the ArtCraft team."));
        ui.add_space(6.0);
        ui.hyperlink_to(crate::tl!("getartcraft.com/apps/wordcraft"), "https://getartcraft.com/apps/wordcraft");
        ui.hyperlink_to(crate::tl!("Join us on Discord: discord.gg/artcraft"), "https://discord.gg/artcraft");
        ui.hyperlink_to(crate::tl!("Source code: github.com/storytold/wordcraft"), "https://github.com/storytold/wordcraft");
    }
}

/// The host's only arm in the existing control dispatcher.
pub fn handle(app: &mut WordApp, ctx: &egui::Context, method: &str, params: &Value) -> Outcome {
    let result = dispatch(app, ctx, method, params);
    if result.is_ok() && !matches!(method, "c0.inspect" | "c0.documentBytes") {
        ctx.request_repaint();
    }
    Outcome::Done(match result {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(error) => json!({"ok": false, "error": error}),
    })
}

fn dispatch(app: &mut WordApp, ctx: &egui::Context, method: &str, params: &Value) -> Result<Value, String> {
    if !params.is_object() && !params.is_null() {
        return Err("params must be an object".into());
    }
    match method {
        "c0.inspect" => Ok(inspect(app, ctx)),
        "c0.goToBlock" => go_to_block(app, params),
        "c0.documentBytes" => c0_files::document_bytes(app, params),
        "c0.openFile" => c0_files::open(app, ctx, params),
        "c0.newDocument" => c0_files::new_document(app, ctx, params),
        "c0.cancelFileDialog" => Ok(c0_files::cancel(app, ctx)),
        "c0.set" => c0_panels::set(app, ctx, params),
        _ => Err(format!("unknown host method {method:?}")),
    }
}

fn inspect(app: &mut WordApp, ctx: &egui::Context) -> Value {
    let present = app.has_document();
    let outline: Vec<Value> = if present {
        app.session
            .doc
            .body
            .iter()
            .enumerate()
            .filter_map(|(block, b)| {
                let Block::Para(p) = b.as_ref() else { return None };
                let props = app.session.doc.styles.resolve_para(&p.props);
                let level = if props.style == "Title" { 0 } else { props.outline_level.filter(|l| *l < 9)?.saturating_add(1) };
                let text = p.plain_text().trim().to_string();
                (!text.is_empty() && !props.style.starts_with("TOC")).then(|| json!({"level": level, "text": text, "block": block}))
            })
            .collect()
    } else {
        Vec::new()
    };
    let chrome = chrome::get(ctx);
    let pages = if present { app.session.layout().pages.len() } else { 0 };
    json!({
        "title": app.title_stem(), "dirty": app.session.dirty, "revision": app.session.rev(),
        "pages": pages, "words": if present { app.session.doc.word_count() } else { 0 },
        "canUndo": present && app.session.can_undo(), "canRedo": present && app.session.can_redo(), "outline": outline,
        "panes": {"ruler": app.session.view.ruler, "navigation": app.session.view.nav_pane,
                  "styles": app.session.view.styles_pane, "comments": app.session.view.comments_pane,
                  "toolbar": !app.ui.ribbon_collapsed, "toolbar_double": chrome.compact, "toolbar_floating": chrome.floating,
                  "right_dock_collapsed": chrome.dock_folded},
        "ribbonCollapsed": app.ui.ribbon_collapsed, "startScreen": app.ui.start_screen, "theme": app.ui.theme.code(),
        "fileDialog": app.file_dialog_open() || c0_files::picker_pending(ctx), "titleBar": app.ui.title_bar, "communityLinks": app.ui.community_links,
    })
}

fn go_to_block(app: &mut WordApp, params: &Value) -> Result<Value, String> {
    if !app.has_document() {
        return Err("no document is open".into());
    }
    let block = params.get("block").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok()).ok_or("block must be a nonnegative integer")?;
    let root = u32::try_from(block).map_err(|_| "block is outside the document")?;
    let target = app.session.doc.body.get(block).ok_or("block is outside the document")?;
    let pos = if target.as_para().is_some() {
        Pos::body(block, 0)
    } else {
        let path =
            app.session.doc.para_paths(StoryRef::Body).into_iter().find(|path| path.0.first() == Some(&root)).ok_or("block contains no paragraph")?;
        Pos { story: StoryRef::Body, path, off: 0 }
    };
    app.run("caret.set", json!({"pos": pos}))?;
    // Same navigation and focus policy as a click in WordCraft's Navigation pane.
    app.canvas.nav_to_caret = true;
    app.canvas.scroll_to_caret = false;
    app.canvas.want_focus = true;
    app.ui.backstage = false;
    Ok(json!({"block": block}))
}

/// Native-style optional boolean setters: absent toggles, a non-boolean is refused.
pub(crate) fn ui_command(app: &mut WordApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    if id == "ui.discord" && !app.ui.community_links {
        return Some(Err("community links are hidden".into()));
    }
    let (switch, key) = match id {
        "ui.titleBar" => (&mut app.ui.title_bar, "titleBar"),
        "ui.communityLinks" => (&mut app.ui.community_links, "communityLinks"),
        _ => return None,
    };
    match params.get("value") {
        None => *switch = !*switch,
        Some(Value::Bool(on)) => *switch = *on,
        Some(_) => return Some(Err("`value` must be true or false".into())),
    }
    Some(Ok(json!({key: *switch})))
}
