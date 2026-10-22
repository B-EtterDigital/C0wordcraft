//! Tool modes (the ribbon as full, compact, collapsed or floating) and the foldable right dock,
//! through the real control dispatcher and the real drawn chrome.

use egui::{Shape, epaint::ClippedShape};
use serde_json::{Value, json};
use wordcraft_doc::Document;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{
    Services, WordApp,
    control::{ControlRequest, Outcome, handle},
};

const KEYS: [&str; 4] = ["toolbar_double", "toolbar_floating", "right_dock_collapsed", "toolbar"];

fn app() -> WordApp {
    WordApp::new(Session::new(Document::from_text("Hello")), Services::default())
}

fn call(app: &mut WordApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _rx) = ControlRequest::new(method, params);
    let Outcome::Done(reply) = handle(app, ctx, &req) else { panic!("{method} must answer immediately") };
    assert_eq!(reply["ok"], true, "{reply}");
    reply["result"].clone()
}

fn set(app: &mut WordApp, ctx: &egui::Context, panels: Value) -> Value {
    call(app, ctx, "c0.set", json!({"panels": panels}))
}

fn panes(app: &mut WordApp, ctx: &egui::Context) -> Value {
    call(app, ctx, "c0.inspect", json!({}))["panes"].clone()
}

fn texts(shapes: &[ClippedShape]) -> Vec<(String, egui::Pos2)> {
    fn walk(shape: &Shape, into: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            Shape::Text(t) => into.push((t.galley.text().to_owned(), t.pos)),
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
            _ => {}
        }
    }
    let mut into = Vec::new();
    shapes.iter().for_each(|s| walk(&s.shape, &mut into));
    into
}

fn draw(app: &mut WordApp, ctx: &egui::Context, width: f32) -> Vec<(String, egui::Pos2)> {
    let mut drawn = Vec::new();
    for _ in 0..6 {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 900.0))), ..Default::default() },
            |ui| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
        );
        drawn = texts(&output.shapes);
        output.textures_delta.clear();
    }
    drawn
}

fn find<'a>(drawn: &'a [(String, egui::Pos2)], text: &str) -> Option<&'a egui::Pos2> {
    drawn.iter().find(|(t, _)| t == text).map(|(_, p)| p)
}

#[test]
fn defaults_are_upstream_and_inspect_reports_every_mode() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let p = panes(&mut app, &ctx);
    assert_eq!(p["toolbar"], true, "the ribbon body shows by default");
    for key in ["toolbar_double", "toolbar_floating", "right_dock_collapsed"] {
        assert_eq!(p[key], false, "{key}");
    }
    assert!(!app.ui.ribbon_collapsed);
}

#[test]
fn each_key_sets_and_reads_back_idempotently() {
    let (mut app, ctx) = (app(), egui::Context::default());
    for key in KEYS {
        let on = key != "toolbar";
        for _ in 0..2 {
            let r = set(&mut app, &ctx, json!({ key: on }));
            assert_eq!(r["applied"], json!([format!("panels.{key}")]), "{r}");
            assert_eq!(panes(&mut app, &ctx)[key], on, "{key}");
        }
        set(&mut app, &ctx, json!({ key: !on }));
        assert_eq!(panes(&mut app, &ctx)[key], !on, "{key} back");
    }
}

#[test]
fn toolbar_is_the_inverse_of_the_collapsed_ribbon() {
    let (mut app, ctx) = (app(), egui::Context::default());
    set(&mut app, &ctx, json!({"toolbar": false}));
    assert!(app.ui.ribbon_collapsed);
    assert_eq!(call(&mut app, &ctx, "c0.inspect", json!({}))["ribbonCollapsed"], true);
    set(&mut app, &ctx, json!({"ribbon_collapsed": false}));
    assert_eq!(panes(&mut app, &ctx)["toolbar"], true);
}

#[test]
fn unknown_and_invalid_keys_are_reported_and_siblings_apply() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let r = set(&mut app, &ctx, json!({"toolbar_triple": true, "toolbar_floating": "yes", "right_dock_collapsed": 1, "toolbar_double": true}));
    assert_eq!(r["unknown"], json!(["panels.toolbar_triple"]));
    assert_eq!(r["invalid"], json!(["panels.right_dock_collapsed", "panels.toolbar_floating"]).as_array().map(|a| json!([a[1], a[0]])).unwrap());
    assert_eq!(r["applied"], json!(["panels.toolbar_double"]));
    assert_eq!(panes(&mut app, &ctx)["toolbar_floating"], false);
}

#[test]
fn modes_are_not_saved_to_preferences() {
    let (mut app, ctx) = (app(), egui::Context::default());
    set(&mut app, &ctx, json!({"toolbar_double": true, "toolbar_floating": true, "right_dock_collapsed": true}));
    let saved = serde_json::to_string(app.prefs()).unwrap();
    for key in ["toolbar_double", "toolbarDouble", "toolbar_floating", "toolbarFloating", "right_dock", "rightDock"] {
        assert!(!saved.contains(key), "{key} leaked into preferences");
    }
}

#[test]
fn defaults_draw_the_docked_full_ribbon() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let drawn = draw(&mut app, &ctx, 1500.0);
    assert!(find(&drawn, "Clipboard").is_some(), "group captions show");
    assert!(find(&drawn, "Home").is_some(), "tabs show");
}

#[test]
fn collapsed_keeps_the_tabs_and_drops_the_body() {
    let (mut app, ctx) = (app(), egui::Context::default());
    set(&mut app, &ctx, json!({"toolbar": false}));
    let drawn = draw(&mut app, &ctx, 1500.0);
    assert!(find(&drawn, "Home").is_some() && find(&drawn, "Clipboard").is_none());
}

#[test]
fn compact_row_drops_the_small_button_labels() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let full = draw(&mut app, &ctx, 1500.0);
    set(&mut app, &ctx, json!({"toolbar_double": true}));
    let compact = draw(&mut app, &ctx, 1500.0);
    assert!(find(&compact, "Home").is_some() && find(&compact, "Clipboard").is_some(), "same ribbon, same groups");
    assert!(compact.len() < full.len(), "{} compact texts vs {} full", compact.len(), full.len());
    set(&mut app, &ctx, json!({"toolbar_double": false}));
    assert_eq!(draw(&mut app, &ctx, 1500.0).len(), full.len(), "back to upstream");
}

#[test]
fn floating_ribbon_leaves_the_dock_and_keeps_every_group() {
    let (mut app, ctx) = (app(), egui::Context::default());
    let docked = draw(&mut app, &ctx, 1500.0);
    set(&mut app, &ctx, json!({"toolbar_floating": true}));
    let floating = draw(&mut app, &ctx, 1500.0);
    assert!(find(&floating, "Home").is_some(), "tabs stay docked");
    let (a, b) = (find(&docked, "Clipboard").unwrap(), find(&floating, "Clipboard").unwrap());
    assert!(b.x > a.x, "the strip floats inset from the edge: {a:?} {b:?}");
    assert!(find(&floating, "Font").is_some() && find(&floating, "Paragraph").is_some());
    set(&mut app, &ctx, json!({"toolbar_floating": false}));
    assert_eq!(find(&draw(&mut app, &ctx, 1500.0), "Clipboard"), Some(a));
}

#[test]
fn folded_dock_hides_right_panes_and_unfolds_when_one_opens() {
    let (mut app, ctx) = (app(), egui::Context::default());
    set(&mut app, &ctx, json!({"comments_pane": true}));
    let open = draw(&mut app, &ctx, 1500.0);
    assert!(find(&open, "Comments").is_some(), "the pane draws its header");
    set(&mut app, &ctx, json!({"right_dock_collapsed": true}));
    let folded = draw(&mut app, &ctx, 1500.0);
    assert!(find(&folded, "Comments").is_none(), "folded to the strip");
    assert!(app.session.view.comments_pane, "folding does not close the pane");
    assert_eq!(panes(&mut app, &ctx)["right_dock_collapsed"], true);
    // A pane opened while folded unfolds the dock; the one already open stays open.
    set(&mut app, &ctx, json!({"styles_pane": true}));
    let unfolded = draw(&mut app, &ctx, 1500.0);
    assert_eq!(panes(&mut app, &ctx)["right_dock_collapsed"], false);
    assert!(find(&unfolded, "Comments").is_some() && find(&unfolded, "Styles").is_some());
    set(&mut app, &ctx, json!({"right_dock_collapsed": false}));
    assert_eq!(draw(&mut app, &ctx, 1500.0).len(), unfolded.len(), "unfolded draws as upstream");
}

#[test]
fn folded_dock_leaves_left_panes_alone() {
    let (mut app, ctx) = (app(), egui::Context::default());
    set(&mut app, &ctx, json!({"navigation_pane": true, "right_dock_collapsed": true}));
    let drawn = draw(&mut app, &ctx, 1500.0);
    assert!(find(&drawn, "Navigation").is_some());
    assert_eq!(panes(&mut app, &ctx)["navigation"], true);
}
