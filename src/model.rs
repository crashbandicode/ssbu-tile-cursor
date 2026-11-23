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
//!
//! On the CSS a tile step needs P1's hand to carry a token. The observed
//! Training owner can also step an empty hand between game-offered coins.

use core::fmt;

#[path = "cpu_guards.rs"]
pub mod cpu_guards;

pub const SUPPORTED_VERSION: &str = "13.0.5";
pub const CSS_TAKE_OFFSET: usize = 0x1a14664;
pub use cpu_guards::TAKE_SITES;

/// A 13.0.5 `.text` offset and the 64 bytes that must be there.
pub struct Site {
    pub name: &'static str,
    pub offset: usize,
    pub guard: [u8; 64],
}

/// CSS players update `fn(css)` (`.text+0x1a2b570`): its `mov x28, x0` after
/// the prologue (x0 = css). The CSS scene task calls it every frame (vtable
/// slot `0x50a8b20` -> `0x1a31860`, css = task `+0x80`) in every state of
/// the CSS state machine, and it runs every hand's own update (`0x32d2ba0`).
/// The fighter-picking state `0x1a12f60` is only one of those states.
pub const CSS_PLAYERS_OFFSET: usize = 0x1a2b5a4;
pub const CSS_PLAYERS: Site = Site {
    name: "css_players",
    offset: CSS_PLAYERS_OFFSET,
    guard: [
        0xfc, 0x03, 0x00, 0xaa, 0x68, 0x00, 0x00, 0x34, 0xe9, 0x03, 0x00, 0x32, 0x04, 0x00, 0x00,
        0x14, 0x89, 0x27, 0x7e, 0x39, 0x3f, 0x01, 0x00, 0x71, 0xe9, 0x07, 0x9f, 0x1a, 0xe9, 0x37,
        0x00, 0xb9, 0x09, 0x00, 0x80, 0x12, 0x89, 0x6b, 0x0f, 0xb9, 0x89, 0x1f, 0x41, 0xf9, 0x8a,
        0x23, 0x41, 0xf9, 0x3f, 0x01, 0x0a, 0xeb, 0x60, 0x01, 0x00, 0x54, 0x28, 0x05, 0x40, 0xf9,
        0x29, 0x41, 0x00, 0x91,
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

/// Stage Select owner continuation. Both first accessory-button paths join
/// here after the Lua update, with x19 still holding the live Stage Select
/// owner. This is outside the Instant Back script-call guard and inline sites.
pub const SSS_OBSERVE_OFFSET: usize = 0x1b29e10;

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

/// Not hooked: the player constructor storing the player index at `+0x28`
/// (`str w8, [x21, #0x28]`) and -1 in the token fields `+0x2c`/`+0x30`.
pub const CSS_PLAYER_INDEX: Site = Site {
    name: "css_player_index",
    offset: 0x1a0e528,
    guard: [
        0x88, 0x02, 0x40, 0xb9, 0x09, 0x00, 0x80, 0x92, 0xa9, 0xc2, 0x02, 0xf8, 0xe9, 0x03, 0x10,
        0x32, 0xa9, 0x36, 0x00, 0xb9, 0xe2, 0x03, 0x00, 0x91, 0xa8, 0x2a, 0x00, 0xb9, 0xe8, 0x03,
        0x00, 0xb9, 0x88, 0x06, 0x40, 0xb9, 0xe8, 0x07, 0x00, 0xb9, 0x88, 0xa2, 0x40, 0x39, 0xe8,
        0x23, 0x00, 0x39, 0xa0, 0x12, 0x40, 0xf9, 0xe1, 0x03, 0x16, 0xaa, 0xb8, 0x10, 0x63, 0x94,
        0xa1, 0x2a, 0x40, 0xb9,
    ],
};

/// Not hooked: a hand picking a token up sets the hand's holding flag
/// (`strb w9, [x8, #0x167]`) and the player's carried slot (`str w21, [x8,
/// #0x30]`) together.
pub const CSS_GRAB: Site = Site {
    name: "css_grab",
    offset: 0x1a14740,
    guard: [
        0x88, 0x06, 0x40, 0xf9, 0x08, 0x11, 0x40, 0xf9, 0xe9, 0x03, 0x00, 0x32, 0x09, 0x9d, 0x05,
        0x39, 0x88, 0x06, 0x40, 0xf9, 0x15, 0x31, 0x00, 0xb9, 0x88, 0x06, 0x40, 0xf9, 0x08, 0x09,
        0x40, 0xf9, 0x16, 0x05, 0x40, 0xf9, 0xc0, 0x62, 0x06, 0x91, 0xe1, 0x03, 0x1f, 0x2a, 0x82,
        0x4c, 0x80, 0x52, 0xcc, 0xae, 0x7e, 0x94, 0xc8, 0x32, 0x10, 0x91, 0x1f, 0x01, 0x00, 0xf9,
        0xc8, 0x92, 0x10, 0x91,
    ],
};

/// Not hooked: the fighter-picking update reads the carried slot (`ldr w28,
/// [x8, #0x30]`), the active flag (`+0x36`), the hand (`+0x20`) and its
/// cursor (`+0x100`) before hit-testing.
pub const CSS_CARRIED: Site = Site {
    name: "css_carried",
    offset: 0x1a13078,
    guard: [
        0x88, 0x06, 0x40, 0xf9, 0x19, 0x09, 0x40, 0xf9, 0x29, 0x07, 0x40, 0xf9, 0x1c, 0x31, 0x40,
        0xb9, 0x96, 0x7f, 0x40, 0x93, 0x2a, 0x85, 0x44, 0xb9, 0x8a, 0x00, 0x00, 0x35, 0x29, 0x89,
        0x44, 0xb9, 0x3f, 0x05, 0x00, 0x31, 0x80, 0x29, 0x00, 0x54, 0x09, 0xd9, 0x40, 0x39, 0xc9,
        0x12, 0x00, 0x34, 0x09, 0x11, 0x40, 0xf9, 0x29, 0x81, 0x40, 0xf9, 0x20, 0x05, 0x43, 0xad,
        0x01, 0xe4, 0x21, 0x4e,
    ],
};

/// Not hooked: with no carried token (-1) the update skips the hover; else it
/// compares the hit tile with the record `[css+0xdd0] + 0xe0 + slot * 0x18`
/// of the carried token's slot.
pub const CSS_HOVER_SLOT: Site = Site {
    name: "css_hover_slot",
    offset: 0x1a13148,
    guard: [
        0xf8, 0x03, 0x05, 0x2a, 0x9f, 0x07, 0x00, 0x31, 0x40, 0xa3, 0x00, 0x54, 0x9f, 0x23, 0x00,
        0x71, 0x42, 0xe7, 0x01, 0x54, 0x60, 0xea, 0x46, 0xf9, 0xe9, 0x07, 0x1d, 0x32, 0xc9, 0x02,
        0x09, 0x9b, 0x29, 0xe1, 0x40, 0xb9, 0x1f, 0x03, 0x09, 0x6b, 0x80, 0x0c, 0x00, 0x54, 0x69,
        0x2e, 0x41, 0xf9, 0x68, 0x2a, 0x41, 0xf9, 0x29, 0x01, 0x08, 0xcb, 0x29, 0xfd, 0x44, 0x93,
        0x3f, 0x01, 0x16, 0xeb,
    ],
};

pub const SITES: [&Site; 10] = [
    &CSS_PLAYERS,
    &SSS_UPDATE,
    &CURSOR_QUERY,
    &INPUT_QUERY,
    &CSS_HIT_TEST,
    &SSS_HIT_TEST,
    &CSS_PLAYER_INDEX,
    &CSS_GRAB,
    &CSS_CARRIED,
    &CSS_HOVER_SLOT,
];

/// The hand (0x170-byte object, constructor `.text+0x32d2400`).
pub mod hand {
    /// The cursor created by the hand's setup (`.text+0x32d2840`).
    pub const CURSOR: usize = 0x100;
    /// Non-zero while the hand is shown and enabled (`set_hand_enable`).
    pub const ENABLED: usize = 0x163;
    pub const AREA: usize = 0x148;
    pub const SHOWN: usize = 0x162;
    pub const HOVERING: usize = 0x166;
    /// Non-zero while the hand holds a token (set and cleared together with
    /// the player's carried slot).
    pub const HOLDING: usize = 0x167;
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
    /// Player object: its hand, its index (0 = P1), the slot of the token
    /// its hand carries (-1 for none) and the "hand active" flag the update
    /// checks before hit-testing.
    pub const PLAYER_HAND: usize = 0x20;
    pub const PLAYER_INDEX: usize = 0x28;
    pub const PLAYER_HOVER_TOKEN: usize = 0x2c;
    pub const PLAYER_CARRIED: usize = 0x30;
    pub const PLAYER_ACTIVE: usize = 0x36;
    /// Player and token slots.
    pub const SLOTS: usize = 8;
    pub const MEDALS: usize = 0xdf0;
    pub const MEDAL_STRIDE: usize = 0x130;
    pub const MEDAL_COUNT: usize = 0xbe0;
    /// Fighter tiles: an object whose `+0`/`+8` are a vector's begin/end.
    pub const TILES: usize = 0xdd0;
    pub const TILE_STRIDE: usize = 0xb0;
    pub const TILE_RECT: usize = 0x48;
    /// The hovered tile index per token slot: `+0xe0 + slot * 0x18`.
    pub const HOVERED: usize = 0xe0;
    pub const HOVERED_STRIDE: usize = 0x18;

    /// The slot of the token a hand carries, from the player's carried slot
    /// and the hand's holding flag; `None` unless both say it holds one.
    pub fn carried(slot: i32, holding: u8) -> Option<usize> {
        usize::try_from(slot)
            .ok()
            .filter(|&slot| slot < SLOTS && holding != 0)
    }

    /// Where, from the tiles object, the game records the hovered tile for a
    /// token slot.
    pub fn hover_record(slot: usize) -> Option<usize> {
        (slot < SLOTS).then(|| HOVERED + slot * HOVERED_STRIDE)
    }
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
pub const BUTTON_B: u64 = 1 << 1;
pub const BUTTON_L3: u64 = 1 << 4;
pub const BUTTON_LEFT: u64 = 1 << 12;
pub const BUTTON_UP: u64 = 1 << 13;
pub const BUTTON_RIGHT: u64 = 1 << 14;
pub const BUTTON_DOWN: u64 = 1 << 15;

/// The CPU's coin slot on Training's two-seat fighter select.
pub const CPU_SLOT: usize = 1;

/// L3 + B presses: B going down while L3 is down, on the newest pad sample of each fighter-select update.
/// The first update only primes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TakePress {
    primed: bool,
    b_down: bool,
}

impl TakePress {
    pub const fn new() -> Self {
        Self { primed: false, b_down: false }
    }

    pub fn update(&mut self, buttons: u64) -> bool {
        let b_down = buttons & BUTTON_B != 0;
        let press = self.primed && b_down && !self.b_down && buttons & BUTTON_L3 != 0;
        *self = Self { primed: true, b_down };
        press
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChordB {
    hidden: bool,
}

impl ChordB {
    pub const fn new() -> Self {
        Self { hidden: false }
    }

    pub fn hidden(&self) -> bool {
        self.hidden
    }

    pub fn update(&mut self, _scope: bool, _buttons: u64, _held: bool) -> bool {
        false
    }
}

pub const TICKS_PER_SECOND: u64 = 19_200_000;
/// The first step fires on the press; held, it repeats every 15 subsequent
/// screen updates (250 ms at 60 fps), independent of host wall time.
pub const REPEAT_FRAMES: u8 = 15;

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
/// single D-pad direction are both held, then one every `REPEAT_FRAMES` updates while
/// they stay held. Releasing L3, changing or releasing the direction resets.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Repeater {
    held: Option<Dir>,
    remaining: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Scan {
    pub first: Option<Dir>,
    pub later: [u8; 4],
    pub repeat: Option<Dir>,
}

impl Repeater {
    pub const fn new() -> Self {
        Self {
            held: None,
            remaining: 0,
        }
    }

    /// Called exactly once per CSS/SSS frame by the screen hook.
    pub fn update(&mut self, buttons: u64) -> Option<Dir> {
        let scan = self.scan(&[buttons]);
        scan.first.or(scan.repeat)
    }

    pub fn scan(&mut self, samples: &[u64]) -> Scan {
        let mut scan = Scan::default();
        for &buttons in samples {
            let dir = if buttons & BUTTON_L3 != 0 {
                Dir::from_buttons(buttons)
            } else {
                None
            };
            if dir != self.held {
                self.held = dir;
                self.remaining = REPEAT_FRAMES;
                if let Some(dir) = dir {
                    if scan.first.is_none() {
                        scan.first = Some(dir);
                    } else {
                        let count = &mut scan.later[dir as usize];
                        *count = count.saturating_add(1);
                    }
                }
            }
        }
        if scan.first.is_none() && self.held.is_some() {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.remaining = REPEAT_FRAMES;
                scan.repeat = self.held;
            }
        }
        scan
    }
}

pub const PAD_RING: usize = 16;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PadTracker {
    last: Option<i64>,
}

impl PadTracker {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn unseen(&self, ring: &[(i64, u64)], out: &mut [(i64, u64); PAD_RING]) -> usize {
        let Some(&newest) = ring.first() else {
            return 0;
        };
        let Some(last) = self.last else {
            out[0] = newest;
            return 1;
        };
        let mut valid = 1;
        while valid < ring.len().min(PAD_RING) && ring[valid].0 < ring[valid - 1].0 {
            valid += 1;
        }
        let mut count = 0;
        for &sample in ring[..valid].iter().rev() {
            if sample.0 > last {
                out[count] = sample;
                count += 1;
            }
        }
        count
    }

    pub fn consume(&mut self, batch: &[(i64, u64)], used: usize) {
        if used > 0 {
            self.last = Some(batch[used - 1].0);
        }
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

pub type PadOwner = (Screen, u64, u64);
pub type PadSource = (u32, u32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PadFrame {
    pub step: Option<Dir>,
    pub blocked: [u8; 4],
}

pub fn empty_hand(mut frame: PadFrame) -> PadFrame {
    if let Some(dir @ (Dir::Up | Dir::Down)) = frame.step {
        frame.step = None;
        let count = &mut frame.blocked[dir as usize];
        *count = count.saturating_add(1);
    }
    frame
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Prev {
    Gap,
    Steppable,
    Blocked,
}

/// Pad history is read only between consecutive callbacks of the same hand (screen, picker
/// object, cursor) on the same pad source; after a gap, a new hand, a pad change, or on the
/// first callback in which the hand may step again, only the newest sample counts. Each update
/// examines every sample it takes exactly once and every press among them gets exactly one
/// outcome in that same update. The first press of an update in which the hand may step is
/// the step. Every other press is counted as blocked: the hand may not step, or this update
/// already stepped. So two taps within one update give one step plus one blocked, never a
/// deferred second step. Nothing is carried to a later update. A hold frame is counted at
/// most once per update, and only in an update that brought no press, so a hold repeat comes
/// 15 updates after its press for steps and for blocked alike.
pub struct PadInput {
    owner: Option<PadOwner>,
    source: Option<PadSource>,
    prev: Prev,
    tracker: PadTracker,
    steps: Repeater,
    blocked: Repeater,
}

impl Default for PadInput {
    fn default() -> Self {
        Self::new()
    }
}

impl PadInput {
    pub const fn new() -> Self {
        Self {
            owner: None,
            source: None,
            prev: Prev::Gap,
            tracker: PadTracker::new(),
            steps: Repeater::new(),
            blocked: Repeater::new(),
        }
    }

    pub fn gap(&mut self) {
        self.tracker.reset();
        self.prev = Prev::Gap;
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn frame(
        &mut self,
        owner: PadOwner,
        steppable: bool,
        ring: Option<(PadSource, &[(i64, u64)])>,
        newest: impl FnOnce() -> u64,
    ) -> PadFrame {
        let ring = ring.filter(|(_, samples)| !samples.is_empty());
        if self.owner.map(|(screen, object, _)| (screen, object)) != Some((owner.0, owner.1)) {
            self.reset();
        } else if self.owner != Some(owner) {
            self.gap();
        }
        self.owner = Some(owner);
        let history = ring.is_some_and(|(source, _)| {
            self.source == Some(source) && self.tracker.last.is_some() && self.prev != Prev::Gap
        });
        let mut batch = [(0, 0); PAD_RING];
        let count = if let Some((source, samples)) = ring {
            self.source = Some(source);
            if history && (!steppable || self.prev == Prev::Steppable) {
                self.tracker.unseen(samples, &mut batch)
            } else {
                batch[0] = samples[0];
                1
            }
        } else {
            self.tracker.reset();
            self.source = None;
            batch[0] = (0, newest());
            1
        };
        let mut buttons = [0; PAD_RING];
        for (out, &(_, value)) in buttons.iter_mut().zip(&batch[..count]) {
            *out = value;
        }
        let frame = if steppable {
            self.blocked = Repeater::new();
            let scan = self.steps.scan(&buttons[..count]);
            self.prev = Prev::Steppable;
            PadFrame { step: scan.first.or(scan.repeat), blocked: scan.later }
        } else {
            self.steps = Repeater::new();
            let scan = self.blocked.scan(&buttons[..count]);
            let mut counts = scan.later;
            for dir in [scan.first, scan.repeat].into_iter().flatten() {
                let count = &mut counts[dir as usize];
                *count = count.saturating_add(1);
            }
            self.prev = Prev::Blocked;
            PadFrame { step: None, blocked: counts }
        };
        if ring.is_some() {
            self.tracker.consume(&batch[..count], count);
        }
        frame
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

pub struct Coins {
    pub pointed: Option<usize>,
    pub at: [Option<(f32, f32)>; css::SLOTS],
}

/// What the game's B handler does for one player's B press with an empty hand (`.text+0x1a14664`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Take {
    /// Not P1's L3 + B on Training's two-seat fighter select: the game's own B.
    Native,
    /// L3 + B there, but the game does not offer the CPU's coin now: B does nothing.
    Nothing,
    /// L3 + B there: the game's own pick-up runs for this coin slot.
    Coin(usize),
}

/// `l3`: L3 is down on P1's pad. `player`: the index of the player whose B press the game is handling.
/// `coins`: the coins the game offers P1's empty hand on Training's two-seat fighter select; `None` anywhere
/// else (another select, a coin in the hand, a hidden or disabled hand).
pub fn take(l3: bool, player: i32, coins: Option<&Coins>) -> Take {
    match coins {
        Some(coins) if l3 && player == 0 => {
            if coins.at[CPU_SLOT].is_some() { Take::Coin(CPU_SLOT) } else { Take::Nothing }
        }
        _ => Take::Native,
    }
}

/// The coin slot and own-coin gate in w21 and w9.
pub fn take_registers(take: Take, pointed: u32, own: u32) -> (u32, u32) {
    match take {
        Take::Native => (pointed, own),
        Take::Nothing => (u32::MAX, 0),
        Take::Coin(slot) => (slot as u32, own),
    }
}

pub struct CoinFacts {
    pub flags: [u8; 8],
    pub rest: [(f32, f32); 2],
    pub pane: Option<(f32, f32)>,
}

pub fn coin_offer(slot: usize, facts: &CoinFacts, area: Rect) -> Option<(f32, f32)> {
    let flags = facts.flags;
    let position = facts.pane?;
    (slot < 2
        && flags[0] != 0
        && flags[1] != 0
        && flags[3] != 0
        && flags[4] != 0
        && flags[7] == 0
        && (slot == 0 || flags[5] == 0)
        && facts.rest[0] == facts.rest[1]
        && position == facts.rest[0]
        && area.contains(position.0, position.1))
        .then_some(position)
}

pub fn token_target(coins: &Coins, dir: Dir) -> Option<usize> {
    match dir {
        Dir::Right => (0..coins.at.len())
            .find(|&slot| coins.at[slot].is_some() && coins.pointed.is_none_or(|p| slot > p)),
        Dir::Left => (0..coins.at.len())
            .rev()
            .find(|&slot| coins.at[slot].is_some() && coins.pointed.is_none_or(|p| slot < p)),
        Dir::Up | Dir::Down => None,
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
pub const EV_BLOCKED: u8 = 4;
pub const EV_TOKEN: u8 = 5;
pub const EV_TAKE: u8 = 6;

pub fn pack_take(screen: Screen, slot: usize) -> (u64, u64) {
    (screen.code() | pack_index(Some(slot)) << 48, 0)
}

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

/// A step, with the slot of the token the hand carries (CSS only).
pub fn pack_step(screen: Screen, dir: Dir, step: &Step, token: Option<usize>) -> (u64, u64) {
    (
        screen.code()
            | u64::from(dir.code()) << 8
            | pack_index(step.from) << 16
            | pack_index(step.to) << 32
            | pack_index(token) << 48,
        pack_point(step.target),
    )
}

pub fn pack_token(
    screen: Screen,
    dir: Dir,
    from: Option<usize>,
    to: Option<usize>,
    coin: Option<usize>,
    point: (f32, f32),
) -> (u64, u64) {
    pack_step(screen, dir, &Step { from, to, target: point }, coin)
}

pub fn pack_blocked(screen: Screen, counts: [u8; 4]) -> (u64, u64) {
    let mut value = screen.code();
    for (index, count) in counts.into_iter().enumerate() {
        value |= u64::from(count) << (8 * (index + 1));
    }
    (value, 0)
}

pub fn unpack_blocked(value: u64) -> [u8; 4] {
    [(value >> 8) as u8, (value >> 16) as u8, (value >> 24) as u8, (value >> 32) as u8]
}

/// ` token=p<n>` for a carried token's slot (`p1` = P1's own); nothing when
/// there is none.
struct Token(u64);

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == NONE16 {
            Ok(())
        } else {
            write!(f, " token=p{}", self.0 + 1)
        }
    }
}

/// The hand stopped after a step: the tile it is on by the game's rectangles
/// and, on the CSS, the hovered tile the game itself recorded for the slot of
/// the carried token.
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
                "step screen={screen} dir={dir} from={} to={}{} x={x:.1} y={y:.1}",
                Index(a >> 16 & 0xffff),
                Index(a >> 32 & 0xffff),
                Token(a >> 48 & 0xffff)
            )
        }
        EV_TOKEN => {
            let (x, y) = unpack_point(b);
            let dir = Dir::from_code((a >> 8 & 0xff) as u8).map_or("?", Dir::name);
            let coin = a >> 48 & 0xffff;
            write!(
                out,
                "token screen={screen} dir={dir} from={} to={} coin=",
                Index(a >> 16 & 0xffff),
                Index(a >> 32 & 0xffff)
            )?;
            if coin == NONE16 {
                write!(out, "none")?;
            } else {
                write!(out, "p{}", coin + 1)?;
            }
            write!(out, " x={x:.1} y={y:.1}")
        }
        EV_TAKE => write!(out, "take screen={screen}{}", Token(a >> 48 & 0xffff)),
        EV_BLOCKED => {
            let counts = unpack_blocked(a);
            let directions = counts.iter().filter(|&&count| count != 0).count();
            let total: u16 = counts.iter().map(|&count| u16::from(count)).sum();
            if directions == 1 {
                let index = counts.iter().position(|&count| count != 0).unwrap();
                write!(out, "blocked screen={screen} dir={} presses={total}", Dir::ALL[index].name()) // restriction: TC-BLOCKED-LOG (logs an existing rule; adds no restriction)
            } else {
                write!(out, "blocked screen={screen} dir=mixed presses={total}")?; // restriction: TC-BLOCKED-LOG (logs an existing rule; adds no restriction)
                for (dir, count) in Dir::ALL.into_iter().zip(counts) {
                    if count != 0 {
                        write!(out, " {}={count}", dir.name())?;
                    }
                }
                Ok(())
            }
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
#[path = "take_tests.rs"]
pub(crate) mod take_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write as _;

    #[test]
    fn guards_start_with_the_hooked_instructions() {
        // mov x28,x0 / mov x19,x0 / mov x0,x20 /
        // sub sp,sp,#0x40 / ldr x9,[x19,#0xdd0] / ldp x9,x10,[x0,#0x168] /
        // ldr w8,[x20] / ldr x8,[x20,#8] / ldr x8,[x20,#8] / mov w24,w5
        let heads = [
            [0xfc, 0x03, 0x00, 0xaa],
            [0xf3, 0x03, 0x00, 0xaa],
            [0xe0, 0x03, 0x14, 0xaa],
            [0xff, 0x03, 0x01, 0xd1],
            [0x69, 0xea, 0x46, 0xf9],
            [0x09, 0xa8, 0x56, 0xa9],
            [0x88, 0x02, 0x40, 0xb9],
            [0x88, 0x06, 0x40, 0xf9],
            [0x88, 0x06, 0x40, 0xf9],
            [0xf8, 0x03, 0x05, 0x2a],
        ];
        for (site, head) in SITES.iter().zip(heads) {
            assert_eq!(site.guard[..4], head, "{}", site.name);
        }
        let optional = [
            &cpu_guards::CSS_MEDAL_COUNT,
            &cpu_guards::CSS_HOVER_HAND_AREA,
            &cpu_guards::CSS_HOVER_MEDAL_FLAGS,
            &cpu_guards::CSS_HOVER_MEDAL_POSITION,
            &cpu_guards::CSS_HOVER_TOKEN_WRITE,
            &cpu_guards::CSS_GRAB_HOVER_TOKEN,
        ];
        let heads = include_bytes!("layouts/cpu-guard-heads.bin");
        for (site, head) in optional.into_iter().zip(heads.as_chunks::<8>().0) {
            assert_eq!(&site.guard[..8], head, "{}", site.name);
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

    /// Instruction `index` of a guard.
    fn word(site: &Site, index: usize) -> u32 {
        let at = index * 4;
        u32::from_le_bytes(site.guard[at..at + 4].try_into().unwrap())
    }

    /// The byte offset of an unsigned-offset LDR/STR (any size).
    fn load_store_offset(word: u32) -> usize {
        assert_eq!(
            word & 0x3b00_0000,
            0x3900_0000,
            "not LDR/STR imm: {word:#x}"
        );
        (((word >> 10) & 0xfff) << (word >> 30)) as usize
    }

    #[test]
    fn the_take_sites_hold_the_instructions_the_hook_relies_on() {
        use cpu_guards::{CSS_TAKE, CSS_TAKE_GATE, CSS_TAKE_OWN, CSS_TAKE_PRESS};
        assert_eq!(word(&CSS_TAKE, 0), 0x3100_06bf);
        assert_eq!(load_store_offset(word(&CSS_TAKE_PRESS, 1)), 0x1ac);
        assert_eq!(load_store_offset(word(&CSS_TAKE_PRESS, 3)), css::PLAYER_HOVER_TOKEN);
        assert_eq!(word(&CSS_TAKE_PRESS, 4), word(&CSS_TAKE, 0));
        assert_eq!(load_store_offset(word(&CSS_TAKE_OWN, 1)), css::PLAYER_INDEX);
        assert_eq!(load_store_offset(word(&CSS_TAKE_OWN, 3)), 0xf68);
        assert_eq!(load_store_offset(word(&CSS_TAKE_GATE, 0)), css::PLAYER_INDEX);
        assert_eq!(load_store_offset(word(&CSS_TAKE_GATE, 3)), css::MEDALS);
        assert_eq!(word(&CSS_TAKE_GATE, 4), 0x5280_260c);
        assert_eq!(load_store_offset(word(&CSS_TAKE_GATE, 6)), 0x11b);
    }

    #[test]
    fn witnesses_use_the_offsets_the_plugin_reads() {
        // The players update walks css+0x238 (ldr x9, [x28, #0x238]).
        assert_eq!(load_store_offset(word(&CSS_PLAYERS, 10)), css::PLAYERS);
        // The constructor stores the player index (str w8, [x21, #0x28]).
        assert_eq!(
            load_store_offset(word(&CSS_PLAYER_INDEX, 6)),
            css::PLAYER_INDEX
        );
        // Grabbing sets the holding flag and the carried slot together.
        assert_eq!(load_store_offset(word(&CSS_GRAB, 1)), css::PLAYER_HAND);
        assert_eq!(load_store_offset(word(&CSS_GRAB, 3)), hand::HOLDING);
        assert_eq!(load_store_offset(word(&CSS_GRAB, 5)), css::PLAYER_CARRIED);
        // The fighter-picking update reads the carried slot, the active flag,
        // the hand and its cursor.
        assert_eq!(
            load_store_offset(word(&CSS_CARRIED, 3)),
            css::PLAYER_CARRIED
        );
        assert_eq!(
            load_store_offset(word(&CSS_CARRIED, 10)),
            css::PLAYER_ACTIVE
        );
        assert_eq!(load_store_offset(word(&CSS_CARRIED, 12)), css::PLAYER_HAND);
        assert_eq!(load_store_offset(word(&CSS_CARRIED, 13)), hand::CURSOR);
        // ... and indexes the hover record by that slot.
        assert_eq!(load_store_offset(word(&CSS_HOVER_SLOT, 5)), css::TILES);
        assert_eq!(load_store_offset(word(&CSS_HOVER_SLOT, 8)), css::HOVERED);
    }

    #[test]
    fn only_a_held_token_counts_as_carried() {
        assert_eq!(css::carried(0, 1), Some(0), "P1's own token");
        assert_eq!(css::carried(1, 1), Some(1), "a CPU's token in slot 2");
        assert_eq!(css::carried(-1, 0), None, "empty hand");
        assert_eq!(css::carried(-1, 1), None);
        assert_eq!(css::carried(0, 0), None, "slot left over, not holding");
        assert_eq!(css::carried(7, 1), Some(7));
        assert_eq!(css::carried(8, 1), None);
        assert_eq!(css::hover_record(0), Some(0xe0));
        assert_eq!(css::hover_record(1), Some(0xf8));
        assert_eq!(css::hover_record(7), Some(0xe0 + 7 * 0x18));
        assert_eq!(css::hover_record(8), None);
    }

    const PRESS: u64 = BUTTON_L3 | BUTTON_RIGHT;

    const PAD_OWNER: PadOwner = (Screen::Css, 0x1000, 0x2000);
    const PAD_SOURCE: PadSource = (0, 1);

    #[test]
    fn fix4_r1_seen_presses_are_outcomes_before_turnover_or_gap() {
        let up = BUTTON_L3 | BUTTON_UP;
        let left = BUTTON_L3 | BUTTON_LEFT;
        let batch = [(104, 0), (103, left), (102, 0), (101, up), (100, 0)];
        let turned_over: Vec<_> = (105..=120).rev().map(|n| (n, 0)).collect();
        for gap in [false, true] {
            let mut input = PadInput::new();
            assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
            assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &batch),
                       PadFrame { step: Some(Dir::Up), blocked: [0, 0, 1, 0] });
            if gap {
                input.gap();
            }
            assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &turned_over), PadFrame::default());
        }
    }

    #[test]
    fn fix4_r2_press_and_trailing_samples_do_not_advance_hold() {
        let up = BUTTON_L3 | BUTTON_UP;
        for steppable in [false, true] {
            for trailing in [false, true] {
                let mut input = PadInput::new();
                assert_eq!(pad_frame(&mut input, PAD_OWNER, steppable, PAD_SOURCE, &[(100, 0)]),
                           PadFrame::default());
                let ring = if trailing { vec![(102, up), (101, up), (100, 0)] }
                           else { vec![(101, up), (100, 0)] };
                let expected = if steppable { step(Dir::Up) } else { blocked(Dir::Up, 1) };
                assert_eq!(pad_frame(&mut input, PAD_OWNER, steppable, PAD_SOURCE, &ring), expected);
                let newest = ring[0].0;
                for following in 1..=15 {
                    let outcome = pad_frame(&mut input, PAD_OWNER, steppable, PAD_SOURCE,
                                            &[(newest + following, up)]);
                    assert_eq!(outcome, if following == 15 { expected } else { PadFrame::default() },
                               "steppable={steppable} trailing={trailing} following={following}");
                }
            }
        }
    }

    #[test]
    fn fix4_r3_mixed_and_single_direction_blocked_records_round_trip() {
        let mut input = PadInput::new();
        let up = BUTTON_L3 | BUTTON_UP;
        let left = BUTTON_L3 | BUTTON_LEFT;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let mixed = [(104, 0), (103, left), (102, 0), (101, up), (100, 0)];
        let counts = pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &mixed).blocked;
        assert_eq!(counts, [1, 0, 1, 0]);
        let (a, b) = pack_blocked(Screen::Css, counts);
        assert_eq!(unpack_blocked(a), counts);
        let mut line = Line::<128>::new();
        describe(&RawEvent { tick: 7, code: EV_BLOCKED, a, b }, &mut line).unwrap();
        assert_eq!(core::str::from_utf8(line.as_bytes()).unwrap(),
                   "7 blocked screen=css dir=mixed presses=2 up=1 left=1");

        let repeated = [(107, up), (106, 0), (105, up), (104, 0)];
        let counts = pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &repeated).blocked;
        assert_eq!(counts, [2, 0, 0, 0]);
        let (a, b) = pack_blocked(Screen::Css, counts);
        assert_eq!(unpack_blocked(a), counts);
        let mut line = Line::<128>::new();
        describe(&RawEvent { tick: 7, code: EV_BLOCKED, a, b }, &mut line).unwrap();
        assert_eq!(core::str::from_utf8(line.as_bytes()).unwrap(),
                   "7 blocked screen=css dir=up presses=2");
    }

    fn fix4_generated_batches(with_gaps: bool) {
        fn draw(seed: &mut u64) -> u32 {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }
        fn chord(buttons: u64) -> Option<usize> {
            if buttons & BUTTON_L3 == 0 {
                return None;
            }
            match buttons & (BUTTON_UP | BUTTON_DOWN | BUTTON_LEFT | BUTTON_RIGHT) {
                BUTTON_UP => Some(0),
                BUTTON_DOWN => Some(1),
                BUTTON_LEFT => Some(2),
                BUTTON_RIGHT => Some(3),
                _ => None,
            }
        }
        let choices = [
            0, BUTTON_L3, BUTTON_L3 | BUTTON_UP, BUTTON_L3 | BUTTON_DOWN,
            BUTTON_L3 | BUTTON_LEFT, BUTTON_L3 | BUTTON_RIGHT, BUTTON_UP,
            BUTTON_L3 | BUTTON_UP | BUTTON_LEFT,
        ];
        let mut seed = 0x72ac_1043_5e67_981b;
        let mut input = PadInput::new();
        let mut owner = PAD_OWNER;
        let mut ring = vec![(100, 0)];
        let mut number = 100;
        let mut steps_held = None;
        let mut blocked_held = None;
        let mut previous_steppable = false;
        let mut history = false;
        let mut hold_age = 0;
        let mut sizes = [0usize; 7];
        let mut gaps = 0;
        let mut cursors = 0;
        for update in 0..6000 {
            if with_gaps && draw(&mut seed).is_multiple_of(19) {
                input.gap();
                history = false;
                gaps += 1;
            }
            if with_gaps && draw(&mut seed).is_multiple_of(23) {
                owner.2 ^= 1;
                history = false;
                cursors += 1;
            }
            let mut n = draw(&mut seed) as usize % 7;
            if update % 4 == 3 && n == 0 {
                n = 1;
            }
            sizes[n] += 1;
            let mut added = Vec::new();
            for index in 0..n {
                let mut buttons = choices[draw(&mut seed) as usize % choices.len()];
                if update % 4 == 3 && index + 1 == n {
                    buttons = 0;
                }
                number += 1;
                ring.insert(0, (number, buttons));
                ring.truncate(PAD_RING);
                added.push(buttons);
            }
            let steppable = !draw(&mut seed).is_multiple_of(3);
            let selected = if !history || (steppable && !previous_steppable) {
                vec![ring[0].1]
            } else {
                added
            };
            let held = if steppable {
                blocked_held = None;
                &mut steps_held
            } else {
                steps_held = None;
                &mut blocked_held
            };
            let mut presses = Vec::new();
            for buttons in selected {
                let next = chord(buttons);
                if next != *held {
                    *held = next;
                    if let Some(dir) = next {
                        presses.push(dir);
                    }
                }
            }
            hold_age = if presses.is_empty() && held.is_some() { hold_age + 1 } else { 0 };
            assert!(hold_age < usize::from(REPEAT_FRAMES), "generator held a chord too long");
            let expected_step = if steppable { presses.first().copied() } else { None };
            let mut expected_blocked = [0u8; 4];
            for &dir in presses.iter().skip(usize::from(expected_step.is_some())) {
                expected_blocked[dir] += 1;
            }
            let outcome = pad_frame(&mut input, owner, steppable, PAD_SOURCE, &ring);
            assert_eq!(outcome.step.map(|d| d as usize), expected_step, "step update {update}");
            assert_eq!(outcome.blocked, expected_blocked, "blocked update {update}");
            assert_eq!(u32::from(outcome.step.is_some()) + outcome.blocked.iter().map(|&n| u32::from(n)).sum::<u32>(),
                       presses.len() as u32, "outcome count update {update}");
            assert!(steppable || outcome.step.is_none());
            history = true;
            previous_steppable = steppable;
        }
        assert!(sizes.iter().all(|&count| count > 0), "{sizes:?}");
        if with_gaps {
            assert!(gaps > 0 && cursors > 0);
        }
    }

    #[test]
    fn fix4_r4_generated_batches_preserve_every_transition() {
        fix4_generated_batches(false);
    }

    #[test]
    fn fix4_r4_generated_batches_after_gaps_and_cursor_changes() {
        fix4_generated_batches(true);
    }

    #[test]
    fn fix4_scan_later_counts_saturate_per_direction() {
        let mut input = Vec::new();
        for _ in 0..300 {
            input.extend([BUTTON_L3 | BUTTON_UP, 0]);
        }
        let mut repeater = Repeater::new();
        assert_eq!(repeater.scan(&input),
                   Scan { first: Some(Dir::Up), later: [u8::MAX, 0, 0, 0], repeat: None });
        assert_eq!(repeater.scan(&[]), Scan::default());
    }

    #[test]
    fn fix4_mixed_blocked_total_does_not_saturate_at_one_byte() {
        let counts = [255, 255, 2, 1];
        let (a, b) = pack_blocked(Screen::Sss, counts);
        assert_eq!(unpack_blocked(a), counts);
        let mut line = Line::<128>::new();
        describe(&RawEvent { tick: 7, code: EV_BLOCKED, a, b }, &mut line).unwrap();
        assert_eq!(core::str::from_utf8(line.as_bytes()).unwrap(),
                   "7 blocked screen=sss dir=mixed presses=513 up=255 down=255 left=2 right=1");
    }

    fn step(dir: Dir) -> PadFrame {
        PadFrame { step: Some(dir), blocked: [0; 4] }
    }

    fn blocked(dir: Dir, count: u8) -> PadFrame {
        let mut counts = [0; 4];
        counts[dir as usize] = count;
        PadFrame { step: None, blocked: counts }
    }

    fn pad_frame(
        input: &mut PadInput,
        owner: PadOwner,
        steppable: bool,
        source: PadSource,
        ring: &[(i64, u64)],
    ) -> PadFrame {
        input.frame(owner, steppable, Some((source, ring)), || {
            panic!("unexpected single-sample fallback")
        })
    }

    fn assert_held_repeat_across(disruption: u8) {
        let mut input = PadInput::new();
        let mut owner = PAD_OWNER;
        let mut source = PAD_SOURCE;
        let chord = BUTTON_L3 | BUTTON_UP;
        for frame in 0..=REPEAT_FRAMES {
            if frame == 5 {
                match disruption {
                    0 => input.gap(),
                    1 => owner.2 += 1,
                    2 => {}
                    3 => source.0 += 1,
                    _ => unreachable!(),
                }
            }
            let outcome = if disruption == 2 && frame == 5 {
                input.frame(owner, true, None, || chord)
            } else {
                pad_frame(&mut input, owner, true, source, &[(100 + i64::from(frame), chord)])
            };
            let expected = if frame == 0 || frame == REPEAT_FRAMES {
                step(Dir::Up)
            } else {
                PadFrame::default()
            };
            assert_eq!(outcome, expected, "callback {}", frame + 1);
        }
    }

    #[test]
    fn fix3b_h1_hold_survives_no_view_gap() {
        assert_held_repeat_across(0);
    }

    #[test]
    fn fix3b_h2_hold_survives_cursor_replacement() {
        assert_held_repeat_across(1);
    }

    #[test]
    fn fix3b_h3_hold_survives_single_sample_fallback() {
        assert_held_repeat_across(2);
    }

    #[test]
    fn fix3b_h3_hold_survives_source_change() {
        assert_held_repeat_across(3);
    }

    #[test]
    fn fix3b_h4_screen_and_object_changes_reset_held_steps() {
        let chord = BUTTON_L3 | BUTTON_UP;
        let mut input = PadInput::new();
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, chord)]),
                   step(Dir::Up));
        input.reset();
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(101, chord)]),
                   step(Dir::Up));
        let other_object = (PAD_OWNER.0, PAD_OWNER.1 + 1, PAD_OWNER.2);
        assert_eq!(pad_frame(&mut input, other_object, true, PAD_SOURCE, &[(102, chord)]),
                   step(Dir::Up));
        let other_screen = (Screen::Sss, other_object.1, other_object.2);
        assert_eq!(pad_frame(&mut input, other_screen, true, PAD_SOURCE, &[(103, chord)]),
                   step(Dir::Up));
    }

    fn compare_single_samples_with_pre_ring_rule(owner_changes: bool) {
        fn draw(seed: &mut u64) -> u32 {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }
        let choices = [
            0, BUTTON_L3, BUTTON_L3 | BUTTON_UP, BUTTON_L3 | BUTTON_DOWN,
            BUTTON_L3 | BUTTON_LEFT, BUTTON_L3 | BUTTON_RIGHT, BUTTON_UP,
            BUTTON_L3 | BUTTON_UP | BUTTON_LEFT,
        ];
        let mut input = PadInput::new();
        let mut reference = Repeater::new();
        let mut blocked_reference = Repeater::new();
        let mut seed = 0x1f4d_2a83_6b70_c951;
        let mut owner = PAD_OWNER;
        let mut source = PAD_SOURCE;
        let mut ring = Vec::new();
        let mut coverage = [0usize; 7];
        for callback in 0..6000 {
            let buttons = choices[draw(&mut seed) as usize % choices.len()];
            ring.insert(0, (i64::from(callback) + 100, buttons));
            ring.truncate(PAD_RING);
            if draw(&mut seed).is_multiple_of(13) {
                coverage[0] += 1;
                input.gap();
                continue;
            }
            if draw(&mut seed).is_multiple_of(9) {
                coverage[1] += 1;
                owner.2 ^= 1;
            }
            if owner_changes && draw(&mut seed).is_multiple_of(43) {
                if draw(&mut seed).is_multiple_of(2) {
                    owner.0 = if owner.0 == Screen::Css { Screen::Sss } else { Screen::Css };
                    coverage[5] += 1;
                } else {
                    owner.1 ^= 1;
                    coverage[6] += 1;
                }
                reference = Repeater::new();
                blocked_reference = Repeater::new();
            }
            if draw(&mut seed).is_multiple_of(11) {
                coverage[2] += 1;
                source.0 ^= 1;
            }
            let steppable = !draw(&mut seed).is_multiple_of(4);
            if !steppable {
                coverage[3] += 1;
            }
            let ring_available = !draw(&mut seed).is_multiple_of(4);
            if !ring_available {
                coverage[4] += 1;
            }
            let expected = reference.update(if steppable { buttons } else { 0 });
            let expected_blocked = if steppable {
                blocked_reference = Repeater::new();
                None
            } else {
                blocked_reference.update(buttons)
            };
            let outcome = if ring_available {
                input.frame(owner, steppable, Some((source, &ring)), || panic!("unexpected fallback"))
            } else {
                input.frame(owner, steppable, None, || buttons)
            };
            assert_eq!(outcome.step, expected, "callback {callback}, buttons {buttons:#x}, steppable {steppable}, ring {ring_available}");
            let mut expected_counts = [0; 4];
            if let Some(dir) = expected_blocked {
                expected_counts[dir as usize] = 1;
            }
            assert_eq!(outcome.blocked, expected_counts, "blocked callback {callback}");
        }
        assert!(coverage[..5].iter().all(|&n| n > 0), "{coverage:?}");
        if owner_changes {
            assert!(coverage[5..].iter().all(|&n| n > 0), "{coverage:?}");
        }
    }

    #[test]
    fn fix3b_d1_matches_pre_ring_rule_for_one_sample_per_callback() {
        compare_single_samples_with_pre_ring_rule(false);
    }

    #[test]
    fn fix3b_d1_matches_pre_ring_rule_with_screen_and_object_changes() {
        compare_single_samples_with_pre_ring_rule(true);
    }

    #[test]
    fn blocked_taps_are_counted_and_never_replayed_as_steps() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let ring = [(104, 0), (103, chord), (102, 0), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring),
                   blocked(Dir::Up, 2));
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(105, 0), (104, 0)]),
                   PadFrame::default());
    }

    #[test]
    fn missing_view_taps_are_not_replayed_as_steps() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        input.gap();
        let ring = [(103, 0), (102, chord), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    #[test]
    fn changing_only_the_cursor_discards_old_history() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let other = (PAD_OWNER.0, PAD_OWNER.1, PAD_OWNER.2 + 1);
        let ring = [(103, 0), (102, chord), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, other, true, PAD_SOURCE, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, other, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    #[test]
    fn changing_pad_source_primes_only_its_newest_sample() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let other = (PAD_SOURCE.0 + 1, PAD_SOURCE.1);
        let ring = [(103, 0), (102, chord), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, other, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, other, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, other, &[(104, chord), (103, 0)]),
                   step(Dir::Up));
    }

    #[test]
    fn missing_ring_uses_newest_release_and_fresh_press() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, chord)]),
                   step(Dir::Up));
        for _ in 0..20 {
            assert_eq!(input.frame(PAD_OWNER, true, None, || 0), PadFrame::default());
        }
        assert_eq!(input.frame(PAD_OWNER, true, None, || chord), step(Dir::Up));
    }

    #[test]
    fn missing_ring_preserves_single_sample_hold_repeat() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, chord)]),
                   step(Dir::Up));
        for _ in 1..REPEAT_FRAMES {
            assert_eq!(input.frame(PAD_OWNER, true, None, || chord), PadFrame::default());
        }
        assert_eq!(input.frame(PAD_OWNER, true, None, || chord), step(Dir::Up));
    }

    #[test]
    fn consecutive_steppable_frames_replay_the_short_chord_once() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        let rings: &[&[(i64, u64)]] = &[
            &[(100, BUTTON_L3), (99, 0)],
            &[(106, 0), (105, chord), (104, chord), (103, chord), (102, BUTTON_L3), (101, BUTTON_L3), (100, BUTTON_L3)],
            &[(109, 0), (108, 0), (107, 0), (106, 0), (105, chord), (104, chord), (103, chord)],
        ];
        let outcomes: Vec<_> = rings.iter().map(|r| pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, r)).collect();
        assert_eq!(outcomes, [PadFrame::default(), step(Dir::Up), PadFrame::default()]);
    }

    #[test]
    fn two_steppable_taps_step_once_and_block_the_second() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let ring = [(104, 0), (103, chord), (102, 0), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring),
                   PadFrame { step: Some(Dir::Up), blocked: [1, 0, 0, 0] });
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    #[test]
    fn held_chord_at_blocked_exit_steps_once() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let ring = [(102, chord), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring),
                   blocked(Dir::Up, 1));
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), step(Dir::Up));
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    #[test]
    fn held_chord_at_gap_exit_steps_once() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        input.gap();
        let ring = [(102, chord), (101, chord), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), step(Dir::Up));
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    #[test]
    fn blocked_hold_repeats_once_per_fifteen_frames_even_without_new_samples() {
        let mut input = PadInput::new();
        let chord = BUTTON_L3 | BUTTON_UP;
        let ring = [(100, chord)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring),
                   blocked(Dir::Up, 1));
        for _ in 1..REPEAT_FRAMES {
            assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring), PadFrame::default());
        }
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring),
                   blocked(Dir::Up, 1));
    }

    #[test]
    fn later_steppable_tap_is_blocked_without_deferral() {
        let mut input = PadInput::new();
        let up = BUTTON_L3 | BUTTON_UP;
        let down = BUTTON_L3 | BUTTON_DOWN;
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &[(100, 0)]), PadFrame::default());
        let ring = [(104, 0), (103, down), (102, 0), (101, up), (100, 0)];
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring),
                   PadFrame { step: Some(Dir::Up), blocked: [0, 1, 0, 0] });
        assert_eq!(pad_frame(&mut input, PAD_OWNER, false, PAD_SOURCE, &ring),
                   PadFrame::default());
        assert_eq!(pad_frame(&mut input, PAD_OWNER, true, PAD_SOURCE, &ring), PadFrame::default());
    }

    fn scan_one(r: &mut Repeater, samples: &[u64]) -> Option<Dir> {
        let result = r.scan(samples);
        assert_eq!(result.later, [0; 4]);
        result.first.or(result.repeat)
    }

    fn play(r: &mut Repeater, frames: &[&[u64]]) -> Vec<Option<Dir>> {
        frames.iter().map(|f| scan_one(r, f)).collect()
    }

    #[test]
    fn a_chord_that_comes_and_goes_between_two_updates_still_steps() {
        let frames: &[&[u64]] = &[
            &[BUTTON_L3],
            &[BUTTON_L3 | BUTTON_UP, BUTTON_L3 | BUTTON_UP, 0],
            &[0],
        ];
        let mut r = Repeater::new();
        let steps = play(&mut r, frames);
        assert_eq!(
            steps.iter().filter(|&&step| step == Some(Dir::Up)).count(),
            1
        );

        let mut tracker = PadTracker::new();
        let mut r = Repeater::new();
        let mut out = [(0, 0); PAD_RING];
        let rings: &[&[(i64, u64)]] = &[
            &[(100, BUTTON_L3), (99, 0)],
            &[
                (106, 0),
                (105, BUTTON_L3 | BUTTON_UP),
                (104, BUTTON_L3 | BUTTON_UP),
                (103, BUTTON_L3 | BUTTON_UP),
                (102, BUTTON_L3),
                (101, BUTTON_L3),
                (100, BUTTON_L3),
            ],
            &[
                (109, 0),
                (108, 0),
                (107, 0),
                (106, 0),
                (105, BUTTON_L3 | BUTTON_UP),
                (104, BUTTON_L3 | BUTTON_UP),
                (103, BUTTON_L3 | BUTTON_UP),
            ],
        ];
        let mut steps = Vec::new();
        for (ring, expected_last) in rings.iter().zip([100, 106, 109]) {
            let count = tracker.unseen(ring, &mut out);
            let buttons: Vec<_> = out[..count].iter().map(|(_, buttons)| *buttons).collect();
            let step = scan_one(&mut r, &buttons);
            tracker.consume(&out[..count], count);
            assert_eq!(tracker.last, Some(expected_last));
            steps.push(step);
        }
        assert_eq!(steps, [None, Some(Dir::Up), None]);
    }

    #[test]
    fn two_taps_in_one_batch_are_reported_together() {
        let chord = BUTTON_L3 | BUTTON_UP;
        let mut r = Repeater::new();
        assert_eq!(r.scan(&[chord, 0, chord, 0]),
                   Scan { first: Some(Dir::Up), later: [1, 0, 0, 0], repeat: None });
        assert_eq!(r.scan(&[0]), Scan::default());
        assert_eq!(r.scan(&[]), Scan::default());
    }

    #[test]
    fn multiple_samples_count_as_one_repeat_frame() {
        let chord = BUTTON_L3 | BUTTON_RIGHT;
        let mut r = Repeater::new();
        assert_eq!(scan_one(&mut r, &[chord]), Some(Dir::Right));
        for _ in 1..REPEAT_FRAMES {
            assert_eq!(scan_one(&mut r, &[chord, chord, chord]), None);
        }
        assert_eq!(scan_one(&mut r, &[chord, chord, chord]), Some(Dir::Right));

        let mut old = Repeater::new();
        let mut batched = Repeater::new();
        let sequence = [0, BUTTON_L3, chord, chord, 0, BUTTON_UP, BUTTON_L3 | BUTTON_UP,
                        BUTTON_L3 | BUTTON_UP | BUTTON_RIGHT, chord];
        for buttons in sequence.into_iter().cycle().take(200) {
            assert_eq!(old.update(buttons), scan_one(&mut batched, &[buttons]));
        }
    }

    #[test]
    fn batch_handles_release_direction_diagonal_and_l3_order() {
        let mut r = Repeater::new();
        assert_eq!(scan_one(&mut r, &[BUTTON_L3 | BUTTON_UP]), Some(Dir::Up));
        assert_eq!(scan_one(&mut r, &[0, BUTTON_L3 | BUTTON_RIGHT]), Some(Dir::Right));
        assert_eq!(scan_one(&mut r, &[BUTTON_L3 | BUTTON_UP | BUTTON_RIGHT]), None);
        assert_eq!(scan_one(&mut r, &[BUTTON_UP]), None);
        assert_eq!(scan_one(&mut r, &[BUTTON_UP, BUTTON_L3 | BUTTON_UP]), Some(Dir::Up));
    }

    #[test]
    fn empty_batch_advances_held_repeats_once() {
        let mut r = Repeater::new();
        assert_eq!(scan_one(&mut r, &[]), None);
        assert_eq!(scan_one(&mut r, &[BUTTON_L3 | BUTTON_UP]), Some(Dir::Up));
        for _ in 1..REPEAT_FRAMES {
            assert_eq!(scan_one(&mut r, &[]), None);
        }
        assert_eq!(scan_one(&mut r, &[]), Some(Dir::Up));
        assert_eq!(scan_one(&mut r, &[0]), None);
        assert_eq!(scan_one(&mut r, &[]), None);
    }

    #[test]
    fn tracker_primes_orders_consumes_and_stops_at_invalid_tail() {
        let mut t = PadTracker::new();
        let mut out = [(0, 0); PAD_RING];
        assert_eq!(t.unseen(&[], &mut out), 0);
        let initial = [(10, 10), (9, 9), (0, 0), (0, 0)];
        assert_eq!(t.unseen(&initial, &mut out), 1);
        assert_eq!(out[0], (10, 10));
        t.consume(&out[..1], 1);
        assert_eq!(t.last, Some(10));

        let ring = [(14, 14), (13, 13), (12, 12), (11, 11), (0, 0), (0, 0)];
        let n = t.unseen(&ring, &mut out);
        assert_eq!(n, 4);
        assert_eq!(&out[..n], &[(11, 11), (12, 12), (13, 13), (14, 14)]);
        t.consume(&out[..n], 2);
        assert_eq!(t.last, Some(12));
        let n = t.unseen(&ring, &mut out);
        assert_eq!(&out[..n], &[(13, 13), (14, 14)]);
        t.consume(&out[..n], 0);
        assert_eq!(t.last, Some(12));
        let n = t.unseen(&[(16, 16), (15, 15), (15, 99), (14, 14)], &mut out);
        assert_eq!(&out[..n], &[(15, 15), (16, 16)]);

        let sixteen: Vec<_> = (13..=28).rev().map(|i| (i, i as u64)).collect();
        let n = t.unseen(&sixteen, &mut out);
        assert_eq!(n, PAD_RING);
        assert_eq!(out[0], (13, 13));
        assert_eq!(out[15], (28, 28));
        t.reset();
        assert_eq!(t.last, None);
        assert_eq!(t.unseen(&sixteen, &mut out), 1);
        assert_eq!(out[0], (28, 28));
    }

    #[test]
    fn blocked_event_round_trips_screen_and_direction() {
        for (screen, name, dir, direction) in [
            (Screen::Css, "css", Dir::Up, "up"),
            (Screen::Sss, "sss", Dir::Left, "left"),
        ] {
            let counts = blocked(dir, 2).blocked;
            let (a, b) = pack_blocked(screen, counts);
            assert_eq!(Screen::from_code(a & 0xff), screen);
            assert_eq!(unpack_blocked(a), counts);
            let mut line = Line::<128>::new();
            describe(&RawEvent { tick: 7, code: EV_BLOCKED, a, b }, &mut line).unwrap();
            assert_eq!(core::str::from_utf8(line.as_bytes()).unwrap(),
                       format!("7 blocked screen={name} dir={direction} presses=2"));
        }
    }

    #[test]
    fn steps_fire_on_press_then_repeat() {
        let mut r = Repeater::new();
        assert_eq!(r.update(BUTTON_RIGHT), None, "no L3, no step");
        assert_eq!(r.update(BUTTON_L3), None);
        assert_eq!(r.update(PRESS), Some(Dir::Right));
        for frame in 1..=45 {
            assert_eq!(
                r.update(PRESS),
                (frame % usize::from(REPEAT_FRAMES) == 0).then_some(Dir::Right),
                "held frame {frame} after the press"
            );
        }
    }

    #[test]
    fn two_frame_taps_never_repeat() {
        let mut r = Repeater::new();
        // No clock argument exists: a frame may take 16 ms, 266 ms or seconds.
        // Exercise repeated two-frame taps in every direction, with release.
        for dir in Dir::ALL {
            let button = match dir {
                Dir::Up => BUTTON_UP,
                Dir::Down => BUTTON_DOWN,
                Dir::Left => BUTTON_LEFT,
                Dir::Right => BUTTON_RIGHT,
            };
            for _ in 0..100 {
                assert_eq!(r.update(BUTTON_L3 | button), Some(dir));
                assert_eq!(r.update(BUTTON_L3 | button), None);
                assert_eq!(r.update(0), None);
            }
        }
    }

    #[test]
    fn repeat_reset_discards_partial_hold() {
        for reset in [0, BUTTON_RIGHT, BUTTON_L3, PRESS | BUTTON_UP] {
            let mut r = Repeater::new();
            assert_eq!(r.update(PRESS), Some(Dir::Right));
            for _ in 1..REPEAT_FRAMES {
                assert_eq!(r.update(PRESS), None);
            }
            assert_eq!(r.update(reset), None);
            assert_eq!(r.update(PRESS), Some(Dir::Right));
            for _ in 1..REPEAT_FRAMES {
                assert_eq!(r.update(PRESS), None);
            }
            assert_eq!(r.update(PRESS), Some(Dir::Right));
        }
        let mut r = Repeater::new();
        assert_eq!(r.update(PRESS), Some(Dir::Right));
        assert_eq!(r.update(BUTTON_L3 | BUTTON_UP), Some(Dir::Up));
        for _ in 1..REPEAT_FRAMES {
            assert_eq!(r.update(BUTTON_L3 | BUTTON_UP), None);
        }
        assert_eq!(r.update(BUTTON_L3 | BUTTON_UP), Some(Dir::Up));
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
        let (a, b) = pack_step(Screen::Css, Dir::Right, &step, Some(0));
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
            "7 step screen=css dir=right from=14 to=15 token=p1 x=-528.0 y=240.0"
        );
        line.clear();
        let (a, b) = pack_step(Screen::Css, Dir::Up, &step, Some(1));
        describe(
            &RawEvent {
                tick: 11,
                code: EV_STEP,
                a,
                b,
            },
            &mut line,
        )
        .unwrap();
        assert_eq!(
            core::str::from_utf8(line.as_bytes()).unwrap(),
            "11 step screen=css dir=up from=14 to=15 token=p2 x=-528.0 y=240.0"
        );
        line.clear();
        let edge = neighbour(&tiles[..], (0.0, -400.0), Dir::Down, &SCREEN);
        let (a, b) = pack_step(Screen::Sss, Dir::Down, &edge, None);
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
