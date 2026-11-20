//! Discovery and observed owner callbacks. `main` is the only hook installer.
#[path = "p1.rs"]
mod p1;
use super::*;
use crate::provider::{Api, Discovery, Header, Mailbox, Owner, Publisher, Registered};
use core::mem::MaybeUninit;

struct MainOwner(UnsafeCell<Owner>);
// SAFETY: only the two accepted, main-thread CSS/SSS callbacks use this value.
unsafe impl Sync for MainOwner {}
static OWNER: MainOwner = MainOwner(UnsafeCell::new(Owner::new()));
static MAIL: Mailbox = Mailbox::new();

/// Retained for this boot, since an exceptionally late first lookup may signal
/// after startup times out. This handle is never used from an input callback.
struct StartupEvent(UnsafeCell<MaybeUninit<nn::os::EventType>>);
unsafe impl Sync for StartupEvent {}
static FIRST_POLL: StartupEvent = StartupEvent(UnsafeCell::new(MaybeUninit::uninit()));
static INSTALL_FINISHED: StartupEvent = StartupEvent(UnsafeCell::new(MaybeUninit::uninit()));
static REGISTERED_AT_START: StartupEvent = StartupEvent(UnsafeCell::new(MaybeUninit::uninit()));

fn event() -> *mut nn::os::EventType {
    FIRST_POLL.0.get().cast()
}

pub(super) fn initialize() {
    unsafe {
        for e in [
            event(),
            INSTALL_FINISHED.0.get().cast(),
            REGISTERED_AT_START.0.get().cast(),
        ] {
            nn::os::InitializeEvent(e, false, nn::os::EventClearMode_EventClearMode_ManualClear);
        }
    }
}

pub(super) fn installation_reported() {
    unsafe { nn::os::SignalEvent(INSTALL_FINISHED.0.get().cast()) };
}

pub(super) fn wait_for_registration() -> bool {
    // The normal first lookup is immediately due. A delayed/failed writer
    // must never hold plugin loading indefinitely or patch a live hook.
    let completed = unsafe {
        nn::os::TimedWaitEvent(
            event(),
            nn::TimeSpan {
                nanoseconds: 100_000_000,
            },
        )
    };
    completed && unsafe { nn::os::TryWaitEvent(REGISTERED_AT_START.0.get().cast()) }
}

/// The owned ELF PT_LOAD maps file offset 0x4f17f3c to virtual 0x4f16f3c.
/// Validate name, type, length and all 20 bytes before any optional owner read.
pub(super) fn stock_build() -> bool {
    const NOTE: [u8; 36] = [
        4, 0, 0, 0, 20, 0, 0, 0, 3, 0, 0, 0, 71, 78, 85, 0, 0x21, 0x45, 0x0c, 0x64, 0x7b, 0x8c,
        0x59, 0x40, 0xed, 0x5b, 0x68, 0x21, 0x8e, 0xa1, 0x28, 0xfd, 0x3c, 0x5d, 0x59, 0x7d,
    ];
    unsafe {
        let addr = getRegionAddress(Region::Text) as usize + 0x4f16f3c;
        addr >= getRegionAddress(Region::Rodata) as usize
            && addr + NOTE.len() <= getRegionAddress(Region::Data) as usize
            && core::slice::from_raw_parts(addr as *const u8, NOTE.len()) == NOTE
    }
}

// The normal-stage preview uses the first typed u64 of the 0x18-stride stage record.
const SSS_KIND: Site = Site {
    name: "sss_typed_grid_kind",
    offset: 0x1b34584,
    guard: [
        0x18, 0x69, 0x6a, 0xf8, 0x28, 0xbf, 0x40, 0xf9, 0x00, 0x05, 0x40, 0xf9, 0x94, 0xf7, 0x5d,
        0x94, 0x1f, 0x9f, 0x40, 0xf2, 0x09, 0x1f, 0x48, 0x92, 0xe8, 0x07, 0x9f, 0x1a, 0x3f, 0x01,
        0x17, 0xeb, 0xea, 0x17, 0x9f, 0x1a, 0x5a, 0x01, 0x08, 0x0a, 0x08, 0x9c, 0x40, 0x92, 0xda,
        0x02, 0x00, 0x36, 0x0a, 0x1c, 0x48, 0x92, 0x5f, 0x01, 0x17, 0xeb, 0x04, 0x09, 0x40, 0xfa,
        0x60, 0x02, 0x00, 0x54,
    ],
};

/// Validate optional reader witnesses only after successful bridge registration.
pub(super) fn optional_mismatch() -> Option<&'static str> {
    p1::GUARDS
        .iter()
        .chain(core::iter::once(&SSS_KIND))
        .find(|site| !unsafe { guard_matches(site) })
        .map(|s| s.name)
}

pub(super) struct Worker {
    start: u64,
    discovery: Discovery,
    provider: Option<Registered>,
    publisher: Publisher,
}

impl Worker {
    pub(super) fn start() -> Self {
        let now = tick();
        let mut worker = Self {
            start: now,
            discovery: Discovery::default(),
            provider: None,
            publisher: Publisher::default(),
        };
        worker.discover(now);
        if worker.provider.is_some() {
            unsafe { nn::os::SignalEvent(REGISTERED_AT_START.0.get().cast()) };
        }
        unsafe {
            nn::os::SignalEvent(event());
            // Preserve install-line-before-data ordering of the accepted logger.
            nn::os::WaitEvent(INSTALL_FINISHED.0.get().cast());
        }
        worker
    }

    fn discover(&mut self, now: u64) {
        if !self.discovery.due(now.saturating_sub(self.start)) {
            return;
        }
        let mut symbol = 0;
        let rc = unsafe {
            nn::ro::LookupSymbol(&mut symbol, c"ssbu_hw_bridge_state_api_v1".as_ptr().cast())
        };
        if rc != 0 || symbol == 0 {
            return;
        }
        // Found once. Reject malformed exports and duplicate registration for the boot.
        self.discovery.finish();
        let get: unsafe extern "C" fn() -> *const Api = unsafe { core::mem::transmute(symbol) };
        let api = unsafe { get() };
        if api.is_null() || !(api as usize).is_multiple_of(core::mem::align_of::<Api>()) {
            return;
        }
        // Inspect only the prefix until struct_size proves the whole table exists.
        let header = unsafe { api.cast::<Header>().read() };
        if header.compatible() {
            self.provider = Registered::register(unsafe { &*api });
        }
    }

    pub(super) fn poll(&mut self) {
        let now = tick();
        if self.provider.is_none() {
            self.discover(now);
        }
        if let Some(provider) = self.provider {
            self.publisher
                .poll(now, MAIL.read(), |bytes| provider.send(bytes));
        }
    }
}

/// Compare stable stock fields directly while the registered owner holds the vector.
unsafe fn grid(screen: Screen, view: &View) -> Option<crate::layout::Layout> {
    let bounds =
        core::slice::from_raw_parts((view.cursor + cursor::BOUNDS as u64) as *const u8, 16);
    crate::layout::Layout::identify(screen, view.tiles.count, bounds, |i, kinds, rect| {
        let base = view.tiles.begin + i as u64 * view.tiles.stride;
        core::slice::from_raw_parts(base as *const u8, kinds.len()) == kinds
            && core::slice::from_raw_parts((base + view.tiles.rect) as *const u8, rect.len())
                == rect
    })
}

pub(super) unsafe fn observe(screen: Screen, object: u64, view: Option<View>) {
    let owner = &mut *OWNER.0.get();
    let now = tick();
    let Some(view) = view else {
        owner.begin(None, now);
        MAIL.offer(owner.latest);
        return;
    };
    let previous_token = owner.latest.p1_carried;
    let previous_seat = owner.latest.local_seat;
    let previous_grid = owner.latest.grid;
    owner.begin(
        Some((
            screen,
            object,
            view.cursor,
            view.tiles.begin,
            view.tiles.count,
        )),
        now,
    );
    // on_screen is the accepted movement implementation, called exactly once.
    // It is also the sole producer during this synchronous owner invocation.
    // Its STEP record is emitted after the native target write (or a blocked edge).
    let first = EVENT_HEAD.load(Ordering::Relaxed);
    on_screen(screen, object, &view);
    let last = EVENT_HEAD.load(Ordering::Relaxed);
    let sample = &mut owner.latest;
    let current = read_point(view.cursor + cursor::CURRENT as u64);
    let target = read_point(view.cursor + cursor::TARGET as u64);
    sample.at = tile_at(&view.tiles, current).map(|i| i as u16);
    sample.p1_carried = view.token.map(|i| i as u16);
    if sample.p1_carried != previous_token {
        // A placed or reacquired token begins a new action scope on the same layout.
        sample.step.new_owner();
    }
    let raw_hover = view
        .records
        .zip(view.token.and_then(model::css::hover_record))
        .map(|(p, off)| read_i32(p + off as u64));
    sample.hover = raw_hover
        .and_then(|n| usize::try_from(n).ok())
        .filter(|&i| i < view.tiles.count)
        .map(|i| i as u16);
    // A fresh pair of still owner frames is required even before the first step.
    let rest = (current == target).then_some((sample.generation, view.cursor, current));
    sample.resting = rest.is_some() && *REST.0.get() == rest;
    *REST.0.get() = rest;
    if last - first > 3 {
        // The accepted owner produces at most SCREEN, SETTLED, STEP. A violated
        // producer bound cannot certify counters or an input outcome.
        owner.begin(None, now);
        MAIL.offer(owner.latest);
        return;
    }
    for index in first..last {
        let slot = &EVENTS[(index % EVENT_RING as u64) as usize];
        let code = slot.code.load(Ordering::Relaxed) as u8;
        match code {
            EV_STEP => sample.step.emitted(slot.a.load(Ordering::Relaxed)),
            EV_SETTLED => {
                let record = slot.a.load(Ordering::Relaxed);
                // Require the original settle_token's native hover AND today's
                // current-token hover. Pickup may switch tokens during easing.
                if sample.at == Some((record >> 16) as u16)
                    && sample.hover == Some((record >> 32) as u16)
                {
                    sample
                        .step
                        .settled(sample.at, sample.hover, sample.p1_carried);
                }
            }
            _ => {}
        }
    }
    sample.grid = grid(screen, &view);
    if screen == Screen::Css {
        p1::collect(object, &view, sample);
        sample.hand_steppable = view.steppable;
        sample.css_entry = view.steppable
            && crate::layout::supported_css_entry(sample.grid, current, sample.at, raw_hover);
    }
    if sample.local_seat != previous_seat || sample.grid != previous_grid {
        sample.generation += 1;
        sample.step.new_owner();
        sample.resting = false;
        sample.fighter = None;
        sample.css_entry = false;
    }
    MAIL.offer(*sample);
}

type Rest = Option<(u64, u64, (f32, f32))>;
struct MainRest(UnsafeCell<Rest>);
// SAFETY: identical ownership to OWNER; no runtime worker touches cursor identity.
unsafe impl Sync for MainRest {}
static REST: MainRest = MainRest(UnsafeCell::new(None));
