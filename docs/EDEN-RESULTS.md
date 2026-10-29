# Eden results (2026-10-29)

Build `fcd1395`, NRO sha256 `24de3f12…fba0e`, Smash 13.0.5 in Eden, with
this plugin as the only Skyline plugin. Driven through the Eden control
API. The full `events.log` of the run is kept with the test evidence.

| Check | Result |
|---|---|
| Install | `hooks=css,sss,cursor_query heap_allocations=0` |
| Stage select | `screen=sss tiles=118`. Three L3+Right steps went 1→2→3→4, Down then Up went 4→15→4 (rows of 11), and six steps from Random landed exactly on Final Destination. Every step was followed by `settled` at the target. |
| Held repeat (stage select) | L3+Left held for 1.1 s stepped 4→3→2→1→0, one step per ~0.26 s, then `to=none` at the row start |
| Plain D-pad | Moved the hand as in the unmodified game; no `step` line |
| Character select | `screen=css tiles=87`. From P1's panel, L3+Up went to the tile above the hand (80, Pyra). Three rights went 80→81→82→83 (Kazuya, Sora, Mii Brawler), and Up/Down went 83→72→83. `hovered` equalled the target every time, and the panel previewed the hovered fighter. |
| Carried token | Steps work while the hand carries a token (P1's own at the start, or taken back with B): 83→72→71 landed on Piranha Plant |
| In a match | L3+Right wrote nothing |

## Gaps

- **Empty hand on the character select (intended).** After P1 placed its
  token ("Ready to fight" showing), L3+D-pad writes nothing and does not move
  the hand. Press B to take the token back; steps then resume.
- **CPU token not tested.** A Random CPU's token could not be picked up from
  its panel in this run, so moving a CPU's token was not exercised. The
  plugin moves the hand, and a carried token follows the hand, as P1's did.
