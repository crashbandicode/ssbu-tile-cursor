use super::*;
use crate::model::{pack_step, Step as NativeStep};

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
    let good = |panel, tile, variant, settled| fighter_identity(panel, tile, variant, settled);
    assert_eq!(good(typed, typed, [0, 0], true), Some(Fighter::Steve));
    assert_eq!(good(0xc1ff_ff00_0000_0000, typed, [0, 0], true), None);
    assert_eq!(good(typed, typed, [1, 0], true), None);
    assert_eq!(good(typed, typed, [0, 1], true), None);
    assert_eq!(good(typed, typed, [0, 0], false), None);
    assert_eq!(good(typed, typed ^ 1, [0, 0], true), None);
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
    p.placed_fighter = Some(Fighter::Steve);
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
        fighter: Some(Fighter::Steve),
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
    s.p1_placed = Some((78, Fighter::Steve));
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
        fighter: Some(Fighter::Steve),
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
    s.p1_placed = Some((78, Fighter::Steve));
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
    let tiles: Vec<_> = css[16..].chunks_exact(32).map(|r| rect(&r[16..])).collect();
    let step = neighbour(tiles.as_slice(), (-599., -236.), Dir::Up, &rect(css));
    assert_eq!(
        (step.from, step.to, step.target),
        (None, Some(78), (-576., -77.))
    );
    let sss = include_bytes!("layouts/sss.bin");
    let tiles: Vec<_> = sss[16..].chunks_exact(24).map(|r| rect(&r[8..])).collect();
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
            Layout::identify(screen, count, &b[..16], |i, k, r| {
                let start = 16 + i * (kinds + 16);
                &b[start..start + kinds] == k && &b[start + kinds..start + kinds + 16] == r
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
    assert!(entry.ends_with("settled.ack=unknown"));
    s.resting = false;
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.resting = true;
    s.css_entry = false;
    s.at = Some(78);
    s.hover = Some(78);
    s.fighter = Some(Fighter::Steve);
    assert!(wire(s).contains("\nfocus=css.grid.steve\n"));
    s.at = Some(77);
    assert!(wire(s).contains("\nfocus=unknown\n"));
    s.p1_carried = None;
    s.p1_token = P1Token::Placed;
    s.hover = None;
    s.fighter = None;
    s.p1_placed = Some((78, Fighter::Steve));
    assert!(wire(s).contains("\nfocus=css.medal.steve\n"));
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
        fighter: Some(Fighter::Steve),
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
    assert!(b.ends_with("settled.ack=unknown"));
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
    s.fighter = Some(Fighter::Steve);
    assert!(show(s).contains("\nfocus=css.grid.steve\n"));
    s.fighter = None;
    s.hand_steppable = false;
    assert!(show(s).contains("\nfocus=unknown\n"));
    s.hand_steppable = true;
    s.local_seat = Some(1);
    assert!(show(s).contains("\nfocus=unknown\n"));
}
