//! Optional page transport for WordCraft's existing control channel.
//! The web runner lends an unpainted egui pass to each call; presentation is never the clock.

use serde_json::{Value, json};

#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod files;
#[cfg(target_arch = "wasm32")]
pub use bridge::{attach, runner};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
#[cfg(target_arch = "wasm32")]
pub const POLL_MS: i32 = 4;
pub const PUMP_MS: f64 = 16.0;
pub const MAX_INLINE_PASSES: usize = 8;

pub fn request_limit(method: &str) -> usize {
    if method.starts_with("c0.") { wordcraft_ui_egui::c0_host::MAX_REQUEST_BYTES } else { MAX_REQUEST_BYTES }
}

pub fn parse(text: &str) -> Result<(String, Value), String> {
    if text.len() > wordcraft_ui_egui::c0_host::MAX_REQUEST_BYTES {
        return Err("request exceeds the host request limit".into());
    }
    let value: Value = serde_json::from_str(text).map_err(|e| format!("request is not JSON: {e}"))?;
    let method = value.get("method").and_then(Value::as_str).ok_or("request has no method")?;
    if text.len() > request_limit(method) {
        return Err("request is larger than 1 MiB".into());
    }
    let params = value.get("params").filter(|v| !v.is_null()).cloned().unwrap_or(json!({}));
    if !params.is_object() {
        return Err("params must be an object".into());
    }
    Ok((method.to_owned(), params))
}

pub fn reads_only(method: &str, params: &Value) -> bool {
    match method {
        "ui.inspect" | "document.inspect" | "ui.parity" | "engine.commands" | "c0.inspect" | "c0.documentBytes" => true,
        "engine.execute" | "command" => params
            .get("command")
            .or_else(|| params.get("id"))
            .and_then(Value::as_str)
            .is_some_and(|id| matches!(id, "document.inspect" | "format.state")),
        _ => false,
    }
}

pub fn inline_passes(method: &str) -> usize {
    // WordCraft acknowledges queued input immediately. Play it before returning to the host.
    if matches!(method, "ui.click" | "ui.clickText" | "ui.move" | "ui.press" | "ui.release" | "ui.drag" | "ui.key" | "ui.text" | "ui.wheel") {
        MAX_INLINE_PASSES
    } else {
        0
    }
}

pub fn pump_due(wants_repaint: bool, now: f64, last: f64) -> bool {
    wants_repaint && now - last >= PUMP_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_desktop_envelope_and_defaults_params() {
        assert_eq!(parse(r#"{"method":"c0.inspect"}"#).unwrap(), ("c0.inspect".into(), json!({})));
        assert_eq!(parse(r#"{"method":"c0.inspect","params":null}"#).unwrap().1, json!({}));
        for text in ["no", "[]", "{}", r#"{"method":3}"#, r#"{"method":"c0.inspect","params":4}"#] {
            assert!(parse(text).is_err());
        }
    }

    #[test]
    fn limits_requests_before_dispatch() {
        assert_eq!(request_limit("c0.openFile"), wordcraft_ui_egui::c0_host::MAX_REQUEST_BYTES);
        assert_eq!(request_limit("engine.execute"), MAX_REQUEST_BYTES);
        let large = json!({"method":"ui.text", "params":{"text":"x".repeat(MAX_REQUEST_BYTES)}}).to_string();
        assert!(parse(&large).is_err());
        assert!(parse(&large.replace("ui.text", "c0.openFile")).is_ok());
    }

    #[test]
    fn read_requests_do_not_ask_for_presentation() {
        for method in ["c0.inspect", "c0.documentBytes", "ui.inspect", "document.inspect", "engine.commands"] {
            assert!(reads_only(method, &json!({})), "{method}");
        }
        for method in ["c0.goToBlock", "c0.openFile", "c0.newDocument", "c0.cancelFileDialog", "c0.set", "ui.screenshot", "unknown"] {
            assert!(!reads_only(method, &json!({})), "{method}");
        }
        assert!(reads_only("engine.execute", &json!({"command":"document.inspect"})));
        assert!(!reads_only("engine.execute", &json!({"command":"text.insert"})));
    }

    #[test]
    fn input_and_waiting_requests_get_bounded_passes() {
        assert!(inline_passes("ui.text") > 0);
        assert_eq!(inline_passes("c0.inspect"), 0);
        assert!(pump_due(true, 1016.0, 1000.0));
        assert!(!pump_due(true, 1015.0, 1000.0));
        assert!(!pump_due(false, 5000.0, 1000.0));
    }

    #[test]
    fn web_shell_attaches_the_bridge_and_lends_its_runner() {
        let web = include_str!("web.rs");
        assert!(web.contains("crate::page_control::runner(&runner);"));
        assert!(web.contains("crate::page_control::attach(app, &cc.egui_ctx)"));
        let bridge = include_str!("page_control/bridge.rs");
        assert!(bridge.contains("app_runner.logic();"));
        assert!(bridge.contains("app.with_control(rx)"));
    }
}
