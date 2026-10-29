//! Switch half: version check, guarded inline hooks, a static event ring and
//! one writer thread. The two screen hooks run on the game's main thread at
//! the start of the per-frame CSS players update and the stage select update.
//! While P1's hand may step (on the CSS: while it carries a token) they poll
//! P1's controller through `nn::hid` (read-only), and on an L3 + D-pad step
//! write the hand cursor's target, which the game's own cursor update then
//! eases to. The third hook sits in that cursor update just before its
//! stick/D-pad query and, only for P1's hand, only while it may step and only
//! while L3 is held, points the query at a copy of the input object with the
//! D-pad cleared. Nothing allocates: state is static, buffers live on the
//! stack, and the counting allocator below reports the plugin's heap
//! allocations (expected 0) in the log.
use crate::model::{
    self, cursor, describe, hand, input, neighbour, pack_screen, pack_settled, pack_step, tile_at,
    Line, RawEvent, Rect, Repeater, Screen, Site, Tiles, BUTTON_L3, EV_SCREEN, EV_SETTLED, EV_STEP,
    TICKS_PER_SECOND,
};
use core::cell::UnsafeCell;
use core::fmt::Write as _;
use skyline::hooks::{getRegionAddress, InlineCtx, Region};
use skyline::nn;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

const DATA_DIR: &str = "sd:/ultimate/ssbu-tile-cursor";
const EVENT_RING: usize = 64;
const WRITER_STACK: usize = 16 * 1024;
const EVENTS_LOG_LIMIT: i64 = 1 << 20;
const OPEN_WRITE_APPEND: i32 = 2 | 4;
const NPAD_NO1: u32 = 0;
const NPAD_HANDHELD: u32 = 0x20;
const STYLE_FULL_KEY: u32 = 1 << 0;
const STYLE_HANDHELD: u32 = 1 << 1;
const STYLE_JOY_DUAL: u32 = 1 << 2;
const STYLE_GC: u32 = 1 << 5;
/// The D-pad filter applies only while a screen hook saw P1's hand able to
/// step this recently.
const LIVE_TICKS: u64 = TICKS_PER_SECOND / 5;
/// Tile vectors longer than this are treated as unreadable.
const MAX_TILES: u64 = 512;

/// Counts calls into the heap. Nothing in the plugin is meant to make one.
struct CountingAllocator;
static HEAP_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        HEAP_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct EventSlot {
    seq: AtomicU64,
    tick: AtomicU64,
    code: AtomicU64,
    a: AtomicU64,
    b: AtomicU64,
}
static EVENTS: [EventSlot; EVENT_RING] = [const {
    EventSlot {
        seq: AtomicU64::new(0),
        tick: AtomicU64::new(0),
        code: AtomicU64::new(0),
        a: AtomicU64::new(0),
        b: AtomicU64::new(0),
    }
}; EVENT_RING];
static EVENT_HEAD: AtomicU64 = AtomicU64::new(0);

/// P1's hand cursor as last seen by a screen hook, and until when that holds.
static LIVE_CURSOR: AtomicU64 = AtomicU64::new(0);
static LIVE_UNTIL: AtomicU64 = AtomicU64::new(0);

/// State only the two screen hooks touch; both run on the game's main thread.
struct GameState {
    repeater: Repeater,
    screen: Option<(Screen, u64)>,
    /// The cursor whose step is waiting to settle, settled frames seen, and
    /// the token slot the hand carried for that step (CSS).
    settle_cursor: u64,
    settle_frames: u8,
    settle_token: Option<usize>,
}
struct Shared(UnsafeCell<GameState>);
// SAFETY: accessed only from the screen hooks, which run on one thread.
unsafe impl Sync for Shared {}
static STATE: Shared = Shared(UnsafeCell::new(GameState {
    repeater: Repeater::new(),
    screen: None,
    settle_cursor: 0,
    settle_frames: 0,
    settle_token: None,
}));

/// The input object copy the D-pad filter hands to the cursor query, and the
/// context pointing at it. Written and read on the game's main thread.
#[repr(C, align(16))]
struct FakeInput(UnsafeCell<[u8; input::COPY_BYTES]>);
#[repr(C, align(16))]
struct FakeContext(UnsafeCell<[u64; 2]>);
// SAFETY: used only by the cursor query hook, on the game's main thread.
unsafe impl Sync for FakeInput {}
unsafe impl Sync for FakeContext {}
static FAKE_INPUT: FakeInput = FakeInput(UnsafeCell::new([0; input::COPY_BYTES]));
static FAKE_CONTEXT: FakeContext = FakeContext(UnsafeCell::new([0; 2]));

/// The writer thread's object and stack, static so no heap is involved.
#[repr(C, align(4096))]
struct ThreadStack(UnsafeCell<[u8; WRITER_STACK]>);
#[repr(C, align(16))]
struct ThreadObject(UnsafeCell<[u8; 0x1c0]>);
// SAFETY: handed to nn::os once, at startup, and never touched again.
unsafe impl Sync for ThreadStack {}
unsafe impl Sync for ThreadObject {}
static WRITER_STACK_MEMORY: ThreadStack = ThreadStack(UnsafeCell::new([0; WRITER_STACK]));
static WRITER_THREAD: ThreadObject = ThreadObject(UnsafeCell::new([0; 0x1c0]));

fn tick() -> u64 {
    unsafe { nn::os::GetSystemTick() as u64 }
}

fn push_event(code: u8, (a, b): (u64, u64)) {
    let index = EVENT_HEAD.fetch_add(1, Ordering::AcqRel);
    let slot = &EVENTS[(index % EVENT_RING as u64) as usize];
    slot.tick.store(tick(), Ordering::Relaxed);
    slot.code.store(u64::from(code), Ordering::Relaxed);
    slot.a.store(a, Ordering::Relaxed);
    slot.b.store(b, Ordering::Relaxed);
    slot.seq.store(index + 1, Ordering::Release);
}

fn valid(address: u64) -> bool {
    address >= 0x10000 && address.is_multiple_of(8)
}

unsafe fn read_u8(address: u64) -> u8 {
    (address as *const u8).read_volatile()
}

unsafe fn read_i32(address: u64) -> i32 {
    (address as *const i32).read_unaligned()
}

unsafe fn read_u64(address: u64) -> u64 {
    (address as *const u64).read_unaligned()
}

unsafe fn read_f32(address: u64) -> f32 {
    (address as *const f32).read_unaligned()
}

unsafe fn read_point(address: u64) -> (f32, f32) {
    (read_f32(address), read_f32(address + 4))
}

/// A `std::vector` of game tiles read in place.
struct GameTiles {
    begin: u64,
    count: usize,
    stride: u64,
    rect: u64,
}

impl GameTiles {
    /// `begin`/`end` of a vector of `stride`-byte elements; empty if unreadable.
    unsafe fn new(begin: u64, end: u64, stride: usize, rect: usize) -> Self {
        let stride = stride as u64;
        let bytes = end.wrapping_sub(begin);
        let count = if valid(begin) && end >= begin && bytes.is_multiple_of(stride) {
            bytes / stride
        } else {
            0
        };
        Self {
            begin,
            count: if count > MAX_TILES { 0 } else { count as usize },
            stride,
            rect: rect as u64,
        }
    }
}

impl Tiles for GameTiles {
    fn count(&self) -> usize {
        self.count
    }

    fn rect(&self, index: usize) -> Rect {
        let at = self.begin + index as u64 * self.stride + self.rect;
        // SAFETY: index < count, inside the vector the game keeps alive for
        // the duration of its update (this hook runs at its start).
        unsafe {
            Rect {
                xmin: read_f32(at),
                xmax: read_f32(at + 4),
                ymin: read_f32(at + 8),
                ymax: read_f32(at + 12),
            }
        }
    }
}

/// What a screen hook needs: P1's hand cursor, whether the hand may step, the
/// tiles and, on the CSS, the slot of the token the hand carries and the
/// tiles object holding the game's hover records.
struct View {
    cursor: u64,
    steppable: bool,
    tiles: GameTiles,
    token: Option<usize>,
    records: Option<u64>,
}

unsafe fn hand_cursor(hand: u64) -> Option<(u64, bool)> {
    if !valid(hand) {
        return None;
    }
    let cursor = read_u64(hand + hand::CURSOR as u64);
    valid(cursor).then(|| (cursor, read_u8(hand + hand::ENABLED as u64) != 0))
}

unsafe fn css_view(css: u64) -> Option<View> {
    use model::css::*;
    let (begin, end) = (
        read_u64(css + PLAYERS as u64),
        read_u64(css + PLAYERS as u64 + 8),
    );
    if !valid(begin) || end < begin || end - begin > SLOTS as u64 * 16 {
        return None;
    }
    let player = (begin..end).step_by(16).find_map(|entry| {
        let player = read_u64(entry + 8);
        (valid(player) && read_i32(player + PLAYER_INDEX as u64) == 0).then_some(player)
    })?;
    let hand = read_u64(player + PLAYER_HAND as u64);
    let (cursor, enabled) = hand_cursor(hand)?;
    let list = read_u64(css + TILES as u64);
    if !valid(list) {
        return None;
    }
    // An empty hand does not step (B takes P1's token back).
    let token = carried(
        read_i32(player + PLAYER_CARRIED as u64),
        read_u8(hand + hand::HOLDING as u64),
    );
    Some(View {
        cursor,
        steppable: enabled && read_u8(player + PLAYER_ACTIVE as u64) != 0 && token.is_some(),
        tiles: GameTiles::new(read_u64(list), read_u64(list + 8), TILE_STRIDE, TILE_RECT),
        token,
        records: Some(list),
    })
}

unsafe fn sss_view(sss: u64) -> Option<View> {
    use model::sss::*;
    let (cursor, enabled) = hand_cursor(read_u64(sss + HAND as u64))?;
    Some(View {
        cursor,
        steppable: enabled,
        tiles: GameTiles::new(
            read_u64(sss + TILES as u64),
            read_u64(sss + TILES as u64 + 8),
            TILE_STRIDE,
            TILE_RECT,
        ),
        token: None,
        records: None,
    })
}

/// P1's buttons (`nn::hid` bits): controller No.1 or handheld, read-only.
fn p1_buttons() -> u64 {
    unsafe fn poll(id: u32) -> u64 {
        let style = nn::hid::GetNpadStyleSet(&id).flags;
        if style & STYLE_GC != 0 {
            let mut state = nn::hid::NpadGcState::default();
            nn::hid::GetNpadGcState(&mut state, &id);
            return state.Buttons;
        }
        let mut state = nn::hid::NpadHandheldState::default();
        if style & STYLE_FULL_KEY != 0 {
            nn::hid::GetNpadFullKeyState(&mut state, &id);
        } else if style & STYLE_HANDHELD != 0 {
            nn::hid::GetNpadHandheldState(&mut state, &id);
        } else if style & STYLE_JOY_DUAL != 0 {
            nn::hid::GetNpadJoyDualState(&mut state, &id);
        } else {
            return 0;
        }
        state.Buttons
    }
    unsafe { poll(NPAD_NO1) | poll(NPAD_HANDHELD) }
}

unsafe fn on_screen(screen: Screen, object: u64, view: &View) {
    let state = &mut *STATE.0.get();
    let now = tick();
    if state.screen != Some((screen, object)) {
        state.screen = Some((screen, object));
        state.repeater = Repeater::new();
        state.settle_cursor = 0;
        push_event(
            EV_SCREEN,
            pack_screen(
                screen,
                view.tiles.count(),
                HEAP_ALLOCATIONS.load(Ordering::Relaxed),
            ),
        );
    }
    // The D-pad filter follows the same gate as the steps.
    let buttons = if view.steppable {
        LIVE_CURSOR.store(view.cursor, Ordering::Relaxed);
        LIVE_UNTIL.store(now + LIVE_TICKS, Ordering::Relaxed);
        p1_buttons()
    } else {
        LIVE_UNTIL.store(0, Ordering::Relaxed);
        0
    };
    let current = read_point(view.cursor + cursor::CURRENT as u64);
    let target = read_point(view.cursor + cursor::TARGET as u64);
    if state.settle_cursor == view.cursor {
        // The game hit-tests once the hand stops; read its answer a frame later.
        state.settle_frames = if current == target {
            state.settle_frames + 1
        } else {
            0
        };
        if state.settle_frames >= 2 {
            state.settle_cursor = 0;
            // The game records the hover for the carried token's slot.
            let hovered = view
                .records
                .zip(state.settle_token.and_then(model::css::hover_record))
                .map(|(records, offset)| read_i32(records + offset as u64))
                .and_then(|i| usize::try_from(i).ok());
            push_event(
                EV_SETTLED,
                pack_settled(screen, tile_at(&view.tiles, current), hovered, current),
            );
        }
    }
    let Some(dir) = state.repeater.update(buttons, now) else {
        return;
    };
    let bounds_at = view.cursor + cursor::BOUNDS as u64;
    let bounds = Rect {
        xmin: read_f32(bounds_at),
        xmax: read_f32(bounds_at + 4),
        ymin: read_f32(bounds_at + 8),
        ymax: read_f32(bounds_at + 12),
    };
    // From the target, so held repeats accumulate whole tiles while the
    // hand is still easing.
    let step = neighbour(&view.tiles, target, dir, &bounds);
    if step.to.is_some() {
        let out = (view.cursor + cursor::TARGET as u64) as *mut f32;
        for (i, value) in [step.target.0, step.target.1, 0.0, 0.0]
            .into_iter()
            .enumerate()
        {
            out.add(i).write_volatile(value);
        }
        state.settle_cursor = view.cursor;
        state.settle_frames = 0;
        state.settle_token = view.token;
    }
    push_event(EV_STEP, pack_step(screen, dir, &step, view.token));
}

#[skyline::hook(offset = model::CSS_PLAYERS_OFFSET, inline)]
unsafe fn tile_cursor_css(ctx: &InlineCtx) {
    let css = ctx.registers[0].x();
    if valid(css) {
        if let Some(view) = css_view(css) {
            on_screen(Screen::Css, css, &view);
        }
    }
}

#[skyline::hook(offset = model::SSS_UPDATE_OFFSET, inline)]
unsafe fn tile_cursor_sss(ctx: &InlineCtx) {
    let sss = ctx.registers[0].x();
    if valid(sss) {
        if let Some(view) = sss_view(sss) {
            on_screen(Screen::Sss, sss, &view);
        }
    }
}

/// In the cursor update, before `mov x0, x20; bl` (the stick/D-pad query):
/// for P1's hand while it may step and L3 is held, x20 becomes a context
/// whose input object is a copy of the real one with the four D-pad words
/// cleared.
#[skyline::hook(offset = model::CURSOR_QUERY_OFFSET, inline)]
unsafe fn tile_cursor_query(ctx: &mut InlineCtx) {
    let cursor = ctx.registers[19].x();
    if cursor == 0
        || cursor != LIVE_CURSOR.load(Ordering::Relaxed)
        || tick() > LIVE_UNTIL.load(Ordering::Relaxed)
        || p1_buttons() & BUTTON_L3 == 0
    {
        return;
    }
    let context = ctx.registers[20].x();
    if !valid(context) {
        return;
    }
    let source = read_u64(context + 8);
    if !valid(source) {
        return;
    }
    let copy = FAKE_INPUT.0.get().cast::<u8>();
    core::ptr::copy_nonoverlapping(source as *const u8, copy, input::COPY_BYTES);
    for word in input::DPAD_WORDS {
        copy.add(word).cast::<u32>().write_unaligned(0);
    }
    let fake = FAKE_CONTEXT.0.get();
    *fake = [read_u64(context), copy as u64];
    ctx.registers[20].set_x(fake as u64);
}

type Path = Line<96>;

fn path(args: core::fmt::Arguments) -> Path {
    let mut path = Path::new();
    let _ = path.write_fmt(args);
    path
}

/// An `nn::fs` file opened for writing at its end; closed on drop. A closed
/// file is on the card at once, while Eden keeps an open file's writes until
/// the host file is closed.
struct NxFile {
    handle: nn::fs::FileHandle,
    offset: i64,
}

impl NxFile {
    fn append(path: &mut Path) -> Option<Self> {
        let path = path.with_nul()?.as_ptr();
        unsafe {
            // Fails harmlessly when the file exists.
            nn::fs::CreateFile(path, 0);
            let mut handle = nn::fs::FileHandle { handle: 0 };
            if nn::fs::OpenFile(&mut handle, path, OPEN_WRITE_APPEND) != 0 {
                return None;
            }
            let mut offset = 0;
            if nn::fs::GetFileSize(&mut offset, handle) != 0 {
                nn::fs::CloseFile(handle);
                return None;
            }
            Some(Self { handle, offset })
        }
    }

    fn truncate(&mut self) -> Option<()> {
        (unsafe { nn::fs::SetFileSize(self.handle, 0) } == 0).then(|| self.offset = 0)
    }

    fn write(&mut self, bytes: &[u8]) -> bool {
        let option = nn::fs::WriteOption { flags: 0 };
        let written = unsafe {
            nn::fs::WriteFile(
                self.handle,
                self.offset,
                bytes.as_ptr(),
                bytes.len() as u64,
                &option,
            )
        } == 0;
        if written {
            self.offset += bytes.len() as i64;
        }
        written
    }
}

impl Drop for NxFile {
    fn drop(&mut self) {
        unsafe {
            nn::fs::FlushFile(self.handle);
            nn::fs::CloseFile(self.handle);
        }
    }
}

fn append_log(bytes: &[u8]) {
    if let Some(mut log) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) {
        log.write(bytes);
    }
}

/// Drains the event ring into `events.log` every 100 ms.
fn writer_run() -> ! {
    let mut tail = 0u64;
    let mut line = Line::<160>::new();
    loop {
        unsafe {
            nn::os::SleepThread(nn::TimeSpan {
                nanoseconds: 100_000_000,
            })
        };
        let head = EVENT_HEAD.load(Ordering::Acquire);
        if head.saturating_sub(tail) > EVENT_RING as u64 {
            tail = head - EVENT_RING as u64;
        }
        let mut batch = [0u8; 2048];
        let mut used = 0;
        while tail < head {
            let slot = &EVENTS[(tail % EVENT_RING as u64) as usize];
            if slot.seq.load(Ordering::Acquire) != tail + 1 {
                break;
            }
            let event = RawEvent {
                tick: slot.tick.load(Ordering::Relaxed),
                code: slot.code.load(Ordering::Relaxed) as u8,
                a: slot.a.load(Ordering::Relaxed),
                b: slot.b.load(Ordering::Relaxed),
            };
            tail += 1;
            line.clear();
            let _ = describe(&event, &mut line);
            let _ = line.write_char('\n');
            let text = line.as_bytes();
            if used + text.len() > batch.len() {
                append_log(&batch[..used]);
                used = 0;
            }
            batch[used..used + text.len()].copy_from_slice(text);
            used += text.len();
        }
        if used > 0 {
            append_log(&batch[..used]);
        }
    }
}

extern "C" fn writer_main(_: *mut skyline::libc::c_void) {
    writer_run();
}

fn start_writer() -> bool {
    unsafe {
        let thread = WRITER_THREAD.0.get().cast::<nn::os::ThreadType>();
        let priority = nn::os::GetThreadPriority(nn::os::GetCurrentThread());
        let created = nn::os::CreateThread(
            thread,
            writer_main,
            core::ptr::null_mut(),
            WRITER_STACK_MEMORY.0.get().cast::<u8>(),
            WRITER_STACK,
            priority,
        );
        if created != 0 {
            return false;
        }
        nn::os::SetThreadNamePointer(thread, c"tile-cursor-log".as_ptr().cast());
        nn::os::StartThread(thread);
        true
    }
}

unsafe fn guard_matches(site: &Site) -> bool {
    let base = getRegionAddress(Region::Text) as usize;
    let end = getRegionAddress(Region::Rodata) as usize;
    let address = base + site.offset;
    address.is_multiple_of(4)
        && address + site.guard.len() <= end
        && core::slice::from_raw_parts(address as *const u8, site.guard.len()) == site.guard
}

fn version_is_supported() -> bool {
    let mut version = nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { nn::oe::GetDisplayVersion(&mut version) };
    let end = version.name.iter().position(|v| *v == 0).unwrap_or(16);
    version.name[..end] == *model::SUPPORTED_VERSION.as_bytes()
}

/// Appends the install line to `events.log` (trimmed when over 1 MiB) and
/// the Skyline log.
fn report_install(report: &mut Line<256>) {
    let _ = write!(
        report,
        " heap_allocations={}",
        HEAP_ALLOCATIONS.load(Ordering::Relaxed)
    );
    skyline::logging::log(core::str::from_utf8(report.as_bytes()).unwrap_or(""));
    let _ = report.write_char('\n');
    for directory in ["sd:/ultimate", DATA_DIR] {
        if let Some(directory) = path(format_args!("{directory}")).with_nul() {
            unsafe { nn::fs::CreateDirectory(directory.as_ptr()) };
        }
    }
    if let Some(mut log) = NxFile::append(&mut path(format_args!("{DATA_DIR}/events.log"))) {
        if log.offset > EVENTS_LOG_LIMIT {
            log.truncate();
        }
        log.write(report.as_bytes());
    }
}

#[skyline::main(name = "ssbu-tile-cursor")]
pub fn main() {
    let mut report = Line::<256>::new();
    let _ = write!(
        report,
        "{} plugin=ssbu-tile-cursor/{}",
        tick(),
        env!("CARGO_PKG_VERSION")
    );
    if !version_is_supported() {
        let _ = write!(report, " result=unsupported_version");
        report_install(&mut report);
        return;
    }
    // Every site is either hooked or a layout the hooks rely on: all or none.
    if let Some(site) = model::SITES
        .iter()
        .find(|site| !unsafe { guard_matches(site) })
    {
        let _ = write!(report, " result=guard_mismatch site={}", site.name);
        report_install(&mut report);
        return;
    }
    skyline::install_hooks!(tile_cursor_css, tile_cursor_sss, tile_cursor_query);
    let _ = write!(report, " hooks=css,sss,cursor_query");
    // The writer appends to the same log, so it starts only after this line
    // (and the folder) exist.
    report_install(&mut report);
    if !start_writer() {
        report.clear();
        let _ = write!(report, "{} writer=not_started", tick());
        report_install(&mut report);
    }
}
