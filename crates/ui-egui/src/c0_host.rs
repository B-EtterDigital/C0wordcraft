//! Host chrome switches; standalone defaults retain the upstream interface.

use crate::WordApp;
use serde_json::{Value, json};

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
