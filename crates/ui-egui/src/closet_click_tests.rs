//! The Closet answers real clicks in the whole app: its tabs switch and its buttons move things.

use crate::{PhotocraftApp, Services};
use egui::{Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use photocraft_engine::Session;

fn harness() -> Harness<'static, PhotocraftApp> {
    let mut h = Harness::builder().with_size(vec2(1440.0, 900.0)).with_max_steps(64).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        PhotocraftApp::new(Session::new(), Services::default())
    });
    h.run_steps(3);
    h
}

fn click_at(h: &mut Harness<'static, PhotocraftApp>, p: Pos2) {
    h.event(egui::Event::PointerMoved(p));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(3);
}

fn first(h: &Harness<'static, PhotocraftApp>, label: &str) -> Pos2 {
    let all: Vec<_> = h.query_all_by_label(label).map(|n| n.rect().center()).collect();
    assert!(!all.is_empty(), "no `{label}` on screen");
    all[0]
}

#[test]
fn everything_tab_and_to_closet_work_with_real_clicks() {
    crate::closet::use_starting_sort();
    let mut h = harness();
    h.state_mut().ui.shell.closet_open = true;
    h.run_steps(4);
    let at = first(&h, "Everything");
    click_at(&mut h, at);
    assert!(h.query_all_by_label("To Closet").count() > 0, "the Everything tab lists things still in use");
    let before = h.state().session.prefs().closet.len();
    let at = first(&h, "To Closet");
    click_at(&mut h, at);
    assert_eq!(h.state().session.prefs().closet.len(), before + 1, "To Closet moves it");
}

/// A real drag: press at `from`, move there in steps, release at `to`.
fn drag(h: &mut Harness<'static, PhotocraftApp>, from: Pos2, to: Pos2) {
    h.event(egui::Event::PointerMoved(from));
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for i in 1..=10 {
        let p = from + (to - from) * (i as f32 / 10.0);
        h.event(egui::Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(4);
}

/// The toolbar button whose tooltip label starts with `name`.
fn tool_button(h: &Harness<'static, PhotocraftApp>, name: &str) -> egui::Rect {
    let found: Vec<_> =
        h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with(name))).map(|n| n.rect()).filter(|r| r.width() < 40.0 && r.left() < 60.0).collect();
    assert!(!found.is_empty(), "no toolbar button `{name}`");
    found[0]
}

#[test]
fn a_tool_dragged_onto_the_basement_shelf_goes_there() {
    crate::closet::use_starting_sort();
    let mut h = harness();
    h.state_mut().ui.shell.closet_open = true;
    h.run_steps(4);
    let brush = tool_button(&h, "Brush").center();
    let shelf = h.get_by_label("Basement shelf").rect().center();
    drag(&mut h, brush, shelf);
    assert_eq!(crate::closet::shelf(h.state(), "tool:Brush"), crate::closet::Shelf::Basement);
}

#[test]
fn a_tool_dragged_along_the_toolbar_moves_there() {
    let mut h = harness();
    h.state_mut().ui.shell.closet_open = true;
    h.run_steps(4);
    let brush = tool_button(&h, "Brush").center();
    let mv = tool_button(&h, "Move");
    // Onto the top edge of the Move tool: before it.
    drag(&mut h, brush, egui::pos2(mv.center().x, mv.top() + 2.0));
    let slots = crate::toolbar_arrange::slots(h.state()).expect("arranged");
    assert_eq!(slots[0], vec![crate::state::Tool::Brush]);
    // Onto the middle of the Lasso tool: into its group.
    let brush = tool_button(&h, "Brush").center();
    let lasso = tool_button(&h, "Lasso").center();
    drag(&mut h, brush, lasso);
    let slots = crate::toolbar_arrange::slots(h.state()).expect("arranged");
    let group = slots.iter().find(|s| s.contains(&crate::state::Tool::Lasso)).unwrap();
    assert!(group.contains(&crate::state::Tool::Brush), "Brush joined Lasso's group: {slots:?}");
}

#[test]
fn tools_dont_drag_while_the_closet_is_closed() {
    let mut h = harness();
    let brush = tool_button(&h, "Brush").center();
    let mv = tool_button(&h, "Move");
    drag(&mut h, brush, egui::pos2(mv.center().x, mv.top() + 2.0));
    assert!(crate::toolbar_arrange::slots(h.state()).is_none());
}

#[test]
fn the_x_on_a_tab_closes_only_that_panel() {
    let mut h = harness();
    h.run_steps(4);
    let close: Vec<_> = h.query_all_by(|n| n.label().is_some_and(|l| l == "Close Panel channels")).map(|n| n.rect().center()).collect();
    assert_eq!(close.len(), 1, "Channels has a ×");
    click_at(&mut h, close[0]);
    h.run_steps(4);
    assert_eq!(h.query_all_by(|n| n.label().is_some_and(|l| l == "Close Panel channels")).count(), 0, "Channels is closed");
    assert_eq!(h.query_all_by(|n| n.label().is_some_and(|l| l == "Close Panel layers")).count(), 1, "Layers stays");
    assert_eq!(h.query_all_by(|n| n.label().is_some_and(|l| l == "Close Panel paths")).count(), 1, "Paths stays");
}

#[test]
fn the_panels_start_in_emmas_groups() {
    crate::closet::use_starting_sort();
    let mut h = harness();
    h.run_steps(6);
    let layout = crate::panel_docking::current_layout(h.state());
    let group_of =
        |panel: &str| -> Vec<String> { crate::panel_docking::groups_for_tests(&layout).into_iter().find(|g| g.iter().any(|p| p == panel)).unwrap_or_default() };
    eprintln!("{:?}", crate::panel_docking::groups_for_tests(&layout));
    assert_eq!(group_of("layers"), vec!["layers", "channels"]);
    assert_eq!(group_of("properties"), vec!["properties", "info"]);
    assert_eq!(group_of("history"), vec!["history"]);
    assert_eq!(group_of("actions"), vec!["actions"]);
    assert_eq!(group_of("paths"), vec!["paths"]);
    assert_eq!(group_of("adjustments"), vec!["adjustments"]);
    assert_eq!(group_of("color"), vec!["color", "swatches", "gradients"]);
}
