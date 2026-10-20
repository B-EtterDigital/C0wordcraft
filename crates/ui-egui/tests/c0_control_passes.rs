//! An unpainted egui pass answers the real queue. No screen presentation is required.

use serde_json::{Value, json};
use std::sync::mpsc::{Sender, channel};
use wordcraft_doc::Document;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{ControlRequest, Services, WordApp};

type Harness = egui_kittest::Harness<'static, WordApp>;

fn idle() -> (Harness, Sender<ControlRequest>) {
    let (tx, rx) = channel();
    let mut app = WordApp::new(Session::new(Document::from_text("One\nTwo")), Services::default()).with_control(rx);
    app.ui.caret_animation = false;
    app.ui.animations = wordcraft_ui_egui::theme::Animations::Off;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    // The real renderer debounces using wall time, independently of the harness clock.
    // Establish an idle baseline before attributing a repaint to a read request.
    for _ in 0..100 {
        h.step();
        if h.output().viewport_output.get(&egui::ViewportId::ROOT).unwrap().repaint_delay != std::time::Duration::ZERO {
            return (h, tx);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("editor did not settle: {:?}", h.ctx.repaint_causes());
}

fn request(h: &mut Harness, tx: &Sender<ControlRequest>, method: &str, params: Value) -> Value {
    let (req, reply) = ControlRequest::new(method, params);
    tx.send(req).unwrap();
    assert!(reply.try_recv().is_err());
    h.step();
    let value = reply.try_recv().expect("one unpainted pass answers the request");
    assert_eq!(value["ok"], true, "{method}: {value}");
    value["result"].clone()
}

#[test]
fn a_read_is_answered_by_one_unpainted_pass() {
    let (mut h, tx) = idle();
    for method in ["c0.inspect", "c0.documentBytes", "ui.inspect", "document.inspect", "engine.commands"] {
        request(&mut h, &tx, method, json!({}));
        let delay = h.output().viewport_output.get(&egui::ViewportId::ROOT).unwrap().repaint_delay;
        assert_ne!(delay, std::time::Duration::ZERO, "{method} should not demand presentation: {:?}", h.ctx.repaint_causes());
    }
}

#[test]
fn queued_changes_are_visible_in_order_in_the_same_pass() {
    let (mut h, tx) = idle();
    let (set, set_reply) = ControlRequest::new("c0.set", json!({"panels":{"title_bar":false,"community_links":false}}));
    let (inspect, inspect_reply) = ControlRequest::new("c0.inspect", json!({}));
    tx.send(set).unwrap();
    tx.send(inspect).unwrap();
    h.step();
    assert_eq!(set_reply.try_recv().unwrap()["ok"], true);
    let r = inspect_reply.try_recv().unwrap();
    assert_eq!(r["result"]["titleBar"], false);
    assert_eq!(r["result"]["communityLinks"], false);
    assert!(h.ctx.has_requested_repaint(), "a change asks to be displayed");
}

#[test]
fn each_host_action_is_answered_without_a_presented_frame() {
    let (mut h, tx) = idle();
    request(&mut h, &tx, "c0.goToBlock", json!({"block":1}));
    request(&mut h, &tx, "c0.cancelFileDialog", json!({}));
    request(&mut h, &tx, "c0.newDocument", json!({}));
    request(&mut h, &tx, "c0.openFile", json!({"name":"host.txt","bytes":"SG9zdA=="}));
    let r = request(&mut h, &tx, "c0.inspect", json!({}));
    assert_eq!(r["words"], 1);
    assert_eq!(h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body).trim(), "Host");
}
