# CPU coin and empty-hand token steps

Base: `236c98d2ee1f1a1d7e313436ab7d9f15c718491d`. The wire contract is
`artifacts/phase23/setup-calls/WIRE.md`; implementation and probes follow
`artifacts/phase23/setup-calls/TILE-DESIGN.md`.

## Requirement trace

The requirement-trace table is kept beside the lane handoff in `artifacts/phase23/tile-cursor-state/gate-b110fda/DESIGN-TRACE.md` and copied into the current gate.

## State and inputs

On Training's two-player fighter select only, an active, enabled, shown P1
hand carrying no coin can select a candidate in slot order. The plugin bounds
slots by the two player entries, not the measured medal-array count of eight;
slots 2 to 7 were not captured.
Candidates use the game's flags, exact rest and pane equality, and the hand
area; no tolerance or stale-state smoothing is added. Up/Down and extra
presses remain blocked, and a missing neighbor is an edge.

Not read: the fighter select's internal state. Not checked at runtime: sub-screens opened with an empty hand (the CPU level slider, the name list, the rules).

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
