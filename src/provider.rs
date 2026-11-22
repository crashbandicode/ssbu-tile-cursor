//! The optional tile.cursor/1 producer. Only the existing writer touches discovery,
//! serialization and revision tracking. An observed main-thread owner sends values.
use crate::css_table::CSS_TILES;
use crate::layout::Layout;
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
    pub blocked: u64,
    pub applied: u64,
    pub take_ack: u64,
    pub take_applied: u64,
    pub dir: Option<Dir>,
    pub from: Option<u16>,
    pub to: Option<u16>,
    pub token: Option<u16>,
    pub last_written: u64,
    pub settled_ack: Option<u64>,
    pub token_step: bool,
    pub token_target: Option<u16>,
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
        self.token_step = false;
        self.token_target = None;
        if self.to.is_some() {
            self.applied += 1;
            self.last_written = self.ack;
        }
    }

    pub fn token_emitted(&mut self, word: u64) {
        let index = |shift: u32| {
            let n = (word >> shift) as u16;
            (n != u16::MAX).then_some(n)
        };
        self.ack += 1;
        self.dir = Dir::from_code((word >> 8) as u8);
        self.from = index(16);
        self.to = index(32);
        self.token = None;
        self.settled_ack = None;
        self.token_step = true;
        self.token_target = index(48);
        if self.token_target.is_some() {
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

    pub(crate) fn new_owner(&mut self) {
        *self = Self {
            ack: self.ack,
            blocked: self.blocked,
            applied: self.applied,
            take_ack: self.take_ack,
            take_applied: self.take_applied,
            ..Self::default()
        };
    }
}

/// Per-player, directly linked native input. The shared aggregate input is never a candidate.
#[derive(Clone, Copy, Debug, Default)]
pub struct SeatInput {
    pub slot: i32,
    pub config: i32,
    pub active: u8,
    pub input: u64,
    pub pad: [i32; 3],
}

pub fn local_seat(rows: &[SeatInput]) -> Option<u8> {
    if !matches!(rows.len(), 2 | 8) {
        return None;
    }
    let mut local = None;
    let mut seen = 0u16;
    for (i, row) in rows.iter().enumerate() {
        let slot = u8::try_from(row.slot).ok()?;
        if usize::from(slot) >= rows.len()
            || row.config != row.slot + 1
            || row.active != 1
            || row.input < 0x10000
            || !row.input.is_multiple_of(8)
            || seen & (1 << slot) != 0
            || rows[..i].iter().any(|s| s.input == row.input)
        {
            return None;
        }
        seen |= 1 << slot;
        match row.pad {
            [0, -1, -1] => (),
            [0, 0, 0] if local.is_none() => local = Some(slot),
            _ => return None,
        }
    }
    // Exact Training and local-wireless guest assignments from the paired stock controls.
    match (rows.len(), local) {
        (2, Some(0)) | (8, Some(1)) => local,
        _ => None,
    }
}

/// A qualified stock CSS tile, as its index into the generated `CSS_TILES`. The index, its typed tile word, the
/// current panel preview and the default variant are checked before exposing the tile's app fighter kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fighter(u16);

impl Fighter {
    pub fn tile(self) -> u16 {
        self.0
    }

    pub fn id(self) -> Option<u16> {
        CSS_TILES[usize::from(self.0)].kind
    }

    pub fn name(self) -> &'static str {
        CSS_TILES[usize::from(self.0)].name
    }

    pub fn hover_focus(self) -> &'static str {
        CSS_TILES[usize::from(self.0)].hover_focus
    }

    pub fn placed_focus(self) -> &'static str {
        CSS_TILES[usize::from(self.0)].placed_focus
    }
}

/// The game's lookup at .text+0x19f1fa0 selects the alternate only when the panel's variant flag is
/// set and the alternate is a valid typed hash. A panel resolving to the tile's default word is
/// qualified; one resolving to the alternate (Pyra/Mythra's second fighter) is not.
pub fn fighter_identity(
    index: u16,
    panel: u64,
    tile: [u64; 2],
    variant: [u8; 2],
    settled: bool,
) -> Option<Fighter> {
    let [word, alternate] = tile;
    let selects_alternate =
        variant != [0, 0] && alternate >> 56 == 0xc1 && alternate & 0xff_ffff_ffff != 0;
    (settled
        && !selects_alternate
        && panel == word
        && CSS_TILES
            .get(usize::from(index))
            .is_some_and(|record| record.word == word)
    )
        .then_some(Fighter(index))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum P1Token {
    #[default]
    Unknown,
    None,
    Held,
    Placed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HandToken {
    #[default]
    Unknown,
    None,
    Slot(u8),
}
impl P1Token {
    fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Held => "held",
            Self::None => "none",
            Self::Placed => "placed",
        }
    }
}

pub struct MedalFacts {
    pub carried: i32,
    pub holding: u8,
    pub visible: u8,
    pub grabbed: u8,
    pub panel_flags: [u8; 2],
    pub placed: Option<u16>,
    pub rect_matches: bool,
    pub placed_fighter: Option<Fighter>,
}

impl MedalFacts {
    pub fn token(&self) -> P1Token {
        match (self.carried, self.holding, self.visible, self.grabbed) {
            (0, 1, 1, 1) => P1Token::Held,
            (-1, 0, 1, 0)
                if self.panel_flags == [0, 1]
                    && self.placed.is_some()
                    && self.rect_matches
                    && self.placed_fighter.is_some() =>
            {
                P1Token::Placed
            }
            _ => P1Token::Unknown,
        }
    }

    pub fn coin(&self, slot: i32) -> P1Token {
        // CPU held and fighter placement are analogies; see measure/scm1/RESULT.md.
        if self.carried == slot && self.holding == 1 && self.visible == 1 && self.grabbed == 1 {
            P1Token::Held
        } else if self.carried != slot
            && self.grabbed == 0
            && self.visible == 1
            && self.panel_flags == [0, 1]
            && self.placed.is_some()
            && self.rect_matches
            && self.placed_fighter.is_some()
        {
            P1Token::Placed
        } else {
            P1Token::Unknown
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sample {
    pub observation: u64,
    pub tick: u64,
    pub generation: u64,
    pub screen: Option<Screen>,
    pub grid: Option<Layout>,
    pub stage: Option<crate::stage::Stage>,
    pub stage_moving: bool,
    pub css_entry: bool,
    pub hand_steppable: bool,
    pub at: Option<u16>,
    pub target: Option<u16>,
    pub hover: Option<u16>,
    pub p1_carried: Option<u16>,
    pub local_seat: Option<u8>,
    pub fighter: Option<Fighter>,
    pub p1_token: P1Token,
    pub p1_placed: Option<(u16, Fighter)>,
    pub hand_token: HandToken,
    pub p1_coin: P1Token,
    pub p1_coin_at: Option<u16>,
    pub cpu_token: P1Token,
    pub cpu_placed: Option<(u16, Fighter)>,
    pub carried_fighter: Option<Fighter>,
    pub resting: bool,
    pub step: Step,
}

impl Sample {
    /// A changed local seat (fighter select) or a changed layout begins a new generation and action scope:
    /// nothing read against the old hover stays published.
    pub fn rebind(&mut self, screen: Screen, previous_seat: Option<u8>, previous_grid: Option<Layout>) {
        if screen == Screen::Css && self.local_seat != previous_seat || self.grid != previous_grid {
            self.generation += 1;
            self.step.new_owner();
            self.resting = false;
            self.fighter = None;
            self.carried_fighter = None;
            self.css_entry = false;
        }
    }
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
                grid: None,
                stage: None,
                stage_moving: false,
                css_entry: false,
                hand_steppable: false,
                at: None,
                target: None,
                hover: None,
                p1_carried: None,
                local_seat: None,
                fighter: None,
                p1_token: P1Token::Unknown,
                p1_placed: None,
                hand_token: HandToken::Unknown,
                p1_coin: P1Token::Unknown,
                p1_coin_at: None,
                cpu_token: P1Token::Unknown,
                cpu_placed: None,
                carried_fighter: None,
                resting: false,
                step: Step {
                    ack: 0,
                    blocked: 0,
                    applied: 0,
                    take_ack: 0,
                    take_applied: 0,
                    dir: None,
                    from: None,
                    to: None,
                    token: None,
                    last_written: 0,
                    settled_ack: None,
                    token_step: false,
                    token_target: None,
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

#[derive(Clone, Copy)]
struct PublishedName(&'static str, bool);
impl fmt::Display for PublishedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.1 {
            write!(f, "\"{}\"", self.0)
        } else {
            f.write_str(self.0)
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
        // A sample stamped after this clock reading is newer than the reading, so its age is zero.
        let active = s.screen.is_some() && now.saturating_sub(s.tick) <= MAX_AGE;
        let identity = (s.observation, active);
        if self.sent == Some(identity) {
            return;
        }
        let rev = self.revision + 1;
        let mut buf = Line::<2048>::new();
        let step = s.step;
        let p1_view = active && s.local_seat != Some(1);
        let result = if !p1_view || step.dir.is_none() {
            "unknown"
        } else if step.to.is_some() || step.token_target.is_some() {
            "handled"
        } else {
            "edge"
        };
        let valid = |v| if active { v } else { None };
        let endpoint = |index: Option<u16>| match (p1_view, step.dir, index) {
            (true, Some(_), Some(i)) => Endpoint::Index(i),
            (true, Some(_), None) => Endpoint::None,
            _ => Endpoint::Unknown,
        };
        let css_grid = active
            && s.screen == Some(Screen::Css)
            && s.grid == Some(Layout::Css)
            && s.local_seat == Some(0);
        let sss_grid = active
            && s.screen == Some(Screen::Sss)
            && s.grid.and_then(Layout::stage_count).is_some()
            && s.local_seat == Some(0)
            && (s.stage.is_some() || s.stage_moving);
        let stage = sss_grid.then_some(s.stage).flatten();
        let entry = css_grid
            && s.css_entry
            && s.resting
            && s.p1_token == P1Token::Held
            && s.p1_carried == Some(0)
            && s.at.is_none()
            && s.hover.is_none();
        let entry_step = css_grid
            && s.hand_steppable
            && s.p1_token == P1Token::Held
            && s.p1_carried == Some(0)
            && step.token == Some(0)
            && step.dir == Some(Dir::Up)
            && step.from.is_none()
            && step.to == Some(78)
            && step.ack > 0
            && step.last_written == step.ack;
        let settled = p1_view
            && s.resting
            && (s.at.is_some() && s.at == s.hover || entry)
            && (!entry_step || s.fighter.map(Fighter::tile) == Some(78))
            && (s.screen != Some(Screen::Sss) || stage.is_some());
        // Tile-local focus is bound to the released direct hand. The separate ssbu.ui
        // foreground and input-owner block must authorize any subsequent controller action.
        let hovered = s.fighter.filter(|f| s.at == Some(f.tile()));
        let placed_on = s
            .p1_placed
            .filter(|&(i, f)| f.tile() == i)
            .map(|(_, f)| f);
        let focus = match (css_grid && s.resting, s.p1_token) {
            (true, P1Token::Held) if entry => "css.grid.entry",
            (true, P1Token::Held)
                if settled && s.p1_carried == Some(0) && hovered.is_some() =>
            {
                hovered.map_or("unknown", Fighter::hover_focus)
            }
            (true, P1Token::Placed) if placed_on.is_some() => {
                placed_on.map_or("unknown", Fighter::placed_focus)
            }
            _ if sss_grid && s.stage_moving => "sss.grid.moving",
            _ if sss_grid && settled && stage.is_some_and(|panel| panel.id().is_none()) => {
                "sss.grid.random"
            }
            _ if sss_grid && settled => "sss.grid.normal",
            _ if entry_step && !settled => "css.grid.moving",
            _ => "unknown",
        };
        let step_still_matches = settled
            && s.at == step.to
            && (s.screen == Some(Screen::Sss) || step.token == s.p1_carried);
        let _ = write!(buf, "schema=tile.cursor/1\nrev={rev}\nboot=0\nack={}\nack.result={result}\nfocus={focus}\nobservation={}\nobservation.active={}", step.ack, s.observation, u8::from(active));
        let fighter = p1_view.then_some(s.fighter).flatten();
        let token = if active { s.p1_token } else { P1Token::Unknown };
        let placed = active.then_some(s.p1_placed).flatten();
        let local_p1 = active && s.local_seat == Some(0);
        let local_token = if local_p1 { token } else { P1Token::Unknown };
        let local_placed = local_p1.then_some(placed).flatten();
        let local_carried = match local_token {
            P1Token::Held if s.p1_carried == Some(0) => Endpoint::Index(0),
            P1Token::Placed | P1Token::None => Endpoint::None,
            _ => Endpoint::Unknown,
        };
        let _ = write!(
            buf,
            "\nscreen={}\ngeneration={}\nlayout={}\ngrid={}\nat={}\nhovered={}",
            Field(active.then_some(s.screen).flatten().map(Screen::name)),
            Field(active.then_some(s.generation)),
            Field(
                (css_grid || sss_grid)
                    .then_some(s.grid)
                    .flatten()
                    .map(Layout::name)
            ),
            Field(active.then_some(s.grid).flatten().map(Layout::name)),
            if entry {
                Endpoint::None
            } else {
                s.at.filter(|_| p1_view)
                    .map_or(Endpoint::Unknown, Endpoint::Index)
            },
            if entry {
                Endpoint::None
            } else {
                s.hover
                    .filter(|_| p1_view)
                    .map_or(Endpoint::Unknown, Endpoint::Index)
            }
        );
        let _ = write!(
            buf,
            "\nkind={}\nname={}\nform={}",
            Field(
                fighter
                    .and_then(Fighter::id)
                    .or_else(|| stage.and_then(crate::stage::Stage::id))
            ),
            Field(
                fighter
                    .map(|fighter| PublishedName(fighter.name(), true))
                    .or_else(|| stage.map(|stage| PublishedName(stage.name(), false)))
            ),
            Field(
                fighter
                    .map(|_| "default")
                    .or_else(|| stage.map(|_| "normal"))
            )
        );
        let _ = write!(
            buf,
            "\ncontroller={}\nseat={}\ncarried={}\np1.carried={}\np1.token={}",
            Field(active.then_some(s.local_seat).flatten().map(|_| 0)),
            Field(active.then_some(s.local_seat).flatten()),
            local_carried,
            Field(valid(s.p1_carried)),
            token.name()
        );
        let _ = write!(
            buf,
            "\np1.placed.at={}\np1.placed.kind={}\np1.placed.name={}\np1.placed.form={}",
            Field(placed.map(|x| x.0)),
            Field(placed.and_then(|x| x.1.id())),
            Field(placed.map(|x| PublishedName(x.1.name(), true))),
            Field(placed.map(|_| "default"))
        );
        let _ = write!(
            buf,
            "\nlocal.token={}\nplaced.at={}\nplaced.kind={}\nplaced.name={}\nplaced.form={}",
            local_token.name(),
            Field(local_placed.map(|x| x.0)),
            Field(local_placed.and_then(|x| x.1.id())),
            Field(local_placed.map(|x| PublishedName(x.1.name(), true))),
            Field(local_placed.map(|_| "default"))
        );
        let _ = write!(
            buf, "\nsettled={}\napplied={}\ndirection={}\nfrom={}\nto={}\nresult={result}\nsettled.ack={}",
            u8::from(settled), step.applied, Field(p1_view.then_some(step.dir).flatten().map(Dir::name)),
            endpoint(step.from), endpoint(step.to), Field(step_still_matches.then_some(step.settled_ack).flatten())
        );
        let phase = if !p1_view {
            "unknown"
        } else if !s.resting {
            "moving"
        } else if settled
            && (!(css_grid && s.p1_token == P1Token::Held && s.p1_carried == Some(0) && !entry)
                || hovered.is_some())
        {
            "settled"
        } else {
            "resting"
        };
        let target_css = (p1_view && s.screen == Some(Screen::Css) && s.grid == Some(Layout::Css))
            .then_some(s.target)
            .flatten()
            .and_then(|i| CSS_TILES.get(usize::from(i)));
        let target_stage = (p1_view && s.screen == Some(Screen::Sss))
            .then_some(s.grid.zip(s.target))
            .flatten()
            .and_then(|(layout, panel)| crate::stage::stock_stage(layout, panel));
        let target = match (p1_view, s.target) {
            (true, Some(i)) => Endpoint::Index(i),
            (true, None) => Endpoint::None,
            _ => Endpoint::Unknown,
        };
        let _ = write!(
            buf, "\nphase={phase}\ntarget.at={target}\ntarget.kind={}\ntarget.name={}\nblocked={}",
            Field(target_css.and_then(|tile| tile.kind).or_else(|| target_stage.and_then(crate::stage::Stage::id))),
            Field(target_css.map(|tile| PublishedName(tile.name, true))
                .or_else(|| target_stage.map(|stage| PublishedName(stage.name(), false)))),
            step.blocked
        );
        let hand = if css_grid && s.p1_carried.is_none() && s.resting {
            s.hand_token
        } else {
            HandToken::Unknown
        };
        let hand_endpoint = match hand {
            HandToken::Unknown => Endpoint::Unknown,
            HandToken::None => Endpoint::None,
            HandToken::Slot(slot) => Endpoint::Index(u16::from(slot)),
        };
        let coin_view = active && s.screen == Some(Screen::Css);
        let cpu_view = coin_view && s.local_seat == Some(0);
        let p1_coin = if coin_view { s.p1_coin } else { P1Token::Unknown };
        let cpu_token = if cpu_view { s.cpu_token } else { P1Token::Unknown };
        let cpu_placed = cpu_view.then_some(s.cpu_placed).flatten();
        let carried = p1_view.then_some(s.carried_fighter).flatten();
        let target_token = match (p1_view, step.token_step, step.token_target) {
            (false, _, _) => Endpoint::Unknown,
            (true, true, Some(slot)) => Endpoint::Index(slot),
            _ => Endpoint::None,
        };
        let token_settled = step.token_step
            && step.ack == step.last_written
            && match hand {
                HandToken::Slot(slot) => step.token_target == Some(u16::from(slot)),
                _ => false,
            };
        let _ = write!(
            buf,
            "\nhand.token={hand_endpoint}\np1.coin={}\np1.coin.at={}\ncpu.token={}\ncpu.placed.at={}\ncpu.placed.kind={}\ncpu.placed.name={}\ncpu.placed.form={}\ncarried.kind={}\ncarried.name={}\ncarried.form={}\ntarget.token={target_token}\ntoken.settled={}\ntake.ack={}\ntake.applied={}",
            p1_coin.name(),
            Field(coin_view.then_some(s.p1_coin_at).flatten()),
            cpu_token.name(),
            Field(cpu_placed.map(|x| x.0)),
            Field(cpu_placed.and_then(|x| x.1.id())),
            Field(cpu_placed.map(|x| PublishedName(x.1.name(), true))),
            Field(cpu_placed.map(|_| "default")),
            Field(carried.and_then(Fighter::id)),
            Field(carried.map(|f| PublishedName(f.name(), true))),
            Field(carried.map(|_| "default")),
            u8::from(token_settled),
            step.take_ack,
            step.take_applied
        );
        if buf.as_bytes().len() < 2048 && send(buf.as_bytes()) {
            self.revision = rev;
            self.sent = Some(identity);
        }
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "provider_cpu_tests.rs"]
mod cpu_tests;
