//! Host tool modes for the ribbon and the foldable right dock. State lives in the egui context,
//! like the host theme tokens: never saved to preferences, and every default is upstream's.
//!
//! WordCraft has a ribbon, not a tool palette, so the PhotoCraft tool-strip keys map onto it:
//! `toolbar` is the ribbon body shown (false: collapsed to its tabs), `toolbar_double` the compact
//! single row (every group at its compact level), `toolbar_floating` the ribbon body in a movable
//! strip over the page. The tab strip is never removed.

use egui::{Context, CornerRadius, Id, Order, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, vec2};
use serde_json::json;

use crate::WordApp;
use crate::ribbon_fit::{self, GroupWidths};
use crate::theme::Tokens;
use crate::widgets::{CONTENT_H, LABEL_H};

/// The ribbon body's height: upstream's docked panel.
const RIBBON_H: f32 = CONTENT_H + LABEL_H + 10.0;
const GRIP_H: f32 = 14.0;
const EDGE: f32 = 12.0;
const MAX_W: f32 = 960.0;
const STRIP_W: f32 = 34.0;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Chrome {
    pub compact: bool,
    pub floating: bool,
    pub dock_folded: bool,
    /// The floating strip's corner in points from the free area's corner; `None` is the default.
    offset: Option<[f32; 2]>,
    /// Which right panes were open last frame, to unfold when one opens.
    seen: [bool; 6],
}

fn state_id() -> Id {
    Id::new("c0_wordcraft_chrome")
}

pub(crate) fn get(ctx: &Context) -> Chrome {
    ctx.data(|d| d.get_temp::<Chrome>(state_id())).unwrap_or_default()
}

pub(crate) fn update(ctx: &Context, change: impl FnOnce(&mut Chrome)) {
    ctx.data_mut(|d| change(d.get_temp_mut_or_default::<Chrome>(state_id())));
}

/// Fold or unfold the right dock. Folding remembers what is open, so only a pane opened later unfolds it.
pub(crate) fn set_dock_folded(app: &WordApp, ctx: &Context, folded: bool) {
    let open = right_open(app);
    update(ctx, |c| {
        c.dock_folded = folded;
        c.seen = open;
    });
}

/// The ribbon fit plan for one tab. Upstream's when the compact row is off.
pub(crate) fn levels(ctx: &Context, mut groups: Vec<GroupWidths>, count: usize, avail: f32) -> Vec<u8> {
    if !get(ctx).compact {
        return if groups.iter().all(|g| g.widths[0].is_some()) { ribbon_fit::plan(&groups, avail) } else { Vec::new() };
    }
    // Plan from the compact widths, so a group is collapsed only when even its compact form is too wide.
    for g in &mut groups {
        g.widths[0] = g.widths[1].or(g.widths[0]);
    }
    let mut levels = if groups.iter().all(|g| g.widths[0].is_some()) { ribbon_fit::plan(&groups, avail) } else { vec![ribbon_fit::COMPACT; count] };
    for level in &mut levels {
        *level = (*level).max(ribbon_fit::COMPACT);
    }
    levels
}

/// The Ui the ribbon body is drawn in when it floats: a child of a movable area over the page.
/// `None` draws it docked, as upstream does.
pub(crate) fn float(ui: &Ui) -> Option<Ui> {
    let ctx = ui.ctx().clone();
    let state = get(&ctx);
    if !state.floating {
        return None;
    }
    let t = Tokens::get(&ctx);
    let free = ui.available_rect_before_wrap();
    let size = vec2((free.width() - 2.0 * EDGE).clamp(1.0, MAX_W), GRIP_H + RIBBON_H);
    let room = (free.size() - size).max(Vec2::ZERO);
    let [dx, dy] = state.offset.unwrap_or([EDGE, EDGE]);
    let at = free.min + vec2(dx.clamp(0.0, room.x), dy.clamp(0.0, room.y));
    let area = egui::Area::new(Id::new("c0_ribbon_float")).order(Order::Middle).fixed_pos(at).show(&ctx, |area| {
        let (grip, drag) = area.allocate_exact_size(vec2(size.x, GRIP_H), Sense::drag());
        let top = CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
        area.painter().rect(grip, top, t.ribbon, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        for i in -1..=1 {
            area.painter().circle_filled(grip.center() + vec2(i as f32 * 6.0, 0.0), 1.5, t.text_dim);
        }
        let drag = drag.on_hover_cursor(egui::CursorIcon::Grab).on_hover_text(crate::tl!("Move Ribbon"));
        if drag.dragged() {
            let moved = drag.drag_delta();
            update(&ctx, |c| c.offset = Some([dx + moved.x, dy + moved.y]));
        }
        let (body, _) = area.allocate_exact_size(vec2(size.x, RIBBON_H), Sense::hover());
        area.new_child(UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(egui::Align::Min)))
    });
    Some(area.inner)
}

/// Open right-hand panes, in strip order.
const RIGHT: [(&str, &str, &str); 6] = [
    ("view.stylesPane", "styles", "Styles"),
    ("styles.inspector", "styles", "Style Inspector"),
    ("view.accessibilityPane", "accessibility", "Accessibility"),
    ("view.altTextPane", "altText", "Alt Text"),
    ("", "restrict", "Restrict Editing"),
    ("view.commentsPane", "comment", "Comments"),
];

fn right_open(app: &WordApp) -> [bool; 6] {
    let v = &app.session.view;
    [v.styles_pane, v.style_inspector, v.accessibility_pane, v.alt_text_pane, app.restrict.open, v.comments_pane]
}

fn set_right(app: &mut WordApp, open: [bool; 6]) {
    let v = &mut app.session.view;
    [v.styles_pane, v.style_inspector, v.accessibility_pane, v.alt_text_pane, app.restrict.open, v.comments_pane] = open;
}

/// The side panes. With the right dock folded, the right panes give way to an icon strip.
pub(crate) fn panes(app: &mut WordApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let open = right_open(app);
    let state = get(&ctx);
    let opened = open.iter().zip(state.seen).any(|(now, before)| *now && !before);
    if !state.dock_folded || opened {
        update(&ctx, |c| {
            c.dock_folded = false;
            c.seen = open;
        });
        return crate::panes::show(app, ui);
    }
    update(&ctx, |c| c.seen = open);
    set_right(app, [false; 6]);
    crate::panes::show(app, ui);
    set_right(app, open);
    strip(app, ui, open);
}

fn strip(app: &mut WordApp, ui: &mut Ui, open: [bool; 6]) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    egui::Panel::right("c0_right_dock")
        .exact_size(STRIP_W)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(4, 8)).stroke(Stroke::new(1.0, t.border)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
            if crate::widgets::icon_button(ui, "chevronLeft", crate::tl!("Expand panels")).clicked() {
                update(&ctx, |c| c.dock_folded = false);
            }
            ui.add_space(4.0);
            for (i, (command, icon, label)) in RIGHT.into_iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                let fill = if open[i] { t.checked } else if resp.hovered() { t.hover } else { egui::Color32::TRANSPARENT };
                ui.painter().rect_filled(r, CornerRadius::same(4), fill);
                crate::icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(14.0, 14.0)), icon, t.icon, t.accent);
                if resp.on_hover_text(crate::tl!(label)).clicked() {
                    if command.is_empty() {
                        app.restrict.open = true;
                    } else {
                        let _ = app.run(command, json!({"value": true}));
                    }
                    update(&ctx, |c| c.dock_folded = false);
                }
            }
        });
}
