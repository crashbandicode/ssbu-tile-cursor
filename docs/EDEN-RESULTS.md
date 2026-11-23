# Eden results

Smash 13.0.5 in Eden, this plugin as the only Skyline plugin, driven through
the Eden control API.

## 2026-11-13, the README clips (the same NRO)

The v0.1.0 release NRO (`b8313ca2…`) in Training's stage and character selects,
with ARCropolis, `libnro_hook` and `libparam_hook` also loaded. Eden
`2f315fc4…`, run `pg-tile-1` on the Surface.

| Check | Result |
|---|---|
| Install | `hooks=css,sss,cursor_query heap_allocations=0` |
| Stage select | `tiles=116`; four L3 + Right steps went 0→1→2→3→4 (Training to Final Destination), L3 + Down 4→15; each with a `settled` line at its target |
| Character select, P1's token | `tiles=87`; L3 + Up from P1's panel went to 78 (Steve); from Mii Swordfighter, L3 + Up 84→73, then 73→74→75 (Terry), `token=p1`, hovered equal to each target; A placed it |
| CPU's coin | P1's empty hand picked up the coin on Random; L3 + Left ×3 and Up went 86→85→84→83→72 with `token=p2`, and dropping it set the CPU to Joker |
| A Mii tile | A on Mii Gunner opened the game's Mii list; B closed it, and the hand carried P1's token again (the next step logged `token=p1`) |

## 2026-10-29, build `15d3f31` (NRO sha256 `b8313ca2…934bc`)

| Check | Result |
|---|---|
| Install | `hooks=css,sss,cursor_query heap_allocations=0` |
| Stage select | Six L3 + Right steps from Random landed on Final Destination (tiles 0→6); Down and Up moved one row and back |
| Character select, P1's token | L3 + Up from P1's panel, then steps along the row; `token=p1`, the game's hover matched each target |
| Empty hand | With P1's token placed, L3 + Right wrote nothing and did not move the hand (intended) |
| CPU's coin | P1's hand picked up the Random CPU's coin; L3 + Left ×3 and Up stepped 86→85→84→83→72 with `token=p2`, the CPU panel previewed each fighter, and dropping it set the CPU to Joker |

## 2026-10-29, build `fcd1395` (first build)

Stage select and P1's own token worked as above. With P1's token placed, or
with a CPU's coin in the hand, nothing stepped: the plugin identified P1 by the
carried token's slot (player `+0x30`), and its hook was the fighter-picking
state, which stops running once P1 has chosen. Fixed in `15d3f31`.

## 2026-11-23: L3 + B takes the CPU's coin (build `0353c5a`)

Eden on the Yoga, run `rc15-ib-17060a5e-yoga-main-2` (the combined TrainingRooms run), bridge epoch
`0x974804ceed15fc33`. Tile Cursor release NRO sha256
`e7e946258b1f5d4634f45a8909f057a94f80903d158527bd906d56b9e6326678`, loaded with TrainingRooms, the bridge and
nro_hook. Every check read `tile.cursor/1`, the roster and the live fighters through the bridge. The rows ran as
same-boot retakes, and the reviewer read their receipts while Eden was up.

| Row | What was done | What the bridge showed |
| --- | --- | --- |
| tk00 | Quit Training | Training's fighter select |
| tk01 | L3 + B, empty hand | `take.ack` 0 to 1, `take.applied` 0 to 1, `p1.carried=1`, `cpu.token=held` |
| tk02 | L3 + B, the CPU's coin in the hand | the coin stayed held and nothing else changed |
| tk03 | L3 + D-pad to Fox, A | nine steps; the CPU's coin placed on Fox (tile 7); Mario's coin unchanged on tile 0 |
| tk04 | L3 + B, P1's own coin in the hand | `take.ack` 2 to 3, `take.applied` stayed 1, both coins unchanged |
| tk04b | L3 + B held 120 frames, P1's own coin in the hand | `take.ack` 3 to 4, `take.applied` stayed 1, both coins unchanged, the fighter select still up |
| tk05 | Mario placed, the CPU's coin on Fox, Training entered | Training loaded |
| tk06 | nothing | roster and live fighters: Mario and Fox |

The clip `docs/media/take-cpu-coin.mp4` is from the same boot (row `doc-tile-integration`): the take from Samus,
four steps and the placing on Fox.

Not run: Instant Back loaded together with the Tile Cursor; a console; a plain B hold as a control for tk04b.
