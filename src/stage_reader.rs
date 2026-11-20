//! Native 13.0.5 stage actor. Any borrowed Lua object is read on its live getter stack.
use super::*;
use crate::provider::{P1Token, Sample};
use crate::stage::{actor_code_matches, qualified_stage, start_move, Preview, StageFacts};

const INVALID_KIND: u64 = 0x69ff_ff00_0000_0000;
const KEYS: [&[u8]; 3] = [b"get_tab_switch", b"TAB_SWITCH_NORMAL", b"TAB_SWITCH_SUB"];

#[derive(Clone, Copy)]
struct Actor {
    closure: u64,
    getter: i32,
    hover: i32,
    slot: i64,
    tab: i64,
}

#[derive(Default)]
struct StageReader {
    owner: u64,
    open: bool,
    lua: u64,
    generation: u64,
    cursor: u64,
    tiles: u64,
    token: Option<P1Token>,
    actor: Option<Actor>,
    invalid_getter: bool,
    actor_identity: u64,
    last_rest: Option<(u64, u64, u16)>,
    moving: Option<(u64, u64, u16, u16)>,
    keys: [u16; 3],
    failed_env: u64,
}
struct MainReader(UnsafeCell<StageReader>);
// Only registered stage owner, getter and post-resume hooks on the game's main thread.
unsafe impl Sync for MainReader {}
static READER: MainReader = MainReader(UnsafeCell::new(StageReader {
    owner: 0,
    open: false,
    lua: 0,
    generation: 0,
    cursor: 0,
    tiles: 0,
    token: None,
    actor: None,
    invalid_getter: false,
    actor_identity: 0,
    last_rest: None,
    moving: None,
    keys: [u16::MAX; 3],
    failed_env: 0,
}));
static GETTER_RETURN: AtomicU64 = AtomicU64::new(0);

pub(super) fn prepare(text: u64) {
    GETTER_RETURN.store(text + 0x1b397ac, Ordering::Release);
}

unsafe fn ptr(addr: u64) -> Option<u64> {
    let value = read_u64(addr);
    valid(value).then_some(value)
}
unsafe fn typed_ptr(value: u64, tag: i32) -> Option<u64> {
    (read_i32(value + 8) == tag).then(|| ptr(value)).flatten()
}
unsafe fn integer(value: u64) -> Option<i64> {
    (read_i32(value + 8) == 0x13).then(|| read_u64(value) as i64)
}
unsafe fn closed(closure: u64, index: u64) -> Option<u64> {
    let uv = ptr(closure + 0x20 + 8 * index)?;
    (read_u64(uv) == uv + 0x10).then_some(uv + 0x10)
}
unsafe fn key_matches(node: u64, key: &[u8]) -> bool {
    let Some(s) = typed_ptr(node + 16, 0x44) else {
        return false;
    };
    read_u8(s + 8) == 4
        && usize::from(read_u8(s + 11)) == key.len()
        && core::slice::from_raw_parts((s + 24) as *const u8, key.len()) == key
}

// Node indices are only hints. Revalidate the actual three names and their typed values.
unsafe fn tab(state: &mut StageReader, env: u64, hover: u64) -> Option<i64> {
    if read_u8(env + 8) != 5 || read_u8(env + 11) != 9 || state.failed_env == env {
        return None;
    }
    let nodes = ptr(env + 24)?;
    let find = |i: u16, key: &[u8]| i < 512 && key_matches(nodes + u64::from(i) * 32, key);
    if !(0..3).all(|k| find(state.keys[k], KEYS[k])) {
        let mut indexes = [u16::MAX; 3];
        for n in 0..512 {
            for k in 0..3 {
                if indexes[k] == u16::MAX && find(n, KEYS[k]) {
                    indexes[k] = n;
                }
            }
            if !indexes.contains(&u16::MAX) {
                break;
            }
        }
        if indexes.contains(&u16::MAX) {
            state.failed_env = env;
            return None;
        }
        state.keys = indexes;
    }
    let node = |k: usize| nodes + u64::from(state.keys[k]) * 32;
    let get = typed_ptr(node(0), 0x46)?;
    if integer(node(1))? != 0
        || integer(node(2))? != 1
        || read_u8(get + 8) != 6
        || read_u8(get + 10) != 1
        || read_u64(get + 0x20) != read_u64(hover + 0x60)
    {
        return None;
    }
    let proto = ptr(get + 0x18)?;
    if read_i32(proto + 0x18) != 3 {
        return None;
    }
    let code = ptr(proto + 0x38)?;
    if read_u64(code) != 0x0100_0026_0000_0005 || read_i32(code + 8) != 0x0080_0026 {
        return None;
    }
    integer(closed(get, 0)?)
}

unsafe fn from_getter(state: &mut StageReader, lua: u64, getter: i32) -> Option<Actor> {
    let ci = ptr(lua + 0x20)?;
    let caller = ptr(ci + 0x10)?;
    let func = ptr(caller)?;
    let stack = ptr(lua + 0x38)?;
    let last = ptr(lua + 0x30)?;
    if func < stack || func.checked_add(16)? > last || !(func - stack).is_multiple_of(16) {
        return None;
    }
    let hover = typed_ptr(func, 0x46)?;
    if read_u8(hover + 8) != 6 || read_u8(hover + 10) != 9 {
        return None;
    }
    let proto = ptr(hover + 0x18)?;
    if read_i32(proto + 0x18) != 84 {
        return None;
    }
    let code = ptr(proto + 0x38)?;
    if read_u64(caller + 0x28) != code + 56
        || !actor_code_matches(core::slice::from_raw_parts(code as *const u8, 336))
    {
        return None;
    }
    let slot = integer(closed(hover, 4)?)?;
    let previews = typed_ptr(closed(hover, 5)?, 0x45)?;
    if slot != 0 || read_u8(previews + 8) != 5 || read_i32(previews + 12) != 4 {
        return None;
    }
    let slot_table = typed_ptr(ptr(previews + 16)?, 0x45)?;
    if read_u8(slot_table + 8) != 5 {
        return None;
    }
    let env = typed_ptr(closed(hover, 1)?, 0x45)?;
    Some(Actor {
        closure: hover,
        getter,
        hover: integer(closed(hover, 7)?)?.try_into().ok()?,
        slot,
        tab: tab(state, env, hover)?,
    })
}

unsafe fn owner_lua(owner: u64) -> Option<u64> {
    let root = ptr(owner + 0x40)?;
    let player = ptr(root)?;
    ptr(player + 8)
}

pub(super) unsafe fn close_update() {
    let s = &mut *READER.0.get();
    if s.open {
        s.last_rest = None;
        s.moving = None;
        s.open = false;
    }
}

pub(super) unsafe fn begin(owner: u64, view: &View, sample: &Sample) -> bool {
    let s = &mut *READER.0.get();
    let lua = (sample.grid == Some(crate::layout::Layout::Sss) && sample.hand_steppable)
        .then(|| owner_lua(owner))
        .flatten()
        .unwrap_or(0);
    if s.owner != owner || s.lua != lua || s.generation != sample.generation {
        *s = StageReader {
            owner,
            lua,
            generation: sample.generation,
            keys: [u16::MAX; 3],
            ..StageReader::default()
        };
    }
    s.cursor = view.cursor;
    s.tiles = view.tiles.begin;
    s.actor = None;
    s.invalid_getter = false;
    s.open = lua != 0;
    if lua == 0 {
        return false;
    }
    if let Some(movement) = s
        .last_rest
        .and_then(|rest| start_move(rest, sample.generation, sample.step))
    {
        s.moving = Some(movement);
    }
    true
}

pub(super) unsafe fn getter(ctx: &InlineCtx) {
    let s = &mut *READER.0.get();
    let lua = ctx.registers[19].x();
    if !s.open || lua != s.lua || ctx.registers[30].x() != GETTER_RETURN.load(Ordering::Acquire) {
        return;
    }
    s.actor = from_getter(s, lua, ctx.registers[0].w() as i32);
    s.invalid_getter |= s.actor.is_none();
}

unsafe fn medal(owner: u64) -> P1Token {
    let Some(medals) = ptr(owner + 0x70) else {
        return P1Token::Unknown;
    };
    match (read_u8(medals + 0x11b), read_u8(medals + 0x11f)) {
        (0 | 1, 1) => P1Token::Held,
        (0, 0) => P1Token::None,
        _ => P1Token::Unknown,
    }
}

pub(super) unsafe fn finish(owner: u64, sample: &mut Sample) -> bool {
    let s = &mut *READER.0.get();
    if !s.open || s.owner != owner {
        return false;
    }
    s.open = false;
    if owner_lua(owner) != Some(s.lua) {
        return true;
    }
    let Some(view) = sss_view(owner) else {
        return true;
    };
    if view.cursor != s.cursor
        || view.tiles.begin != s.tiles
        || !view.steppable
        || view.tiles.count != 116
        || sample.grid != Some(crate::layout::Layout::Sss)
    {
        return true;
    }
    let rec: [Preview; 3] = core::array::from_fn(|i| {
        let p = owner + 0xf8 + i as u64 * 40;
        Preview {
            kind: read_u64(p),
            form: read_i32(p + 8),
            panel: read_i32(p + 24),
        }
    });
    let tok = medal(owner);
    if s.token.is_some_and(|old| old != tok) {
        sample.generation += 1;
        s.generation = sample.generation;
        sample.step.new_owner();
        sample.resting = false;
        s.last_rest = None;
        s.moving = None;
    }
    s.token = Some(tok);
    if s.invalid_getter
        || tok == P1Token::Unknown
        || rec[0].form != 0
        || rec[1..].iter().any(|p| p.kind != INVALID_KIND)
    {
        s.last_rest = None;
        s.moving = None;
        return true;
    }
    if let Some(actor) = s.actor.take() {
        if s.actor_identity != 0 && s.actor_identity != actor.closure {
            sample.generation += 1;
            s.generation = sample.generation;
            sample.step.new_owner();
            sample.resting = false;
            s.last_rest = None;
            s.moving = None;
        }
        // Identity only; never dereferenced on a later callback.
        s.actor_identity = actor.closure;
        if let Ok(i) = usize::try_from(actor.getter) {
            if i < view.tiles.count {
                sample.stage = qualified_stage(StageFacts {
                    tab: actor.tab,
                    slot: actor.slot,
                    getter: actor.getter,
                    actor_hover: actor.hover,
                    grid_kind: read_u64(view.tiles.begin + i as u64 * view.tiles.stride),
                    records: rec,
                })
                .filter(|_| sample.at == Some(i as u16) && sample.resting);
            }
        }
        if sample.stage.is_some() {
            sample.hover = sample.at;
            s.last_rest = sample.at.map(|at| (sample.generation, sample.step.ack, at));
            s.moving = None;
        } else if actor.tab != 0 || actor.slot != 0 {
            s.moving = None;
        }
    }
    if let Some((generation, ack, from, to)) = s.moving {
        let target = read_point(view.cursor + cursor::TARGET as u64);
        let endpoints = [i32::from(from), i32::from(to)];
        sample.stage_moving = sample.stage.is_none()
            && !sample.resting
            && generation == sample.generation
            && ack == sample.step.ack
            && ack == sample.step.last_written
            && endpoints.contains(&rec[0].panel)
            && rec[0].kind == read_u64(view.tiles.begin + rec[0].panel as u64 * view.tiles.stride)
            && tile_at(&view.tiles, target) == Some(usize::from(to));
    }
    if sample.stage.is_some() || sample.stage_moving {
        sample.local_seat = Some(0);
        sample.p1_token = tok;
        sample.p1_carried = (tok == P1Token::Held).then_some(0);
    }
    true
}

pub(super) static GUARDS: &[Site] = &[
    Site {
        name: "sss_positive_getter",
        offset: 0x1b3304c,
        guard: [
            0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x80, 0x12, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x43, 0x03, 0xd1, 0xef, 0x3b, 0x03, 0x6d,
            0xed, 0x33, 0x04, 0x6d, 0xeb, 0x2b, 0x05, 0x6d, 0xe9, 0x23, 0x06, 0x6d, 0xfc, 0x6f,
            0x07, 0xa9, 0xfa, 0x67, 0x08, 0xa9, 0xf8, 0x5f, 0x09, 0xa9, 0xf6, 0x57, 0x0a, 0xa9,
            0xf4, 0x4f, 0x0b, 0xa9, 0xfd, 0x7b, 0x0c, 0xa9,
        ],
    },
    Site {
        name: "sss_owner_continuation",
        offset: model::SSS_OBSERVE_OFFSET,
        guard: [
            0x68, 0xda, 0x40, 0xf9, 0x00, 0x05, 0x40, 0xf9, 0xa0, 0x00, 0x00, 0xb4, 0x08, 0x00,
            0x40, 0xf9, 0x08, 0x9d, 0x40, 0xf9, 0x00, 0x01, 0x3f, 0xd6, 0x80, 0x00, 0x00, 0x36,
            0xe1, 0x03, 0x1f, 0x32, 0xe0, 0x03, 0x13, 0xaa, 0x73, 0x00, 0x00, 0x94, 0x68, 0xe2,
            0x40, 0xf9, 0x00, 0x05, 0x40, 0xf9, 0xa0, 0x00, 0x00, 0xb4, 0x08, 0x00, 0x40, 0xf9,
            0x08, 0x9d, 0x40, 0xf9, 0x00, 0x01, 0x3f, 0xd6,
        ],
    },
    Site {
        name: "sss_lua_getter_binding",
        offset: 0x1b397a8,
        guard: [
            0x00, 0x01, 0x3f, 0xd6, 0x69, 0x0a, 0x40, 0xf9, 0x08, 0x7c, 0x40, 0x93, 0x28, 0x01,
            0x00, 0xf9, 0x68, 0x02, 0x80, 0x52, 0x28, 0x09, 0x00, 0xb9, 0x68, 0x0a, 0x40, 0xf9,
            0x08, 0x41, 0x00, 0x91, 0x68, 0x0a, 0x00, 0xf9, 0xfd, 0x7b, 0x41, 0xa9, 0xf3, 0x07,
            0x42, 0xf8, 0xc0, 0x03, 0x5f, 0xd6, 0x08, 0xa4, 0x41, 0xa9, 0x40, 0x05, 0x89, 0x8b,
            0x09, 0xfe, 0x07, 0x37, 0xf1, 0xff, 0xff, 0x17,
        ],
    },
    // Pin the native owner before the shared script-call span and the
    // resume implementation itself. Instant Back owns the call's inline sites.
    Site {
        name: "sss_lua_owner_prefix",
        offset: 0x1b29d70,
        guard: [
            0x08, 0x01, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x01,
            0x43, 0xf9, 0x08, 0x11, 0x12, 0x91, 0x5f, 0x01, 0x0b, 0xeb, 0x0c, 0x01, 0x00, 0xf9,
            0x41, 0xfe, 0xff, 0x54, 0x60, 0x32, 0x40, 0xf9, 0x83, 0xa3, 0x5e, 0x94, 0x60, 0x3a,
            0x40, 0xf9, 0x91, 0x96, 0x5e, 0x94, 0x60, 0xc2, 0x40, 0xf9, 0x08, 0x00, 0x40, 0xf9,
            0x08, 0x15, 0x40, 0xf9, 0x00, 0x01, 0x3f, 0xd6,
        ],
    },
    Site {
        name: "sss_script_resume",
        offset: 0x3376860,
        guard: [
            0xff, 0x03, 0x01, 0xd1, 0xf6, 0x57, 0x01, 0xa9, 0xf4, 0x4f, 0x02, 0xa9, 0xfd, 0x7b,
            0x03, 0xa9, 0xfd, 0xc3, 0x00, 0x91, 0xc0, 0x07, 0x00, 0xb4, 0x14, 0x04, 0x40, 0xf9,
            0xf3, 0x03, 0x00, 0xaa, 0x74, 0x07, 0x00, 0xb4, 0xe0, 0x03, 0x14, 0xaa, 0xe1, 0x03,
            0x1f, 0xaa, 0xe2, 0x03, 0x1f, 0x2a, 0xb4, 0x16, 0x16, 0x94, 0x1f, 0x08, 0x00, 0x71,
            0xa3, 0x00, 0x00, 0x54, 0x88, 0x0a, 0x40, 0xf9,
        ],
    },
    Site {
        name: "sss_medal_held",
        offset: 0x1b338b0,
        guard: [
            0x3f, 0x28, 0x00, 0x71, 0xc2, 0x00, 0x00, 0x54, 0x08, 0x38, 0x40, 0xf9, 0x09, 0x26,
            0x80, 0x52, 0x28, 0x20, 0x29, 0x9b, 0x00, 0x7d, 0x44, 0x39, 0xc0, 0x03, 0x5f, 0xd6,
            0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91, 0x5f, 0x3b, 0x7a, 0x94, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfd, 0x7b, 0xbf, 0xa9, 0xfd, 0x03, 0x00, 0x91,
            0x02, 0xe4, 0x00, 0x6f, 0x02, 0x04, 0x04, 0x6e,
        ],
    },
    Site {
        name: "sss_preview_form",
        offset: 0x1b325dc,
        guard: [
            0xe0, 0x03, 0x13, 0xaa, 0x08, 0x0c, 0x41, 0xf8, 0x09, 0x04, 0x40, 0xf9, 0x3f, 0x01,
            0x08, 0xeb, 0x80, 0x0d, 0x00, 0x54, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9,
            0x08, 0x01, 0x40, 0xf9, 0x17, 0x21, 0x00, 0x91, 0x41, 0x41, 0x01, 0xb0, 0x21, 0xf0,
            0x08, 0x91, 0xe8, 0x83, 0x00, 0x91, 0xe0, 0x03, 0x17, 0xaa, 0xe2, 0x03, 0x14, 0x2a,
            0x0b, 0x10, 0x71, 0x94, 0x08, 0x10, 0x2e, 0x1e,
        ],
    },
];
