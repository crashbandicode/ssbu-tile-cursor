# Eden results

Smash 13.0.5 in Eden, this plugin as the only Skyline plugin, driven through
the Eden control API.

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
