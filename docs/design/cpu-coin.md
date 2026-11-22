# CPU coin and empty-hand token steps

Base: `236c98d2ee1f1a1d7e313436ab7d9f15c718491d`. The wire contract is
`artifacts/phase23/setup-calls/WIRE.md`; implementation and probes follow
`artifacts/phase23/setup-calls/TILE-DESIGN.md`.

## Requirement trace

The 13:54:31Z and the 14:06:33Z approval entries below are the primary's
records in `artifacts/phase23/HANDOFF-PRIMARY-tr-primary-2.md`, not verbatim
operator quotations. The 14:06:33Z setup-time sentence is recorded there as
the operator's own words.

| Operator clause | Design | Test |
| --- | --- | --- |
| Primary record, 13:54:31Z: “CPU for strip fixtures: use the tile selector to pick the CPU at character select; also extend it to recognise TR character menus (cycles etc.) and select there; the bridge already reports the current CPU.” | The observed owner publishes P1 and CPU coin state and the carried fighter independently, including the game's chosen pickup token. TrainingRooms menu support is a separate lane. | `an_empty_hand_reports_the_token_it_points_at`, `the_cpu_medal_reports_its_tile_and_fighter`, `a_carried_cpu_coin_reports_its_hover_and_leaves_p1_placed` |
| Primary record, 13:54:31Z: “CPU for strip fixtures: use the tile selector to pick the CPU at character select; also extend it to recognise TR character menus (cycles etc.) and select there; the bridge already reports the current CPU.” | The observed owner steps an empty hand to the next or previous game-offered coin with L3 + Left/Right, writes the native cursor target, and reports which coin the game points at after settling. | `coins_are_offered_by_the_games_own_test`, `a_token_step_targets_the_coin_from_the_banner`, `an_empty_hand_token_step_targets_the_cpu_coin` |
| Primary record, 14:06:33Z: “7k tile macro hold 2->6 approved (fusion-g4 told).” | Keep one outcome per update and frame-based repeat for carried and empty hands; extra presses block instead of being deferred. | `a_six_frame_hold_steps_once`, `an_empty_hand_token_step_targets_the_cpu_coin` |
| Operator, 14:06:33Z: “I do not want most work being spent setting up tests so we need to improve things there” | Append self-verifying token, coin and carried-fighter fields to the existing block; keep the existing 43 lines byte-identical except for the requested step outcome semantics. | `an_empty_hand_reports_the_token_it_points_at`, `a_token_step_shows_in_the_old_lines_only_as_a_step`, `an_edge_token_step_is_an_edge`, `old_lines_are_unchanged_for_p1s_coin`, `old_lines_are_unchanged_for_an_empty_hand`, `old_lines_are_unchanged_for_a_carried_cpu_coin` |
| Operator, 14:06:33Z: “I do not want most work being spent setting up tests so we need to improve things there” | Bound publication to the bridge payload; guard every additional native layout read, validate pointers, and keep standalone hooks unchanged. | `the_longest_block_fits_its_buffer`, guard-head test, three old-reader pins, standalone hook diff |

## State and inputs

On Training's two-player fighter select only, an active, enabled, shown P1
hand carrying no coin can select a candidate in slot order. Slots are bounded
by the two player entries, not the measured medal-array count of eight.
Candidates use the game's flags, exact rest and pane equality, and the hand
area; no tolerance or stale-state smoothing is added. Up/Down and extra
presses remain blocked, and a missing neighbor is an edge.

The provider appends the 13 fields in `WIRE.md`. Token movement uses the
existing ack and applied counters; token settling is based on the game's
pointed slot and never impersonates carried-coin settling. The standalone
hooks pass no coin candidates, so their movement and `View`/`css_view` path
remain as at the base. New observations are guarded and use no heap or
blocking lock in hooks.

`measure/scm1/RESULT.md` measured slot 1 on Random and the P1 empty-hand
banner position. A CPU coin held in P1's hand and a CPU coin placed on a
fighter were **not** measured; their rules remain explicitly named analogies.
There is no new hook or user-facing refusal.

## Evidence and limits

Produce three first-43-line goldens from a disposable checkout of the base;
commit the named red probes before implementation. Extract guard bytes from
the owned 13.0.5 ELF, and run the base gate sequence and trace-bound
compliance. No Eden, Switch-card action, release or push is authorized by
this design. Runtime proof and independent review remain subsequent work.
