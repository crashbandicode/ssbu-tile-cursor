# ssbu-tile-cursor

Skyline plugin for Super Smash Bros. Ultimate 13.0.5: on the character and
stage select screens, holding L3 + a D-pad direction moves P1's hand exactly
one fighter or stage tile.

Stick input overshoots tiles in automation, because how far the hand travels
depends on how long the stick is held and on the emulator's frame pacing. With
this plugin a test presses L3 + a direction and the hand lands on the centre of
the neighbouring tile.

On the character select screen the hand steps while it carries a token: P1's
own token, or a CPU's token that P1's hand has picked up. The token moves with
the hand. Once P1 has placed its token and the hand is empty, L3 + D-pad does
nothing; press B to take the token back.

- The first step fires on the press. While the combination is held, it repeats
  every 15 game frames (250 ms at 60 fps), independent of host speed.
- Left and right stay in the row. Up and down go to the nearest tile in the next
  row. At an edge the hand does not move, and nothing wraps.
- If the hand is not on a tile, the first press moves it to the nearest tile in
  that direction.
- The tiles are the rectangles the game itself hit-tests the hand against, so
  the layout always matches what is on screen.
- The game's own hand easing performs the move, taking about 8 frames.

Without L3, the D-pad and stick behave as in the unmodified game. While L3 is
held and P1's hand can step, the plugin drops D-pad movement for that hand.
Its three hooks do nothing anywhere else.

## Install

Copy `libssbu_tile_cursor.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. The plugin
needs game version 13.0.5 and checks it at boot. It also checks every hook site
against the 13.0.5 bytes. If any check fails it installs nothing and says so in
the log.

## Log

Everything is written to `sd:/ultimate/ssbu-tile-cursor/events.log`:

```text
<tick> plugin=ssbu-tile-cursor/0.1.0 hooks=css,sss,cursor_query heap_allocations=0
<tick> screen=css tiles=<n> heap_allocations=0
<tick> step screen=css dir=right from=14 to=15 token=p1 x=-528.0 y=240.0
<tick> settled screen=css at=15 hovered=15 x=-528.0 y=240.0
```

| Field | Meaning |
|---|---|
| `from`, `to` | Indices in the game's tile vector. `none` means the hand was off every tile, or there was no tile in that direction. |
| `token` | CSS only: the slot of the token the hand carries. `p1` is P1's own token and `p2` is slot 2's token, for example a CPU's. |
| `x`, `y` | The target point, in layout units. |
| `settled` | Logged once the hand stops. `at` is the tile under the hand. `hovered` is the tile the game recorded for the carried token's slot (CSS only). |

A tick is 1/19.2 MHz.

## Optional bridge state

A compatible bridge can register Tile Cursor as `tile.cursor/1`. The writer
polls for the export at most once per second for the first 30 seconds. The
startup hook installer chooses the observed CSS/SSS callbacks only if the
first poll registered successfully and the full 13.0.5 build note matches. A
later registration can expose an inactive block. The standalone callbacks
continue to handle cursor movement when initial registration is absent.

The provider distinguishes movement attempts (`ack`), successful target
writes (`applied`), and the handled step that reached its native hover
(`settled.ack`). `p1.carried` is the native slot-zero hand's carried token.
On CSS, `kind`, `name` and `form` require a settled carried P1 token and
agreement between its native hover, the default tile identity and the
slot-zero panel. The qualified default Steve preview is app fighter 88.
`p1.token` and `p1.placed.*` describe the native seat-zero medal. The
placed identity also requires its native index, tile rectangle and
committed panel flags to agree. The qualified Training and guest slot assignments report controller 0 and native
seat 0 or 1 from the direct input objects. Guest movement and local token state
remain unqualified, and the guest cannot inherit native P1 geometry. The layout
identifier remains unknown. A stale owner expires after 300 ms.

## Build

```text
cargo test
cargo skyline build --release   # target/aarch64-skyline-switch/release/libssbu_tile_cursor.nro
```

[docs/HOOKS.md](docs/HOOKS.md) describes the hook sites and the game data the
plugin uses. [docs/EDEN-TEST.md](docs/EDEN-TEST.md) is the test procedure for
the Eden emulator.

## License

MIT
