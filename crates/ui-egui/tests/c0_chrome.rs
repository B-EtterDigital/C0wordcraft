//! Draw the actual chrome, backstage, Help and About surfaces with the switch both ways.

use egui::{Shape, epaint::ClippedShape};
use serde_json::json;
use wordcraft_doc::Document;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{Services, WordApp};

fn text(shape: &Shape, into: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => into.push(t.galley.text().to_owned()),
        Shape::Vec(shapes) => {
            for shape in shapes {
                text(shape, into);
            }
        }
        _ => {}
    }
}

fn texts(shapes: &[ClippedShape]) -> String {
    let mut into = Vec::new();
    for s in shapes {
        text(&s.shape, &mut into);
    }
    into.join("\n")
}

fn draw(app: &mut WordApp, ctx: &egui::Context) -> String {
    let mut drawn = String::new();
    for _ in 0..5 {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 1000.0))), ..Default::default() },
            |ui| {
                app.logic(ui.ctx());
                app.ui(ui);
            },
        );
        drawn = texts(&output.shapes);
        // This text-only harness does not upload textures; acknowledge the unpainted delta.
        output.textures_delta.clear();
    }
    drawn
}

fn app() -> WordApp {
    WordApp::new(Session::new(Document::from_text("Hello")), Services::default())
}

#[test]
fn community_switch_also_blocks_the_help_keytip_and_direct_command() {
    for on in [true, false, true] {
        let mut app = app();
        app.execute("ui.communityLinks", json!({"value":on})).unwrap();
        assert_eq!(app.execute("ui.discord", json!({})).is_ok(), on);
        assert_eq!(app.canvas.open_url.take().is_some(), on);
        let mut opened = false;
        for key in egui::Key::ALL {
            app.ui.tab = "Help".into();
            app.ui.keytips = wordcraft_ui_egui::keytips::Phase::Commands;
            app.ui.keytip_prefix.clear();
            wordcraft_ui_egui::keytips::handle_key(&mut app, *key);
            opened |= app.canvas.open_url.take().is_some();
        }
        assert_eq!(opened, on, "Help keytips must honor the community switch");
    }
}

#[test]
fn title_bar_switch_removes_the_custom_caption_row() {
    let (mut app, ctx) = (app(), egui::Context::default());
    app.session.doc.core.title = "Host caption marker".into();
    let before = draw(&mut app, &ctx);
    assert!(before.contains("Host caption marker"), "{before}");
    app.execute("ui.titleBar", json!({"value":false})).unwrap();
    let after = draw(&mut app, &ctx);
    assert!(!after.contains("Host caption marker"), "{after}");
    app.execute("ui.titleBar", json!({"value":true})).unwrap();
    assert!(draw(&mut app, &ctx).contains("Host caption marker"));
}

#[test]
fn community_links_switch_removes_every_promotional_entry_but_keeps_authors_and_license() {
    let (mut app, ctx) = (app(), egui::Context::default());
    for on in [true, false, true] {
        app.execute("ui.communityLinks", json!({"value":on})).unwrap();
        app.execute("ui.tab", json!({"tab":"Home"})).unwrap();
        app.ui.show_discord = true;
        let title = draw(&mut app, &ctx);
        assert_eq!(title.contains("Discord"), on, "title: {title}");
        app.execute("ui.tab", json!({"tab":"Help"})).unwrap();
        let help = draw(&mut app, &ctx);
        assert_eq!(help.contains("Community"), on, "help: {help}");
        app.execute("ui.backstage", json!({"value":true})).unwrap();
        let backstage = draw(&mut app, &ctx);
        assert_eq!(backstage.contains("Discord community"), on, "backstage: {backstage}");
        app.execute("ui.backstage", json!({"value":false})).unwrap();
        app.execute("ui.dialog", json!({"name":"about"})).unwrap();
        let about = draw(&mut app, &ctx);
        for entry in
            ["Part of the Crafting Apps", "getartcraft.com/apps/wordcraft", "Join us on Discord:", "Source code: github.com/storytold/wordcraft"]
        {
            assert_eq!(about.contains(entry), on, "{entry}, on={on}: {about}");
        }
        assert!(about.contains("WordCraft") && about.contains("MIT OR Apache-2.0") && about.contains("WordCraft contributors"), "{about}");
        app.execute("ui.closeDialog", json!({})).unwrap();
    }
}
