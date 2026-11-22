use super::*;

pub(crate) fn measured_facts() -> CoinFacts {
    CoinFacts {
        flags: [1, 1, 1, 1, 1, 0, 0, 0],
        rest: [(576.0, -77.0), (576.0, -77.0)],
        pane: Some((576.0, -77.0)),
    }
}

pub(crate) fn area() -> Rect {
    Rect { xmin: -960.0, xmax: 960.0, ymin: -152.0, ymax: 458.0 }
}

fn offered(cpu: &CoinFacts) -> Coins {
    let mut coins = Coins { pointed: None, at: [None; css::SLOTS] };
    coins.at[0] = coin_offer(0, &measured_facts(), area());
    coins.at[1] = coin_offer(1, cpu, area());
    coins
}

#[test]
fn l3_b_takes_the_cpu_coin_into_an_empty_hand() {
    let coins = offered(&measured_facts());
    assert!(coins.at[0].is_some() && coins.at[1].is_some());
    assert_eq!(take(true, 0, Some(&coins)), Take::Coin(1));
    assert_eq!(take_registers(Take::Coin(1), u32::MAX, 1), (1, 1));
    assert_eq!(take_registers(Take::Coin(1), 0, 1), (1, 1));
}

#[test]
fn l3_b_does_nothing_while_the_cpu_coin_is_not_offered() {
    let mut moving = measured_facts();
    moving.rest[1].0 += 0.001;
    let coins = offered(&moving);
    assert!(coins.at[0].is_some() && coins.at[1].is_none());
    assert_eq!(take(true, 0, Some(&coins)), Take::Nothing);
    assert_eq!(take_registers(Take::Nothing, 0, 1), (u32::MAX, 0));
    assert_eq!(take_registers(Take::Nothing, 1, 1), (u32::MAX, 0));
}

#[test]
fn plain_b_and_other_selects_keep_the_games_b() {
    let coins = offered(&measured_facts());
    assert_eq!(take(false, 0, Some(&coins)), Take::Native);
    assert_eq!(take(true, 1, Some(&coins)), Take::Native);
    assert_eq!(take(true, 0, None), Take::Native);
    for (pointed, own) in [(u32::MAX, 0), (0, 1), (1, 1)] {
        assert_eq!(take_registers(Take::Native, pointed, own), (pointed, own));
    }
}

#[test]
fn l3_b_presses_are_counted_once_per_press() {
    let chord = BUTTON_L3 | BUTTON_B;
    let samples = [0, BUTTON_L3, chord, chord, BUTTON_L3, chord, 0, BUTTON_B, chord, 0, chord | BUTTON_RIGHT];
    let mut press = TakePress::new();
    let indices: Vec<_> = samples.into_iter().enumerate()
        .filter_map(|(index, buttons)| press.update(buttons).then_some(index))
        .collect();
    assert_eq!(indices, [2, 5, 10]);
    let mut first = TakePress::new();
    assert!(!first.update(chord));
    assert!(!first.update(chord));
    let mut b_alone = TakePress::new();
    assert!(!b_alone.update(0));
    assert!(!b_alone.update(BUTTON_B));
}

#[test]
fn take_events_are_logged_with_the_coin() {
    let (a, b) = pack_take(Screen::Css, 1);
    let mut line = Line::<128>::new();
    describe(&RawEvent { tick: 0, code: EV_TAKE, a, b }, &mut line).unwrap();
    let text = core::str::from_utf8(line.as_bytes()).unwrap().strip_prefix("0 ").unwrap();
    assert_eq!(text, "take screen=css token=p2");
}
