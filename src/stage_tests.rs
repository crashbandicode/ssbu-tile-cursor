//! Stock r3 captures from process 81 and the single native Training hand.
//! Source sequence 545 = Training; 737 = Battlefield; 1001 = wrong tile;
//! 1180 = Battlefield with the first nonnormal form selected.
use crate::stage::{actor_code_matches, qualified_stage, Preview, Stage, StageFacts};

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
