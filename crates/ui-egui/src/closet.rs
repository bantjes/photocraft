//! The Closet and the Basement (Window › Closet…): tools, panels and menu items put out of sight
//! without deleting anything. The Closet holds what is used now and then, one click from coming
//! back; the Basement, a tab further away, holds what will probably never be used. A tool put away
//! leaves the toolbar and its letter key; a panel leaves the Window menu and its dock; a menu item
//! or a whole submenu leaves its menu. Restoring puts each back where it was.
//!
//! Everything is named by a key: `tool:<Tool name>` (`tool:MagneticLasso`), `panel:<id>`
//! (`panel:navigator`), `menu:<command id>` (`menu:file.automate.createDroplet`) or
//! `submenu:<Menu › Submenu>` (`submenu:Filter › Video`). [`SHIPPED`] is the sort this build
//! starts with; `Preferences::closet` keeps only what was moved since, so it is saved with the
//! preferences.

use std::collections::BTreeSet;

use egui::{RichText, vec2};
use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::menus::MenuItem;
use crate::state::Tool;
use crate::theme::Tokens;

/// The Closet's own commands.
pub const IDS: [&str; 4] = ["window.closet", "closet.set", "closet.reset", "closet.list"];

/// Where something lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    InUse,
    Closet,
    Basement,
}

impl Shelf {
    const ALL: [Self; 3] = [Self::InUse, Self::Closet, Self::Basement];

    pub fn name(self) -> &'static str {
        match self {
            Self::InUse => "inUse",
            Self::Closet => "closet",
            Self::Basement => "basement",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|shelf| shelf.name() == s)
    }
}

use Shelf::{Basement, Closet};

/// The sort this build starts with (Emma's plan, 28 October 2026).
pub const SHIPPED: &[(&str, Shelf)] = &[
    // Tools.
    ("tool:Artboard", Closet),
    ("tool:SingleRowMarquee", Basement),
    ("tool:SingleColumnMarquee", Basement),
    ("tool:MagneticLasso", Closet),
    ("tool:PerspectiveCrop", Closet),
    ("tool:Slice", Basement),
    ("tool:SliceSelect", Basement),
    ("tool:Frame", Basement),
    ("tool:ColorSampler", Closet),
    ("tool:Ruler", Closet),
    ("tool:Note", Closet),
    ("tool:Count", Closet),
    ("tool:ContentAwareMove", Closet),
    ("tool:RedEye", Closet),
    ("tool:MixerBrush", Closet),
    ("tool:ColorReplacement", Closet),
    ("tool:PatternStamp", Basement),
    ("tool:ArtHistoryBrush", Closet),
    ("tool:Blur", Closet),
    ("tool:Sharpen", Closet),
    ("tool:Smudge", Closet),
    ("tool:Pen", Basement),
    ("tool:FreeformPen", Basement),
    ("tool:CurvaturePen", Basement),
    ("tool:AddAnchorPoint", Basement),
    ("tool:DeleteAnchorPoint", Basement),
    ("tool:ConvertPoint", Basement),
    ("tool:PathSelection", Basement),
    ("tool:DirectSelection", Basement),
    ("tool:Rectangle", Closet),
    ("tool:EllipseShape", Closet),
    ("tool:Triangle", Closet),
    ("tool:Polygon", Closet),
    ("tool:Line", Closet),
    ("tool:CustomShape", Closet),
    // Panels.
    ("panel:navigator", Closet),
    ("panel:histogram", Closet),
    ("panel:patterns", Closet),
    ("panel:shapes", Closet),
    ("panel:styles", Closet),
    ("panel:toolPresets", Closet),
    ("panel:layerComps", Closet),
    ("panel:timeline", Basement),
    ("panel:measurementLog", Basement),
    ("panel:notes", Basement),
    ("panel:modifierKeys", Basement),
    // Menu items: video, data sets and variables, and the Automate extras.
    ("menu:file.export.renderVideo", Basement),
    ("menu:file.import.videoFramesToLayers", Basement),
    ("submenu:Filter › Video", Basement),
    ("submenu:Layer › Video Layers", Basement),
    ("menu:file.export.dataSetsAsFiles", Basement),
    ("menu:file.import.variableDataSets", Basement),
    ("submenu:Image › Variables", Basement),
    ("menu:image.applyDataSet", Basement),
    ("menu:file.automate.createDroplet", Basement),
    ("menu:file.automate.contactSheetII", Basement),
    ("menu:file.automate.mergeToHdrPro", Basement),
    ("menu:file.import.wiaSupport", Basement),
    ("menu:window.workspace.motion", Basement),
];

/// Where [`SHIPPED`] puts `key` (in use when it doesn't list it).
fn shipped(app: &PhotocraftApp, key: &str) -> Shelf {
    if !sorted(app) {
        return Shelf::InUse;
    }
    SHIPPED.iter().find(|(k, _)| *k == key).map_or(Shelf::InUse, |(_, s)| *s)
}

#[cfg(test)]
thread_local! {
    /// Tests start with everything in use, so the app's own tests see every tool, panel and menu
    /// item; the Closet's tests turn the starting sort on.
    static SORTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the starting sort applies (`ShellUi::without_starting_sort` turns it off).
pub(crate) fn sorted(app: &PhotocraftApp) -> bool {
    #[cfg(test)]
    if !SORTED.get() {
        return false;
    }
    !app.ui.shell.without_starting_sort
}

#[cfg(test)]
pub(crate) fn use_starting_sort() {
    SORTED.set(true);
}

/// Where `key` lives now.
pub fn shelf(app: &PhotocraftApp, key: &str) -> Shelf {
    app.session.prefs().closet.get(key).and_then(|s| Shelf::parse(s)).unwrap_or_else(|| shipped(app, key))
}

/// Is `key` put away (in the Closet or the Basement)?
pub fn hidden(app: &PhotocraftApp, key: &str) -> bool {
    shelf(app, key) != Shelf::InUse
}

/// Every key put away now.
fn put_away(app: &PhotocraftApp) -> BTreeSet<String> {
    let moved = &app.session.prefs().closet;
    let mut out: BTreeSet<String> = if sorted(app) {
        SHIPPED.iter().filter(|(k, s)| *s != Shelf::InUse && !moved.contains_key(*k)).map(|(k, _)| k.to_string()).collect()
    } else {
        BTreeSet::new()
    };
    out.extend(moved.iter().filter(|(_, s)| Shelf::parse(s).is_some_and(|s| s != Shelf::InUse)).map(|(k, _)| k.clone()));
    out
}

pub fn tool_key(tool: Tool) -> String {
    format!("tool:{tool:?}")
}

pub fn tool_hidden(app: &PhotocraftApp, tool: Tool) -> bool {
    hidden(app, &tool_key(tool))
}

/// The tools put away, by name (as `Preferences::toolbar.hidden` lists them).
pub fn hidden_tool_names(app: &PhotocraftApp) -> Vec<String> {
    put_away(app).iter().filter_map(|k| k.strip_prefix("tool:")).map(str::to_owned).collect()
}

pub fn panel_hidden(app: &PhotocraftApp, id: &str) -> bool {
    hidden(app, &format!("panel:{id}"))
}

// ---------- menus ----------

/// `items` without what is put away: items, whole submenus and the Window menu's put-away panels,
/// with no separator left first, last or twice running in a menu.
pub fn prune_menu(app: &PhotocraftApp, items: &mut Vec<MenuItem>) {
    let away = put_away(app);
    if away.is_empty() {
        return;
    }
    let submenus: Vec<Vec<String>> = away
        .iter()
        .filter_map(|k| k.strip_prefix("submenu:"))
        .map(|p| p.split(" › ").map(str::to_owned).collect::<Vec<_>>())
        .filter(|p| !p.is_empty())
        .collect();
    items.retain(|item| {
        if submenus.iter().any(|sub| item.path.starts_with(sub)) {
            return false;
        }
        if item.id == "---" {
            return true;
        }
        if let Some(panel) = item.id.strip_prefix("window.panel.")
            && away.contains(&format!("panel:{panel}"))
        {
            return false;
        }
        !away.contains(&format!("menu:{}", item.id))
    });
    tidy(items);
}

/// Drop separators with nothing of their menu before them (since the last separator) or after them.
fn tidy(items: &mut Vec<MenuItem>) {
    let all = std::mem::take(items);
    let sep = |item: &MenuItem| item.id == "---";
    let mut kept: Vec<MenuItem> = Vec::with_capacity(all.len());
    for (i, item) in all.iter().enumerate() {
        if sep(item) {
            let inside = |other: &&MenuItem| other.path.starts_with(&item.path);
            let before = kept.iter().rev().find(inside).is_some_and(|prev| !(sep(prev) && prev.path == item.path));
            let after = all.iter().skip(i + 1).filter(inside).any(|next| !(sep(next) && next.path == item.path));
            if !before || !after {
                continue;
            }
        }
        kept.push(item.clone());
    }
    *items = kept;
}

// ---------- what can be put away ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Tool,
    Panel,
    Menu,
}

impl Kind {
    fn heading(self) -> &'static str {
        match self {
            Self::Tool => "Tools",
            Self::Panel => "Panels",
            Self::Menu => "Menus",
        }
    }
}

/// One thing that can be put away.
#[derive(Clone, Debug)]
pub struct Thing {
    pub key: String,
    pub kind: Kind,
    /// Its name in the UI language (a menu item's whole path).
    pub label: String,
}

/// Panels that stay: the Tools bar holds the way back to the Closet, and the Options bar.
const KEPT_PANELS: [&str; 2] = ["tools", "options"];

/// Commands that stay in their menus: the way into the Closet and what Edit › Menus keeps.
fn keeps(id: &str) -> bool {
    id == "window.closet" || !photocraft_engine::prefs::MenuCustomization::hideable(id)
}

/// Every tool, panel and menu item, in that order (menus in menu-bar order).
pub fn things() -> Vec<Thing> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for section in crate::panels::toolbar_sections() {
        for slot in section.iter() {
            for tool in slot.iter() {
                if seen.insert(*tool) {
                    out.push(Thing { key: tool_key(*tool), kind: Kind::Tool, label: tl!(tool.label()).to_string() });
                }
            }
        }
    }
    let mut menu: Vec<(&str, &str, Vec<&str>)> = Vec::new();
    for &(path, label, _, id) in crate::menu_catalog::CATALOG {
        menu.push((id, label, path.to_vec()));
    }
    for &(id, label, path, _) in crate::menus::UI_COMMANDS {
        menu.push((id, label, path.to_vec()));
    }
    for spec in photocraft_engine::command_specs().iter().filter(|c| !c.menu.is_empty()) {
        menu.push((spec.id, spec.label, spec.menu.to_vec()));
    }
    let mut ids = BTreeSet::new();
    for (id, label, path) in &menu {
        if let Some(panel) = id.strip_prefix("window.panel.")
            && !KEPT_PANELS.contains(&panel)
            && path.first() == Some(&"Window")
            && ids.insert(id.to_string())
        {
            out.push(Thing { key: format!("panel:{panel}"), kind: Kind::Panel, label: tl!(label).to_string() });
        }
    }
    let shown = |path: &[&str]| path.iter().map(|p| tl!(p).to_string()).collect::<Vec<_>>().join(" › ");
    let mut menus: Vec<Thing> = Vec::new();
    let mut subs = BTreeSet::new();
    for (id, label, path) in &menu {
        if path.is_empty() || *id == "---" || id.starts_with("window.panel.") || id.starts_with("window.toggle.") || keeps(id) || !ids.insert(id.to_string()) {
            continue;
        }
        for depth in 2..=path.len() {
            let sub = path.get(..depth).unwrap_or_default();
            if subs.insert(sub.join(" › ")) {
                menus.push(Thing { key: format!("submenu:{}", sub.join(" › ")), kind: Kind::Menu, label: format!("{} ›", shown(sub)) });
            }
        }
        menus.push(Thing { key: format!("menu:{id}"), kind: Kind::Menu, label: format!("{} › {}", shown(path), tl!(label)) });
    }
    // Menu-bar order, then each menu's items in their order.
    let top = |t: &Thing| {
        let first = t.key.split_once(':').map_or("", |(_, rest)| rest);
        let name =
            menu.iter().find(|(id, ..)| first == *id).and_then(|(_, _, p)| p.first().copied()).unwrap_or_else(|| first.split(" › ").next().unwrap_or(""));
        crate::menus::TOP_MENUS.iter().position(|m| *m == name).unwrap_or(crate::menus::TOP_MENUS.len())
    };
    menus.sort_by_key(top);
    out.extend(menus);
    out
}

// ---------- commands ----------

/// `window.closet`, `closet.set {key, shelf}`, `closet.reset {all?}` and `closet.list {shelf?}`;
/// `None` for other ids.
pub fn command(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str, p: &Value) -> Option<Result<Value, String>> {
    Some(match id {
        "window.closet" => {
            let open = p.get("show").and_then(Value::as_bool).unwrap_or(!app.ui.shell.closet_open);
            app.ui.shell.closet_open = open;
            Ok(json!({ "visible": open }))
        }
        "closet.set" => (|| {
            let key = p.get("key").and_then(Value::as_str).ok_or("key is required: tool:<name>, panel:<id>, menu:<command id> or submenu:<Menu › Submenu>")?;
            let to = p.get("shelf").and_then(Value::as_str).and_then(Shelf::parse).ok_or("shelf must be inUse, closet or basement")?;
            if !things().iter().any(|t| t.key == key) {
                return Err(format!("nothing called `{key}` can be put away"));
            }
            set(app, key, to);
            tidy_away(app, ctx);
            Ok(json!({ "key": key, "shelf": to.name() }))
        })(),
        "closet.reset" => (|| {
            let all = match p.get("all") {
                None | Some(Value::Null) => false,
                Some(v) => v.as_bool().ok_or("all must be a boolean")?,
            };
            let moved =
                if all && sorted(app) { SHIPPED.iter().map(|(k, _)| (k.to_string(), Shelf::InUse.name().to_string())).collect() } else { Default::default() };
            app.session.prefs.edit(|prefs| prefs.closet = moved);
            tidy_away(app, ctx);
            Ok(list(app, None))
        })(),
        "closet.list" => (|| {
            let shelf = match p.get("shelf").and_then(Value::as_str) {
                None => None,
                Some(s) => Some(Shelf::parse(s).ok_or("shelf must be inUse, closet or basement")?),
            };
            Ok(list(app, shelf))
        })(),
        _ => return None,
    })
}

pub fn checked(app: &PhotocraftApp, id: &str) -> Option<bool> {
    (id == "window.closet").then_some(app.ui.shell.closet_open)
}

/// Move `key` to `to`, remembering only what differs from [`SHIPPED`].
fn set(app: &mut PhotocraftApp, key: &str, to: Shelf) {
    let shipped = shipped(app, key);
    app.session.prefs.edit(|prefs| {
        if to == shipped {
            prefs.closet.remove(key);
        } else {
            prefs.closet.insert(key.to_string(), to.name().to_string());
        }
    });
}

/// After a move: the current tool, if put away, gives way to the Move tool, and panels put away
/// close.
fn tidy_away(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if tool_hidden(app, app.ui.tool) {
        crate::tool_memory::select(app, Tool::Move);
    }
    let panels: Vec<String> = put_away(app).iter().filter_map(|k| k.strip_prefix("panel:")).map(str::to_owned).collect();
    for panel in panels {
        close_panel(app, ctx, &panel);
    }
}

/// Close `panel` if it is showing (docked, floating or in its own window).
fn close_panel(app: &mut PhotocraftApp, ctx: &egui::Context, panel: &str) {
    let id = format!("window.panel.{panel}");
    if app.ui.docking.is_none()
        && let Some((group, label)) = crate::panel_docking::group_of(panel)
    {
        let pro = Tokens::get(ctx).pro;
        if let Some(tab) = group.tabs(pro).iter().position(|t| *t == label)
            && app.ui.dock.visible_tabs(group, pro).iter().any(|(i, _)| *i == tab)
        {
            crate::dock::close_tab(app, group, tab, pro);
        }
        return;
    }
    if crate::menus::item_checked(app, &id) == Some(true) {
        // Closing a panel nobody can see is no error worth reporting.
        let _ = crate::menus::invoke(app, ctx, &id, json!({ "show": false }));
    }
}

/// The keys on `shelf` (every key put away, by shelf, without one).
fn list(app: &PhotocraftApp, shelf: Option<Shelf>) -> Value {
    let things = things();
    let on = |s: Shelf| things.iter().filter(|t| self::shelf(app, &t.key) == s).map(|t| t.key.clone()).collect::<Vec<_>>();
    match shelf {
        Some(s) => json!(on(s)),
        None => json!({ "closet": on(Shelf::Closet), "basement": on(Shelf::Basement) }),
    }
}

// ---------- the window ----------

/// The window's tabs.
const TABS: [&str; 3] = ["Closet", "Basement", "Everything"];

fn tab_id() -> egui::Id {
    egui::Id::unique("closet-tab")
}

fn search_id() -> egui::Id {
    egui::Id::unique("closet-search")
}

fn started_id() -> egui::Id {
    egui::Id::unique("closet-started")
}

/// The panel groups this build starts with, made from the standard ones: (panel, the panel it
/// goes beside, in a group of its own just below that panel's group rather than in it).
/// Layers + Channels, Properties + Info, Color + Swatches + Gradients, Actions alone, History
/// alone, Character + Paragraph, Brushes; Adjustments and Paths get groups of their own.
const GROUPS: &[(&str, &str, bool)] =
    &[("info", "properties", false), ("adjustments", "properties", true), ("paths", "layers", true), ("actions", "history", true)];

/// Panels this build starts with showing, besides the standard ones.
const SHOWN: &[&str] = &["info", "history", "actions"];

/// While the panels have the standard arrangement (at the start, and after Reset Workspace),
/// regroup them as [`GROUPS`] says. A workspace the user changed stays as it is.
fn starting_groups(app: &mut PhotocraftApp, ctx: &egui::Context) {
    if !sorted(app) || app.ui.docking.is_some() {
        return;
    }
    let done = egui::Id::unique("closet-groups-generation");
    if ctx.data(|d| d.get_temp::<u64>(done)) == Some(app.ui.docking_generation) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, app.ui.docking_generation));
    for panel in SHOWN {
        if !crate::panel_docking::current_layout(app).contains(&(*panel).to_owned()) {
            // Showing a panel that is already showing is no error worth reporting.
            let _ = crate::menus::invoke(app, ctx, &format!("window.panel.{panel}"), json!({ "show": true }));
        }
    }
    for (panel, anchor, own) in GROUPS {
        let layout = crate::panel_docking::current_layout(app);
        if !layout.contains(&(*anchor).to_owned()) {
            continue;
        }
        use craft_ui::docking::{Action, Placement};
        let placement = if *own { Placement::Group { below: true, height: None } } else { Placement::Tab { before: None } };
        let action = Action::Move { panel: (*panel).to_owned(), anchor: (*anchor).to_owned(), placement };
        // A panel that can't move there keeps its standard place: nothing worth reporting.
        let _ = app.run("window.panel.layout", json!({ "action": action }));
    }
    // Each group shows its first panel.
    for panel in ["properties", "history"] {
        let _ = app.run("window.panel.layout", json!({ "action": craft_ui::docking::Action::Activate { panel: panel.to_owned() } }));
    }
    // Panels the standard groups brought along that are put away close again.
    tidy_away(app, ctx);
    ctx.data_mut(|d| d.insert_temp(done, app.ui.docking_generation));
}

/// Draws the Closet window when it is open. On the first frame, closes the panels the starting
/// sort put away.
pub fn windows(app: &mut PhotocraftApp, ctx: &egui::Context) {
    starting_groups(app, ctx);
    if !ctx.data(|d| d.get_temp::<bool>(started_id()).unwrap_or(false)) {
        ctx.data_mut(|d| d.insert_temp(started_id(), true));
        tidy_away(app, ctx);
    }
    if !app.ui.shell.closet_open {
        return;
    }
    let t = Tokens::get(ctx);
    let mut tab: usize = ctx.data(|d| d.get_temp(tab_id()).unwrap_or(0));
    let mut query: String = ctx.data(|d| d.get_temp(search_id()).unwrap_or_default());
    let mut close = false;
    let mut moves: Vec<(String, Shelf)> = Vec::new();
    let mut reset: Option<bool> = None;
    let mut restore_order = false;
    let things = things();
    let shelves: Vec<Shelf> = things.iter().map(|th| shelf(app, &th.key)).collect();
    crate::analysis_ui::panel_window(app, ctx, "closet", tl!("Closet"), vec2(-360.0, 60.0), 380.0, |ui| {
        close = crate::analysis_ui::title_row(ui, tl!("Closet"));
        ui.horizontal(|ui| {
            for (i, name) in TABS.iter().enumerate() {
                if ui.selectable_label(tab == i, RichText::new(tl!(name)).size(12.0)).clicked() {
                    tab = i;
                }
            }
        });
        let blurb = match tab {
            0 => "Put away, one click from coming back.",
            1 => "Things you'll probably never need. Nothing here is deleted.",
            _ => "Everything that can be put away. Pick where each one lives.",
        };
        ui.label(RichText::new(tl!(blurb)).size(11.5).color(t.text_dim));
        ui.add_space(4.0);
        if let Some((tool, to)) = drop_shelves(ui, &t) {
            moves.push((tool_key(tool), to));
        }
        ui.label(
            RichText::new(tl!("While this window is open, drag tools up and down the toolbar to rearrange them, or onto another tool to group them."))
                .size(11.0)
                .color(t.text_faint),
        );
        ui.add_space(4.0);
        ui.add(egui::TextEdit::singleline(&mut query).hint_text(tl!("Search")).desired_width(f32::INFINITY));
        ui.add_space(4.0);
        let q = query.trim().to_lowercase();
        let wanted = |s: Shelf| match tab {
            0 => s == Shelf::Closet,
            1 => s == Shelf::Basement,
            _ => true,
        };
        let rows: Vec<(&Thing, Shelf)> =
            things.iter().zip(shelves.iter().copied()).filter(|(th, s)| wanted(*s) && (q.is_empty() || th.label.to_lowercase().contains(&q))).collect();
        egui::ScrollArea::vertical().id_salt("closet-rows").auto_shrink([false, true]).max_height(440.0).show(ui, |ui| {
            if rows.is_empty() {
                ui.add_space(8.0);
                let empty = match (tab, q.is_empty()) {
                    (0, true) => "The Closet is empty.",
                    (1, true) => "The Basement is empty.",
                    _ => "Nothing matches.",
                };
                ui.label(RichText::new(tl!(empty)).color(t.text_dim));
            }
            let mut kind = None;
            for (th, now) in &rows {
                if kind != Some(th.kind) {
                    kind = Some(th.kind);
                    ui.add_space(6.0);
                    ui.label(RichText::new(tl!(th.kind.heading())).size(11.0).strong().color(t.text_dim));
                }
                ui.horizontal(|ui| {
                    ui.set_min_height(24.0);
                    let buttons: [(&str, Shelf); 2] = match now {
                        Shelf::Closet => [("To Basement", Shelf::Basement), ("Restore", Shelf::InUse)],
                        Shelf::Basement => [("To Closet", Shelf::Closet), ("Restore", Shelf::InUse)],
                        Shelf::InUse => [("To Basement", Shelf::Basement), ("To Closet", Shelf::Closet)],
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Right to left: the far shelf first.
                        for (label, to) in buttons {
                            if ui.small_button(tl!(label)).clicked() {
                                moves.push((th.key.clone(), to));
                            }
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(&th.label).color(t.text)).truncate());
                        });
                    });
                });
            }
        });
        ui.add_space(6.0);
        crate::widgets::hairline(ui);
        ui.horizontal(|ui| {
            if crate::widgets::secondary_button(ui, tl!("Restore My Starting Sort"), 0.0).clicked() {
                reset = Some(false);
            }
            if crate::widgets::secondary_button(ui, tl!("Bring Everything Back"), 0.0).clicked() {
                reset = Some(true);
            }
        });
        if crate::toolbar_arrange::rearranged(app) && crate::widgets::secondary_button(ui, tl!("Restore Toolbar Order"), 0.0).clicked() {
            restore_order = true;
        }
    });
    ctx.data_mut(|d| {
        d.insert_temp(tab_id(), tab);
        d.insert_temp(search_id(), query);
    });
    for (key, to) in moves {
        if let Err(e) = crate::menus::invoke(app, ctx, "closet.set", json!({ "key": key, "shelf": to.name() })) {
            app.ui.status = e;
        }
    }
    if let Some(all) = reset
        && let Err(e) = crate::menus::invoke(app, ctx, "closet.reset", json!({ "all": all }))
    {
        app.ui.status = e;
    }
    if restore_order && let Err(e) = crate::menus::invoke(app, ctx, "toolbar.resetArrangement", json!({})) {
        app.ui.status = e;
    }
    if close {
        app.ui.shell.closet_open = false;
    }
}

/// The Closet and Basement shelves a toolbar tool drops on: the tool dropped and where it goes.
fn drop_shelves(ui: &mut egui::Ui, t: &Tokens) -> Option<(Tool, Shelf)> {
    let mut dropped = None;
    let dragging = egui::DragAndDrop::has_payload_of_type::<crate::toolbar_arrange::ToolDrag>(ui.ctx());
    ui.columns(2, |cols| {
        for (col, (name, icon, to)) in cols.iter_mut().zip([("Closet", "closet", Shelf::Closet), ("Basement", "basement", Shelf::Basement)]) {
            let (r, resp) = col.allocate_exact_size(vec2(col.available_width(), 46.0), egui::Sense::hover());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Group, true, format!("{} shelf", tl!(name))));
            let over = resp.dnd_hover_payload::<crate::toolbar_arrange::ToolDrag>().is_some();
            let fill = if over { t.hover } else { t.field };
            let stroke = egui::Stroke::new(if dragging { 2.0 } else { 1.0 }, if dragging { t.accent } else { t.card_border });
            col.painter().rect_filled(r, 6.0, fill);
            col.painter().rect_stroke(r, 6.0, stroke, egui::StrokeKind::Inside);
            crate::icons::paint(col, egui::Rect::from_center_size(egui::pos2(r.left() + 20.0, r.center().y), vec2(20.0, 20.0)), icon, 18.0, t.icon);
            col.painter().text(egui::pos2(r.left() + 38.0, r.center().y - 7.0), egui::Align2::LEFT_CENTER, tl!(name), egui::FontId::proportional(12.5), t.text);
            col.painter().text(
                egui::pos2(r.left() + 38.0, r.center().y + 8.0),
                egui::Align2::LEFT_CENTER,
                tl!("Drop a tool here"),
                egui::FontId::proportional(10.5),
                t.text_dim,
            );
            if let Some(drag) = resp.dnd_release_payload::<crate::toolbar_arrange::ToolDrag>() {
                dropped = Some((drag.0, to));
            }
        }
    });
    dropped
}

#[cfg(test)]
mod tests;
