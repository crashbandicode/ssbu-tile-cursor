use super::*;
use crate::css_table::CSS_GEOMETRY;
use crate::model::{pack_step, Step as NativeStep};

const SENTINEL: u64 = 0xc1ff_ff00_0000_0000;
const MYTHRA: u64 = 0xc100_7414_c6fc_852a;

#[test]
fn a_sample_stamped_after_the_clock_reading_stays_active() {
    use crate::layout::Layout;

    let sample = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 100,
        observation: 1,
        at: Some(67),
        hover: Some(67),
        p1_carried: Some(0),
        p1_token: P1Token::Held,
        resting: true,
        fighter: Some(Fighter(67)),
        ..Sample::default()
    };
    let actual = block(&mut Publisher::default(), 99, Some(sample), true);
    for line in ["\nobservation.active=1\n", "\nscreen=css\n", "\nphase=settled\n", "\nname=\"Richter\"\n"] {
        assert!(actual.contains(line), "expected {line:?}: {actual}");
    }
}

#[test]
fn a_late_clock_reading_at_rest_does_not_interrupt_the_settled_block() {
    let mut publisher = Publisher::default();
    for (observation, tick, now) in [(1, 10, 12), (2, 20, 19), (3, 30, 31)] {
        let sample = Sample {
            observation,
            tick,
            at: Some(0),
            target: Some(0),
            hover: Some(0),
            fighter: Some(Fighter(0)),
            resting: true,
            step: Step {
                ack: 24,
                applied: 24,
                dir: Some(Dir::Left),
                from: Some(1),
                to: Some(0),
                token: Some(0),
                last_written: 24,
                settled_ack: Some(24),
                ..Step::default()
            },
            ..css_reliability_sample(24)
        };
        let actual = block(&mut publisher, now, Some(sample), true);
        let revision = format!("rev={observation}");
        let observed = format!("observation={observation}");
        assert_lines(&actual, &[
            &revision, &observed, "observation.active=1", "focus=css.grid.mario",
            "settled=1", "phase=settled", "direction=left", "from=1", "to=0",
            "settled.ack=24",
        ]);
    }
}

#[test]
fn staleness_bound_is_unchanged() {
    let sample = Sample {
        screen: Some(Screen::Css),
        tick: 10,
        observation: 1,
        ..Sample::default()
    };
    let mut publisher = Publisher::default();
    let active = block(&mut publisher, 10 + MAX_AGE, Some(sample), true);
    assert_lines(&active, &["observation.active=1"]);
    let expired = block(&mut publisher, 11 + MAX_AGE, None, true);
    assert_lines(&expired, &["observation.active=0", "phase=unknown"]);
}

#[test]
fn fighter_identity_follows_the_native_variant_lookup() {
    let richter = CSS_TILES[67].word;
    let pyra = CSS_TILES[80].word;
    for variant in [[1, 0], [0, 1], [1, 1]] {
        assert_eq!(
            fighter_identity(67, richter, [richter, SENTINEL], variant, true),
            Some(Fighter(67))
        );
    }
    assert_eq!(fighter_identity(67, richter, [richter, 0], [1, 0], true), Some(Fighter(67)));
    assert_eq!(fighter_identity(80, pyra, [pyra, MYTHRA], [0, 0], true), Some(Fighter(80)));
    assert_eq!(fighter_identity(80, pyra, [pyra, MYTHRA], [0, 1], true), None);
    assert_eq!(fighter_identity(80, MYTHRA, [pyra, MYTHRA], [0, 1], true), None);
    assert_eq!(fighter_identity(80, MYTHRA, [pyra, MYTHRA], [0, 0], true), None);
    assert_eq!(fighter_identity(67, richter, [richter, SENTINEL], [1, 0], false), None);
    assert_eq!(fighter_identity(66, richter, [richter, SENTINEL], [1, 0], true), None);
    assert_eq!(fighter_identity(67, richter ^ 1, [richter, SENTINEL], [1, 0], true), None);
}

#[test]
fn a_restored_echo_fighter_reads_placed() {
    use crate::layout::Layout;

    let richter = CSS_TILES[67].word;
    let fighter = fighter_identity(67, richter, [richter, SENTINEL], [1, 0], true);
    let token = MedalFacts {
        carried: -1,
        holding: 0,
        visible: 1,
        grabbed: 0,
        panel_flags: [0, 1],
        placed: Some(67),
        rect_matches: true,
        placed_fighter: fighter,
    }.token();
    assert_eq!(token, P1Token::Placed);
    let sample = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 1,
        resting: true,
        at: None,
        hover: None,
        p1_carried: None,
        p1_token: token,
        p1_placed: Some((67, fighter.unwrap())),
        ..Sample::default()
    };
    let actual = block(&mut Publisher::default(), 1, Some(sample), true);
    assert_lines(&actual, &[
        "p1.token=placed", "local.token=placed", "carried=none", "placed.at=67",
        "placed.kind=68", "placed.name=\"Richter\"", "focus=css.medal.richter", "phase=resting",
    ]);
}

fn api() -> Api {
    extern "C" fn register(name: *const u8, len: u32, id: u32, version: u32) -> i32 {
        let name = unsafe { core::slice::from_raw_parts(name, len as usize) };
        if (name, id, version) == (&b"tile.cursor"[..], 3, 1) {
            4
        } else {
            -4
        }
    }
    extern "C" fn publish(handle: i32, _: *const u8, len: u32) -> i32 {
        if handle == 4 && len > 0 && len <= 2048 {
            0
        } else {
            -1
        }
    }
    Api {
        header: Header {
            magic: u32::from_le_bytes(*b"BRST"),
            abi: 1,
            size: core::mem::size_of::<Api>() as u32,
            readers: 1,
            max_payload: 2048,
            max_providers: 8,
        },
        register: Some(register),
        publish: Some(publish),
        event: Some(publish),
    }
}

#[test]
fn discovery_never_bursts_or_restarts_after_cutoff() {
    let mut d = Discovery::default();
    let ticks: Vec<_> = (0..=310)
        .filter(|&n| d.due(n * TICKS_PER_SECOND / 10))
        .collect();
    assert_eq!(ticks, (0..300).step_by(10).collect::<Vec<_>>());
    assert!(!d.due(300 * TICKS_PER_SECOND));
    let mut late = Discovery::default();
    assert!(late.due(2 * TICKS_PER_SECOND + 20));
    assert!(!late.due(3 * TICKS_PER_SECOND));
    late.finish();
    assert!(!late.due(4 * TICKS_PER_SECOND));
}

#[test]
fn generated_table_reproduces_the_captured_stock_layout() {
    use crate::layout::Layout;

    let captured = include_bytes!("layouts/css.bin");
    let bounds = [
        u64::from_le_bytes(captured[0..8].try_into().unwrap()),
        u64::from_le_bytes(captured[8..16].try_into().unwrap()),
    ];
    for (i, tile) in CSS_TILES.iter().enumerate() {
        let offset = 16 + 32 * i;
        let word = u64::from_le_bytes(captured[offset..offset + 8].try_into().unwrap());
        let alternate =
            u64::from_le_bytes(captured[offset + 8..offset + 16].try_into().unwrap());
        let rect = [
            f32::from_le_bytes(captured[offset + 16..offset + 20].try_into().unwrap()),
            f32::from_le_bytes(captured[offset + 20..offset + 24].try_into().unwrap()),
            f32::from_le_bytes(captured[offset + 24..offset + 28].try_into().unwrap()),
            f32::from_le_bytes(captured[offset + 28..offset + 32].try_into().unwrap()),
        ];
        assert_eq!(tile.word, word, "tile {i}");
        assert_eq!(CSS_GEOMETRY[i].0, alternate, "tile {i}");
        assert_eq!(CSS_GEOMETRY[i].1, rect, "tile {i}");
    }
    assert_eq!(
        Layout::identify(Screen::Css, 87, bounds, |i| {
            let rect = CSS_GEOMETRY[i].1;
            (
                [CSS_TILES[i].word, CSS_GEOMETRY[i].0],
                [
                    u64::from(rect[0].to_bits()) | (u64::from(rect[1].to_bits()) << 32),
                    u64::from(rect[2].to_bits()) | (u64::from(rect[3].to_bits()) << 32),
                ],
            )
        }),
        Some(Layout::Css)
    );
}

#[test]
fn css_table_identifies_every_stock_fighter_tile() {
    let identify = |index: usize| {
        let word = CSS_TILES[index].word;
        fighter_identity(index as u16, word, [word, SENTINEL], [0, 0], true).expect("stock CSS tile")
    };
    for index in 0..CSS_TILES.len() {
        assert_eq!(identify(index).tile(), index as u16);
    }
    for (index, kind, name) in [
        (0, Some(0), "Mario"),
        (67, Some(68), "Richter"),
        (82, Some(93), "Sora"),
        (78, Some(88), "Steve"),
        (86, None, "Random"),
    ] {
        let fighter = identify(index);
        assert_eq!(fighter.id(), kind);
        assert_eq!(fighter.name(), name);
    }
    let richter = CSS_TILES[67].word;
    assert_eq!(
        fighter_identity(66, richter, [richter, SENTINEL], [0, 0], true),
        None
    );
    assert_eq!(
        fighter_identity(67, richter, [richter, MYTHRA], [1, 0], true),
        None
    );
    assert_eq!(
        fighter_identity(67, richter, [richter, SENTINEL], [0, 0], false),
        None
    );
    assert_eq!(
        fighter_identity(67, richter ^ 1, [richter, SENTINEL], [0, 0], true),
        None
    );
}

#[test]
fn publisher_reports_qualified_focus_for_any_settled_fighter() {
    use crate::layout::Layout;

    let hovered = |index: usize| {
        block(
            &mut Publisher::default(),
            1,
            Some(Sample {
                screen: Some(Screen::Css),
                grid: Some(Layout::Css),
                local_seat: Some(0),
                tick: 1,
                observation: 1,
                at: Some(index as u16),
                hover: Some(index as u16),
                p1_carried: Some(0),
                p1_token: P1Token::Held,
                resting: true,
                fighter: Some(Fighter(index as u16)),
                ..Sample::default()
            }),
            true,
        )
    };
    for (index, focus, kind, name) in [
        (67, "css.grid.richter", "68", "\"Richter\""),
        (0, "css.grid.mario", "0", "\"Mario\""),
        (86, "css.grid.random", "unknown", "\"Random\""),
        (78, "css.grid.pickel", "88", "\"Steve\""),
    ] {
        let body = hovered(index);
        for row in [
            format!("focus={focus}"),
            format!("kind={kind}"),
            format!("name={name}"),
            "form=default".to_owned(),
        ] {
            assert!(body.contains(&format!("\n{row}\n")), "{row}");
        }
    }

    let sora = block(
        &mut Publisher::default(),
        1,
        Some(Sample {
            screen: Some(Screen::Css),
            grid: Some(Layout::Css),
            local_seat: Some(0),
            tick: 1,
            observation: 1,
            at: Some(82),
            hover: Some(82),
            p1_carried: None,
            p1_token: P1Token::Placed,
            p1_placed: Some((82, Fighter(82))),
            resting: true,
            ..Sample::default()
        }),
        true,
    );
    for row in [
        "focus=css.medal.trail",
        "placed.kind=93",
        "placed.name=\"Sora\"",
        "p1.placed.kind=93",
        "p1.placed.name=\"Sora\"",
    ] {
        assert!(sora.contains(&format!("\n{row}\n")), "{row}");
    }
}

#[test]
fn every_stock_tile_fits_the_published_block() {
    use crate::layout::Layout;

    let publish = |sample| {
        let mut publisher = Publisher::default();
        let mut sends = 0;
        let mut bytes = Vec::new();
        publisher.poll(1, Some(sample), |block| {
            sends += 1;
            bytes.extend_from_slice(block);
            true
        });
        (sends, bytes)
    };
    for (index, tile) in CSS_TILES.iter().enumerate() {
        let hover = Sample {
            screen: Some(Screen::Css),
            grid: Some(Layout::Css),
            local_seat: Some(0),
            tick: 1,
            observation: 1,
            at: Some(index as u16),
            hover: Some(index as u16),
            p1_carried: Some(0),
            p1_token: P1Token::Held,
            resting: true,
            fighter: Some(Fighter(index as u16)),
            ..Sample::default()
        };
        let (sends, bytes) = publish(hover);
        let body = String::from_utf8(bytes).unwrap();
        assert_eq!(sends, 1, "hover tile {index}");
        assert!(body.len() < 1024, "hover tile {index}");
        assert!(
            body.contains(&format!("\nfocus={}\n", tile.hover_focus)),
            "hover tile {index}"
        );
        assert!(
            body.contains(&format!("\nname=\"{}\"\n", tile.name)),
            "hover tile {index}"
        );

        let placed = Sample {
            p1_carried: None,
            p1_token: P1Token::Placed,
            p1_placed: Some((index as u16, Fighter(index as u16))),
            fighter: None,
            ..hover
        };
        let (sends, bytes) = publish(placed);
        let body = String::from_utf8(bytes).unwrap();
        assert_eq!(sends, 1, "placed tile {index}");
        assert!(body.len() < 1024, "placed tile {index}");
        assert!(
            body.contains(&format!("\nfocus={}\n", tile.placed_focus)),
            "placed tile {index}"
        );
        assert!(
            body.contains(&format!("\np1.placed.name=\"{}\"\n", tile.name)),
            "placed tile {index}"
        );
        assert!(
            body.contains(&format!("\nplaced.name=\"{}\"\n", tile.name)),
            "placed tile {index}"
        );
    }
}

#[test]
fn validates_header_callbacks_and_negative_registration() {
    for field in 0..9 {
        let mut a = api();
        match field {
            0 => a.header.magic ^= 1,
            1 => a.header.abi += 1,
            2 => a.header.size -= 1,
            3 => a.header.readers = 2,
            4 => a.header.max_payload = 2047,
            5 => a.header.max_providers = 1,
            6 => a.register = None,
            7 => a.publish = None,
            _ => a.event = None,
        }
        assert!(Registered::register(&a).is_none());
    }
    extern "C" fn rejected(_: *const u8, _: u32, _: u32, _: u32) -> i32 {
        -4
    }
    let mut a = api();
    a.register = Some(rejected);
    assert!(Registered::register(&a).is_none());
    assert!(Registered::register(&api()).unwrap().send(b"snapshot"));
}

fn word(dir: Dir, from: Option<usize>, to: Option<usize>) -> u64 {
    pack_step(
        Screen::Css,
        dir,
        &NativeStep {
            from,
            to,
            target: (1.0, 2.0),
        },
        Some(0),
    )
    .0
}

fn block(p: &mut Publisher, now: u64, s: Option<Sample>, accept: bool) -> String {
    let mut bytes = Vec::new();
    p.poll(now, s, |b| {
        bytes.extend_from_slice(b);
        accept
    });
    String::from_utf8(bytes).unwrap()
}

#[test]
fn actual_write_ack_and_settlement_have_distinct_clocks() {
    let mut step = Step::default();
    step.emitted(word(Dir::Left, Some(0), None));
    step.settled(Some(0), Some(0), Some(0));
    assert_eq!((step.ack, step.applied, step.settled_ack), (1, 0, None));
    step.emitted(word(Dir::Right, Some(0), Some(1)));
    step.settled(Some(1), None, Some(0));
    step.settled(Some(1), Some(0), Some(0));
    assert_eq!((step.ack, step.applied, step.settled_ack), (2, 1, None));
    step.settled(Some(1), Some(1), Some(1));
    assert_eq!(step.settled_ack, None);
    step.settled(Some(1), Some(1), Some(0));
    assert_eq!(
        (step.dir, step.from, step.to, step.settled_ack),
        (Some(Dir::Right), Some(0), Some(1), Some(2))
    );
    step.emitted(word(Dir::Right, Some(1), None));
    step.settled(Some(1), Some(1), Some(0));
    assert_eq!((step.applied, step.settled_ack), (1, None));
}

#[test]
fn replaced_owner_or_expired_heartbeat_cannot_inherit_settlement() {
    let mut o = Owner::new();
    let css = Some((Screen::Css, 100, 150, 200, 86));
    o.begin(css, 1);
    o.latest.step.emitted(word(Dir::Right, Some(2), Some(3)));
    o.latest.step.settled(Some(3), Some(3), Some(0));
    o.begin(css, 2);
    assert_eq!(o.latest.step.settled_ack, Some(1));
    o.begin(css, MAX_AGE + 3);
    assert_eq!(
        (
            o.latest.generation,
            o.latest.step.ack,
            o.latest.step.settled_ack
        ),
        (2, 1, None)
    );
    o.begin(Some((Screen::Sss, 100, 150, 200, 86)), MAX_AGE + 4);
    assert_eq!(o.latest.generation, 3);
    o.begin(None, MAX_AGE + 5);
    assert_eq!(o.latest.screen, None);
}

#[test]
fn drops_retry_latest_and_heartbeat_expiry_publishes_unknown() {
    let mut p = Publisher::default();
    let mut o = Owner::new();
    o.begin(Some((Screen::Css, 100, 150, 200, 86)), 1);
    o.latest.at = Some(1);
    let first = block(&mut p, 1, Some(o.latest), false);
    assert!(first.starts_with(
        "schema=tile.cursor/1\nrev=1\nboot=0\nack=0\nack.result=unknown\nfocus=unknown\n"
    ));
    o.begin(Some((Screen::Css, 100, 150, 200, 86)), 2);
    o.latest.at = Some(2);
    let new = block(&mut p, 2, Some(o.latest), true);
    assert!(
        new.contains("\nrev=1\n") && new.contains("\nat=2\n") && new.contains("\nobservation=2\n")
    );
    assert!(block(&mut p, MAX_AGE + 2, None, true).is_empty());
    let expired = block(&mut p, MAX_AGE + 3, None, true);
    assert!(
        expired.contains("\nrev=2\n")
            && expired.contains("\nscreen=unknown\n")
            && expired.contains("\nobservation.active=0\n")
    );
}

#[test]
fn native_p1_carrying_any_slot_never_certifies_local_seat_or_placement() {
    for carried in [None, Some(0), Some(1), Some(7)] {
        let sample = Sample {
            screen: Some(Screen::Css),
            tick: 1,
            observation: 1,
            resting: true,
            at: Some(1),
            hover: Some(0),
            p1_carried: carried,
            ..Sample::default()
        };
        let s = block(&mut Publisher::default(), 1, Some(sample), true);
        for key in [
            "focus",
            "seat",
            "controller",
            "local.token",
            "p1.token",
            "carried",
            "kind",
            "placed.kind",
            "layout",
        ] {
            assert!(s.contains(&format!("\n{key}=unknown\n")));
        }
        assert!(s.contains("\nsettled=0\n"));
    }
}

#[test]
fn overwritten_mailbox_preserves_newest_complete_owner_and_counters() {
    let m = Mailbox::new();
    let mut a = Sample {
        observation: 1,
        at: Some(2),
        ..Sample::default()
    };
    assert_eq!(m.read(), None);
    m.offer(a);
    m.claimed(|_| {
        a.observation = 2;
        a.at = Some(3);
        m.offer(a); // reader owns the buffer
        assert!(m.read().is_none());
    });
    assert_eq!(m.read().unwrap().observation, 1);
    a.observation = 3;
    a.at = Some(4);
    a.step.ack = 2;
    m.offer(a);
    assert_eq!(m.read(), Some(a));
}

#[test]
fn observed_default_steve_requires_matching_typed_tile_and_current_panel() {
    let typed = 0xc100_700f_dc91_574e;
    let good = |index, panel, tile, variant, settled| {
        fighter_identity(index, panel, [tile, SENTINEL], variant, settled)
    };
    assert_eq!(good(78, typed, typed, [0, 0], true), Some(Fighter(78)));
    assert_eq!(good(78, 0xc1ff_ff00_0000_0000, typed, [0, 0], true), None);
    assert_eq!(fighter_identity(78, typed, [typed, MYTHRA], [1, 0], true), None);
    assert_eq!(fighter_identity(78, typed, [typed, MYTHRA], [0, 1], true), None);
    assert_eq!(good(78, typed, typed, [0, 0], false), None);
    assert_eq!(good(66, typed, typed, [0, 0], true), None);
    assert_eq!(good(78, typed, typed ^ 1, [0, 0], true), None);
}

#[test]
fn native_p1_medal_requires_own_slot_and_matching_committed_placement() {
    let mut p = MedalFacts {
        carried: 0,
        holding: 1,
        visible: 1,
        grabbed: 1,
        panel_flags: [0, 0],
        placed: None,
        rect_matches: false,
        placed_fighter: None,
    };
    assert_eq!(p.token(), P1Token::Held);
    p.carried = 1;
    assert_eq!(p.token(), P1Token::Unknown);
    p.carried = -1;
    p.holding = 0;
    assert_eq!(p.token(), P1Token::Unknown);
    p.grabbed = 0;
    p.panel_flags = [0, 1];
    p.placed = Some(78);
    p.rect_matches = true;
    p.placed_fighter = Some(Fighter(78));
    assert_eq!(p.token(), P1Token::Placed);
    p.panel_flags = [1, 1];
    assert_eq!(p.token(), P1Token::Unknown);
    p.panel_flags = [0, 1];
    p.rect_matches = false;
    assert_eq!(p.token(), P1Token::Unknown);
    p.rect_matches = true;
    p.visible = 0;
    assert_eq!(p.token(), P1Token::Unknown);
}

#[test]
fn qualified_native_p1_identity_is_separate_from_local_and_expires() {
    let mut p = Publisher::default();
    let mut s = Sample {
        screen: Some(Screen::Css),
        tick: 1,
        observation: 1,
        resting: true,
        at: Some(78),
        hover: Some(78),
        p1_carried: Some(0),
        p1_token: P1Token::Held,
        fighter: Some(Fighter(78)),
        ..Sample::default()
    };
    let held = block(&mut p, 1, Some(s), true);
    assert!(held.contains("\nkind=88\nname=\"Steve\"\nform=default\n"));
    assert!(held.contains("\np1.token=held\n"));
    assert!(held.contains("\np1.placed.kind=unknown\n"));
    s.tick += 1;
    s.observation += 1;
    s.p1_carried = None;
    s.fighter = None;
    s.hover = None;
    s.p1_token = P1Token::Placed;
    s.p1_placed = Some((78, Fighter(78)));
    let placed = block(&mut p, 2, Some(s), true);
    assert!(placed.contains("\np1.token=placed\np1.placed.at=78\np1.placed.kind=88\np1.placed.name=\"Steve\"\np1.placed.form=default\n"));
    assert!(placed.contains("\nkind=unknown\n"));
    for key in [
        "seat",
        "controller",
        "carried",
        "local.token",
        "placed.kind",
    ] {
        assert!(placed.contains(&format!("\n{key}=unknown\n")));
    }
    let expired = block(&mut p, 3 + MAX_AGE, None, true);
    for key in ["p1.token", "p1.placed.kind", "kind"] {
        assert!(expired.contains(&format!("\n{key}=unknown\n")));
    }
}

#[test]
fn direct_local_pad_requires_an_exact_distinct_slot_in_the_observed_rosters() {
    let roster = |count: usize, local: usize| -> Vec<SeatInput> {
        (0..count)
            .map(|i| SeatInput {
                slot: i as i32,
                config: i as i32 + 1,
                active: 1,
                input: 0x10000 + 0x500 * i as u64,
                pad: if i == local { [0, 0, 0] } else { [0, -1, -1] },
            })
            .collect()
    };
    let mut offline = roster(2, 0);
    let mut guest = roster(8, 1);
    assert_eq!(local_seat(&offline), Some(0));
    assert_eq!(local_seat(&guest), Some(1));
    guest[0].pad = [0, -1, 0]; // aggregate/cached index alone
    assert_eq!(local_seat(&guest), None);
    guest[0].pad = [0, 0, 0];
    assert_eq!(local_seat(&guest), None);
    guest[0].pad = [0, -1, -1];
    guest[1].input = guest[0].input;
    assert_eq!(local_seat(&guest), None);
    offline[0].config = 2;
    assert_eq!(local_seat(&offline), None);
    assert_eq!(local_seat(&roster(2, 1)), None);
    assert_eq!(local_seat(&roster(8, 0)), None);
    assert_eq!(local_seat(&roster(3, 1)), None);
    let mut duplicate = roster(8, 1);
    duplicate[1].slot = 0;
    assert_eq!(local_seat(&duplicate), None);
    let mut inactive = roster(8, 1);
    inactive[1].active = 0;
    assert_eq!(local_seat(&inactive), None);
}

#[test]
fn guest_seat_identity_does_not_publish_native_p1_geometry_as_a_local_hover() {
    let mut s = Sample {
        screen: Some(Screen::Css),
        local_seat: Some(1),
        at: Some(78),
        hover: Some(78),
        fighter: Some(Fighter(78)),
        tick: 1,
        observation: 1,
        resting: true,
        p1_carried: Some(0),
        p1_token: P1Token::Held,
        ..Sample::default()
    };
    s.step.emitted(word(Dir::Up, None, Some(78)));
    let block = block(&mut Publisher::default(), 1, Some(s), true);
    for row in [
        "controller=0",
        "seat=1",
        "at=unknown",
        "hovered=unknown",
        "kind=unknown",
        "local.token=unknown",
        "settled=0",
        "ack.result=unknown",
        "direction=unknown",
        "p1.token=held",
    ] {
        assert!(block.contains(&format!("\n{row}\n")), "{row}");
    }
}

#[test]
fn offline_local_identity_requires_positive_owned_medal_and_direct_seat_zero() {
    let mut p = Publisher::default();
    let mut s = Sample {
        screen: Some(Screen::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 1,
        p1_carried: Some(0),
        p1_token: P1Token::Held,
        ..Sample::default()
    };
    let held = block(&mut p, 1, Some(s), true);
    for row in [
        "controller=0",
        "seat=0",
        "carried=0",
        "local.token=held",
        "placed.kind=unknown",
    ] {
        assert!(held.contains(&format!("\n{row}\n")));
    }
    s.tick = 2;
    s.observation = 2;
    s.p1_carried = None;
    s.p1_token = P1Token::Placed;
    s.p1_placed = Some((78, Fighter(78)));
    let placed = block(&mut p, 2, Some(s), true);
    for row in [
        "carried=none",
        "local.token=placed",
        "placed.at=78",
        "placed.kind=88",
        "placed.name=\"Steve\"",
        "placed.form=default",
    ] {
        assert!(placed.contains(&format!("\n{row}\n")));
    }
}

#[test]
fn captured_vectors_reproduce_offline_entry_and_normal_stage_neighbors() {
    use crate::model::{neighbour, Rect};
    let rect = |b: &[u8]| {
        let f = |i: usize| f32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        Rect {
            xmin: f(0),
            xmax: f(4),
            ymin: f(8),
            ymax: f(12),
        }
    };
    let css = include_bytes!("layouts/css.bin");
    let tiles: Vec<_> = css[16..].as_chunks::<32>().0.iter().map(|r| rect(&r[16..])).collect();
    let step = neighbour(tiles.as_slice(), (-599., -236.), Dir::Up, &rect(css));
    assert_eq!(
        (step.from, step.to, step.target),
        (None, Some(78), (-576., -77.))
    );
    let sss = include_bytes!("layouts/sss.bin");
    let tiles: Vec<_> = sss[16..].as_chunks::<24>().0.iter().map(|r| rect(&r[8..])).collect();
    let step = neighbour(tiles.as_slice(), (-189., 358.), Dir::Right, &rect(sss));
    assert_eq!(
        (step.from, step.to, step.target),
        (Some(0), Some(1), (-126., 394.))
    );
    let second = neighbour(tiles.as_slice(), step.target, Dir::Right, &rect(sss));
    assert_eq!(second.to, Some(2));
}

#[test]
fn stock_grid_recognition_rejects_an_in_place_tile_or_clamp_change() {
    use crate::layout::{supported_css_entry, Layout};
    for (screen, kinds, bytes, layout) in [
        (
            Screen::Css,
            16usize,
            include_bytes!("layouts/css.bin").as_slice(),
            Layout::Css,
        ),
        (
            Screen::Sss,
            8,
            include_bytes!("layouts/sss.bin").as_slice(),
            Layout::Sss,
        ),
    ] {
        let n = (bytes.len() - 16) / (kinds + 16);
        let identify = |b: &[u8], count| {
            let word =
                |offset: usize| u64::from_le_bytes(b[offset..offset + 8].try_into().unwrap());
            Layout::identify(screen, count, [word(0), word(8)], |i| {
                let start = 16 + i * (kinds + 16);
                (
                    [word(start), if kinds == 16 { word(start + 8) } else { 0 }],
                    [word(start + kinds), word(start + kinds + 8)],
                )
            })
        };
        assert_eq!(identify(bytes, n), Some(layout));
        assert_eq!(identify(bytes, n - 1), None);
        for i in [0, 16, 16 + kinds - 1, bytes.len() - 1] {
            let mut changed = bytes.to_vec();
            changed[i] ^= 1;
            assert_eq!(identify(&changed, n), None);
        }
    }
    assert!(supported_css_entry(
        Some(Layout::Css),
        (-599., -236.),
        None,
        Some(-1)
    ));
    assert!(!supported_css_entry(
        Some(Layout::Css),
        (-598., -236.),
        None,
        Some(-1)
    ));
    assert!(!supported_css_entry(
        Some(Layout::Css),
        (-599., -236.),
        None,
        Some(-2)
    ));
    assert!(!supported_css_entry(
        Some(Layout::Css),
        (-599., -236.),
        Some(78),
        Some(-1)
    ));
}

#[test]
fn qualified_css_control_has_fresh_entry_held_and_placed_focus() {
    use crate::layout::Layout;
    let mut s = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 1,
        p1_carried: Some(0),
        p1_token: P1Token::Held,
        css_entry: true,
        resting: true,
        ..Sample::default()
    };
    let wire = |s: Sample| block(&mut Publisher::default(), 1, Some(s), true);
    let entry = wire(s);
    for row in [
        "layout=css.9f1fb03bf7c98fb9",
        "grid=css.9f1fb03bf7c98fb9",
        "focus=css.grid.entry",
        "at=none",
        "hovered=none",
        "settled=1",
    ] {
        assert!(entry.contains(&format!("\n{row}\n")), "{row}");
    }
    assert_eq!(entry.lines().nth(37), Some("settled.ack=unknown"));
    s.resting = false;
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.resting = true;
    s.css_entry = false;
    s.at = Some(78);
    s.hover = Some(78);
    s.fighter = Some(Fighter(78));
    assert!(wire(s).contains("\nfocus=css.grid.pickel\n"));
    s.at = Some(77);
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.p1_carried = None;
    s.p1_token = P1Token::Placed;
    s.hover = None;
    s.fighter = None;
    s.p1_placed = Some((78, Fighter(78)));
    assert!(wire(s).contains("\nfocus=css.medal.pickel\n"));
    s.local_seat = Some(1);
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.local_seat = Some(0);
    s.grid = None;
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.screen = Some(Screen::Sss);
    s.grid = Some(Layout::Sss);
    let raw = wire(s);
    assert!(raw.contains("\nlayout=unknown\ngrid=sss.9892b3a32e5c0164\n"));
    assert!(raw.contains("\nfocus=unknown\n"));
}

#[test]
fn takeback_starts_an_idle_action_scope_on_the_same_grid() {
    let mut s = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        generation: 7,
        observation: 9,
        tick: 5,
        p1_token: P1Token::Held,
        p1_carried: Some(0),
        resting: true,
        fighter: Some(Fighter(78)),
        at: Some(78),
        hover: Some(78),
        ..Sample::default()
    };
    s.step.emitted(word(Dir::Up, None, Some(78)));
    s.step.settled(s.at, s.hover, s.p1_carried);
    s.step.new_owner();
    let b = block(&mut Publisher::default(), 5, Some(s), true);
    for row in [
        "generation=7",
        "ack=1",
        "applied=1",
        "settled=1",
        "ack.result=unknown",
        "direction=unknown",
    ] {
        assert!(b.contains(&format!("\n{row}\n")));
    }
    assert_eq!(b.lines().nth(37), Some("settled.ack=unknown"));
}

#[test]
fn only_the_direct_qualified_entry_step_keeps_tile_focus_while_easing() {
    let mut s = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        generation: 7,
        observation: 2,
        tick: 5,
        p1_token: P1Token::Held,
        p1_carried: Some(0),
        hand_steppable: true,
        ..Sample::default()
    };
    s.step.emitted(word(Dir::Up, None, Some(78)));
    let show = |s: Sample| block(&mut Publisher::default(), 5, Some(s), true);
    let moving = show(s);
    for row in [
        "focus=css.grid.moving",
        "settled=0",
        "ack.result=handled",
        "local.token=held",
    ] {
        assert!(moving.contains(&format!("\n{row}\n")));
    }
    s.resting = true;
    s.at = Some(78);
    s.hover = Some(78);
    assert!(show(s).contains("\nsettled=0\n"));
    s.fighter = Some(Fighter(78));
    assert!(show(s).contains("\nfocus=css.grid.pickel\n"));
    s.fighter = None;
    s.hand_steppable = false;
    assert!(show(s).contains("\nfocus=unknown\n"));
    s.hand_steppable = true;
    s.local_seat = Some(1);
    assert!(show(s).contains("\nfocus=unknown\n"));
}

#[test]
fn stage_wire_preserves_idle_and_ties_a_settled_pick_to_its_own_step() {
    use crate::layout::Layout;
    use crate::stage::Stage;
    let base = Sample {
        screen: Some(Screen::Sss),
        grid: Some(Layout::Sss),
        generation: 1,
        observation: 10,
        tick: 1,
        at: Some(0),
        hover: Some(0),
        stage: Some(Stage::Training),
        local_seat: Some(0),
        p1_token: P1Token::None,
        resting: true,
        ..Sample::default()
    };
    let wire = |s: Sample| block(&mut Publisher::default(), 1, Some(s), true);
    let has = |body: &str, s: &str| body.lines().any(|line| line == s);
    let idle = wire(base);
    for s in [
        "focus=sss.grid.normal",
        "layout=sss.9892b3a32e5c0164",
        "kind=308",
        "name=Training",
        "form=normal",
        "local.token=none",
        "carried=none",
        "seat=0",
        "controller=0",
        "settled=1",
        "settled.ack=unknown",
    ] {
        assert!(has(&idle, s), "{s}");
    }
    let step = Step {
        ack: 1,
        applied: 1,
        last_written: 1,
        dir: Some(Dir::Right),
        from: Some(0),
        to: Some(1),
        ..Step::default()
    };
    let moving = Sample {
        stage: None,
        stage_moving: true,
        resting: false,
        hover: None,
        step,
        ..base
    };
    let progress = wire(moving);
    for s in [
        "focus=sss.grid.moving",
        "ack.result=handled",
        "settled=0",
        "settled.ack=unknown",
        "kind=unknown",
        "name=unknown",
        "form=unknown",
    ] {
        assert!(has(&progress, s), "{s}");
    }
    let landed = wire(Sample {
        at: Some(1),
        hover: Some(1),
        stage: Some(Stage::Battlefield),
        step: Step {
            settled_ack: Some(1),
            ..step
        },
        ..base
    });
    for s in [
        "focus=sss.grid.normal",
        "kind=0",
        "name=Battlefield",
        "form=normal",
        "settled=1",
        "settled.ack=1",
    ] {
        assert!(has(&landed, s), "{s}");
    }
    let lost = wire(Sample {
        stage: None,
        local_seat: None,
        hover: None,
        p1_token: P1Token::Unknown,
        ..moving
    });
    for s in [
        "focus=unknown",
        "layout=unknown",
        "seat=unknown",
        "settled=0",
        "settled.ack=unknown",
    ] {
        assert!(has(&lost, s), "{s}");
    }
}

#[test]
fn css_mid_move_has_a_reliable_phase() {
    let sample = Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 1,
        hand_steppable: true,
        p1_token: P1Token::Held,
        p1_carried: Some(0),
        at: Some(67),
        hover: None,
        resting: false,
        fighter: None,
        step: Step {
            ack: 11,
            blocked: 0,
            applied: 11,
            dir: Some(Dir::Up),
            from: Some(78),
            to: Some(67),
            token: Some(0),
            last_written: 11,
            settled_ack: None,
        },
        ..Sample::default()
    };
    let actual = block(&mut Publisher::default(), 1, Some(sample), true);
    for line in ["focus=unknown", "hovered=unknown", "settled=0", "from=78", "to=67"] {
        assert!(actual.lines().any(|got| got == line), "missing {line}: {actual}");
    }
    assert!(actual.lines().any(|got| got == "phase=moving"), "{actual}");
}

fn css_reliability_sample(ack: u64) -> Sample {
    Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 3316,
        generation: 21,
        hand_steppable: true,
        p1_token: P1Token::Held,
        p1_carried: Some(0),
        at: Some(67),
        target: Some(67),
        step: Step {
            ack,
            blocked: 0,
            applied: ack,
            dir: Some(Dir::Up),
            from: Some(78),
            to: Some(67),
            token: Some(0),
            last_written: ack,
            settled_ack: None,
        },
        ..Sample::default()
    }
}

fn assert_lines(actual: &str, lines: &[&str]) {
    for line in lines {
        assert!(actual.lines().any(|got| got == *line), "missing {line}: {actual}");
    }
}

#[test]
fn css_state_move_preserves_captured_legacy_and_publishes_target() {
    let actual = block(&mut Publisher::default(), 1, Some(css_reliability_sample(11)), true);
    let expected = [
        "schema=tile.cursor/1", "rev=1", "boot=0", "ack=11", "ack.result=handled",
        "focus=unknown", "observation=3316", "observation.active=1", "screen=css",
        "generation=21", "layout=css.9f1fb03bf7c98fb9", "grid=css.9f1fb03bf7c98fb9",
        "at=67", "hovered=unknown", "kind=unknown", "name=unknown", "form=unknown",
        "controller=0", "seat=0", "carried=0", "p1.carried=0", "p1.token=held",
        "p1.placed.at=unknown", "p1.placed.kind=unknown", "p1.placed.name=unknown",
        "p1.placed.form=unknown", "local.token=held", "placed.at=unknown",
        "placed.kind=unknown", "placed.name=unknown", "placed.form=unknown",
        "settled=0", "applied=11", "direction=up", "from=78", "to=67",
        "result=handled", "settled.ack=unknown",
    ];
    assert_eq!(actual.lines().take(38).collect::<Vec<_>>(), expected);
    assert_lines(&actual, &[
        "phase=moving", "target.at=67", "target.kind=68", "target.name=\"Richter\"",
    ]);
    println!("T1-BEGIN\n{actual}\nT1-END");
}

#[test]
fn css_state_resting_waits_for_qualified_fighter_identity() {
    let sample = Sample {
        observation: 4785,
        generation: 28,
        resting: true,
        hover: Some(67),
        step: Step { settled_ack: Some(24), ..css_reliability_sample(24).step },
        ..css_reliability_sample(24)
    };
    let actual = block(&mut Publisher::default(), 1, Some(sample), true);
    let expected = [
        "schema=tile.cursor/1", "rev=1", "boot=0", "ack=24", "ack.result=handled",
        "focus=unknown", "observation=4785", "observation.active=1", "screen=css",
        "generation=28", "layout=css.9f1fb03bf7c98fb9", "grid=css.9f1fb03bf7c98fb9",
        "at=67", "hovered=67", "kind=unknown", "name=unknown", "form=unknown",
        "controller=0", "seat=0", "carried=0", "p1.carried=0", "p1.token=held",
        "p1.placed.at=unknown", "p1.placed.kind=unknown", "p1.placed.name=unknown",
        "p1.placed.form=unknown", "local.token=held", "placed.at=unknown",
        "placed.kind=unknown", "placed.name=unknown", "placed.form=unknown",
        "settled=1", "applied=24", "direction=up", "from=78", "to=67",
        "result=handled", "settled.ack=24",
    ];
    assert_eq!(actual.lines().take(38).collect::<Vec<_>>(), expected);
    assert_lines(&actual, &[
        "phase=resting", "target.at=67", "target.kind=68", "target.name=\"Richter\"",
    ]);
    println!("T2-BEGIN\n{actual}\nT2-END");
}

#[test]
fn css_qualified_fighter_is_settled_with_stock_target() {
    let sample = Sample {
        resting: true,
        hover: Some(67),
        fighter: Some(Fighter(67)),
        ..css_reliability_sample(11)
    };
    let actual = block(&mut Publisher::default(), 1, Some(sample), true);
    assert_lines(&actual, &[
        "focus=css.grid.richter", "kind=68", "name=\"Richter\"", "phase=settled",
        "target.at=67", "target.kind=68", "target.name=\"Richter\"",
    ]);
}

#[test]
fn ordinary_random_targets_survive_move_and_unqualified_arrival() {
    use crate::stage::Stage;
    for (panel, name, stage) in [
        (1, "Random Battlefield", Stage::RandomBattlefield),
        (2, "Random Omega", Stage::RandomOmega),
    ] {
        let moving = Sample {
            screen: Some(Screen::Sss),
            grid: Some(Layout::OrdinarySss),
            local_seat: Some(0),
            tick: 1,
            observation: 1,
            stage_moving: true,
            at: Some(0),
            target: Some(panel),
            p1_token: P1Token::Held,
            p1_carried: Some(0),
            ..Sample::default()
        };
        let wire = |s| block(&mut Publisher::default(), 1, Some(s), true);
        let target = [
            format!("target.at={panel}"), "target.kind=unknown".into(), format!("target.name={name}"),
        ];
        let check = |body: &str| {
            for line in &target {
                assert!(body.lines().any(|got| got == line), "{line}: {body}");
            }
        };
        let flight = wire(moving);
        assert_lines(&flight, &["focus=sss.grid.moving", "phase=moving"]);
        check(&flight);
        let arrival = Sample {
            at: Some(panel), hover: Some(panel), resting: true, stage_moving: false,
            local_seat: None, ..moving
        };
        let waiting = wire(arrival);
        assert_lines(&waiting, &["focus=unknown", "layout=unknown", "settled=0", "phase=resting"]);
        check(&waiting);
        let qualified = wire(Sample {
            local_seat: Some(0), stage: Some(stage), ..arrival
        });
        assert_lines(&qualified, &["settled=1", "phase=settled"]);
        check(&qualified);
    }
}

#[test]
fn training_stage_target_and_out_of_table_target_are_bounded() {
    use crate::stage::Stage;
    let sample = Sample {
        screen: Some(Screen::Sss), grid: Some(Layout::Sss), local_seat: Some(0),
        tick: 1, observation: 1, resting: true, at: Some(0), hover: Some(0),
        stage: Some(Stage::Training), target: Some(1), ..Sample::default()
    };
    let wire = |s| block(&mut Publisher::default(), 1, Some(s), true);
    assert_lines(&wire(sample), &[
        "phase=settled", "target.at=1", "target.kind=0", "target.name=Battlefield",
    ]);
    assert_lines(&wire(Sample { target: Some(20), ..sample }), &[
        "phase=settled", "target.at=20", "target.kind=unknown", "target.name=unknown",
    ]);
}

#[test]
fn cpu_coin_and_css_entry_do_not_require_fighter_identity() {
    let wire = |s| block(&mut Publisher::default(), 1, Some(s), true);
    let cpu = Sample {
        resting: true, at: Some(67), hover: Some(67), p1_carried: Some(1),
        p1_token: P1Token::Placed, fighter: None, ..css_reliability_sample(11)
    };
    assert_lines(&wire(cpu), &[
        "phase=settled", "target.at=67", "target.kind=68", "target.name=\"Richter\"",
    ]);
    let entry = Sample {
        resting: true, css_entry: true, at: None, hover: None, target: None,
        step: Step::default(), ..css_reliability_sample(11)
    };
    assert_lines(&wire(entry), &[
        "focus=css.grid.entry", "phase=settled", "target.at=none",
        "target.kind=unknown", "target.name=unknown",
    ]);
}

#[test]
fn stale_no_screen_and_guest_have_unknown_phase_and_target() {
    let sample = css_reliability_sample(11);
    for (now, s) in [
        (MAX_AGE + 2, sample),
        (1, Sample { screen: None, ..sample }),
        (1, Sample { local_seat: Some(1), ..sample }),
    ] {
        let actual = block(&mut Publisher::default(), now, Some(s), true);
        assert_lines(&actual, &[
            "phase=unknown", "target.at=unknown", "target.kind=unknown", "target.name=unknown",
        ]);
    }
}

#[test]
fn appended_state_keys_preserve_legacy_order() {
    let actual = block(&mut Publisher::default(), 1, Some(css_reliability_sample(11)), true);
    let keys: Vec<_> = actual.lines().map(|line| line.split_once('=').unwrap().0).collect();
    assert_eq!(keys, [
        "schema", "rev", "boot", "ack", "ack.result", "focus", "observation",
        "observation.active", "screen", "generation", "layout", "grid", "at", "hovered",
        "kind", "name", "form", "controller", "seat", "carried", "p1.carried", "p1.token",
        "p1.placed.at", "p1.placed.kind", "p1.placed.name", "p1.placed.form", "local.token",
        "placed.at", "placed.kind", "placed.name", "placed.form", "settled", "applied",
        "direction", "from", "to", "result", "settled.ack", "phase", "target.at",
        "target.kind", "target.name", "blocked",
    ]);
}

#[test]
fn all_css_stock_targets_fit_worst_case_legacy_content() {
    let longest = CSS_TILES.iter().enumerate().max_by_key(|(_, tile)| tile.name.len()).unwrap().0 as u16;
    for target in 0..CSS_TILES.len() as u16 {
        for (token, carried) in [(P1Token::Held, Some(0)), (P1Token::Placed, None)] {
            let sample = Sample {
                screen: Some(Screen::Css), grid: Some(Layout::Css), local_seat: Some(0),
                tick: 1, observation: u64::MAX, generation: u64::MAX,
                resting: true, at: Some(longest), hover: Some(longest), target: Some(target),
                fighter: Some(Fighter(longest)), p1_token: token, p1_carried: carried,
                p1_placed: Some((longest, Fighter(longest))),
                step: Step {
                    ack: u64::MAX, blocked: u64::MAX, applied: u64::MAX, last_written: u64::MAX,
                    dir: Some(Dir::Right), from: Some(u16::MAX), to: Some(longest),
                    token: carried, settled_ack: Some(u64::MAX),
                },
                ..Sample::default()
            };
            let actual = block(&mut Publisher::default(), 1, Some(sample), true);
            assert!(!actual.is_empty(), "no publication for target {target} and {token:?}");
            assert!(actual.len() < 1024, "target {target} and {token:?}: {}", actual.len());
        }
    }
}

#[test]
fn blocked_counter_is_appended_and_survives_owner_changes() {
    let original = css_reliability_sample(11);
    let mut changed = original;
    changed.step.blocked = 2;
    let a = block(&mut Publisher::default(), 1, Some(original), true);
    let b = block(&mut Publisher::default(), 1, Some(changed), true);
    assert_eq!(a.lines().take(42).collect::<Vec<_>>(), b.lines().take(42).collect::<Vec<_>>());
    assert_eq!(b.lines().count(), 43);
    assert_eq!(b.lines().nth(42), Some("blocked=2"));
    changed.step.new_owner();
    assert_eq!(changed.step.blocked, 2);
    assert_eq!(changed.step.ack, original.step.ack);
}

#[test]
fn an_up_at_the_top_row_edge_always_logs_to_none() {
    use crate::model::{describe, neighbour, tile_at, Line, PadFrame, PadInput, RawEvent, Rect,
                       BUTTON_L3, BUTTON_UP, EV_STEP};

    let css = include_bytes!("layouts/css.bin");
    assert_eq!(css.len(), 16 + 87 * 32);
    let word = |bytes: &[u8]| u64::from_le_bytes(bytes.try_into().unwrap());
    let rect = |bytes: &[u8]| {
        let f = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        Rect { xmin: f(0), xmax: f(4), ymin: f(8), ymax: f(12) }
    };
    let bounds = rect(&css[..16]);
    let rows = css[16..].as_chunks::<32>().0;
    assert_eq!(Layout::identify(Screen::Css, rows.len(), [word(&css[..8]), word(&css[8..16])], |i| {
        let row = &rows[i];
        ([word(&row[..8]), word(&row[8..16])], [word(&row[16..24]), word(&row[24..32])])
    }), Some(Layout::Css));
    let tiles: Vec<_> = rows.iter().map(|row| rect(&row[16..])).collect();
    let point = tiles[2].centre();
    assert_eq!(point, (-576.0, 415.0));
    assert_eq!(tile_at(tiles.as_slice(), point), Some(2));
    let event = |dir| {
        let step = neighbour(tiles.as_slice(), point, dir, &bounds);
        assert_eq!((step.from, step.to, step.target), (Some(2), None, point));
        let (a, b) = pack_step(Screen::Css, dir, &step, Some(0));
        let mut line = Line::<128>::new();
        describe(&RawEvent { tick: 0, code: EV_STEP, a, b }, &mut line).unwrap();
        let line = core::str::from_utf8(line.as_bytes()).unwrap().strip_prefix("0 ").unwrap().to_owned();
        (a, line)
    };
    let expected = "step screen=css dir=up from=2 to=none token=p1 x=-576.0 y=415.0";
    let owner = (Screen::Css, 0x1000, 0x2000);
    let source = (0, 1);
    let chord = BUTTON_L3 | BUTTON_UP;

    let mut live = PadInput::new();
    let live_frame = live.frame(owner, true, Some((source, &[(200, chord)])), || 0);
    assert_eq!(live_frame.blocked, [0; 4]);
    let live_dir = match live_frame.step {
        Some(dir) => dir,
        other => panic!("expected live-chord step, got {other:?}"),
    };
    assert_eq!(event(live_dir).1, expected);

    let mut input = PadInput::new();
    let a = [(100, BUTTON_L3), (99, 0)];
    let b = [(104, 0), (103, chord), (102, chord), (101, BUTTON_L3), (100, BUTTON_L3)];
    let outcomes = [
        input.frame(owner, true, Some((source, &a)), || 0),
        input.frame(owner, true, Some((source, &b)), || 0),
        input.frame(owner, true, Some((source, &b)), || 0),
    ];
    for frame in &outcomes {
        assert_eq!(frame.blocked, [0; 4]);
    }
    assert_eq!(outcomes, [
        PadFrame { step: None, blocked: [0; 4] },
        PadFrame { step: Some(Dir::Up), blocked: [0; 4] },
        PadFrame { step: None, blocked: [0; 4] },
    ]);
    let events: Vec<_> = outcomes.into_iter().filter_map(|frame| frame.step.map(event)).collect();
    assert_eq!(events.len(), 1, "expected one top-edge step log");
    assert_eq!(events[0].1, expected);

    let mut sample = css_reliability_sample(11);
    sample.at = Some(2);
    sample.hover = Some(2);
    sample.target = Some(2);
    sample.resting = true;
    sample.fighter = Some(Fighter(2));
    let before = block(&mut Publisher::default(), 1, Some(sample), true);
    sample.step.emitted(events[0].0);
    let after = block(&mut Publisher::default(), 1, Some(sample), true);
    assert_lines(&after, &[
        "ack=12", "applied=11", "ack.result=edge", "direction=up", "from=2",
        "to=none", "result=edge", "at=2", "settled=1", "phase=settled",
        "target.at=2", "blocked=0",
    ]);
    for key in ["kind", "name", "form", "p1.placed.kind", "p1.placed.name", "p1.placed.form",
                "placed.kind", "placed.name", "placed.form", "target.kind", "target.name"] {
        let prefix = format!("{key}=");
        assert_eq!(before.lines().find(|line| line.starts_with(&prefix)),
                   after.lines().find(|line| line.starts_with(&prefix)), "{key}");
    }
}
