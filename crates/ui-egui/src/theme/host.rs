//! A context-local host palette, replaced as a set and never persisted in app preferences.

use super::Tokens;
use egui::Color32;
use serde_json::{Map, Value, json};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub applied: Vec<String>,
    pub unknown: Vec<String>,
    pub invalid: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({"applied": self.applied, "unknown": self.unknown, "invalid": self.invalid})
    }
}

fn store() -> egui::Id {
    egui::Id::new("wordcraft-theme-host")
}

pub fn current(ctx: &egui::Context) -> Option<Arc<Value>> {
    ctx.data(|d| d.get_temp::<Arc<Value>>(store()))
}

/// Replace the full set; null or an empty object clears it. Good keys apply even alongside bad ones.
pub fn set(ctx: &egui::Context, value: &Value) -> Result<Report, String> {
    let mut clean = Map::new();
    let mut report = Report::default();
    let mut scratch = Tokens::light();
    let entries = match value {
        Value::Null => None,
        Value::Object(entries) => Some(entries),
        _ => return Err("themeTokens must be an object or null".into()),
    };
    for (key, value) in entries.into_iter().flatten() {
        if !assign(&mut scratch, key, Color32::TRANSPARENT) {
            report.unknown.push(key.clone());
        } else if let Some(color) = colour(value) {
            clean.insert(key.clone(), value.clone());
            // Keep the registry and validation on exactly the same path.
            assign(&mut scratch, key, color);
            report.applied.push(key.clone());
        } else {
            report.invalid.push(key.clone());
        }
    }
    ctx.data_mut(|d| {
        d.remove::<Arc<Value>>(store());
        if !clean.is_empty() {
            d.insert_temp(store(), Arc::new(Value::Object(clean)));
        }
    });
    apply(ctx);
    Ok(report)
}

/// Called at the single token getter, so every surface sees the host's palette.
pub(super) fn resolve(ctx: &egui::Context, mut tokens: Tokens) -> Tokens {
    if let Some(value) = current(ctx)
        && let Some(entries) = value.as_object()
    {
        for (key, value) in entries {
            if let Some(color) = colour(value) {
                assign(&mut tokens, key, color);
            }
        }
    }
    tokens
}

/// Keep egui widgets in step too, including after upstream reapplies its appearance.
pub(super) fn apply(ctx: &egui::Context) {
    ctx.set_visuals_of(egui::Theme::Light, super::visuals(&resolve(ctx, Tokens::light())));
    ctx.set_visuals_of(egui::Theme::Dark, super::visuals(&resolve(ctx, Tokens::dark())));
}

fn colour(value: &Value) -> Option<Color32> {
    let hex = value.as_str()?.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |at| hex.get(at..at + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
    Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, if hex.len() == 8 { byte(6)? } else { 255 }))
}

// WordCraft has no radius tokens. Non-color fields (dark, author/leader arrays) are not scalar colors.
fn assign(t: &mut Tokens, key: &str, color: Color32) -> bool {
    macro_rules! colors {
        ($($field:ident),* $(,)?) => {
            match key { $(stringify!($field) => { t.$field = color; true },)* _ => false }
        };
    }
    colors!(
        title_bar,
        ribbon,
        tab_strip,
        canvas,
        panel,
        border,
        border_strong,
        text,
        text_dim,
        text_disabled,
        icon,
        hover,
        pressed,
        checked,
        accent,
        accent_text,
        on_accent,
        input,
        input_border,
        group_label,
        ruler,
        ruler_margin,
        ruler_tick,
        status_bar,
        selection,
        find_highlight,
        page_shadow,
        handle,
        handle_shadow,
        guide,
        crop_handle,
        crop_dim,
        balloon,
        markup_area,
        markup_edge,
        resolved_text,
        menu,
        blue,
        red,
        green,
        orange,
        close_hover,
        close_pressed,
        on_close,
    )
}
