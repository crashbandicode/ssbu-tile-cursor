//! Host-testable half: hook sites, game struct offsets, the L3 + D-pad repeat
//! timing, the neighbour-tile search and the fixed-buffer log lines.
//!
//! Smash Ultimate 13.0.5 gives the character select screen (CSS) and the stage
//! select screen the same hand class. Its cursor holds a current and a target
//! position; while they differ, the cursor's per-frame update eases current
//! 30 % of the way to the target (snapping within 10 units) and ignores the
//! stick and D-pad. The game uses that for its own `move_hand_interpolated`.
//! Both screens hit-test the hand's current position against a vector of tile
//! rectangles in the hand's coordinate space (layout units, origin at the
//! screen centre, y up). Setting the target to a neighbouring rectangle's
//! centre therefore moves the hand exactly one tile, through the game's own
//! movement, whatever the frame pacing.

use core::fmt;

pub const SUPPORTED_VERSION: &str = "13.0.5";

/// A 13.0.5 `.text` offset and the 64 bytes that must be there.
pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub guard: [u8; 64],
}

/// CSS main-state update `fn(css)` (`.text+0x1a12f60`), once per frame while
/// players pick fighters: its `mov x19, x0` right after the prologue (x0 =
/// css). Past the entry, so a plugin that replaces the entry (as the lobby
/// window does) and this hook do not overlap.
pub const CSS_UPDATE_OFFSET: usize = 0x1a12f8c;
pub const CSS_UPDATE: Site = Site {
    name: "css_update",
    offset: CSS_UPDATE_OFFSET,
    guard: [
        0xf3, 0x03, 0x00, 0xaa, 0x08, 0x01, 0x00, 0x35, 0x68, 0xce, 0x40, 0xf9, 0x08, 0x01, 0x40,
        0xf9, 0x00, 0x01, 0x40, 0xf9, 0xe1, 0x03, 0x00, 0x32, 0x53, 0x8c, 0x75, 0x94, 0x60, 0xfa,
        0x46, 0xf9, 0x89, 0xf3, 0x62, 0x94, 0x74, 0x1e, 0x41, 0xf9, 0x6e, 0x22, 0x41, 0xf9, 0x6d,
        0xd2, 0x05, 0x91, 0x9f, 0x02, 0x0e, 0xeb, 0x60, 0x46, 0x01, 0x54, 0x68, 0x42, 0x09, 0x91,
        0xe8, 0x03, 0x00, 0xf9,
    ],
};

/// Stage select update `fn(sss)` (`.text+0x1b29cb0`, vtable `0x50aa728` slot
/// 0), which runs the hand (`sss+0x60`) and token updates every frame: its
/// `mov x19, x0` after the prologue (x0 = sss).
pub const SSS_UPDATE_OFFSET: usize = 0x1b29cbc;
pub const SSS_UPDATE: Site = Site {
    name: "sss_update",
    offset: SSS_UPDATE_OFFSET,
    guard: [
        0xf3, 0x03, 0x00, 0xaa, 0x08, 0x0c, 0x41, 0xf8, 0x09, 0x04, 0x40, 0xf9, 0x2a, 0x01, 0x08,
        0xeb, 0x00, 0x0f, 0x00, 0x54, 0x0b, 0x01, 0x40, 0xf9, 0x6b, 0x01, 0x40, 0xf9, 0x6b, 0x01,
        0x40, 0xf9, 0x6b, 0x01, 0x40, 0xf9, 0x6b, 0x01, 0x43, 0xf9, 0x6c, 0x85, 0x44, 0xb9, 0x4c,
        0x05, 0x00, 0x35, 0x8c, 0xc0, 0x01, 0x90, 0x8c, 0x59, 0x44, 0xf9, 0x6b, 0x89, 0x84, 0xb9,
        0x7f, 0x25, 0x00, 0x71,
    ],
};

/// Inside the hand cursor's update (`.text+0x3786140`, x19 = cursor, x20 =
/// input context): `mov x0, x20` before `bl 0x3763840`, the stick/D-pad query.
/// When the query returns zero the rest of the update never reads x20.
pub const CURSOR_QUERY_OFFSET: usize = 0x3786244;
pub const CURSOR_QUERY: Site = Site {
    name: "cursor_query",
    offset: CURSOR_QUERY_OFFSET,
    guard: [
        0xe0, 0x03, 0x14, 0xaa, 0x7e, 0x75, 0xff, 0x97, 0x05, 0x1c, 0xa0, 0x4e, 0x00, 0xd8, 0xa0,
        0x4e, 0x01, 0x40, 0x00, 0x6e, 0x00, 0x1c, 0x21, 0x0e, 0x08, 0x00, 0x26, 0x1e, 0x09, 0x3c,
        0x0c, 0x0e, 0x1f, 0x01, 0x09, 0x6a, 0x01, 0x04, 0x00, 0x54, 0x88, 0x06, 0x40, 0xf9, 0x09,
        0xa9, 0x52, 0x39, 0xe9, 0x05, 0x00, 0x34, 0x08, 0xc1, 0x10, 0x91, 0x33, 0x00, 0x00, 0x14,
        0xc8, 0x06, 0x40, 0xf9,
    ],
};

/// Not hooked: the query itself, whose input-object layout the D-pad filter
/// copies (enabled flag `+0x4aa`, stick `+0x430`, D-pad words `+0x264..+0x270`).
pub const INPUT_QUERY: Site = Site {
    name: "input_query",
    offset: 0x3763840,
    guard: [
        0xff, 0x03, 0x01, 0xd1, 0xe8, 0x13, 0x00, 0xfd, 0xf3, 0x17, 0x00, 0xf9, 0xfd, 0x7b, 0x03,
        0xa9, 0xfd, 0xc3, 0x00, 0x91, 0x08, 0x04, 0x40, 0xf9, 0x09, 0xa9, 0x52, 0x39, 0x28, 0x1c,
        0xa1, 0x4e, 0x69, 0x00, 0x00, 0x34, 0x08, 0xc1, 0x10, 0x91, 0x07, 0x00, 0x00, 0x14, 0xc8,
        0x0e, 0x01, 0x90, 0x08, 0x61, 0x26, 0x91, 0x08, 0xfd, 0xdf, 0x08, 0x88, 0x0a, 0x00, 0x36,
        0xc8, 0x0e, 0x01, 0x90,
    ],
};

/// Not hooked: the CSS fighter-tile hit test (`ldr x9, [x19, #0xdd0]`, 0xb0-byte
/// tiles, rectangle at `+0x48`), the layout `css::TILES` mirrors.
pub const CSS_HIT_TEST: Site = Site {
    name: "css_hit_test",
    offset: 0x1a130d0,
    guard: [
        0x69, 0xea, 0x46, 0xf9, 0x2a, 0x25, 0x40, 0xa9, 0x29, 0x01, 0x0a, 0xeb, 0xf8, 0x03, 0x05,
        0x2a, 0x60, 0x03, 0x00, 0x54, 0x01, 0x04, 0x0c, 0x5e, 0x6b, 0x74, 0x91, 0xd2, 0xcb, 0x45,
        0xb7, 0xf2, 0x29, 0xfd, 0x44, 0x93, 0x0b, 0x5d, 0xd4, 0xf2, 0x6b, 0xd1, 0xe5, 0xf2, 0x29,
        0x7d, 0x0b, 0x9b, 0xf8, 0x03, 0x1f, 0xaa, 0x4a, 0x51, 0x01, 0x91, 0x42, 0x81, 0x5f, 0xbc,
        0x40, 0x20, 0x20, 0x1e,
    ],
};

/// Not hooked: the stage select's `get_hand_on_stage_panel_id` Lua binding
/// (`ldp x9, x10, [x0, #0x168]`, 0x18-byte rectangles), mirrored by `sss`.
pub const SSS_HIT_TEST: Site = Site {
    name: "sss_hit_test",
    offset: 0x1b32fe0,
    guard: [
        0x09, 0xa8, 0x56, 0xa9, 0x3f, 0x01, 0x0a, 0xeb, 0x40, 0x03, 0x00, 0x54, 0xe8, 0x03, 0x00,
        0xaa, 0xe0, 0x03, 0x1f, 0x2a, 0x08, 0x31, 0x40, 0xf9, 0x08, 0x81, 0x40, 0xf9, 0x00, 0x19,
        0xc0, 0x3d, 0x08, 0x00, 0x80, 0x12, 0x01, 0x04, 0x0c, 0x5e, 0x22, 0x0d, 0x40, 0xbd, 0x40,
        0x20, 0x20, 0x1e, 0x4b, 0x01, 0x00, 0x54, 0x22, 0x09, 0x40, 0xbd, 0x40, 0x20, 0x20, 0x1e,
        0xe8, 0x00, 0x00, 0x54,
    ],
};

pub const SITES: [&Site; 6] = [
    &CSS_UPDATE,
    &SSS_UPDATE,
    &CURSOR_QUERY,
    &INPUT_QUERY,
    &CSS_HIT_TEST,
    &SSS_HIT_TEST,
];

/// The hand (0x170-byte object, constructor `.text+0x32d2400`).
pub mod hand {
    /// The cursor created by the hand's setup (`.text+0x32d2840`).
    pub const CURSOR: usize = 0x100;
    /// Non-zero while the hand is shown and enabled (`set_hand_enable`).
    pub const ENABLED: usize = 0x163;
}

/// The hand's cursor (0xb0 bytes, vtable `0x523dc80`). Positions are 16-byte
/// vectors `(x, y, 0, 0)`; the clamp rectangle is four floats.
pub mod cursor {
    pub const CURRENT: usize = 0x60;
    pub const TARGET: usize = 0x70;
    pub const BOUNDS: usize = 0x90;
}

/// The CSS object passed to the CSS update.
pub mod css {
    /// `std::vector` of 16-byte entries; entry `+8` is a player object.
    pub const PLAYERS: usize = 0x238;
    /// Player object: its hand, its slot (0 = P1) and the "hand active" flag
    /// the update checks before hit-testing.
    pub const PLAYER_HAND: usize = 0x20;
    pub const PLAYER_SLOT: usize = 0x30;
    pub const PLAYER_ACTIVE: usize = 0x36;
    /// Fighter tiles: an object whose `+0`/`+8` are a vector's begin/end.
    pub const TILES: usize = 0xdd0;
    pub const TILE_STRIDE: usize = 0xb0;
    pub const TILE_RECT: usize = 0x48;
    /// The hovered tile index per player slot: `+0xe0 + slot * 0x18`.
    pub const HOVERED: usize = 0xe0;
    pub const HOVERED_STRIDE: usize = 0x18;
}

/// The stage select object passed to its update.
pub mod sss {
    pub const HAND: usize = 0x60;
    /// `std::vector` begin/end of 0x18-byte stage tile rectangles at `+8`.
    pub const TILES: usize = 0x168;
    pub const TILE_STRIDE: usize = 0x18;
    pub const TILE_RECT: usize = 0x8;
}

/// The input object the cursor query reads (`[ctx + 8]`).
pub mod input {
    /// Bytes the D-pad filter copies (covers every field the query reads).
    pub const COPY_BYTES: usize = 0x4b0;
    /// D-pad words whose bit 4 the query tests: up, down, left, right.
    pub const DPAD_WORDS: [usize; 4] = [0x264, 0x268, 0x26c, 0x270];
}

/// `nn::hid` button bits.
pub const BUTTON_L3: u64 = 1 << 4;
pub const BUTTON_LEFT: u64 = 1 << 12;
pub const BUTTON_UP: u64 = 1 << 13;
pub const BUTTON_RIGHT: u64 = 1 << 14;
pub const BUTTON_DOWN: u64 = 1 << 15;

pub const TICKS_PER_SECOND: u64 = 19_200_000;
/// The first step fires on the press; held, it repeats after this long and
/// then at this interval.
pub const REPEAT_TICKS: u64 = TICKS_PER_SECOND / 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn name(self) -> &'static str {
        match self {
            Dir::Up => "up",
            Dir::Down => "down",
            Dir::Left => "left",
            Dir::Right => "right",
        }
    }

    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn from_code(code: u8) -> Option<Dir> {
        Dir::ALL.get(usize::from(code)).copied()
    }

    /// Unit vector in layout space (y up).
    fn vector(self) -> (f32, f32) {
        match self {
            Dir::Up => (0.0, 1.0),
            Dir::Down => (0.0, -1.0),
            Dir::Left => (-1.0, 0.0),
            Dir::Right => (1.0, 0.0),
        }
    }

    /// The one D-pad direction held in `buttons`; none for zero or a diagonal.
    pub fn from_buttons(buttons: u64) -> Option<Dir> {
        let held = [
            (BUTTON_UP, Dir::Up),
            (BUTTON_DOWN, Dir::Down),
            (BUTTON_LEFT, Dir::Left),
            (BUTTON_RIGHT, Dir::Right),
        ];
        let mut found = None;
        for (mask, dir) in held {
            if buttons & mask != 0 {
                if found.is_some() {
                    return None;
                }
                found = Some(dir);
            }
        }
        found
    }
}

/// Turns the polled buttons into steps: one on the first frame L3 and a
/// single D-pad direction are both held, then one every `REPEAT_TICKS` while
/// they stay held. Releasing L3, changing or releasing the direction resets.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Repeater {
    held: Option<Dir>,
    next: u64,
}

impl Repeater {
    pub const fn new() -> Self {
        Self {
            held: None,
            next: 0,
        }
    }

    pub fn update(&mut self, buttons: u64, now: u64) -> Option<Dir> {
        let dir = if buttons & BUTTON_L3 != 0 {
            Dir::from_buttons(buttons)
        } else {
            None
        };
        if dir != self.held {
            self.held = dir;
            self.next = now.wrapping_add(REPEAT_TICKS);
            return dir;
        }
        if dir.is_some() && now >= self.next {
            self.next = now.wrapping_add(REPEAT_TICKS);
            return dir;
        }
        None
    }
}

/// A tile rectangle in layout units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub xmin: f32,
    pub xmax: f32,
    pub ymin: f32,
    pub ymax: f32,
}

impl Rect {
    pub fn centre(&self) -> (f32, f32) {
        ((self.xmin + self.xmax) * 0.5, (self.ymin + self.ymax) * 0.5)
    }

    /// The game's own test (inclusive edges).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.xmin <= x && x <= self.xmax && self.ymin <= y && y <= self.ymax
    }

    fn usable(&self) -> bool {
        let finite = [self.xmin, self.xmax, self.ymin, self.ymax]
            .iter()
            .all(|v| v.is_finite() && v.abs() < 1.0e5);
        finite && self.xmax - self.xmin > 1.0 && self.ymax - self.ymin > 1.0
    }

    pub fn clamp(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (
            x.max(self.xmin).min(self.xmax),
            y.max(self.ymin).min(self.ymax),
        )
    }
}

/// Tiles in the game's order (indices are the game's).
pub trait Tiles {
    fn count(&self) -> usize;
    fn rect(&self, index: usize) -> Rect;
}

impl Tiles for [Rect] {
    fn count(&self) -> usize {
        self.len()
    }

    fn rect(&self, index: usize) -> Rect {
        self[index]
    }
}

/// The first tile containing the point, as the game's hit tests pick it.
pub fn tile_at(tiles: &(impl Tiles + ?Sized), (x, y): (f32, f32)) -> Option<usize> {
    (0..tiles.count()).find(|&i| tiles.rect(i).contains(x, y))
}

/// A step's outcome: the tile the hand was on, the tile it goes to, and the
/// clamped centre of that tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub target: (f32, f32),
}

/// The neighbouring tile in `dir` from the hand at `point`, among tiles whose
/// centre lies inside the hand's clamp rectangle `bounds` (so tiles scrolled
/// off screen are skipped).
///
/// On a tile: left/right go to the nearest tile in the same row; up/down go to
/// the nearest row in that direction and, in it, the tile nearest in x (rows
/// of different length still connect). Off every tile: the nearest tile in
/// that half-plane, favouring straight ahead. At an edge there is no move.
pub fn neighbour(
    tiles: &(impl Tiles + ?Sized),
    point: (f32, f32),
    dir: Dir,
    bounds: &Rect,
) -> Step {
    let from = tile_at(tiles, point);
    let (vx, vy) = dir.vector();
    let horizontal = vx != 0.0;
    // A row is the band within half a tile height of the row's centre line.
    let (origin, along_min, row_band) = match from.map(|i| tiles.rect(i)) {
        Some(rect) => {
            let (half_w, half_h) = ((rect.xmax - rect.xmin) * 0.5, (rect.ymax - rect.ymin) * 0.5);
            (
                rect.centre(),
                if horizontal { half_w } else { half_h },
                half_h,
            )
        }
        None => (point, 0.0, 0.0),
    };
    // (index, along, |across|)
    let candidates = (0..tiles.count()).filter_map(|i| {
        let rect = tiles.rect(i);
        let centre = rect.centre();
        if Some(i) == from || !rect.usable() || !bounds.contains(centre.0, centre.1) {
            return None;
        }
        let (dx, dy) = (centre.0 - origin.0, centre.1 - origin.1);
        let along = dx * vx + dy * vy;
        let across = (dx * vy - dy * vx).abs();
        (along > along_min).then_some((i, along, across))
    });
    let best = if from.is_none() {
        candidates.min_by(|a, b| {
            let score = |c: &(usize, f32, f32)| c.1 * c.1 + 4.0 * c.2 * c.2;
            score(a).total_cmp(&score(b))
        })
    } else if horizontal {
        candidates
            .filter(|c| c.2 < row_band)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))
    } else {
        let nearest_row = candidates.clone().map(|c| c.1).min_by(f32::total_cmp);
        nearest_row.and_then(|row| {
            candidates
                .filter(|c| c.1 <= row + row_band.max(1.0))
                .min_by(|a, b| a.2.total_cmp(&b.2).then(a.1.total_cmp(&b.1)))
        })
    };
    match best {
        Some((to, _, _)) => Step {
            from,
            to: Some(to),
            target: bounds.clamp(tiles.rect(to).centre()),
        },
        None => Step {
            from,
            to: None,
            target: point,
        },
    }
}

/// A text line built in a fixed buffer; anything past `N` bytes is dropped.
pub struct Line<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> Line<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The text plus a NUL, for `nn::fs` paths; `None` when there is no room.
    pub fn with_nul(&mut self) -> Option<&[u8]> {
        if self.len >= N {
            return None;
        }
        self.buf[self.len] = 0;
        Some(&self.buf[..=self.len])
    }
}

impl<const N: usize> Default for Line<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for Line<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let take = s.len().min(N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    Css,
    Sss,
}

impl Screen {
    pub fn name(self) -> &'static str {
        match self {
            Screen::Css => "css",
            Screen::Sss => "sss",
        }
    }

    fn from_code(code: u64) -> Self {
        if code == 1 {
            Screen::Sss
        } else {
            Screen::Css
        }
    }

    pub fn code(self) -> u64 {
        self as u64
    }
}

/// Events the hooks queue for the log, as plain integers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RawEvent {
    pub tick: u64,
    pub code: u8,
    pub a: u64,
    pub b: u64,
}

pub const EV_SCREEN: u8 = 1;
pub const EV_STEP: u8 = 2;
pub const EV_SETTLED: u8 = 3;

const NONE16: u64 = 0xffff;

fn pack_index(index: Option<usize>) -> u64 {
    index.map_or(NONE16, |i| (i as u64).min(NONE16 - 1))
}

fn pack_point((x, y): (f32, f32)) -> u64 {
    u64::from(x.to_bits()) | u64::from(y.to_bits()) << 32
}

fn unpack_point(b: u64) -> (f32, f32) {
    (f32::from_bits(b as u32), f32::from_bits((b >> 32) as u32))
}

struct Index(u64);

impl fmt::Display for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == NONE16 {
            f.write_str("none")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// A screen became active (or its object changed): tile count and the
/// plugin's heap allocations so far.
pub fn pack_screen(screen: Screen, tiles: usize, heap_allocations: u64) -> (u64, u64) {
    (
        screen.code() | (tiles as u64 & 0xffff) << 8,
        heap_allocations,
    )
}

pub fn pack_step(screen: Screen, dir: Dir, step: &Step) -> (u64, u64) {
    (
        screen.code()
            | u64::from(dir.code()) << 8
            | pack_index(step.from) << 16
            | pack_index(step.to) << 32,
        pack_point(step.target),
    )
}

/// The hand stopped after a step: the tile it is on by the game's rectangles
/// and, on the CSS, the hovered tile the game itself recorded.
pub fn pack_settled(
    screen: Screen,
    at: Option<usize>,
    hovered: Option<usize>,
    point: (f32, f32),
) -> (u64, u64) {
    (
        screen.code() | pack_index(at) << 16 | pack_index(hovered) << 32,
        pack_point(point),
    )
}

/// One `events.log` line (without the newline) for a queued event.
pub fn describe(event: &RawEvent, out: &mut impl fmt::Write) -> fmt::Result {
    let (a, b) = (event.a, event.b);
    let screen = Screen::from_code(a & 0xff).name();
    write!(out, "{} ", event.tick)?;
    match event.code {
        EV_SCREEN => write!(
            out,
            "screen={screen} tiles={} heap_allocations={b}",
            a >> 8 & 0xffff
        ),
        EV_STEP => {
            let (x, y) = unpack_point(b);
            let dir = Dir::from_code((a >> 8 & 0xff) as u8).map_or("?", Dir::name);
            write!(
                out,
                "step screen={screen} dir={dir} from={} to={} x={x:.1} y={y:.1}",
                Index(a >> 16 & 0xffff),
                Index(a >> 32 & 0xffff)
            )
        }
        EV_SETTLED => {
            let (x, y) = unpack_point(b);
            write!(
                out,
                "settled screen={screen} at={} hovered={} x={x:.1} y={y:.1}",
                Index(a >> 16 & 0xffff),
                Index(a >> 32 & 0xffff)
            )
        }
        other => write!(out, "unknown code={other} a=0x{a:x} b=0x{b:x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write as _;

    #[test]
    fn guards_start_with_the_hooked_instructions() {
        // mov x19,x0 / mov x19,x0 / mov x0,x20 /
        // sub sp,sp,#0x40 / ldr x9,[x19,#0xdd0] / ldp x9,x10,[x0,#0x168]
        let heads = [
            [0xf3, 0x03, 0x00, 0xaa],
            [0xf3, 0x03, 0x00, 0xaa],
            [0xe0, 0x03, 0x14, 0xaa],
            [0xff, 0x03, 0x01, 0xd1],
            [0x69, 0xea, 0x46, 0xf9],
            [0x09, 0xa8, 0x56, 0xa9],
        ];
        for (site, head) in SITES.iter().zip(heads) {
            assert_eq!(site.guard[..4], head, "{}", site.name);
        }
        // The query site is followed by the call it feeds: bl 0x3763840.
        let bl = u32::from_le_bytes([
            CURSOR_QUERY.guard[4],
            CURSOR_QUERY.guard[5],
            CURSOR_QUERY.guard[6],
            CURSOR_QUERY.guard[7],
        ]);
        let imm = ((bl & 0x03ff_ffff) as i32) << 6 >> 6;
        let target = (CURSOR_QUERY_OFFSET as i64 + 4 + i64::from(imm) * 4) as usize;
        assert_eq!(bl >> 26, 0b100101);
        assert_eq!(target, INPUT_QUERY.offset);
    }

    const PRESS: u64 = BUTTON_L3 | BUTTON_RIGHT;

    #[test]
    fn steps_fire_on_press_then_repeat() {
        let mut r = Repeater::new();
        let t = 1_000;
        assert_eq!(r.update(BUTTON_RIGHT, t), None, "no L3, no step");
        assert_eq!(r.update(BUTTON_L3, t), None);
        assert_eq!(r.update(PRESS, t + 1), Some(Dir::Right));
        assert_eq!(r.update(PRESS, t + 2), None);
        assert_eq!(r.update(PRESS, t + REPEAT_TICKS), None);
        assert_eq!(r.update(PRESS, t + 1 + REPEAT_TICKS), Some(Dir::Right));
        assert_eq!(r.update(PRESS, t + REPEAT_TICKS + 2), None);
        assert_eq!(r.update(PRESS, t + 1 + 2 * REPEAT_TICKS), Some(Dir::Right));
        // A new direction steps at once; a diagonal does nothing.
        assert_eq!(r.update(BUTTON_L3 | BUTTON_UP, t + 5), Some(Dir::Up));
        assert_eq!(r.update(BUTTON_L3 | BUTTON_UP | BUTTON_LEFT, t + 6), None);
        // Releasing L3 resets; pressing it again with the D-pad held steps.
        assert_eq!(r.update(BUTTON_DOWN, t + 7), None);
        assert_eq!(r.update(BUTTON_L3 | BUTTON_DOWN, t + 8), Some(Dir::Down));
        assert_eq!(Dir::from_buttons(BUTTON_UP | BUTTON_DOWN), None);
    }

    /// A CSS-like grid: rows of 13 tiles 120 wide at a 132 pitch, rows 105
    /// apart, and a centred last row of 9.
    fn css_grid() -> Vec<Rect> {
        let mut tiles = Vec::new();
        for row in 0..7 {
            let (first, count) = if row == 6 { (2, 9) } else { (0, 13) };
            for col in first..first + count {
                let (x, y) = ((col as f32 - 6.0) * 132.0, 345.0 - row as f32 * 105.0);
                tiles.push(Rect {
                    xmin: x - 60.0,
                    xmax: x + 60.0,
                    ymin: y - 48.0,
                    ymax: y + 48.0,
                });
            }
        }
        tiles
    }

    const SCREEN: Rect = Rect {
        xmin: -960.0,
        xmax: 960.0,
        ymin: -540.0,
        ymax: 540.0,
    };

    fn centre(tiles: &[Rect], i: usize) -> (f32, f32) {
        tiles[i].centre()
    }

    #[test]
    fn moves_one_tile_and_stops_at_edges() {
        let tiles = css_grid();
        let start = centre(&tiles, 14); // row 1, col 1
        let right = neighbour(&tiles[..], start, Dir::Right, &SCREEN);
        assert_eq!((right.from, right.to), (Some(14), Some(15)));
        assert_eq!(right.target, centre(&tiles, 15));
        assert_eq!(
            neighbour(&tiles[..], start, Dir::Left, &SCREEN).to,
            Some(13)
        );
        assert_eq!(neighbour(&tiles[..], start, Dir::Up, &SCREEN).to, Some(1));
        assert_eq!(
            neighbour(&tiles[..], start, Dir::Down, &SCREEN).to,
            Some(27)
        );
        // From anywhere on a tile, not just its centre.
        let corner = (tiles[14].xmax - 1.0, tiles[14].ymin + 1.0);
        assert_eq!(
            neighbour(&tiles[..], corner, Dir::Right, &SCREEN).to,
            Some(15)
        );
        // No wrap at the row ends or the top.
        let edge = neighbour(&tiles[..], centre(&tiles, 12), Dir::Right, &SCREEN);
        assert_eq!((edge.from, edge.to), (Some(12), None));
        assert_eq!(edge.target, centre(&tiles, 12));
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 0), Dir::Up, &SCREEN).to,
            None
        );
        // Into and out of the short last row: nearest in x.
        let last_row = 78;
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 65), Dir::Down, &SCREEN).to,
            Some(last_row)
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 66), Dir::Down, &SCREEN).to,
            Some(last_row)
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 67), Dir::Down, &SCREEN).to,
            Some(last_row)
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, last_row), Dir::Up, &SCREEN).to,
            Some(67)
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, last_row), Dir::Down, &SCREEN).to,
            None
        );
    }

    #[test]
    fn repeated_steps_walk_the_row() {
        let tiles = css_grid();
        let mut point = centre(&tiles, 13);
        let mut visited = Vec::new();
        for _ in 0..20 {
            let step = neighbour(&tiles[..], point, Dir::Right, &SCREEN);
            if step.to.is_none() {
                break;
            }
            visited.push(step.to.unwrap());
            point = step.target;
        }
        assert_eq!(visited, (14..26).collect::<Vec<_>>());
    }

    #[test]
    fn off_tile_snaps_to_the_nearest_tile_ahead() {
        let tiles = css_grid();
        // Below the grid, near column 6 (the P1 panel area).
        let below = (10.0, -400.0);
        let up = neighbour(&tiles[..], below, Dir::Up, &SCREEN);
        assert_eq!(up.from, None);
        assert_eq!(up.to, Some(82)); // last row, column 6
        assert_eq!(neighbour(&tiles[..], below, Dir::Down, &SCREEN).to, None);
        // Between two tiles of a row (the 12-unit gap).
        let gap = (tiles[14].xmax + 6.0, tiles[14].centre().1);
        assert_eq!(neighbour(&tiles[..], gap, Dir::Right, &SCREEN).to, Some(15));
        assert_eq!(neighbour(&tiles[..], gap, Dir::Left, &SCREEN).to, Some(14));
    }

    #[test]
    fn skips_tiles_outside_the_hand_bounds_and_empty_rects() {
        let mut tiles = css_grid();
        // A stacked or hidden tile with no size, and one scrolled off screen.
        tiles.push(Rect::default());
        let off = Rect {
            xmin: 1000.0,
            xmax: 1120.0,
            ymin: 297.0,
            ymax: 393.0,
        };
        tiles.push(off);
        let step = neighbour(&tiles[..], centre(&tiles, 12), Dir::Right, &SCREEN);
        assert_eq!(step.to, None);
        let wide = Rect {
            xmax: 2000.0,
            ..SCREEN
        };
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 12), Dir::Right, &wide).to,
            Some(tiles.len() - 1)
        );
        // Targets stay inside the clamp rectangle.
        let tight = Rect {
            xmin: -100.0,
            xmax: 100.0,
            ..SCREEN
        };
        let step = neighbour(&tiles[..], centre(&tiles, 6), Dir::Up, &tight);
        assert_eq!(step.to, None);
    }

    #[test]
    fn stage_grid_with_unequal_rows() {
        // Four columns of 420 x 180 and a last row of two, left aligned.
        let mut tiles = Vec::new();
        for cell in 0..10 {
            let (x, y) = (
                (cell % 4) as f32 * 460.0 - 690.0,
                300.0 - (cell / 4) as f32 * 200.0,
            );
            tiles.push(Rect {
                xmin: x - 210.0,
                xmax: x + 210.0,
                ymin: y - 90.0,
                ymax: y + 90.0,
            });
        }
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 7), Dir::Down, &SCREEN).to,
            Some(9)
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 9), Dir::Right, &SCREEN).to,
            None
        );
        assert_eq!(
            neighbour(&tiles[..], centre(&tiles, 9), Dir::Up, &SCREEN).to,
            Some(5)
        );
    }

    #[test]
    fn lines_read_back() {
        let tiles = css_grid();
        let step = neighbour(&tiles[..], centre(&tiles, 14), Dir::Right, &SCREEN);
        let (a, b) = pack_step(Screen::Css, Dir::Right, &step);
        let mut line = Line::<128>::new();
        describe(
            &RawEvent {
                tick: 7,
                code: EV_STEP,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "7 step screen=css dir=right from=14 to=15 x=-528.0 y=240.0"
        );
        line.clear();
        let edge = neighbour(&tiles[..], (0.0, -400.0), Dir::Down, &SCREEN);
        let (a, b) = pack_step(Screen::Sss, Dir::Down, &edge);
        describe(
            &RawEvent {
                tick: 8,
                code: EV_STEP,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "8 step screen=sss dir=down from=none to=none x=0.0 y=-400.0"
        );
        line.clear();
        let (a, b) = pack_screen(Screen::Sss, 117, 0);
        describe(
            &RawEvent {
                tick: 9,
                code: EV_SCREEN,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "9 screen=sss tiles=117 heap_allocations=0"
        );
        line.clear();
        let (a, b) = pack_settled(Screen::Css, Some(15), Some(15), (-528.0, 240.0));
        describe(
            &RawEvent {
                tick: 10,
                code: EV_SETTLED,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "10 settled screen=css at=15 hovered=15 x=-528.0 y=240.0"
        );
    }

    #[test]
    fn paths_end_in_nul() {
        let mut path = Line::<8>::new();
        write!(path, "sd:/a").unwrap();
        assert_eq!(path.with_nul(), Some(&b"sd:/a\0"[..]));
        write!(path, "bcdef").unwrap();
        assert_eq!(path.with_nul(), None);
    }
}
