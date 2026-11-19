//! The optional tile.cursor/1 producer. Only the existing writer touches discovery,
//! serialization and revision tracking. An observed main-thread owner sends values.
use crate::model::{Dir, Line, Screen, TICKS_PER_SECOND};
use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

pub const NAME: &[u8] = b"tile.cursor";
pub const SCHEMA_ID: u32 = 3;
pub const VERSION: u32 = 1;
pub const MAX_AGE: u64 = TICKS_PER_SECOND * 3 / 10;

#[derive(Default)]
pub struct Discovery {
    next: u64,
    finished: bool,
}

impl Discovery {
    /// Elapsed ticks since writer startup. A late wake never catches up missed lookups.
    pub fn due(&mut self, elapsed: u64) -> bool {
        if elapsed >= 30 * TICKS_PER_SECOND {
            self.finished = true;
        }
        if self.finished || elapsed < self.next {
            return false;
        }
        self.next = elapsed + TICKS_PER_SECOND;
        true
    }

    pub fn finish(&mut self) {
        self.finished = true;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub magic: u32,
    pub abi: u32,
    pub size: u32,
    pub readers: u32,
    pub max_payload: u32,
    pub max_providers: u32,
}

pub type Register = extern "C" fn(*const u8, u32, u32, u32) -> i32;
pub type Publish = extern "C" fn(i32, *const u8, u32) -> i32;

#[repr(C)]
pub struct Api {
    pub header: Header,
    pub register: Option<Register>,
    pub publish: Option<Publish>,
    pub event: Option<Publish>,
}

impl Header {
    pub fn compatible(self) -> bool {
        self.magic == u32::from_le_bytes(*b"BRST")
            && self.abi == 1
            && self.size as usize >= core::mem::size_of::<Api>()
            && self.readers == 1
            && self.max_payload == 2048
            && self.max_providers == 8
    }
}

#[derive(Clone, Copy)]
pub struct Registered {
    handle: i32,
    publish: Publish,
}

impl Registered {
    pub fn register(api: &Api) -> Option<Self> {
        if !api.header.compatible() || api.event.is_none() {
            return None;
        }
        let publish = api.publish?;
        let handle = api.register?(NAME.as_ptr(), NAME.len() as u32, SCHEMA_ID, VERSION);
        (handle >= 0).then_some(Self { handle, publish })
    }

    pub fn send(&self, bytes: &[u8]) -> bool {
        (self.publish)(self.handle, bytes.as_ptr(), bytes.len() as u32) == 0
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Step {
    pub ack: u64,
    pub applied: u64,
    pub dir: Option<Dir>,
    pub from: Option<u16>,
    pub to: Option<u16>,
    pub token: Option<u16>,
    pub last_written: u64,
    pub settled_ack: Option<u64>,
}

impl Step {
    pub fn emitted(&mut self, word: u64) {
        let index = |shift: u32| {
            let n = (word >> shift) as u16;
            (n != u16::MAX).then_some(n)
        };
        self.ack += 1;
        self.dir = Dir::from_code((word >> 8) as u8);
        self.from = index(16);
        self.to = index(32);
        self.token = index(48);
        self.settled_ack = None;
        if self.to.is_some() {
            self.applied += 1;
            self.last_written = self.ack;
        }
    }

    /// An edge can never settle. The CSS/SSS owner independently confirms native hover.
    pub fn settled(&mut self, at: Option<u16>, native: Option<u16>, token: Option<u16>) {
        if self.to.is_some()
            && self.ack == self.last_written
            && self.token == token
            && self.to == at
            && at == native
        {
            self.settled_ack = Some(self.ack);
        }
    }

    fn new_owner(&mut self) {
        *self = Self {
            ack: self.ack,
            applied: self.applied,
            ..Self::default()
        };
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sample {
    pub observation: u64,
    pub tick: u64,
    pub generation: u64,
    pub screen: Option<Screen>,
    pub at: Option<u16>,
    pub hover: Option<u16>,
    pub p1_carried: Option<u16>,
    pub resting: bool,
    pub step: Step,
}

#[derive(Default)]
pub struct Owner {
    /// Used only on the owner thread, for identity comparison, never dereferenced later.
    key: Option<(Screen, u64, u64, u64, usize)>,
    pub latest: Sample,
}

impl Owner {
    pub const fn new() -> Self {
        Self {
            key: None,
            latest: Sample {
                observation: 0,
                tick: 0,
                generation: 0,
                screen: None,
                at: None,
                hover: None,
                p1_carried: None,
                resting: false,
                step: Step {
                    ack: 0,
                    applied: 0,
                    dir: None,
                    from: None,
                    to: None,
                    token: None,
                    last_written: 0,
                    settled_ack: None,
                },
            },
        }
    }

    pub fn begin(&mut self, key: Option<(Screen, u64, u64, u64, usize)>, now: u64) {
        if self.key != key || now.saturating_sub(self.latest.tick) > MAX_AGE {
            self.latest.generation += 1;
            self.latest.step.new_owner();
        }
        self.key = key;
        self.latest = Sample {
            observation: self.latest.observation + 1,
            tick: now,
            generation: self.latest.generation,
            screen: key.map(|x| x.0),
            step: self.latest.step,
            ..Sample::default()
        };
    }
}

/// One bounded-copy claim, never held across formatting, I/O or game memory access.
/// A busy cell skips this handoff; the owner retains its exact cumulative counters.
pub struct Mailbox {
    busy: AtomicBool,
    sample: UnsafeCell<Option<Sample>>,
}

// SAFETY: only the holder of the Acquire claim may access the non-atomic value.
unsafe impl Sync for Mailbox {}
impl Mailbox {
    pub const fn new() -> Self {
        Self {
            busy: AtomicBool::new(false),
            sample: UnsafeCell::new(None),
        }
    }

    fn claimed<T>(&self, f: impl FnOnce(&mut Option<Sample>) -> T) -> Option<T> {
        self.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()?;
        let value = f(unsafe { &mut *self.sample.get() });
        self.busy.store(false, Ordering::Release);
        Some(value)
    }

    pub fn offer(&self, value: Sample) {
        self.claimed(|slot| *slot = Some(value));
    }

    pub fn read(&self) -> Option<Sample> {
        self.claimed(|slot| *slot).flatten()
    }
}
impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

enum Endpoint {
    Index(u16),
    None,
    Unknown,
}
impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(i) => i.fmt(f),
            Self::None => f.write_str("none"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

struct Field<T>(Option<T>);
impl<T: fmt::Display> fmt::Display for Field<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(n) => n.fmt(f),
            None => f.write_str("unknown"),
        }
    }
}

#[derive(Default)]
pub struct Publisher {
    revision: u64,
    sent: Option<(u64, bool)>,
    cached: Sample,
}

impl Publisher {
    /// The bridge replaces boot=0 with its authoritative boot before framing.
    /// A failed send leaves the next revision unchanged. Always retry the latest sample.
    pub fn poll(&mut self, now: u64, sample: Option<Sample>, send: impl FnOnce(&[u8]) -> bool) {
        if let Some(s) = sample {
            self.cached = s;
        }
        let s = self.cached;
        let active =
            s.screen.is_some() && now.checked_sub(s.tick).is_some_and(|age| age <= MAX_AGE);
        let identity = (s.observation, active);
        if self.sent == Some(identity) {
            return;
        }
        let rev = self.revision + 1;
        let mut buf = Line::<1024>::new();
        let step = s.step;
        let result = if !active || step.dir.is_none() {
            "unknown"
        } else if step.to.is_some() {
            "handled"
        } else {
            "edge"
        };
        let valid = |v| if active { v } else { None };
        let endpoint = |index: Option<u16>| match (active, step.dir, index) {
            (true, Some(_), Some(i)) => Endpoint::Index(i),
            (true, Some(_), None) => Endpoint::None,
            _ => Endpoint::Unknown,
        };
        let settled = active && s.resting && s.at.is_some() && s.at == s.hover;
        let step_still_matches = settled && s.at == step.to && step.token == s.p1_carried;
        let _ = write!(buf, "schema=tile.cursor/1\nrev={rev}\nboot=0\nack={}\nack.result={result}\nfocus=unknown\nobservation={}\nobservation.active={}", step.ack, s.observation, u8::from(active));
        let _ = write!(buf, "\nscreen={}\ngeneration={}\nlayout=unknown\nat={}\nhovered={}\nkind=unknown\nname=unknown\nform=unknown", Field(active.then_some(s.screen).flatten().map(Screen::name)), Field(active.then_some(s.generation)), Field(valid(s.at)), Field(valid(s.hover)));
        let _ = write!(buf, "\ncontroller=unknown\nseat=unknown\ncarried=unknown\np1.carried={}\np1.token=unknown\nlocal.token=unknown\nplaced.at=unknown\nplaced.kind=unknown\nplaced.name=unknown\nplaced.form=unknown\nsettled={}\napplied={}\ndirection={}\nfrom={}\nto={}\nresult={result}\nsettled.ack={}", Field(valid(s.p1_carried)), u8::from(settled), step.applied, Field(active.then_some(step.dir).flatten().map(Dir::name)), endpoint(step.from), endpoint(step.to), Field(step_still_matches.then_some(step.settled_ack).flatten()));
        if send(buf.as_bytes()) {
            self.revision = rev;
            self.sent = Some(identity);
        }
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
