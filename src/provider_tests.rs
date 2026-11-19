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
    step.settled(Some(0), Some(0));
    assert_eq!((step.ack, step.applied, step.settled_ack), (1, 0, None));
    step.emitted(word(Dir::Right, Some(0), Some(1)));
    step.settled(Some(1), None);
    step.settled(Some(1), Some(0));
    assert_eq!((step.ack, step.applied, step.settled_ack), (2, 1, None));
    step.settled(Some(1), Some(1));
    assert_eq!(
        (step.dir, step.from, step.to, step.settled_ack),
        (Some(Dir::Right), Some(0), Some(1), Some(2))
    );
    step.emitted(word(Dir::Right, Some(1), None));
    step.settled(Some(1), Some(1));
    assert_eq!((step.applied, step.settled_ack), (1, None));
}

#[test]
fn replaced_owner_or_expired_heartbeat_cannot_inherit_settlement() {
    let mut o = Owner::new();
    let css = Some((Screen::Css, 100, 200, 86));
    o.begin(css, 1);
    o.latest.step.emitted(word(Dir::Right, Some(2), Some(3)));
    o.latest.step.settled(Some(3), Some(3));
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
    o.begin(Some((Screen::Sss, 100, 200, 86)), MAX_AGE + 4);
    assert_eq!(o.latest.generation, 3);
    o.begin(None, MAX_AGE + 5);
    assert_eq!(o.latest.screen, None);
}

#[test]
fn drops_retry_latest_and_heartbeat_expiry_publishes_unknown() {
    let mut p = Publisher::default();
    let mut o = Owner::new();
    o.begin(Some((Screen::Css, 100, 200, 86)), 1);
    o.latest.at = Some(1);
    let first = block(&mut p, 1, Some(o.latest), false);
    assert!(first.starts_with(
        "schema=tile.cursor/1\nrev=1\nboot=0\nack=0\nack.result=unknown\nfocus=unknown\n"
    ));
    o.begin(Some((Screen::Css, 100, 200, 86)), 2);
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
