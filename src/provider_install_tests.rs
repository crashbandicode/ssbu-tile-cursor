use crate::model::{Dir, Repeater, BUTTON_L3, BUTTON_RIGHT};

#[test]
fn chooses_owner_once_at_install_after_weak_registration() {
    let runtime = include_str!("runtime.rs");
    let main = runtime.split("pub fn main() {").last().unwrap();
    assert!(
        main.contains("install_cursor_hooks(observe)"),
        "Tile Cursor must select observed or standalone owners only at guarded install"
    );
    let hooks = runtime
        .split("fn install_cursor_hooks(")
        .nth(1)
        .expect("startup hook installer");
    assert!(hooks.contains("tile_cursor_css, tile_cursor_sss, tile_cursor_query"));
    assert!(hooks.contains("observed_css, observed_sss, tile_cursor_query"));
}

#[test]
fn accepted_repeat_contract_crosses_discovery_expiry() {
    for _screen in ["css", "sss"] {
        let mut repeat = Repeater::new();
        let mut emitted = Vec::new();
        for frame in 0..=32 {
            if repeat.update(BUTTON_L3 | BUTTON_RIGHT) == Some(Dir::Right) {
                emitted.push(frame);
            }
        }
        assert_eq!(emitted, [0, 15, 30]);
        assert_eq!(repeat.update(0), None);
        assert_eq!(repeat.update(BUTTON_L3 | BUTTON_RIGHT), Some(Dir::Right));
    }
}
