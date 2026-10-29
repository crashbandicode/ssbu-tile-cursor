# Eden test procedure

This procedure checks the plugin without screenshots. Every step writes a line
to `events.log`, and the game's own record of which tile P1 is hovering is
logged next to the plugin's.

## Build under test

| Item | Value |
|---|---|
| NRO | `dist/libssbu_tile_cursor.nro` (a copy of `target/aarch64-skyline-switch/release/libssbu_tile_cursor.nro`) |
| sha256 | `24de3f122799d5942500e1d8beeac2e32e8a6109e508faec760bd2232fefba0e` |
| Game | 13.0.5 |
| Install to | `<eden sdmc>/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_tile_cursor.nro` |
| Log | `<eden sdmc>/ultimate/ssbu-tile-cursor/events.log`. It is appended in batches every 100 ms and closed after each batch, so the host file is current. |

## Input commands

The commands below use the Eden control API's `input.button`. Use these button
names: `lstick` (L3), `up`, `down`, `left`, `right`, `a`.

**Press L3 before the direction.** A D-pad press that arrives before L3 is an
ordinary D-pad press, and it moves the hand as in the unmodified game.

The existing driver does this when L3 is listed first:

```text
drive.py <a|b> press lstick,right 150    # presses lstick, then right, holds 150 ms, releases both
```

With `edenctl.py` directly:

```text
edenctl.py --socket <sock> button lstick --state press --lease-ms 3000
edenctl.py --socket <sock> button right --state tap --duration-ms 150 --lease-ms 1000
edenctl.py --socket <sock> button lstick --state release
```

Wait about 0.6 s between steps. The hand eases over about 8 game frames, which
takes longer when Eden runs below 60 fps. The `settled` line appears once the
hand has stopped.

## Steps and expected log

A tick is 1/19.2 MHz, so 250 ms is about 4,800,000 ticks. Tile indices are
positions in the game's tile vector: consecutive within a row on the CSS.

1. **Boot to the title or main menu.** The first new line is:
   ```text
   <t> plugin=ssbu-tile-cursor/0.1.0 hooks=css,sss,cursor_query heap_allocations=0
   ```
   If it reads `result=guard_mismatch site=<name>` or `result=unsupported_version`,
   stop and report the line.

2. **Open the CSS** (Smash, or Training), using the usual route.
   ```text
   <t> screen=css tiles=<n> heap_allocations=0
   ```
   `<n>` is the number of fighter tiles, including Random. It depends on the
   DLC on that save.

3. **Unmodified D-pad.** Run `press right 150` with no L3. The hand moves the
   usual small D-pad distance, and no `step` line is written.

4. **First step from the starting position.** The hand starts on the P1 token,
   below the grid. Run `press lstick,up 150`.
   ```text
   <t> step screen=css dir=up from=none to=<k> x=.. y=..
   <t> settled screen=css at=<k> hovered=<k> x=.. y=..
   ```
   The `x`/`y` in `settled` equal those in `step`.

5. **Single steps right.** Run `press lstick,right 150` three times. Expect
   `from=<k> to=<k+1>`, then `from=<k+1> to=<k+2>`, then `from=<k+2> to=<k+3>`,
   each followed by `settled ... at=<to> hovered=<to>`. At the end of a row the
   line is `to=none`: the hand does not move and there is no `settled` line.

6. **Vertical steps.** Run `press lstick,up 150`, then `press lstick,down 150`.
   The first `to` is in the row above, and the second returns to the previous
   tile's index.

7. **Held repeat.** Run `press lstick,left 1100`. Expect 5 `step` lines (at
   about 0, 250, 500, 750 and 1000 ms, so tick gaps of about 4.8 M), each with
   `to` one less than the line before, until the row starts (`to=none`).
   Whether a held step gets its own `settled` line depends on the frame rate:
   at 60 fps the hand settles in about 8 frames, before the next repeat. A step
   that retargets an easing hand cancels the earlier `settled`. The last tile
   reached always gets one.

8. **CPU token.**
   1. Add a CPU in slot 2.
   2. Move P1's hand onto the CPU's token with steps, or with the existing
      route, and press `a` to pick the token up.
   3. Run `press lstick,right 150`. The `step` and `settled` lines are as in
      step 5, and the token moves with the hand.
   4. Press `a` to drop the token.

   The CPU panel now shows the fighter at tile `<to>`. Optionally take one
   screenshot to confirm this. `hovered` is P1's slot hover, which follows the
   hand.

9. **Stage select.** Pick a fighter and continue to stage select.
   ```text
   <t> screen=sss tiles=<m> heap_allocations=0
   ```
   Repeat steps 4–7 on this screen. The lines read `screen=sss`. `hovered` is
   always `none` here, because the game keeps the stage hover in Lua; `at` is the
   game's rectangle under the hand and must equal `to`. Switching the stage form
   tab or page does not change `tiles`; the vector is the one the game
   hit-tests. Report any step whose `to` is an off-screen tile.

10. **Nothing outside the two screens.** In a match, run `press lstick,right 150`.
    No `step` line is written and the taunt plays as usual.

## Pass criteria

- Step 1 shows `hooks=css,sss,cursor_query`.
- Every `screen` line shows `heap_allocations=0`.
- Each `step` with `to=<i>` is followed by `settled ... at=<i>`, with
  `hovered=<i>` on the CSS. In a held repeat, at least the final step is.
- Consecutive single steps in one direction visit consecutive tiles, and none
  are skipped.
- The unmodified D-pad check (step 3) writes no `step` line.

Report the full `events.log` with the run.
