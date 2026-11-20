//! Stock r3 captures from process 81 and the single native Training hand.
//! Source sequence 545 = Training; 737 = Battlefield; 1001 = wrong tile;
//! 1180 = Battlefield with the first nonnormal form selected.
use crate::model::{neighbour, pack_step, Dir, Rect, Screen};
use crate::provider::Step;
use crate::stage::{actor_code_matches, qualified_stage, start_move, Preview, Stage, StageFacts};

const TRAINING: &[u8; 120] = include_bytes!("stage_fixtures/preview-0-0.bin");
const BATTLEFIELD: &[u8; 120] = include_bytes!("stage_fixtures/preview-1-0.bin");
const SMALL: &[u8; 120] = include_bytes!("stage_fixtures/preview-2-0.bin");
const NON_NORMAL: &[u8; 120] = include_bytes!("stage_fixtures/preview-1-1.bin");

fn facts(bytes: &[u8; 120], panel: i32, kind: u64) -> StageFacts {
    let records = std::array::from_fn(|i| {
        let row = &bytes[i * 40..][..40];
        Preview {
            kind: u64::from_le_bytes(row[..8].try_into().unwrap()),
            form: i32::from_le_bytes(row[8..12].try_into().unwrap()),
            panel: i32::from_le_bytes(row[24..28].try_into().unwrap()),
        }
    });
    StageFacts { tab: 0, slot: 0, getter: panel, actor_hover: panel, grid_kind: kind, records }
}

#[test]
fn release_hover_code_includes_the_final_ten_words() {
    let mut code = *include_bytes!("stage_fixtures/hover-actor.bin");
    assert!(actor_code_matches(&code));
    assert!(!actor_code_matches(&code[..296]));
    code[335] ^= 1;
    assert!(!actor_code_matches(&code));
}

#[test]
fn one_local_normal_preview_agrees_with_its_active_actor_and_grid() {
    assert_eq!(qualified_stage(facts(TRAINING, 0, 0x69006d116b904091)), Some(Stage::Training));
    let bf = facts(BATTLEFIELD, 1, 0x690004154b5c4392);
    assert_eq!(qualified_stage(bf), Some(Stage::Battlefield));
    assert_eq!(qualified_stage(facts(SMALL, 2, 0x690088171954c00c)), Some(Stage::SmallBattlefield));
    assert_eq!(qualified_stage(facts(NON_NORMAL, 1, bf.grid_kind)), None);

    for bad in [
        StageFacts { tab: 1, ..bf },
        StageFacts { slot: 1, ..bf },
        StageFacts { actor_hover: 2, ..bf },
        StageFacts { getter: 2, ..bf },
        StageFacts { grid_kind: 0x690088171954c00c, ..bf },
        StageFacts { records: [bf.records[0]; 3], ..bf },
    ] {
        assert_eq!(qualified_stage(bad), None);
    }
}

#[test]
fn stage_move_follows_the_next_applied_ack_from_qualified_rest() {
    // Stock panels 0 through 4 and clamp from the pinned Training layout.
    let row = [-294.0, -182.0, -70.0, 42.0, 154.0].map(|xmin| Rect {
        xmin,
        xmax: xmin + 112.0,
        ymin: 351.0,
        ymax: 437.0,
    });
    let bounds = Rect { xmin: -940.0, xmax: 940.0, ymin: -500.0, ymax: 540.0 };
    let actual = neighbour(&row[..], row[2].centre(), Dir::Right, &bounds);
    assert_eq!((actual.from, actual.to), (Some(2), Some(3)));
    let mut step = Step { ack: 2, applied: 2, ..Step::default() };
    step.emitted(pack_step(Screen::Sss, Dir::Right, &actual, None).0);
    let rest = (2, 2, 2);
    assert_eq!(start_move(rest, 2, step), Some((2, 3, 2, 3)));
    assert_eq!(start_move(rest, 3, step), None);
    assert_eq!(start_move((2, 1, 2), 2, step), None);
    assert_eq!(start_move((2, 2, 1), 2, step), None);
    assert_eq!(start_move(rest, 2, Step { last_written: 2, ..step }), None);
    assert_eq!(start_move(rest, 2, Step { to: None, ..step }), None);
    assert_eq!(start_move(rest, 2, Step { dir: None, ..step }), None);

    let actual = neighbour(&row[..], row[3].centre(), Dir::Right, &bounds);
    assert_eq!((actual.from, actual.to), (Some(3), Some(4)));
    step.emitted(pack_step(Screen::Sss, Dir::Right, &actual, None).0);
    assert_eq!(start_move((2, 3, 3), 2, step), Some((2, 4, 3, 4)));
}

#[test]
fn native_normal_preview_supports_the_route_through_final_destination() {
    // These fixture inputs combine the Normal single-player slot shape with the
    // typed panel values from the pinned stock vector and identity databases.
    for (panel, kind, id, name) in [
        (3, 0x69000517945ccdf9, 2, "Big Battlefield"),
        (4, 0x6900060c40bf5050, 3, "Final Destination"),
    ] {
        let mut facts = facts(TRAINING, panel, kind);
        facts.records[0] = Preview { kind, panel, form: 0 };
        let identity = qualified_stage(facts).map(|stage| (stage.id(), stage.name()));
        assert_eq!(identity, Some((id, name)), "stock panel {panel}");
        assert_eq!(qualified_stage(StageFacts { actor_hover: 2, ..facts }), None);
        assert_eq!(qualified_stage(StageFacts { grid_kind: kind ^ 1, ..facts }), None);
        // Agreement among arbitrary fields cannot forge an exact stock identity.
        facts.grid_kind ^= 1;
        facts.records[0].kind ^= 1;
        assert_eq!(qualified_stage(facts), None);
    }
}
