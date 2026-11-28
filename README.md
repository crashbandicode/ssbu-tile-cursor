# ssbu-tile-cursor

> **Disclaimer:** this project was developed with the assistance of AI
> (Claude, Codex and other coding agents) under the operator's direction and
> review. Nothing is claimed beyond the dated tests cited below.

Skyline plugin for Super Smash Bros. Ultimate 13.0.5: on the character and
stage select screens, **L3 + D-pad** moves P1's hand exactly one tile. Built
for automation, where stick input overshoots.

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/ac88358ed16d6e42af50940aba2ee6989aa9fdde" aria-label="Stage select: four L3 + Right steps move the hand from the Training tile to Final Destination, then L3 + Down moves it one row to Jungle Japes" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/stage-select.mp4)

*Training's stage select: four L3 + Right steps from the Training tile land on
Final Destination; L3 + Down then moves one row, to Jungle Japes.*

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/172a3dd51b8ac7f4eff4f860f2019965457f942f" aria-label="Character select: P1&#x27;s hand carries its token from Sephiroth right to Kazuya and up to Incineroar, and A places it" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/character-select-p1.mp4)

*P1 carries its token from Sephiroth: L3 + Right twice to Kazuya, L3 + Up to
Incineroar, then A places it.*

<video src="https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/media/blob/523420529a8a0ea7e37d2f11c5e40f7740e9f7b9" aria-label="Character select: P1&#x27;s empty hand takes the CPU&#x27;s coin from Random, steps it left to Mii Gunner, back right to Random and up to Terry, and drops it" autoplay muted loop playsinline controls></video> [Open the clip](docs/media/character-select-cpu.mp4)

*With P1's token placed, L3 + B takes the CPU's coin from Random into P1's
empty hand. L3 + Left carries it to Mii Gunner, L3 + Right back to Random and
L3 + Up to Terry, then A drops it.*

All three clips: Eden, 2026-11-28, run `filmx1-b1-yoga-2`, at 60 frames per
second, with build 0.2.1 (`0b34f454…`), which is newer than the release
below, and the test card's other plugins loaded. The game shows no input on
these screens. The steps above are the ones in the plugin's own log for each
take.

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

[v0.1.0+frame-repeat.1](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/tag/v0.1.0+frame-repeat.1) (2026-11-18) lets P1 hold L3 + D-pad to repeat tile steps every 15 screen updates.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
|---|---|---|---|
| Switch firmware | 23.5.0 | — | — |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Atmosphère | [1.12.2](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [intpa/atmosphere-ro-cleanup-p1](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Frees resources after a module fails to load. |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | [intpa/skyline](https://git.syntaxbyexample.com/intpa/skyline) | Protects the log queue when threads write together. |

### What to copy

1. Fully close Smash.
2. Download [libssbu_tile_cursor.nro](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/download/v0.1.0+frame-repeat.1/libssbu_tile_cursor.nro) and copy it to `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_tile_cursor.nro`.
3. Keep `manifest.json` and `SHA256SUMS.txt` on your computer. Relaunch Smash.

**Update** — Close Smash and replace the existing `libssbu_tile_cursor.nro` at the same path. Keep other plugins.

**Uninstall** — Close Smash, remove `libssbu_tile_cursor.nro` from that path and relaunch.

### What changed

| Change | Details |
|---|---|
| Holding L3 + D-pad repeats the first step every 15 screen updates. | [Guide](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/src/tag/v0.1.0+frame-repeat.1/README.md) |

### Tested on

| Feature | Hardware | Eden |
|---|---|---|
| Held repeat every 15 screen updates | No recorded test on this build | No recorded test on this build |

### Known limits

On character select, P1 must hold a token. Press B to take P1’s token back. The cursor stops at grid edges. [Details](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/src/tag/v0.1.0+frame-repeat.1/README.md)

<details>
<summary>Earlier release · v0.1.0</summary>

[v0.1.0](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/tag/v0.1.0) (2026-10-29) lets P1 press L3 + D-pad to move one tile on character and stage select.

### What you need

| Thing | Upstream | Our fork | What the fork changes |
|---|---|---|---|
| Switch firmware | [latest](https://en-americas-support.nintendo.com/app/answers/detail/a_id/22525) | — | — |
| Super Smash Bros. Ultimate | 13.0.5 | — | — |
| Atmosphère | [latest](https://github.com/Atmosphere-NX/Atmosphere/releases/latest) | [intpa/atmosphere-ro-cleanup-p1](https://git.syntaxbyexample.com/intpa/atmosphere-ro-cleanup-p1) | Frees resources after a module fails to load. |
| Skyline | [latest](https://github.com/skyline-dev/skyline/releases/latest) | [intpa/skyline](https://git.syntaxbyexample.com/intpa/skyline) | Protects the log queue when threads write together. |

### What to copy

1. Fully close Smash.
2. Download [ssbu-tile-cursor-v0.1.0-sd.zip](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/download/v0.1.0/ssbu-tile-cursor-v0.1.0-sd.zip) and unzip it on your computer. Copy its `atmosphere` folder to the SD root, merging folders. Keep `SSBU-TILE-CURSOR-README.txt` on your computer.
3. Check that the plugin landed at `sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_tile_cursor.nro`. The separate [libssbu_tile_cursor.nro](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/releases/download/v0.1.0/libssbu_tile_cursor.nro) is the same plugin for manual installation at that path.
4. Keep `manifest.json` and `SHA256SUMS.txt` on your computer. Relaunch Smash.

**Update** — Close Smash and replace the existing `libssbu_tile_cursor.nro` at the same path. Keep other plugins.

**Uninstall** — Close Smash, remove `libssbu_tile_cursor.nro` from that path and relaunch.

### What changed

| Change | Details |
|---|---|
| Press L3 + D-pad to step one stage tile. | [Guide](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/src/tag/v0.1.0/README.md) |
| Carry P1’s token or a CPU coin one fighter tile at a time. | [Guide](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/src/tag/v0.1.0/README.md) |

### Tested on

| Feature | Hardware | Eden |
|---|---|---|
| One-tile stage steps | No | Yes |
| Character steps carrying P1’s token or a CPU coin | No | Yes |

### Known limits

On character select, P1 must hold a token. Press B to take P1’s token back. The cursor stops at grid edges. [Details](https://git.syntaxbyexample.com/intpa/ssbu-tile-cursor/src/tag/v0.1.0/README.md)

</details>

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
