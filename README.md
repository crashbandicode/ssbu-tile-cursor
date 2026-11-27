# ssbu-tile-cursor

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

Skyline plugin for Super Smash Bros. Ultimate 13.0.5: on the character and
stage select screens, **L3 + D-pad** moves P1's hand exactly one tile. Built
for automation, where stick input overshoots.

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/339627c2c86d9da7cc54695359591c47357ef470" aria-label="Stage select: four L3 + Right steps move the hand from the Training tile to Final Destination, then L3 + Down moves it one row to Jungle Japes" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/stage-select.mp4)

*Training's stage select: four L3 + Right steps from the Training tile land on
Final Destination; L3 + Down then moves one row, to Jungle Japes.*

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/2298bc26e4909a902ac6815d4417c9302b0e5b5e" aria-label="Character select: P1&#x27;s hand carries its token from Mii Swordfighter up to Hero and right to Terry, and A places it" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/character-select-p1.mp4)

*P1 carries its token from Mii Swordfighter: L3 + Up to Hero, L3 + Right twice
to Terry, then A places it.*

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/10d7f5860466751f71faaca107a883b175a88b2d" aria-label="Character select: P1&#x27;s empty hand picks up the CPU&#x27;s coin on Random, steps it left three tiles and up one to Joker, and drops it" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/character-select-cpu.mp4)

*With P1's token placed, its empty hand picks up the CPU's coin on Random (A),
carries it to Joker (L3 + Left three times, L3 + Up) and drops it (A).*

All three clips: Eden, 2026-11-13, the v0.1.0 release NRO (`b8313ca2…`, build
`15d3f31`). The game shows no input on these screens, so the input labels are
drawn on the clips from the run's own input log.

## Behaviour

- At most one step per update. Holding the combination repeats every 15 game
  frames (250 ms at 60 fps). The cursor stops at the edges.
- Recent pad samples are read only between consecutive updates of the same hand
  on the same pad source: after a missing view, a new hand or a pad change, and
  on the first update in which the hand may step again, only the newest sample
  counts. Each press among the samples taken gets one outcome in that update;
  nothing is carried over. Two taps within one update therefore give one step
  and one `blocked`, never a deferred second step. Extra presses and presses
  while the hand may not step are logged as `blocked … presses=<n>` and counted
  in `blocked=` (see docs/RESTRICTIONS.md: this logs an existing rule). Mixed
  directions use `dir=mixed presses=<n> up=<n> …` with only non-zero directions
  listed. The state counters (`ack`, `applied`, `blocked`) are authoritative;
  `events.log` is written through a 64-event ring and is best-effort.
- Character select: tile steps work while the hand carries a token (P1's or a
  CPU's). With a registered bridge on Training's two-player fighter select,
  an empty hand can use L3 + Left/Right to target the previous/next coin the
  game offers. Up/Down are blocked. Standalone movement is unchanged.
  Not read: the fighter select's internal state. Not checked at runtime: sub-screens opened with an empty hand (the CPU level slider, the name list, the rules).
- On Training's two-seat fighter select, L3 + B takes an offered CPU coin into
  P1's empty hand. With a coin already held the combination does nothing. L3
  must be down when B goes down for the take. While L3 is held there, and
  until that B is released, the game's Back hold does not see B: L3 + B never
  leaves the fighter select, however long it is held. B without L3 is the
  game's.
- State block: `observation.active=0` means the newest sample is older than
  0.3 s or the screen has no P1 view; a sample is never inactive merely because
  it was stamped after the publisher read the clock. A placed or hovered fighter
  is identified by the game's own lookup: a set variant flag on the panel selects
  the tile's alternate only if the tile has one (Pyra/Mythra), so an echo fighter
  restored on re-entry (Richter after Training) reads `placed` with its name.
- Without L3 the D-pad and stick behave as in the unmodified game. Nothing
  happens outside these two screens.

## Install

Tested on: Super Smash Bros. Ultimate 13.0.5 · Switch firmware 23.5.0 · Atmosphère 1.12.2

Copy the approved `libssbu_tile_cursor.nro` to
`sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`. The plugin
checks 13.0.5 and the native movement sites before installing movement. Its
optional bridge observations have additional guards. Each step is logged to
`sd:/ultimate/ssbu-tile-cursor/events.log` after the tick.
When the take and Back-hold hooks are installed (both or neither) the install line includes `take=1`.

```text
<tick> step screen=css dir=left from=85 to=84 token=p2 x=288.0 y=-77.0
<tick> settled screen=css at=84 hovered=84 x=288.0 y=-77.0
```

The accepted frame-repeat release `66f00442…` is installed on the Switch card.
The clips above use the earlier [v0.1.0 release](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/tag/v0.1.0).

## Optional bridge state

A compatible bridge can register Tile Cursor as `tile.cursor/1`. The writer
polls for the export at most once per second for the first 30 seconds. The
startup hook installer chooses the observed CSS/SSS callbacks only if the
first poll registered successfully and the full 13.0.5 build note matches. A
later registration can expose an inactive block. The standalone callbacks
continue to handle cursor movement when initial registration is absent.

The provider distinguishes movement attempts (`ack`), blocked chords (`blocked`), successful target
writes (`applied`), and the handled step that reached its native hover
(`settled.ack`). `p1.carried` is the native slot-zero hand's carried token.
The final two lines are `take.ack` (L3 + B presses) and `take.applied`
(CPU-coin takes); both are cumulative for the boot.
An L3 + direction that cannot move logs `step … to=none` and publishes
`result=edge` with `ack` advanced and `applied` unchanged.
`phase` is `unknown`, `moving`, `resting` or `settled`. `target.at`,
`target.kind` and `target.name` identify the tile at the native target point
and its stock identity, not the live preview.
On CSS, `kind`, `name` and `form` require a settled carried P1 token and
agreement between its native hover, the default tile identity and the
slot-zero panel. The qualified default Steve preview is app fighter 88.
`p1.token` and `p1.placed.*` describe the native seat-zero medal. The
placed identity also requires its native index, tile rectangle and
committed panel flags to agree. The qualified Training and guest slot assignments report controller 0 and native
seat 0 or 1 from the direct input objects. Guest movement and local token state
remain unqualified, and the guest cannot inherit native P1 geometry. For the captured offline Training CSS, `layout=css.9f1fb03bf7c98fb9`
requires an exact match of the hand bounds and every tile's ordered primary and
alternate identity and rectangle. A fresh native P1 hand or committed token
supplies tile-local focus. The native foreground and input-owner check from
`ssbu.ui` is also required. The sole qualified below-grid entry is (-599,-236)
with native hover -1; an Up step reaches default Steve, index 78. See
[topologies/training-css.json](topologies/training-css.json).

After the existing 43 lines, `hand.token` reports the coin the game points
at for a resting empty hand; `p1.coin` / `p1.coin.at` report P1's own coin
independently of the hand, and `cpu.token` / `cpu.placed.*` report the CPU
coin. `carried.*` identifies the qualified fighter under either carried coin.
A seat or layout change clears `carried.*` in the same block.
`target.token` and `token.settled` identify and confirm an empty-hand token
step. An edge advances `ack` without `applied`. When both coins overlap, the
game may point at P1's coin instead; `token.settled` reflects that decision.
The earlier `p1.token` and `placed.*` lines retain their existing meaning.
CPU coin held and placed-on-fighter rules are analogies to P1, not measured
by scm1; the at-rest Random coin was measured.

The independent `grid` field identifies a matching native vector. On normal
Training SSS, `grid=sss.9892b3a32e5c0164` identifies the captured vector and
clamp. [topologies/training-sss.json](topologies/training-sss.json) records
the observed rightward edges Training → Battlefield → Small Battlefield.
On the offline Training route, the active stage actor on the Normal tab must
agree with its single slot-zero native preview, typed tile, form and the current
hand. The covered panels are Training, Battlefield, Small Battlefield, Big
Battlefield and Final Destination. The ordinary Smash grid also covers the
three Random controls and those four normal stages. From a qualified rest, the
next applied step can report `sss.grid.moving` until the hand and live actor
agree on the settled tile. The runner independently checks the committed
Training stage scene. An ambiguous medal, another occupied preview slot or an
unexpected preview form clears actionable stage fields; Random Battlefield
and Random Omega require their native forms 1 and 2. A stale owner expires
after 300 ms.

## Build

```text
cargo test
cargo skyline build --release
```

Since `a052fd5` (2026-10-30), `main` builds as a cdylib only so the release
profile's LTO applies. Match the NRO hash against the receipt when reproducing
a dated run.

Hook sites: [docs/HOOKS.md](docs/HOOKS.md). Eden test procedure and results:
[docs/EDEN-TEST.md](docs/EDEN-TEST.md), [docs/EDEN-RESULTS.md](docs/EDEN-RESULTS.md).

## License

MIT
