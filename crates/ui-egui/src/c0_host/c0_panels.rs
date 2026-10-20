//! Per-key host settings reports. Pane commands retain WordCraft's view semantics.

use crate::{WordApp, theme::host};
use serde_json::{Value, json};

pub(super) fn set(app: &mut WordApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    // Validate the containers before applying either family.
    if params.get("panels").is_some_and(|v| !v.is_object()) {
        return Err("panels must be an object".into());
    }
    if params.get("themeTokens").is_some_and(|v| !v.is_object() && !v.is_null()) {
        return Err("themeTokens must be an object or null".into());
    }
    let mut report = host::Report::default();
    for key in params.as_object().into_iter().flat_map(|p| p.keys()) {
        if !matches!(key.as_str(), "theme" | "themeTokens" | "panels") {
            report.unknown.push(key.clone());
        }
    }
    if let Some(theme) = params.get("theme") {
        if matches!(theme.as_str(), Some("light" | "dark")) {
            app.execute("ui.theme", json!({"value": theme}))?;
            crate::theme::apply(ctx, app.ui.theme);
            report.applied.push("theme".into());
        } else {
            report.invalid.push("theme".into());
        }
    }
    if let Some(tokens) = params.get("themeTokens") {
        let tokens = host::set(ctx, tokens)?;
        report.applied.extend(tokens.applied.into_iter().map(|key| format!("themeTokens.{key}")));
        report.unknown.extend(tokens.unknown.into_iter().map(|key| format!("themeTokens.{key}")));
        report.invalid.extend(tokens.invalid.into_iter().map(|key| format!("themeTokens.{key}")));
    }
    if let Some(panels) = params.get("panels").and_then(Value::as_object) {
        let mut applied = Vec::new();
        let mut unknown = Vec::new();
        let mut invalid = Vec::new();
        for (key, value) in panels {
            let command = match key.as_str() {
                "title_bar" => "ui.titleBar",
                "community_links" => "ui.communityLinks",
                "start_screen" => "ui.startScreen",
                "ruler" => "view.ruler",
                "navigation_pane" => "view.navigationPane",
                "styles_pane" => "view.stylesPane",
                "comments_pane" => "view.commentsPane",
                "ribbon_collapsed" => "ui.collapseRibbon",
                _ => {
                    unknown.push(key);
                    continue;
                }
            };
            let Some(on) = value.as_bool() else {
                invalid.push(key);
                continue;
            };
            let r = if key == "ribbon_collapsed" {
                if app.ui.ribbon_collapsed == on { Ok(json!({})) } else { app.execute(command, json!({})) }
            } else if command.starts_with("view.") && !app.has_document() {
                app.session.run(command, &json!({"value": on})).map_err(|e| e.to_string())
            } else {
                app.execute(command, json!({"value": on}))
            };
            if r.is_ok() {
                applied.push(key);
            } else {
                invalid.push(key);
            }
        }
        report.applied.extend(applied.into_iter().map(|key| format!("panels.{key}")));
        report.unknown.extend(unknown.into_iter().map(|key| format!("panels.{key}")));
        report.invalid.extend(invalid.into_iter().map(|key| format!("panels.{key}")));
    }
    Ok(report.to_json())
}
