use super::*;
use crate::layout::Layout;
use crate::model::{
    coin_offer, describe, empty_hand, pack_token, tile_at, token_target, CoinFacts, Coins, Line,
    PadFrame, PadInput, RawEvent, Rect, BUTTON_L3, BUTTON_RIGHT, EV_TOKEN,
};

const SENTINEL: u64 = 0xc1ff_ff00_0000_0000;
const RANDOM: u64 = 0xc100_000f_d5f7_fa78;

fn wire(sample: Sample) -> String {
    let mut bytes = Vec::new();
    Publisher::default().poll(1, Some(sample), |value| {
        bytes.extend_from_slice(value);
        true
    });
    String::from_utf8(bytes).unwrap()
}

fn lines(actual: &str, expected: &[&str]) {
    for line in expected {
        assert!(actual.lines().any(|got| got == *line), "missing {line}: {actual}");
    }
}

fn sample() -> Sample {
    Sample {
        screen: Some(Screen::Css),
        grid: Some(Layout::Css),
        local_seat: Some(0),
        tick: 1,
        observation: 1,
        generation: 1,
        p1_token: P1Token::Placed,
        p1_placed: Some((67, Fighter(67))),
        hand_token: HandToken::Slot(1),
        p1_coin: P1Token::Placed,
        p1_coin_at: Some(67),
        cpu_token: P1Token::Placed,
        cpu_placed: Some((86, Fighter(86))),
        resting: true,
        ..Sample::default()
    }
}

#[test]
fn a_seat_or_layout_change_unpublishes_the_carried_fighter() {
    // What collect leaves for a held coin on a settled hover (p1.rs: P1's coin, then the CPU's).
    let p1_held = Sample {
        p1_carried: Some(0), p1_token: P1Token::Held, p1_placed: None, p1_coin: P1Token::Held, p1_coin_at: None,
        at: Some(67), hover: Some(67), fighter: Some(Fighter(67)), carried_fighter: Some(Fighter(67)),
        hand_token: HandToken::Unknown, hand_steppable: true, ..sample()
    };
    let cpu_held = Sample {
        p1_carried: Some(1), p1_token: P1Token::Unknown, p1_placed: None, cpu_token: P1Token::Held,
        cpu_placed: None, at: Some(67), hover: Some(67), carried_fighter: Some(Fighter(67)),
        hand_token: HandToken::Unknown, hand_steppable: true, ..sample()
    };
    for held in [p1_held, cpu_held] {
        lines(&wire(held), &["settled=1", "carried.kind=68", "carried.name=\"Richter\"", "carried.form=default"]);
        let mut same = held;
        same.rebind(Screen::Css, Some(0), Some(Layout::Css));
        assert_eq!(same, held, "an unchanged seat and layout invalidate nothing");
        // The seat only now became recognisable as local seat 0; the layout changed.
        for (seat, grid) in [(None, Some(Layout::Css)), (Some(0), None)] {
            let mut changed = held;
            changed.rebind(Screen::Css, seat, grid);
            assert_eq!(changed.generation, held.generation + 1);
            lines(&wire(changed), &[
                "settled=0", "kind=unknown", "name=unknown", "form=unknown",
                "carried.kind=unknown", "carried.name=unknown", "carried.form=unknown",
            ]);
        }
    }
}

// CPU held and placed-on-fighter states are P1 analogies, not scm1 measurements.
fn cpu_pick_samples() -> Vec<(&'static str, Sample)> {
    let step = |dir, from, to| crate::model::pack_step(
        Screen::Css, dir, &crate::model::Step { from: Some(from), to: Some(to), target: (0.0, 0.0) }, Some(1)).0;
    let rest = Sample {
        observation: 6000, generation: 28, at: Some(45), target: Some(45),
        p1_placed: Some((0, Fighter(0))), p1_coin_at: Some(0), hand_token: HandToken::None,
        step: Step { ack: 13, applied: 13, ..Step::default() },
        ..sample()
    };
    let mut reach = rest.step;
    reach.token_emitted(pack_token(Screen::Css, Dir::Left, Some(45), Some(86), Some(1), (576.0, -77.0)).0);
    let reach_moving = Sample { observation: 6003, at: None, target: Some(86), resting: false, step: reach, ..rest };
    let reach_settled = Sample {
        observation: 6020, at: Some(86), target: Some(86), hand_token: HandToken::Slot(1), step: reach, ..rest
    };
    let mut scope = reach;
    scope.new_owner(); // observed.rs: a picked-up coin begins a new action scope
    let held = Sample {
        observation: 6030, at: Some(86), target: Some(86), hover: Some(86), p1_carried: Some(1),
        p1_token: P1Token::Unknown, p1_placed: None, hand_token: HandToken::Unknown,
        cpu_token: P1Token::Held, cpu_placed: None, carried_fighter: Some(Fighter(86)),
        hand_steppable: true, step: scope, ..rest
    };
    let held_moving = Sample { observation: 6028, hover: None, carried_fighter: None, resting: false, ..held };
    let mut left = scope;
    left.emitted(step(Dir::Left, 86, 85));
    let step1_moving = Sample {
        observation: 6033, at: Some(85), target: Some(85), hover: None, carried_fighter: None, resting: false,
        step: left, ..held
    };
    let mut left_done = left;
    left_done.settled(Some(85), Some(85), Some(1));
    let step1_settled = Sample {
        observation: 6045, at: Some(85), target: Some(85), hover: Some(85), carried_fighter: Some(Fighter(85)),
        step: left_done, ..held
    };
    let mut up = left_done;
    up.emitted(step(Dir::Up, 85, 74));
    let step2_moving = Sample {
        observation: 6050, at: Some(74), target: Some(74), hover: None, carried_fighter: None, resting: false,
        step: up, ..held
    };
    let mut up_done = up;
    up_done.settled(Some(74), Some(74), Some(1));
    let step2_settled = Sample {
        observation: 6062, at: Some(74), target: Some(74), hover: Some(74), carried_fighter: Some(Fighter(74)),
        step: up_done, ..held
    };
    let mut after = up_done;
    after.new_owner();
    // The hand is modelled back over the banner, as after P1's placement.
    let placed = Sample { observation: 6080, cpu_placed: Some((74, Fighter(74))), step: after, ..rest };
    // Producer-shaped faults for the runner's refusals.
    let wrong_coin = Sample {
        observation: 6030, at: Some(0), target: Some(0), hover: Some(0), p1_carried: Some(0),
        p1_token: P1Token::Held, p1_placed: None, p1_coin: P1Token::Held, p1_coin_at: None,
        fighter: Some(Fighter(0)), carried_fighter: Some(Fighter(0)), hand_token: HandToken::Unknown,
        hand_steppable: true, step: scope, ..rest
    };
    let p1_moved = Sample { p1_coin_at: Some(1), ..step1_settled };
    // The step was written to 85 but the hand settled on 84: `settled=1`, no `settled.ack`.
    let unacknowledged = Sample {
        observation: 6045, at: Some(84), target: Some(84), hover: Some(84), carried_fighter: Some(Fighter(84)),
        step: left, ..held
    };
    vec![
        ("rest", rest), ("reach-moving", reach_moving), ("reach-settled", reach_settled),
        ("held-moving", held_moving), ("held", held), ("step1-moving", step1_moving),
        ("step1-settled", step1_settled), ("step2-moving", step2_moving), ("step2-settled", step2_settled),
        ("placed", placed), ("wrong-coin", wrong_coin), ("p1-moved", p1_moved), ("unacknowledged", unacknowledged),
    ]
}

#[test]
fn the_cpu_pick_wire_fixture_is_current() {
    let samples = cpu_pick_samples();
    let block = |name| wire(samples.iter().find(|(n, _)| *n == name).unwrap().1);
    lines(&block("held"), &[
        "focus=unknown", "kind=unknown", "settled=1", "at=86", "hovered=86", "settled.ack=unknown",
        "ack.result=unknown", "direction=unknown", "carried=unknown", "p1.carried=1", "local.token=unknown",
        "cpu.token=held", "p1.coin=placed", "p1.coin.at=0", "carried.kind=unknown", "carried.name=\"Random\"",
        "carried.form=default", "hand.token=unknown", "ack=14", "applied=14",
    ]);
    lines(&block("step1-settled"), &[
        "focus=unknown", "kind=unknown", "settled=1", "at=85", "hovered=85", "ack=15", "applied=15",
        "settled.ack=15", "direction=left", "from=86", "to=85", "carried.kind=74", "carried.name=\"Mii Gunner\"",
    ]);
    lines(&block("step1-moving"), &[
        "settled=0", "hovered=unknown", "settled.ack=unknown", "carried.kind=unknown",
    ]);
    lines(&block("reach-settled"), &[
        "hand.token=1", "target.token=1", "token.settled=1", "local.token=placed", "carried=none",
    ]);
    lines(&block("placed"), &[
        "cpu.token=placed", "cpu.placed.at=74", "cpu.placed.kind=84", "cpu.placed.name=\"Banjo & Kazooie\"",
        "local.token=placed", "placed.at=0", "placed.kind=0", "carried=none", "settled.ack=unknown",
        "direction=unknown", "ack=16", "applied=16",
    ]);
    lines(&block("unacknowledged"), &[
        "settled=1", "at=84", "hovered=84", "settled.ack=unknown", "direction=left", "to=85",
    ]);
    let generated: String = samples.into_iter().map(|(name, sample)| format!("### {name}\n{}\n\n", wire(sample))).collect();
    assert_eq!(generated, include_str!("cpu-pick-wire.txt"), "generated fixture:\n{generated}");
}

fn measured_facts() -> CoinFacts {
    CoinFacts {
        flags: [1, 1, 1, 1, 1, 0, 0, 0],
        rest: [(576.0, -77.0), (576.0, -77.0)],
        pane: Some((576.0, -77.0)),
    }
}

fn area() -> Rect {
    Rect { xmin: -960.0, xmax: 960.0, ymin: -152.0, ymax: 458.0 }
}

fn bounds() -> Rect {
    Rect { xmin: -935.0, xmax: 929.0, ymin: -490.0, ymax: 520.0 }
}

fn css_rectangles() -> Vec<Rect> {
    let bytes = include_bytes!("layouts/css.bin");
    bytes[16..].as_chunks::<32>().0.iter().map(|row| {
        let f = |at| f32::from_le_bytes(row[at..at + 4].try_into().unwrap());
        Rect { xmin: f(16), xmax: f(20), ymin: f(24), ymax: f(28) }
    }).collect()
}

#[test]
fn an_empty_hand_reports_the_token_it_points_at() {
    for (token, expected) in [
        (HandToken::None, "hand.token=none"),
        (HandToken::Slot(0), "hand.token=0"),
        (HandToken::Slot(1), "hand.token=1"),
    ] {
        lines(&wire(Sample { hand_token: token, ..sample() }), &[expected]);
    }
    for changed in [
        Sample { resting: false, ..sample() },
        Sample { p1_carried: Some(0), ..sample() },
        Sample { local_seat: Some(1), ..sample() },
        Sample { screen: Some(Screen::Sss), ..sample() },
        Sample { screen: None, ..sample() },
    ] {
        lines(&wire(changed), &["hand.token=unknown", "token.settled=0"]);
    }
    let mut publisher = Publisher::default();
    publisher.poll(1, Some(sample()), |_| true);
    let mut stale = Vec::new();
    publisher.poll(MAX_AGE + 2, None, |bytes| { stale.extend_from_slice(bytes); true });
    lines(core::str::from_utf8(&stale).unwrap(), &["hand.token=unknown", "token.settled=0"]);
}

#[test]
fn the_new_lines_follow_the_old_lines_in_wire_order() {
    let value = wire(sample());
    let keys: Vec<_> = value.lines().map(|line| line.split_once('=').unwrap().0).collect();
    assert_eq!(&keys[43..], &[
        "hand.token", "p1.coin", "p1.coin.at", "cpu.token", "cpu.placed.at",
        "cpu.placed.kind", "cpu.placed.name", "cpu.placed.form", "carried.kind",
        "carried.name", "carried.form", "target.token", "token.settled",
    ]);
}

#[test]
fn the_cpu_medal_reports_its_tile_and_fighter() {
    let random = fighter_identity(86, RANDOM, [RANDOM, SENTINEL], [0, 0], true);
    assert_eq!(random, Some(Fighter(86)));
    let medal_rect = Rect { xmin: 504.0, xmax: 648.0, ymin: -118.0, ymax: -36.0 };
    assert_eq!(css_rectangles()[86], medal_rect);
    let mut facts = MedalFacts {
        carried: -1, holding: 0, visible: 1, grabbed: 0, panel_flags: [0, 1],
        placed: Some(86), rect_matches: css_rectangles()[86] == medal_rect, placed_fighter: random,
    };
    assert_eq!(facts.coin(1), P1Token::Placed);
    lines(&wire(sample()), &[
        "cpu.token=placed", "cpu.placed.at=86", "cpu.placed.kind=unknown",
        "cpu.placed.name=\"Random\"", "cpu.placed.form=default",
    ]);
    let richter = CSS_TILES[67].word;
    facts.placed = Some(67);
    facts.placed_fighter = fighter_identity(67, richter, [richter, SENTINEL], [1, 0], true);
    assert_eq!(facts.coin(1), P1Token::Placed);
    let pyra = CSS_TILES[80].word;
    facts.placed_fighter = fighter_identity(80, pyra, [pyra, 0xc100_7414_c6fc_852a], [0, 1], true);
    assert_eq!(facts.coin(1), P1Token::Unknown);
    facts.placed_fighter = random;
    facts.placed = Some(86);
    facts.panel_flags = [1, 1];
    assert_eq!(facts.coin(1), P1Token::Unknown);
}

#[test]
fn cpu_lines_are_unknown_outside_trainings_two_seat_select() {
    lines(&wire(sample()), &["cpu.token=placed"]);
    let expected = [
        "cpu.token=unknown", "cpu.placed.at=unknown", "cpu.placed.kind=unknown",
        "cpu.placed.name=unknown", "cpu.placed.form=unknown",
        "p1.coin=placed", "p1.coin.at=67",
    ];
    let actual: Vec<Vec<String>> = [Some(1), None].into_iter().map(|local_seat| {
        let value = wire(Sample { local_seat, ..sample() });
        expected.iter().map(|wanted| {
            let key = wanted.split_once('=').unwrap().0;
            value.lines().find(|line| line.split_once('=').map(|x| x.0) == Some(key))
                .unwrap_or("").to_owned()
        }).collect()
    }).collect();
    let expected: Vec<_> = expected.iter().map(|value| (*value).to_owned()).collect();
    assert_eq!(actual, [expected.clone(), expected]);
}

#[test]
fn coins_are_offered_by_the_games_own_test() {
    let valid = measured_facts();
    assert_eq!(coin_offer(1, &valid, area()), Some((576.0, -77.0)));
    let mut eight_medals = Coins { pointed: None, at: [None; crate::model::css::SLOTS] };
    assert_eq!(eight_medals.at.len(), 8);
    eight_medals.at[1] = coin_offer(1, &valid, area());
    assert_eq!(eight_medals.at[1], Some((576.0, -77.0)));
    for flag in [0, 1, 3, 4] {
        let mut changed = measured_facts();
        changed.flags[flag] = 0;
        assert_eq!(coin_offer(1, &changed, area()), None, "flag +{:#x}", 0x118 + flag);
    }
    for flag in [5, 7] {
        let mut changed = measured_facts();
        changed.flags[flag] = 1;
        assert_eq!(coin_offer(1, &changed, area()), None, "flag +{:#x}", 0x118 + flag);
    }
    let mut own = measured_facts();
    own.flags[5] = 1;
    assert_eq!(coin_offer(0, &own, area()), Some((576.0, -77.0)));
    assert_eq!(coin_offer(2, &valid, area()), None);
    let mut moving = measured_facts();
    moving.rest[1].0 += 0.001;
    assert_eq!(coin_offer(1, &moving, area()), None);
    let mut pane = measured_facts();
    pane.pane = Some((576.0, -76.999));
    assert_eq!(coin_offer(1, &pane, area()), None);
    pane.pane = None;
    assert_eq!(coin_offer(1, &pane, area()), None);
    assert_eq!(coin_offer(1, &valid, Rect { xmax: 575.0, ..area() }), None);
    let mut boundary = measured_facts();
    boundary.rest = [(960.0, 458.0); 2];
    boundary.pane = Some((960.0, 458.0));
    assert_eq!(coin_offer(1, &boundary, area()), Some((960.0, 458.0)));
}

#[test]
fn a_token_step_targets_the_coin_from_the_banner() {
    let tiles = css_rectangles();
    let point = (0.0001, 180.0);
    let mut coins = Coins { pointed: None, at: [None; crate::model::css::SLOTS] };
    coins.at[1] = coin_offer(1, &measured_facts(), area());
    let slot = token_target(&coins, Dir::Left).unwrap();
    let target = bounds().clamp(coins.at[slot].unwrap());
    let from = tile_at(tiles.as_slice(), point);
    let to = tile_at(tiles.as_slice(), target);
    assert_eq!((from, to, slot, target), (Some(45), Some(86), 1, (576.0, -77.0)));
    let (a, b) = pack_token(Screen::Css, Dir::Left, from, to, Some(slot), target);
    let mut event = Line::<128>::new();
    describe(&RawEvent { tick: 0, code: EV_TOKEN, a, b }, &mut event).unwrap();
    assert_eq!(
        core::str::from_utf8(event.as_bytes()).unwrap(),
        "0 token screen=css dir=left from=45 to=86 coin=p2 x=576.0 y=-77.0"
    );
    let mut step = Step::default();
    step.token_emitted(a);
    assert_eq!((step.ack, step.applied, step.to, step.token_target), (1, 1, Some(86), Some(1)));
}

#[test]
fn a_token_step_can_start_outside_the_hand_area() {
    let current = (0.0, -300.0);
    assert!(!area().contains(current.0, current.1));
    let mut coins = Coins { pointed: None, at: [None; crate::model::css::SLOTS] };
    coins.at[1] = coin_offer(1, &measured_facts(), area());
    assert_eq!(token_target(&coins, Dir::Right), Some(1));
    assert_eq!(coins.at[1], Some((576.0, -77.0)));
}

#[test]
fn a_carried_cpu_coin_reports_its_hover_and_leaves_p1_placed() {
    let carried = Sample {
        p1_carried: Some(1), p1_token: P1Token::Unknown,
        p1_placed: None, cpu_token: P1Token::Held, cpu_placed: None,
        carried_fighter: Some(Fighter(67)), at: Some(67), hover: Some(67),
        hand_token: HandToken::Unknown, ..sample()
    };
    lines(&wire(carried), &[
        "p1.token=unknown", "p1.carried=1", "p1.coin=placed", "p1.coin.at=67",
        "cpu.token=held", "carried.kind=68", "carried.name=\"Richter\"",
        "carried.form=default", "hand.token=unknown",
    ]);
    let held = MedalFacts {
        carried: 1, holding: 1, visible: 1, grabbed: 1, panel_flags: [0, 1],
        placed: Some(67), rect_matches: true, placed_fighter: Some(Fighter(67)),
    };
    assert_eq!(held.coin(1), P1Token::Held);
    let own = MedalFacts { grabbed: 0, ..held };
    assert_eq!(own.coin(0), P1Token::Placed);
}

#[test]
fn an_empty_hand_token_step_targets_the_cpu_coin() {
    let mut coins = Coins { pointed: None, at: [None; crate::model::css::SLOTS] };
    coins.at[0] = Some((-100.0, 0.0));
    coins.at[1] = Some((576.0, -77.0));
    assert_eq!(token_target(&coins, Dir::Left), Some(1));
    coins.pointed = Some(0);
    assert_eq!(token_target(&coins, Dir::Right), Some(1));
    coins.pointed = Some(1);
    assert_eq!(token_target(&coins, Dir::Right), None);
    assert_eq!(token_target(&coins, Dir::Up), None);
    assert_eq!(token_target(&coins, Dir::Down), None);
    for dir in [Dir::Up, Dir::Down] {
        let frame = empty_hand(PadFrame { step: Some(dir), blocked: [0; 4] });
        assert_eq!(frame.step, None);
        assert_eq!(frame.blocked[dir as usize], 1);
    }
    let owner = (Screen::Css, 0x1000, 0x2000);
    let source = (0, 1);
    let chord = BUTTON_L3 | BUTTON_RIGHT;
    let mut input = PadInput::new();
    input.frame(owner, true, Some((source, &[(100, BUTTON_L3)])), || 0);
    let frame = input.frame(owner, true, Some((source, &[
        (104, chord), (103, BUTTON_L3), (102, chord), (101, BUTTON_L3), (100, BUTTON_L3)
    ])), || 0);
    let frame = empty_hand(frame);
    assert_eq!(frame.step, Some(Dir::Right));
    assert_eq!(frame.blocked[Dir::Right as usize], 1);
    let mut standalone = PadInput::new();
    let no_coins = standalone.frame(owner, false, None, || chord);
    assert_eq!(no_coins.step, None);
    assert_eq!(no_coins.blocked[Dir::Right as usize], 1);
}

#[test]
fn the_longest_block_fits_its_buffer() {
    let index = CSS_TILES.iter().enumerate().max_by_key(|(_, tile)| tile.name.len()).unwrap().0 as u16;
    let longest = Sample {
        at: Some(index), target: Some(index), hover: Some(index), fighter: Some(Fighter(index)),
        p1_placed: Some((index, Fighter(index))), p1_coin_at: Some(index),
        cpu_placed: Some((index, Fighter(index))), carried_fighter: Some(Fighter(index)),
        observation: u64::MAX, generation: u64::MAX,
        step: Step {
            ack: u64::MAX, blocked: u64::MAX, applied: u64::MAX, last_written: u64::MAX,
            dir: Some(Dir::Right), from: Some(index), to: Some(index),
            token: Some(0), settled_ack: Some(u64::MAX),
            token_step: true, token_target: Some(1),
        },
        ..sample()
    };
    let value = wire(longest);
    assert!(!value.is_empty(), "publisher refused longest sample");
    println!("longest block: {} bytes", value.len());
    assert!(value.len() <= 2048, "length {}", value.len());
    assert_eq!(value.lines().count(), 56);
}

// Generated by the printing test in a disposable 236c98d checkout; see
// artifacts/phase23/setup-calls/tile/golden-manifest.json for provenance.
fn legacy_sample(which: &str) -> Sample {
    let placed = Sample {
        observation: 5286, tick: 1, generation: 28, screen: Some(Screen::Css),
        grid: Some(Layout::Css), local_seat: Some(0), at: Some(45), target: Some(45),
        p1_token: P1Token::Placed, p1_placed: Some((67, Fighter(67))), resting: false,
        step: Step { ack: 54, applied: 54, ..Step::default() },
        ..Sample::default()
    };
    match which {
        "p1" => placed,
        "empty" => Sample { observation: 5292, resting: true, ..placed },
        "cpu" => Sample {
            observation: 5300, at: Some(85), target: Some(85), hover: Some(85),
            p1_carried: Some(1), p1_token: P1Token::Unknown, p1_placed: None,
            hand_steppable: true, resting: true,
            step: Step {
                ack: 55, applied: 55, dir: Some(Dir::Left), from: Some(86), to: Some(85),
                token: Some(1), last_written: 55, settled_ack: Some(55), ..Step::default()
            },
            ..placed
        },
        _ => panic!("unknown legacy fixture"),
    }
}

fn legacy(sample: Sample) -> String {
    let mut publisher = Publisher { revision: 856, ..Publisher::default() };
    let mut bytes = Vec::new();
    publisher.poll(1, Some(sample), |value| { bytes.extend_from_slice(value); true });
    String::from_utf8(bytes).unwrap().lines().take(43).collect::<Vec<_>>().join("\n")
}

#[test]
fn old_lines_are_unchanged_for_p1s_coin() {
    assert_eq!(legacy(legacy_sample("p1")), include_str!("golden-p1-236c98d.txt"));
}

#[test]
fn old_lines_are_unchanged_for_an_empty_hand() {
    assert_eq!(legacy(legacy_sample("empty")), include_str!("golden-empty-236c98d.txt"));
}

#[test]
fn old_lines_are_unchanged_for_a_carried_cpu_coin() {
    assert_eq!(legacy(legacy_sample("cpu")), include_str!("golden-cpu-236c98d.txt"));
}

#[test]
fn a_six_frame_hold_steps_once() {
    let owner = (Screen::Css, 0x1000, 0x2000);
    for empty in [false, true] {
        let mut input = PadInput::new();
        let mut count = 0;
        for pressed in [true, true, true, true, true, true, false] {
            let frame = input.frame(owner, true, None, || {
                if pressed { BUTTON_L3 | BUTTON_RIGHT } else { 0 }
            });
            let frame = if empty { empty_hand(frame) } else { frame };
            count += usize::from(frame.step == Some(Dir::Right));
            assert_eq!(frame.blocked, [0; 4]);
        }
        assert_eq!(count, 1, "empty={empty}");
    }
}

#[test]
fn a_token_step_shows_in_the_old_lines_only_as_a_step() {
    let mut value = sample();
    value.hand_token = HandToken::Slot(1);
    let (a, _) = pack_token(Screen::Css, Dir::Left, Some(45), Some(86), Some(1), (576.0, -77.0));
    value.step.token_emitted(a);
    let actual = wire(value);
    lines(&actual, &[
        "ack=1", "applied=1", "ack.result=handled", "result=handled", "direction=left",
        "from=45", "to=86", "settled=0", "settled.ack=unknown",
        "target.token=1", "token.settled=1",
    ]);
    value.hand_token = HandToken::Slot(0);
    lines(&wire(value), &["token.settled=0"]);
    let (no_tile, _) = pack_token(Screen::Css, Dir::Left, Some(45), None, Some(1), (1.0, 1.0));
    let mut no_tile_step = Step::default();
    no_tile_step.token_emitted(no_tile);
    lines(&wire(Sample { step: no_tile_step, ..value }), &[
        "ack.result=handled", "applied=1", "target.token=1",
    ]);
    no_tile_step.emitted(crate::model::pack_step(
        Screen::Css, Dir::Right,
        &crate::model::Step { from: Some(1), to: Some(2), target: (0.0, 0.0) },
        Some(0),
    ).0);
    assert!(!no_tile_step.token_step);
    assert_eq!(no_tile_step.token_target, None);
}

#[test]
fn an_edge_token_step_is_an_edge() {
    let (a, _) = pack_token(Screen::Css, Dir::Right, Some(86), None, None, (576.0, -77.0));
    let mut step = Step::default();
    step.token_emitted(a);
    let value = wire(Sample { step, ..sample() });
    lines(&value, &[
        "ack=1", "applied=0", "ack.result=edge", "result=edge", "to=none",
        "target.token=none", "token.settled=0",
    ]);
}
