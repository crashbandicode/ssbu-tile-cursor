# Take the CPU coin

On Training's two-seat fighter select, L3 + B takes the offered CPU coin
into P1's empty hand. L3 + D-pad then moves the coin as before, and A places
it. When the CPU coin is not offered, the combination does nothing. The
game's B handling is unchanged outside this scope.

The fighter-picking update reaches `cmn w21, #1` at `.text+0x1a14664` for a
B press with an empty hand. The inline hook changes only `w21` and `w9`:
the native path leaves both unchanged; a take sets `w21` to slot 1; an
unavailable coin sets `w21` to -1 and `w9` to 0. The game performs the
pickup. The scope uses the captured CSS layout and the existing empty-hand
coin offer. The hook is separately guarded and can install without a bridge.

`take.ack` counts P1's L3 + B press edges read on CSS, including no-ops.
`take.applied` counts the hook's CPU-coin takes. Both are cumulative for the
boot and are appended to `tile.cursor/1`.

Not measured on a console: local seat 0 is recognised by the pad ids Eden
reported; an unrecognised pad keeps the game's B. The game still sees B,
so holding L3 + B long enough leaves the fighter select as a held B does.
L3 must be down when B goes down. The fighter select's internal state is not
read. The held and fighter-placement rules for the CPU coin remain analogies,
not console measurements.

The requirement trace is kept beside the lane handoff at
`artifacts/phase23/tile-take/DESIGN-TRACE.md`.
