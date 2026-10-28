use super::*;
use crate::state::Tool;

fn app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), Default::default())
}

fn run(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str, p: Value) -> Result<Value, String> {
    crate::menus::invoke(app, ctx, id, p)
}

#[test]
fn every_starting_key_names_something_that_can_be_put_away() {
    let keys: BTreeSet<String> = things().into_iter().map(|t| t.key).collect();
    for (key, _) in SHIPPED {
        assert!(keys.contains(*key), "{key}");
    }
    assert!(keys.iter().all(|k| k != "menu:window.closet"), "the way into the Closet stays");
    assert!(!keys.contains("panel:tools"), "the toolbar stays");
}

#[test]
fn tests_start_with_everything_in_use() {
    let app = app();
    assert!(put_away(&app).is_empty());
    assert!(hidden_tool_names(&app).is_empty());
}

#[test]
fn the_starting_sort_puts_tools_away_and_their_keys_skip_them() {
    use_starting_sort();
    let mut app = app();
    let names = hidden_tool_names(&app);
    for tool in ["Pen", "PathSelection", "Rectangle", "MagneticLasso", "Blur", "Frame"] {
        assert!(names.iter().any(|n| n == tool), "{tool} put away: {names:?}");
    }
    for tool in [Tool::Move, Tool::Lasso, Tool::Brush, Tool::Type, Tool::VerticalType, Tool::Gradient, Tool::HistoryBrush, Tool::RotateView] {
        assert!(!tool_hidden(&app, tool), "{tool:?} stays");
    }
    // The toolbar leaves out what is put away.
    let sections = crate::panels::toolbar_sections();
    let mut hidden = app.session.prefs().toolbar.hidden.clone();
    hidden.extend(names);
    let shown: Vec<Tool> = crate::panels::visible_sections_for_tests(&hidden).into_iter().flatten().flat_map(|(_, s)| s).collect();
    assert!(!shown.contains(&Tool::Pen) && shown.contains(&Tool::Brush));
    assert!(sections.iter().flat_map(|s| s.iter()).flat_map(|s| s.iter()).any(|t| *t == Tool::Pen));
    // A put-away current tool gives way to the Move tool.
    app.ui.tool = Tool::Pen;
    let ctx = egui::Context::default();
    run(&mut app, &ctx, "closet.set", json!({"key": "tool:Smudge", "shelf": "basement"})).unwrap();
    assert_eq!(app.ui.tool, Tool::Move);
}

#[test]
fn moves_are_remembered_only_where_they_differ_from_the_starting_sort() {
    use_starting_sort();
    let mut app = app();
    let ctx = egui::Context::default();
    assert_eq!(shelf(&app, "tool:Pen"), Shelf::Basement);
    run(&mut app, &ctx, "closet.set", json!({"key": "tool:Pen", "shelf": "inUse"})).unwrap();
    assert!(!tool_hidden(&app, Tool::Pen));
    assert_eq!(app.session.prefs().closet.get("tool:Pen").map(String::as_str), Some("inUse"));
    run(&mut app, &ctx, "closet.set", json!({"key": "tool:Pen", "shelf": "basement"})).unwrap();
    assert!(app.session.prefs().closet.is_empty(), "back where the sort put it");
    run(&mut app, &ctx, "closet.set", json!({"key": "tool:Brush", "shelf": "closet"})).unwrap();
    assert!(tool_hidden(&app, Tool::Brush));
    let listed = run(&mut app, &ctx, "closet.list", json!({"shelf": "closet"})).unwrap();
    assert!(listed.as_array().unwrap().iter().any(|k| k == "tool:Brush"));
    // Restore My Starting Sort, then Bring Everything Back.
    run(&mut app, &ctx, "closet.reset", json!({})).unwrap();
    assert!(!tool_hidden(&app, Tool::Brush) && tool_hidden(&app, Tool::Pen));
    run(&mut app, &ctx, "closet.reset", json!({"all": true})).unwrap();
    assert!(put_away(&app).is_empty());
}

#[test]
fn bad_keys_and_shelves_are_errors() {
    let mut app = app();
    let ctx = egui::Context::default();
    assert!(run(&mut app, &ctx, "closet.set", json!({"key": "tool:Nope", "shelf": "closet"})).is_err());
    assert!(run(&mut app, &ctx, "closet.set", json!({"key": "tool:Pen", "shelf": "attic"})).is_err());
    assert!(run(&mut app, &ctx, "closet.set", json!({"shelf": "closet"})).is_err());
    assert!(run(&mut app, &ctx, "closet.set", json!({"key": "menu:window.closet", "shelf": "closet"})).is_err());
    assert!(run(&mut app, &ctx, "closet.reset", json!({"all": "yes"})).is_err());
    assert!(run(&mut app, &ctx, "closet.list", json!({"shelf": 3})).is_ok(), "a shelf that isn't text lists everything");
    assert!(app.session.prefs().closet.is_empty());
}

#[test]
fn menus_leave_out_what_is_put_away_without_stray_separators() {
    use_starting_sort();
    let app = app();
    let items = crate::menus::menu_items(&app);
    let ids: BTreeSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
    for gone in [
        "file.automate.createDroplet",
        "file.export.renderVideo",
        "window.panel.navigator",
        "window.panel.timeline",
        "filter.video.deInterlace",
        "image.variables.define",
    ] {
        assert!(!ids.contains(gone), "{gone}");
    }
    for kept in ["window.closet", "window.panel.layers", "file.automate.batch", "filter.blur.gaussianBlur"] {
        assert!(ids.contains(kept), "{kept}");
    }
    assert!(!items.iter().any(|i| i.path.starts_with(&["Filter".to_string(), "Video".to_string()])));
    // No menu starts or ends with a separator, or has two in a row.
    let mut paths: Vec<&Vec<String>> = items.iter().map(|i| &i.path).collect();
    paths.dedup();
    for path in paths {
        let menu: Vec<&MenuItem> = items.iter().filter(|i| i.path.starts_with(path)).collect();
        let own: Vec<bool> = menu.iter().map(|i| i.id == "---" && &i.path == path).collect();
        assert_ne!(own.first(), Some(&true), "{path:?} starts with a separator");
        assert_ne!(own.last(), Some(&true), "{path:?} ends with a separator");
        assert!(!own.windows(2).any(|w| w == [true, true]), "{path:?} has two separators in a row");
    }
}

#[test]
fn the_window_opens_from_its_menu_item_and_draws() {
    use_starting_sort();
    let mut app = app();
    let ctx = egui::Context::default();
    run(&mut app, &ctx, "window.closet", json!({})).unwrap();
    assert!(app.ui.shell.closet_open);
    assert_eq!(crate::menus::item_checked(&app, "window.closet"), Some(true));
    for tab in 0..3usize {
        ctx.data_mut(|d| d.insert_temp(tab_id(), tab));
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| windows(&mut app, ui.ctx()));
        out.textures_delta.clear();
    }
    run(&mut app, &ctx, "window.closet", json!({"show": false})).unwrap();
    assert!(!app.ui.shell.closet_open);
}

#[test]
fn the_first_frame_closes_docked_panels_the_sort_put_away() {
    use_starting_sort();
    let mut app = app();
    let ctx = egui::Context::default();
    let pro = Tokens::get(&ctx).pro;
    let (group, label) = crate::panel_docking::group_of("navigator").unwrap();
    let tab = group.tabs(pro).iter().position(|t| *t == label).unwrap();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| windows(&mut app, ui.ctx()));
    out.textures_delta.clear();
    assert!(!app.ui.dock.visible_tabs(group, pro).iter().any(|(i, _)| *i == tab), "Navigator closed");
    let info = group.tabs(pro).iter().position(|t| *t == "Info").unwrap();
    assert!(app.ui.dock.visible_tabs(group, pro).iter().any(|(i, _)| *i == info), "Info stays");
}
