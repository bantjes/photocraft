//! Marian PhotoCraft: the toolbar as its user arranges it. While the Closet window is open, a
//! toolbar tool drags up and down the toolbar (dropped between two tools it gets a button of its
//! own there; dropped onto a tool it joins that tool's group) and onto the Closet window's Closet
//! and Basement shelves. `toolbar.arrange` and `toolbar.resetArrangement` make the changes;
//! Preferences `toolbar.slots` keeps them.

use std::collections::HashSet;

use egui::{Sense, Stroke, StrokeKind, vec2};
use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::state::Tool;
use crate::theme::Tokens;

/// A toolbar tool being dragged (egui's drag-and-drop payload).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ToolDrag(pub Tool);

/// Where a dropped tool goes, beside the tool it is dropped on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Its own button, before that tool's.
    Before,
    /// Its own button, after that tool's.
    After,
    /// Into that tool's group (its flyout), last.
    Group,
}

impl Place {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "before" => Some(Self::Before),
            "after" => Some(Self::After),
            "group" => Some(Self::Group),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
            Self::Group => "group",
        }
    }

    /// Where a drop at `along` (0 at the button's start, 1 at its end, along the toolbar)
    /// lands: the outer quarters go between buttons, the middle into the group.
    pub fn at(along: f32) -> Self {
        if along < 0.25 {
            Self::Before
        } else if along > 0.75 {
            Self::After
        } else {
            Self::Group
        }
    }
}

/// The standard toolbar's slots, top to bottom.
pub fn standard() -> Vec<Vec<Tool>> {
    crate::panels::toolbar_sections().iter().flat_map(|section| section.iter().map(|slot| slot.to_vec())).collect()
}

/// The selection tools this build groups under one button; Magic Wand keeps one of its own.
const SELECTION: [Tool; 9] = [
    Tool::RectMarquee,
    Tool::EllipseMarquee,
    Tool::SingleRowMarquee,
    Tool::SingleColumnMarquee,
    Tool::Lasso,
    Tool::PolygonLasso,
    Tool::MagneticLasso,
    Tool::ObjectSelection,
    Tool::QuickSelection,
];

/// The toolbar this build starts with: the standard one with every selection tool in the
/// marquee's group, and Magic Wand alone just after it.
pub fn starting() -> Vec<Vec<Tool>> {
    let mut out: Vec<Vec<Tool>> = Vec::new();
    for slot in standard() {
        if slot.contains(&Tool::RectMarquee) {
            out.push(SELECTION.to_vec());
            out.push(vec![Tool::MagicWand]);
        }
        let rest: Vec<Tool> = slot.into_iter().filter(|tool| !SELECTION.contains(tool) && *tool != Tool::MagicWand).collect();
        if !rest.is_empty() {
            out.push(rest);
        }
    }
    out
}

/// The arrangement before any is saved: [`starting`], or the standard one where the build's
/// starting sort is off (tests).
fn base(app: &PhotocraftApp) -> Vec<Vec<Tool>> {
    if crate::closet::sorted(app) { starting() } else { standard() }
}

/// The arranged toolbar's slots, or `None` while it has the standard arrangement.
pub fn slots(app: &PhotocraftApp) -> Option<Vec<Vec<Tool>>> {
    let saved = &app.session.prefs().toolbar.slots;
    if saved.is_empty() {
        return crate::closet::sorted(app).then(starting);
    }
    Some(complete(saved))
}

/// Whether the toolbar was rearranged since the build's starting arrangement.
pub fn rearranged(app: &PhotocraftApp) -> bool {
    !app.session.prefs().toolbar.slots.is_empty()
}

/// `saved` read back: names nobody knows and repeats are dropped, and a tool it lacks (one added
/// to the app since, say) joins the slot holding a tool of its standard slot, or gets a slot of
/// its own at the end.
fn complete(saved: &[Vec<String>]) -> Vec<Vec<Tool>> {
    let mut seen = HashSet::new();
    let mut out: Vec<Vec<Tool>> = saved
        .iter()
        .take(200)
        .map(|slot| slot.iter().take(50).filter_map(|name| Tool::from_name(name)).filter(|tool| seen.insert(*tool)).collect::<Vec<_>>())
        .filter(|slot| !slot.is_empty())
        .collect();
    for slot in standard() {
        for tool in &slot {
            if !seen.insert(*tool) {
                continue;
            }
            match out.iter_mut().find(|s| s.iter().any(|t| slot.contains(t))) {
                Some(home) => home.push(*tool),
                None => out.push(vec![*tool]),
            }
        }
    }
    out
}

/// Move `tool` to `place` beside `to`. False when nothing changes.
pub fn arrange(slots: &mut Vec<Vec<Tool>>, tool: Tool, to: Tool, place: Place) -> bool {
    let before = slots.clone();
    if tool == to {
        // Out of its group, onto a button of its own beside it.
        let Some(i) = slots.iter().position(|s| s.contains(&tool)) else { return false };
        if place == Place::Group || slots.get(i).is_none_or(|s| s.len() < 2) {
            return false;
        }
        if let Some(slot) = slots.get_mut(i) {
            slot.retain(|t| *t != tool);
        }
        slots.insert(if place == Place::Before { i } else { i + 1 }, vec![tool]);
        return true;
    }
    for slot in slots.iter_mut() {
        slot.retain(|t| *t != tool);
    }
    slots.retain(|s| !s.is_empty());
    let Some(i) = slots.iter().position(|s| s.contains(&to)) else {
        *slots = before;
        return false;
    };
    match place {
        Place::Group => {
            if let Some(slot) = slots.get_mut(i) {
                slot.push(tool);
            }
        }
        Place::Before => slots.insert(i, vec![tool]),
        Place::After => slots.insert(i + 1, vec![tool]),
    }
    *slots != before
}

fn names(slots: &[Vec<Tool>]) -> Vec<Vec<String>> {
    slots.iter().map(|slot| slot.iter().map(|tool| format!("{tool:?}")).collect()).collect()
}

fn save(app: &mut PhotocraftApp, slots: &[Vec<Tool>]) {
    let saved = if slots == base(app).as_slice() { Vec::new() } else { names(slots) };
    app.session.prefs.edit(|prefs| prefs.toolbar.slots = saved);
}

/// `toolbar.arrange {tool, to, place?}` and `toolbar.resetArrangement`; `None` for other ids.
pub fn command(app: &mut PhotocraftApp, id: &str, p: &Value) -> Option<Result<Value, String>> {
    Some(match id {
        "toolbar.arrange" => (|| {
            let tool_of = |key: &str| -> Result<Tool, String> {
                let name = p.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} is required: a tool name"))?;
                Tool::from_name(name).ok_or_else(|| format!("no tool called `{name}`"))
            };
            let (tool, to) = (tool_of("tool")?, tool_of("to")?);
            let place = match p.get("place") {
                None | Some(Value::Null) => Place::Before,
                Some(v) => v.as_str().and_then(Place::parse).ok_or("place must be before, after or group")?,
            };
            let mut slots = slots(app).unwrap_or_else(|| base(app));
            if arrange(&mut slots, tool, to, place) {
                save(app, &slots);
            }
            Ok(json!({ "slots": names(&slots), "place": place.name() }))
        })(),
        "toolbar.resetArrangement" => {
            app.session.prefs.edit(|prefs| prefs.toolbar.slots = Vec::new());
            Ok(json!({ "slots": names(&base(app)) }))
        }
        _ => return None,
    })
}

/// While the toolbar is being arranged: the tool button `button` showing `tool` drags, shows where
/// a dragged tool would land, and takes the drop. `across`: the toolbar has two columns, so
/// buttons sit side by side and "before" is to the left.
pub(crate) fn slot(app: &mut PhotocraftApp, ui: &mut egui::Ui, button: &egui::Response, tool: Tool, across: bool) {
    let ctx = ui.ctx().clone();
    let drag = ui.interact(button.rect, button.id.with("arrange"), Sense::drag());
    if drag.drag_started() {
        egui::DragAndDrop::set_payload(&ctx, ToolDrag(tool));
    }
    if drag.dragged()
        && let Some(at) = ctx.pointer_interact_pos()
    {
        // The tool follows the pointer.
        let t = Tokens::get(&ctx);
        egui::Area::new(button.id.with("ghost")).order(egui::Order::Tooltip).interactable(false).fixed_pos(at - vec2(15.0, 15.0)).show(&ctx, |ui| {
            let (ghost, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
            ui.painter().rect_filled(ghost, 4.0, t.card);
            ui.painter().rect_stroke(ghost, 4.0, Stroke::new(1.0, t.accent), StrokeKind::Inside);
            crate::icons::paint(ui, ghost, crate::icons::tool_icon(tool), 18.0, t.icon);
        });
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    let place = |pos: egui::Pos2| {
        let r = button.rect;
        let along = if across { (pos.x - r.left()) / r.width().max(1.0) } else { (pos.y - r.top()) / r.height().max(1.0) };
        Place::at(along)
    };
    // Checked by position: while one button drags, egui reports no other button under the pointer.
    let over = |pos: Option<egui::Pos2>| pos.is_some_and(|p| button.rect.contains(p)) && egui::DragAndDrop::has_payload_of_type::<ToolDrag>(&ctx);
    if over(ctx.pointer_hover_pos())
        && let Some(pos) = ctx.pointer_hover_pos()
    {
        let t = Tokens::get(&ctx);
        let r = button.rect;
        let stroke = Stroke::new(3.0, t.accent);
        let painter = ui.painter();
        match (place(pos), across) {
            (Place::Group, _) => {
                painter.rect_stroke(r.expand(1.0), 4.0, Stroke::new(2.0, t.accent), StrokeKind::Outside);
            }
            (Place::Before, false) => {
                painter.line_segment([r.left_top() - vec2(0.0, 2.0), r.right_top() - vec2(0.0, 2.0)], stroke);
            }
            (Place::After, false) => {
                painter.line_segment([r.left_bottom() + vec2(0.0, 2.0), r.right_bottom() + vec2(0.0, 2.0)], stroke);
            }
            (Place::Before, true) => {
                painter.line_segment([r.left_top() - vec2(2.0, 0.0), r.left_bottom() - vec2(2.0, 0.0)], stroke);
            }
            (Place::After, true) => {
                painter.line_segment([r.right_top() + vec2(2.0, 0.0), r.right_bottom() + vec2(2.0, 0.0)], stroke);
            }
        }
    }
    let released = ctx.input(|i| i.pointer.any_released());
    if released
        && over(ctx.pointer_interact_pos())
        && let Some(pos) = ctx.pointer_interact_pos()
        && let Some(dropped) = egui::DragAndDrop::take_payload::<ToolDrag>(&ctx)
    {
        let params = json!({ "tool": format!("{:?}", dropped.0), "to": format!("{tool:?}"), "place": place(pos).name() });
        if let Err(e) = crate::menus::invoke(app, &ctx, "toolbar.arrange", params) {
            app.ui.status = e;
        }
    }
}

#[cfg(test)]
mod tests;
