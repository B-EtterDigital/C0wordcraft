//! Contract and lifecycle checks for the C0doc host. Exercise the real control dispatcher.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use wordcraft_doc::{Document, StoryRef};
use wordcraft_engine::Session;
use wordcraft_ui_egui::{
    Services, UiState, WordApp, c0_host,
    control::{ControlRequest, Outcome, handle},
    theme::{self, Tokens, host},
};

fn app() -> WordApp {
    WordApp::new(Session::new(Document::from_text("First\nSecond\nThird")), Services::default())
}

fn call(app: &mut WordApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    let Outcome::Done(reply) = handle(app, ctx, &req) else { panic!("{method} must answer immediately") };
    assert!(reply["ok"].is_boolean());
    if reply["ok"] == true {
        assert!(reply.get("result").is_some());
    } else {
        assert!(reply["error"].is_string());
    }
    reply
}

fn ok(app: &mut WordApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let reply = call(app, ctx, method, params);
    assert_eq!(reply["ok"], true, "{reply}");
    reply["result"].clone()
}

#[test]
fn inspect_shape_outline_and_revision() {
    let (mut app, ctx) = (app(), egui::Context::default());
    for (block, style) in [(0, "Title"), (1, "Heading1"), (2, "Heading9")] {
        app.execute("caret.set", json!({"pos": wordcraft_doc::Pos::body(block, 0)})).unwrap();
        app.execute("para.style", json!({"style": style})).unwrap();
    }
    let rev = app.session.rev();
    let r = ok(&mut app, &ctx, "c0.inspect", json!({}));
    for key in ["title", "dirty", "revision", "pages", "words", "canUndo", "canRedo", "outline", "panes", "ribbonCollapsed", "startScreen", "theme"] {
        assert!(r.get(key).is_some(), "{key}: {r}");
    }
    assert!(r["title"].is_string() && r["dirty"].is_boolean() && r["canUndo"].is_boolean() && r["canRedo"].is_boolean());
    assert_eq!(r["revision"], rev);
    assert_eq!(r["words"], 3);
    assert!(r["pages"].as_u64().unwrap() >= 1);
    assert_eq!(
        r["outline"],
        json!([
            {"level":0,"text":"First","block":0}, {"level":1,"text":"Second","block":1}, {"level":9,"text":"Third","block":2}
        ])
    );
    assert_eq!(app.session.rev(), rev, "inspection is read-only");
}

#[test]
fn upstream_defaults_and_boolean_commands() {
    let defaults = UiState::default();
    let read = UiState::from_json(b"{}").unwrap();
    for state in [&defaults, &read] {
        assert!(state.title_bar && state.community_links && state.quick_access_toolbar && state.show_discord);
        assert!(state.start_screen);
        assert!(!state.ribbon_collapsed);
    }
    let (mut app, ctx) = (app(), egui::Context::default());
    for (command, key) in [("ui.titleBar", "titleBar"), ("ui.communityLinks", "communityLinks")] {
        for value in [false, true] {
            let r = ok(&mut app, &ctx, "engine.execute", json!({"command":command,"params":{"value":value}}));
            assert_eq!(r[key], value);
        }
        assert_eq!(call(&mut app, &ctx, "engine.execute", json!({"command":command,"params":{"value":"no"}}))["ok"], false);
        app.execute(command, json!({})).unwrap();
    }
    assert!(!app.ui.title_bar && !app.ui.community_links);
    let saved = serde_json::to_value(app.prefs()).unwrap();
    assert!(saved.get("titleBar").is_none() && saved.get("communityLinks").is_none());
    let restored = UiState::from_json(&serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(restored.title_bar && restored.community_links, "host switches are not preferences");
    app.start_screen();
    for command in ["ui.titleBar", "ui.communityLinks"] {
        app.execute(command, json!({"value":true})).unwrap();
    }
}

#[test]
fn panels_apply_and_report_each_key_without_toggling_twice() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let params = json!({"panels":{"title_bar":false,"community_links":false,"ribbon_collapsed":true,"start_screen":false,
        "ruler":false,"navigation_pane":true,"styles_pane":true,"comments_pane":true,"status_bar":false,"made_up":true,"bad":1}});
    for _ in 0..2 {
        let r = ok(&mut app, &ctx, "c0.set", params.clone());
        assert_eq!(r["applied"].as_array().unwrap().len(), 8);
        assert_eq!(r["unknown"], json!(["panels.bad", "panels.made_up", "panels.status_bar"]));
        assert_eq!(r["invalid"], json!([]));
        assert!(app.ui.ribbon_collapsed && !app.ui.title_bar && !app.ui.community_links && !app.ui.start_screen);
        assert!(!app.session.view.ruler && app.session.view.nav_pane && app.session.view.styles_pane && app.session.view.comments_pane);
    }
    let r = ok(&mut app, &ctx, "c0.set", json!({"panels":{"ruler":"false","title_bar":null}}));
    assert_eq!(r["invalid"], json!(["panels.ruler", "panels.title_bar"]));
    assert_eq!(call(&mut app, &ctx, "c0.set", json!({"panels":false,"themeTokens":{"accent":"#123456"}}))["ok"], false);
    assert!(host::current(&ctx).is_none(), "invalid containers cause no partial palette update");
}

#[test]
fn host_set_reports_theme_tokens_panels_and_top_level_keys_together() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.start_screen();
    for theme in ["dark", "light"] {
        let r = ok(
            &mut app,
            &ctx,
            "c0.set",
            json!({"theme":theme,"themeTokens":{"accent":"#123456"},
            "panels":{"title_bar":false,"ruler":"bad","future":true},"future":true}),
        );
        assert_eq!(
            r,
            json!({"applied":["theme","themeTokens.accent","panels.title_bar"],
            "unknown":["future","panels.future"],"invalid":["panels.ruler"]})
        );
        assert_eq!(ok(&mut app, &ctx, "c0.inspect", json!({}))["theme"], theme);
        assert_eq!(ctx.global_style().visuals.dark_mode, theme == "dark");
        assert_eq!(Tokens::get(&ctx).accent, egui::Color32::from_rgb(0x12, 0x34, 0x56));
    }
    for theme in [json!("system"), json!("purple"), Value::Null, json!(true)] {
        assert_eq!(ok(&mut app, &ctx, "c0.set", json!({"theme":theme})), json!({"applied":[],"unknown":[],"invalid":["theme"]}));
        assert_eq!(app.ui.theme, theme::Appearance::Light);
    }
    assert_eq!(ok(&mut app, &ctx, "c0.set", json!({})), json!({"applied":[],"unknown":[],"invalid":[]}));
}

#[test]
fn theme_tokens_apply_to_both_presets_and_widgets_and_clear() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let preset = Tokens::light();
    theme::apply(&ctx, theme::Appearance::Light);
    assert_eq!(Tokens::get(&ctx).accent, preset.accent);
    let r = ok(
        &mut app,
        &ctx,
        "c0.set",
        json!({"themeTokens":{"accent":"#123456","input":"#010203ff","text":"wrong",
        "ribbon":"#ééé","radius":3,"renamed_token":"#000000"}}),
    );
    assert_eq!(
        r,
        json!({"applied":["themeTokens.accent","themeTokens.input"],"unknown":["themeTokens.radius","themeTokens.renamed_token"],"invalid":["themeTokens.ribbon","themeTokens.text"]})
    );
    for appearance in [theme::Appearance::Dark, theme::Appearance::Light, theme::Appearance::System] {
        theme::apply(&ctx, appearance);
        let t = Tokens::get(&ctx);
        assert_eq!(t.accent, egui::Color32::from_rgb(0x12, 0x34, 0x56));
        assert_eq!(t.input, egui::Color32::from_rgb(1, 2, 3));
        for style in [ctx.style_of(egui::Theme::Dark), ctx.style_of(egui::Theme::Light)] {
            assert_eq!(style.visuals.extreme_bg_color, t.input);
        }
    }
    ok(&mut app, &ctx, "c0.newDocument", json!({}));
    assert_eq!(Tokens::get(&ctx).accent, egui::Color32::from_rgb(0x12, 0x34, 0x56));
    ok(&mut app, &ctx, "c0.set", json!({"themeTokens":null}));
    assert_eq!(Tokens::get(&ctx).accent, Tokens::light().accent);
    assert!(host::current(&ctx).is_none());
}

#[test]
fn go_to_block_uses_caret_command_and_navigation_scroll() {
    let (mut app, ctx) = (app(), egui::Context::default());
    assert_eq!(ok(&mut app, &ctx, "c0.goToBlock", json!({"block":2})), json!({"block":2}));
    assert_eq!(app.session.sel.focus, wordcraft_doc::Pos::body(2, 0));
    assert!(app.canvas.want_focus);
    assert!(!app.canvas.scroll_to_caret);
    for value in [json!(-1), json!(u64::MAX), json!(1.5), json!("1"), Value::Null] {
        assert_eq!(call(&mut app, &ctx, "c0.goToBlock", json!({"block":value}))["ok"], false);
    }
    // Prove the private navigation flag's observable effect: a distant paragraph scrolls
    // into the viewport with Navigation's top margin, rather than merely moving the caret.
    let document = Document::from_text(&["Navigation target"; 200].join("\n"));
    let app = WordApp::new(Session::new(document), Services::default());
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        app,
    );
    for _ in 0..8 {
        h.step();
    }
    let ctx = h.ctx.clone();
    ok(h.state_mut(), &ctx, "c0.goToBlock", json!({"block":120}));
    for _ in 0..8 {
        h.step();
    }
    let viewport = h.state().canvas.canvas_rect.unwrap();
    let (caret, _) = wordcraft_ui_egui::canvas::caret_screen(h.state_mut()).unwrap();
    assert!(viewport.contains(caret), "caret {caret:?} must be visible in {viewport:?}");
    assert!((caret.y - viewport.top() - 40.0).abs() < 2.0, "navigation top alignment: {caret:?}, {viewport:?}");
}

#[test]
fn document_bytes_roundtrip_is_read_only_and_uses_the_importer() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.execute("text.insert", json!({"text":"Roundtrip "})).unwrap();
    let before =
        (app.session.rev(), app.session.dirty, app.session.can_undo(), app.session.can_redo(), app.session.path.clone(), app.session.doc.clone());
    let bytes = ok(&mut app, &ctx, "c0.documentBytes", json!({}));
    assert_eq!(bytes["format"], "docx");
    assert!(bytes["name"].as_str().unwrap().ends_with(".docx"));
    assert_eq!(bytes["revision"], before.0);
    let data = STANDARD.decode(bytes["bytes"].as_str().unwrap()).unwrap();
    assert!(data.starts_with(b"PK"));
    assert_eq!(
        (app.session.rev(), app.session.dirty, app.session.can_undo(), app.session.can_redo(), app.session.path.clone(), app.session.doc.clone()),
        before
    );
    let mut reopened = super_app();
    let r = ok(&mut reopened, &ctx, "c0.openFile", json!({"name":bytes["name"],"bytes":bytes["bytes"]}));
    assert!(r.get("path").is_some(), "{r}");
    assert_eq!(reopened.session.doc.plain_text(StoryRef::Body), app.session.doc.plain_text(StoryRef::Body));
    assert!(!reopened.session.dirty);
    assert!(!reopened.ui.backstage);
    let pdf = ok(&mut app, &ctx, "c0.documentBytes", json!({"format":"pdf"}));
    assert_eq!((pdf["format"].as_str(), pdf["name"].as_str().map(|n| n.ends_with(".pdf"))), (Some("pdf"), Some(true)));
    assert!(STANDARD.decode(pdf["bytes"].as_str().unwrap()).unwrap().starts_with(b"%PDF-"));
    assert_eq!((app.session.rev(), app.session.dirty), (before.0, before.1), "the PDF snapshot is read-only too");
    assert_eq!(call(&mut app, &ctx, "c0.documentBytes", json!({"format":"txt"}))["ok"], false);
}

fn super_app() -> WordApp {
    WordApp::new(Session::new(Document::new()), Services::default())
}

#[test]
fn open_and_new_preserve_unsaved_work_until_normal_prompt_is_answered() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.execute("text.insert", json!({"text":"Unsaved "})).unwrap();
    let rev = app.session.rev();
    let r = ok(&mut app, &ctx, "c0.openFile", json!({"name":"host.txt","bytes":STANDARD.encode("Replacement")}));
    assert_eq!(r["pending"], "saveChanges");
    assert_eq!(app.session.rev(), rev);
    app.execute("ui.saveChanges", json!({"answer":"cancel"})).unwrap();
    assert!(app.session.doc.plain_text(StoryRef::Body).contains("Unsaved"));
    ok(&mut app, &ctx, "c0.openFile", json!({"name":"host.txt","bytes":STANDARD.encode("Replacement")}));
    app.execute("ui.saveChanges", json!({"answer":"dontSave"})).unwrap();
    assert_eq!(app.session.doc.plain_text(StoryRef::Body).trim(), "Replacement");
    app.execute("text.insert", json!({"text":"changed"})).unwrap();
    let r = ok(&mut app, &ctx, "c0.newDocument", json!({"template":"blank"}));
    assert_eq!(r["pending"], "saveChanges");
    app.execute("ui.saveChanges", json!({"answer":"dontSave"})).unwrap();
    assert!(app.has_document() && !app.ui.backstage);
    assert!(app.session.doc.plain_text(StoryRef::Body).trim().is_empty());
}

#[test]
fn malformed_files_and_unknown_methods_reply_with_errors() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let rev = app.session.rev();
    for params in [
        json!({}),
        json!({"name":"../a.txt","bytes":""}),
        json!({"name":"a.txt","bytes":"!"}),
        json!({"name":"a.txt","bytes":"a"}),
        json!({"name":"bad.docx","bytes":STANDARD.encode("bad zip")}),
    ] {
        assert_eq!(call(&mut app, &ctx, "c0.openFile", params)["ok"], false);
        assert_eq!(app.session.rev(), rev);
    }
    assert_eq!(call(&mut app, &ctx, "c0.newDocument", json!({"template":123}))["ok"], false);
    assert_eq!(call(&mut app, &ctx, "c0.unknown", json!({}))["ok"], false);
    app.start_screen();
    assert_eq!(ok(&mut app, &ctx, "c0.inspect", json!({}))["pages"], 0);
    assert_eq!(call(&mut app, &ctx, "c0.documentBytes", json!({}))["ok"], false);
    assert_eq!(call(&mut app, &ctx, "c0.goToBlock", json!({"block":0}))["ok"], false);
    ok(&mut app, &ctx, "c0.newDocument", json!({}));
    assert!(app.has_document() && !app.ui.backstage);
}

#[test]
fn every_writable_import_format_uses_the_same_file_open_path() {
    let source = Document::from_text("Host import formats");
    let ctx = egui::Context::default();
    for extension in ["docx", "docm", "dotx", "dotm", "odt", "rtf", "txt", "md", "html", "tex", "json", "xml"] {
        let name = format!("synthetic.{extension}");
        let bytes = wordcraft_engine::io::save_bytes(&name, &source).unwrap_or_else(|e| panic!("{extension}: {e}"));
        let mut reopened = super_app();
        ok(&mut reopened, &ctx, "c0.openFile", json!({"name":name,"bytes":STANDARD.encode(bytes)}));
        assert!(reopened.session.doc.plain_text(StoryRef::Body).contains("Host import formats"), "{extension}");
    }
}

#[test]
fn initial_host_settings_apply_even_on_the_no_document_start_screen() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.start_screen();
    let r = ok(
        &mut app,
        &ctx,
        "c0.set",
        json!({"panels":{"title_bar":false,"community_links":false,"navigation_pane":false,
        "styles_pane":false,"comments_pane":false,"ruler":false}}),
    );
    assert_eq!(r["applied"].as_array().unwrap().len(), 6);
    assert_eq!(r["invalid"], json!([]));
    assert!(!app.has_document());
}

#[test]
fn new_document_forwards_the_native_template_parameter() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let reply = ok(&mut app, &ctx, "c0.newDocument", json!({"template":"report"}));
    assert!(reply["anchor"].is_object() && reply["focus"].is_object(), "{reply}");
    assert!(app.has_document() && !app.ui.backstage);
    assert!(app.session.doc.word_count() > 0);
    ok(&mut app, &ctx, "c0.newDocument", json!({}));
    assert_eq!(app.session.doc.word_count(), 0);
}

#[test]
fn encrypted_snapshot_keeps_save_policy_and_never_opens_a_password_dialog() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.execute("file.encrypt", json!({"password":"synthetic test password"})).unwrap();
    let rev = app.session.rev();
    let r = ok(&mut app, &ctx, "c0.documentBytes", json!({"format":"docx"}));
    let bytes = STANDARD.decode(r["bytes"].as_str().unwrap()).unwrap();
    let (opened, encrypted) = wordcraft_engine::io::open_bytes_with("snapshot.docx", &bytes, Some("synthetic test password")).unwrap();
    assert!(encrypted);
    assert_eq!(opened.plain_text(StoryRef::Body), app.session.doc.plain_text(StoryRef::Body));
    assert_eq!(app.session.rev(), rev);
    let mut target = super_app();
    assert_eq!(call(&mut target, &ctx, "c0.openFile", json!({"name":"snapshot.docx","bytes":r["bytes"]}))["ok"], false);
    assert!(target.dialog.is_none());
}

#[test]
fn cancel_file_dialog_is_idempotent_and_retires_a_late_picker_reply() {
    let (mut app, ctx) = (app(), egui::Context::default());
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":false}));
    let pending = c0_host::begin_picker(&ctx);
    assert_eq!(ok(&mut app, &ctx, "c0.inspect", json!({}))["fileDialog"], true);
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":true}));
    assert!(!pending.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":false}));
    // Also cancel WordCraft's own asynchronous file-dialog continuation.
    let (tx, rx) = std::sync::mpsc::channel();
    let rx = std::cell::RefCell::new(Some(rx));
    app.services.file_dialog = Some(Box::new(move |_| rx.borrow_mut().take().unwrap()));
    app.execute("ui.openFileDialog", json!({})).unwrap();
    assert!(app.file_dialog_open());
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":true}));
    assert!(tx.send(Ok(vec!["late.txt".into()])).is_err());
}

#[test]
fn open_and_new_supersede_an_older_open_picker_request() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let pending = c0_host::begin_picker(&ctx);
    ok(&mut app, &ctx, "c0.newDocument", json!({}));
    assert!(!pending.load(std::sync::atomic::Ordering::Relaxed));
    let pending = c0_host::begin_picker(&ctx);
    ok(&mut app, &ctx, "c0.openFile", json!({"name":"host.txt","bytes":STANDARD.encode("Host")}));
    assert!(!pending.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(ok(&mut app, &ctx, "c0.inspect", json!({}))["fileDialog"], false);
}
