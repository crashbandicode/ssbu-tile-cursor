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
    StageFacts { layout: crate::layout::Layout::Sss, tab: 0, slot: 0, getter: panel, actor_hover: panel, grid_kind: kind, records }
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
        assert_eq!(identity, Some((Some(id), name)), "stock panel {panel}");
        assert_eq!(qualified_stage(StageFacts { actor_hover: 2, ..facts }), None);
        assert_eq!(qualified_stage(StageFacts { grid_kind: kind ^ 1, ..facts }), None);
        // Agreement among arbitrary fields cannot forge an exact stock identity.
        facts.grid_kind ^= 1;
        facts.records[0].kind ^= 1;
        assert_eq!(qualified_stage(facts), None);
    }
}

// Yoga's Ordinary Smash owner at main+0x1b29dc0, with a same-frame getter at
// main+0x1b3304c. All twelve captures agree on clamp, ordered vector and preview.
const ORDINARY: &[u8; 16 + 118 * 24] = include_bytes!("stage_fixtures/ordinary-sss.bin");
const ORDINARY_PREVIEW: &[u8; 120] = include_bytes!("stage_fixtures/ordinary-preview.bin");

fn ordinary_layout(bytes: &[u8], count: usize) -> Option<crate::layout::Layout> {
    let word = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
    crate::layout::Layout::identify(Screen::Sss, count, [word(0), word(8)], |i| {
        let offset = 16 + i * 24;
        ([word(offset), 0], [word(offset + 8), word(offset + 16)])
    })
}

#[test]
fn ordinary_stock_grid_is_distinct_and_checks_every_row_and_clamp() {
    let name = ordinary_layout(ORDINARY, 118).map(|x| x.name());
    assert_eq!(name, Some("sss.5008a63ffaa7de18"));
    assert_eq!(ordinary_layout(ORDINARY, 116), None);
    for offset in [0, 16, ORDINARY.len() - 1] {
        let mut changed = *ORDINARY;
        changed[offset] ^= 1;
        assert_eq!(ordinary_layout(&changed, 118), None);
    }
    assert_eq!(facts(ORDINARY_PREVIEW, 0, 0).records[0].kind, 0x690001166fbc7641);
}

#[test]
fn ordinary_random_controls_and_normal_stages_follow_typed_right_steps() {
    use crate::provider::{P1Token, Publisher, Sample};

    let layout = ordinary_layout(ORDINARY, 118).unwrap();
    let rect = |bytes: &[u8]| {
        let [xmin, xmax, ymin, ymax] = std::array::from_fn(|i| {
            f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        Rect { xmin, xmax, ymin, ymax }
    };
    let bounds = rect(&ORDINARY[..16]);
    let rows = ORDINARY[16..].as_chunks::<24>().0;
    let rects: Vec<_> = rows.iter().map(|r| rect(&r[8..])).collect();
    let route = [
        ("Random Normal", None),
        ("Random Battlefield", None),
        ("Random Omega", None),
        ("Battlefield", Some(0)),
        ("Small Battlefield", Some(347)),
        ("Big Battlefield", Some(2)),
        ("Final Destination", Some(3)),
    ];
    let mut point = (-189.0, 358.0);
    let mut step = Step::default();
    for (i, (name, id)) in route.into_iter().enumerate() {
        let panel = i as i32;
        let kind = u64::from_le_bytes(rows[i][..8].try_into().unwrap());
        let mut actor = facts(ORDINARY_PREVIEW, panel, kind);
        actor.layout = layout;
        // Only panel 0's preview is observed. Model the later landing iff the live
        // native actor returns the matching Normal preview for that exact panel.
        if i != 0 {
            actor.records[0] = Preview { kind, panel, form: 0 };
        }
        let stage = qualified_stage(actor).unwrap();
        assert_eq!((stage.name(), stage.id()), (name, id));
        assert_eq!(qualified_stage(StageFacts { actor_hover: -1, ..actor }), None);
        assert_eq!(qualified_stage(StageFacts { layout: crate::layout::Layout::Sss, ..actor }), None);
        let sample = Sample {
            screen: Some(Screen::Sss), grid: Some(layout), resting: true,
            at: Some(i as u16), hover: Some(i as u16), stage: Some(stage),
            local_seat: Some(0), p1_token: P1Token::Held, p1_carried: Some(0),
            step, ..Sample::default()
        };
        Publisher::default().poll(0, Some(sample), |bytes| {
            let body = std::str::from_utf8(bytes).unwrap();
            for field in [
                format!("name={name}"),
                format!("kind={}", id.map_or("unknown".into(), |k| k.to_string())),
                format!("focus=sss.grid.{}", if id.is_none() { "random" } else { "normal" }),
                "layout=sss.5008a63ffaa7de18".to_owned(),
                "settled=1".to_owned(),
            ] {
                assert!(body.lines().any(|line| line == field), "{field}: {body}");
            }
            true
        });
        if i < route.len() - 1 {
            let movement = neighbour(&rects[..], point, Dir::Right, &bounds);
            assert_eq!((movement.from, movement.to), (Some(i), Some(i + 1)));
            let rest = (1, step.ack, i as u16);
            step.emitted(pack_step(Screen::Sss, Dir::Right, &movement, None).0);
            assert_eq!(start_move(rest, 1, step), Some((1, step.ack, i as u16, i as u16 + 1)));
            point = movement.target;
        }
    }
}
