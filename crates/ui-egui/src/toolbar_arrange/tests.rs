use super::*;
use serde_json::json;

fn app() -> PhotocraftApp {
    PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default())
}

fn slot_of(slots: &[Vec<Tool>], tool: Tool) -> usize {
    slots.iter().position(|s| s.contains(&tool)).unwrap()
}

#[test]
fn a_tool_moves_before_after_and_into_a_group() {
    let mut slots = standard();
    assert!(arrange(&mut slots, Tool::Pen, Tool::Move, Place::After));
    assert_eq!(slots[1], vec![Tool::Pen], "Pen gets its own button right under Move");
    assert!(!slots.iter().skip(2).any(|s| s.contains(&Tool::Pen)), "and leaves its old group");
    assert!(arrange(&mut slots, Tool::Zoom, Tool::Move, Place::Before));
    assert_eq!(slots[0], vec![Tool::Zoom]);
    assert!(arrange(&mut slots, Tool::Hand, Tool::Brush, Place::Group));
    assert!(slots[slot_of(&slots, Tool::Brush)].ends_with(&[Tool::Hand]), "Hand joins Brush's flyout");
    assert!(!arrange(&mut slots, Tool::Brush, Tool::Brush, Place::Group), "onto itself changes nothing");
    // Out of a group onto a button of its own beside it.
    let i = slot_of(&slots, Tool::Hand);
    assert!(arrange(&mut slots, Tool::Hand, Tool::Hand, Place::After));
    assert_eq!(slots[i + 1], vec![Tool::Hand]);
    let all: usize = slots.iter().map(Vec::len).sum();
    assert_eq!(all, standard().iter().map(Vec::len).sum::<usize>(), "no tool is lost or doubled");
}

#[test]
fn the_arrangement_is_saved_by_name_and_read_back_whole() {
    let mut app = app();
    assert!(slots(&app).is_none(), "the standard toolbar to start with");
    let out = command(&mut app, "toolbar.arrange", &json!({"tool": "pen", "to": "move", "place": "after"})).unwrap().unwrap();
    assert_eq!(out["slots"][1], json!(["Pen"]));
    let arranged = slots(&app).unwrap();
    assert_eq!(arranged[1], vec![Tool::Pen]);
    // A saved toolbar missing tools (say, ones added since) gets them back with their group.
    app.session.prefs.edit(|p| p.toolbar.slots = vec![vec!["Zoom".into()], vec!["Brush".into(), "nonsense".into()]]);
    let read = slots(&app).unwrap();
    assert_eq!(read[0], vec![Tool::Zoom]);
    assert!(read[1].contains(&Tool::Pencil), "Pencil goes back with Brush");
    assert_eq!(read.iter().map(Vec::len).sum::<usize>(), standard().iter().map(Vec::len).sum::<usize>());
    command(&mut app, "toolbar.resetArrangement", &json!({})).unwrap().unwrap();
    assert!(slots(&app).is_none());
    assert!(app.session.prefs().toolbar.slots.is_empty());
}

#[test]
fn moving_back_to_the_standard_arrangement_saves_nothing() {
    let mut app = app();
    command(&mut app, "toolbar.arrange", &json!({"tool": "zoom", "to": "move", "place": "before"})).unwrap().unwrap();
    assert!(!app.session.prefs().toolbar.slots.is_empty());
    let last = standard().iter().rev().nth(1).unwrap()[0];
    command(&mut app, "toolbar.arrange", &json!({"tool": "zoom", "to": format!("{last:?}"), "place": "after"})).unwrap().unwrap();
    assert!(app.session.prefs().toolbar.slots.is_empty(), "the standard toolbar is stored as nothing");
}

#[test]
fn bad_params_are_errors() {
    let mut app = app();
    assert!(command(&mut app, "toolbar.arrange", &json!({})).unwrap().is_err());
    assert!(command(&mut app, "toolbar.arrange", &json!({"tool": "pen", "to": "nope"})).unwrap().is_err());
    assert!(command(&mut app, "toolbar.arrange", &json!({"tool": "pen", "to": "move", "place": "sideways"})).unwrap().is_err());
    assert!(command(&mut app, "something.else", &json!({})).is_none());
}

#[test]
fn drops_land_between_buttons_or_in_the_group() {
    assert_eq!(Place::at(0.1), Place::Before);
    assert_eq!(Place::at(0.5), Place::Group);
    assert_eq!(Place::at(0.9), Place::After);
}

#[test]
fn the_build_starts_with_the_selection_tools_together_and_magic_wand_alone() {
    crate::closet::use_starting_sort();
    let mut app = app();
    let first = slots(&app).unwrap();
    assert_eq!(first[1], SELECTION.to_vec());
    assert_eq!(first[2], vec![Tool::MagicWand]);
    assert_eq!(first.iter().map(Vec::len).sum::<usize>(), standard().iter().map(Vec::len).sum::<usize>());
    assert!(!rearranged(&app));
    command(&mut app, "toolbar.arrange", &json!({"tool": "zoom", "to": "move", "place": "before"})).unwrap().unwrap();
    assert!(rearranged(&app));
    command(&mut app, "toolbar.resetArrangement", &json!({})).unwrap().unwrap();
    assert_eq!(slots(&app).unwrap(), starting(), "Restore goes back to this build's toolbar");
}
