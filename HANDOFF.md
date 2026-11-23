# Handoff: ssbu-tile-cursor, branch `tile/cpu-coin` (2026-11-22)

## CPU coin work

This branch starts at `236c98d`. `docs/design/cpu-coin.md` describes the design;
the requirement trace is in `tile-cursor-state/gate-b110fda/DESIGN-TRACE.md`.
The red tests include the scm1 Random coin and empty-hand
banner fixtures, and three old-reader goldens from the base. The observed
Training owner publishes the 13 appended coin fields and permits an empty
hand to target the game's offered coins; standalone movement stays unchanged.
The CPU's at-rest Random coin was measured. Its held state and placement on
a fighter are analogies, not scm1 measurements.
CPU fields are qualified by local seat 0 on Training's two-seat select; P1
coin fields retain their original scope. The active player byte must equal 1.
The fighter select's internal state is not read; empty-hand sub-screens have
not been checked at runtime.

Finding 1's assertion-level red is `1923f6f`; the CPU scope and active-byte
fix is `34b2dc2`. No Eden proof, release, card install or push is authorized
by this work. Gate and compliance receipts are under
`smash-training-lobby/artifacts/phase23/tile-cursor-state/gate-<sha>/`;
see the external `setup-calls/tile/HANDOFF-tile-cpu.md` for current status.
The following sections describe the inherited `tile/state-moving` baseline.

## Branch and commits

- `tile/state-moving`, based on `tile/random-panels` 75a90e3, which is based on `tile/css-sss` 2f0a56c.
- Behaviour commits: 66bad13 (pad input) and the state fix of 2026-11-22 that follows 908ba92 (item 4 below).
  The commits between them change comments, docs and the Cargo feature list only. Not pushed, not merged, no tag.

## What the branch adds to 2f0a56c

1. 75a90e3: Random Battlefield and Random Omega qualify on the ordinary stage select (their native preview forms
   are 1 and 2).
2. d24a61f: five state lines after the 38 existing ones: `phase`, `target.at`, `target.kind`, `target.name`, and
   (9f828a5) `blocked`.
3. 9f828a5 to 66bad13: the pad's 16 newest samples are read at each update, so a chord shorter than the gap
   between two updates can still step. Every press among the samples taken gets one outcome in that update: the
   first press while the hand may step is the step (`to=<n>`, or `to=none` at an edge); every other press is
   counted and logged as `blocked`.
4. State fix after bridge load 8 (two defects of the published block):
   - A sample the owner stamped after the publisher's clock reading was published as inactive
     (`observation.active=0`, every field unknown) for one publish, about 100 ms. Such a sample is newer than the
     reading; it now counts as age 0. The 0.3 s staleness bound is unchanged.
   - A fighter whose panel carries a variant flag (`panel+0x6be` / `+0x6bf`) was never qualified. The game rebuilds
     the panel with `+0x6be = 1` for an echo fighter when the fighter select is re-entered, so a placed Richter
     read `p1.token=unknown` after Training. The plugin now applies the game's own lookup rule (`.text+0x19f1fa0`):
     a set flag selects the tile's alternate word only when that word is a valid typed hash; otherwise the fighter
     is the tile's default and qualifies.

## The `blocked` line (TC-BLOCKED-LOG)

- The log line `blocked screen=… dir=… presses=<n>` and the state line `blocked=<n>` carry the restriction tag
  `TC-BLOCKED-LOG` (`src/model.rs`, the two lines that write it; `docs/RESTRICTIONS.md`).
- They log an existing rule: an L3 + direction press does nothing while the hand may not step, or once that
  update has taken its one step. They add no restriction and change no input. The label was not renamed.
- The tag follows the primary's compliance ruling of 2026-11-22; it is not a separate operator approval.

## Release NRO

- The release NRO of the head commit, with its size, SHA-256 and build ID, is recorded in that commit's gate
  folder (`tile-cursor-state/gate-<sha7>/nro-identity.json`).
- The previous release (66bad13 to 908ba92, byte-identical): 98,304 bytes, SHA-256
  `21f6b629767764216e9cb9b1b75975c89cbc1909b93ec8cc0864126b1ba03179`. It has the two defects of item 4.

## Evidence

Under `smash-training-lobby/artifacts/phase23/`:

- `tile-cursor-random-panels/`: `capture-1` (native preview records), `gate-75a90e3`, `eden-check-75a90e3`.
- `tile-cursor-state/gate-*`: one folder per green commit, each with its red receipts, test, clippy, no-default
  and release logs, the NRO and the compliance reports.
- `tile-cursor-state/eden-check-d24a61f/` (boots ts1, ts2): phase and target lines on every published block.
- `tile-cursor-state/top-edge-d24a61f/te1`: 108 route presses on d24a61f, 14 without an outcome.
- `tile-cursor-state/top-edge-9f828a5/te2`, `te3`: the same presses, 108 of 108, on two hosts.
- `tile-cursor-state/top-edge-03bd88c/te4`, `top-edge-66bad13/te5`: 108 of 108 and the `blocked` path; te5 is on
  the previous release NRO.
- `tile-cursor-state/load8/`: the analysis of the two load-8 defects and the Eden check of item 4.
- Reviews are in `INBOX-opus-rc6-integration.md` (review-games-astra-3, 2026-11-22 08:43:02Z and 09:01:33Z).

## Live state

- No Eden run and no reservation belong to this branch.
- The NRO is offered to the bridge's Eden loads only. It is not on the Switch card.

## Known limits

- A seat or layout change clears `carried.*` in the same block.
- `events.log` is written through a 64-event ring and can drop lines; the state counters `ack`, `applied` and
  `blocked` are the record.
- A chord that is no longer among the 16 newest samples at the next update is not seen. In te4 and te5 Eden
  delivered about 16.7 samples per displayed frame; no other host and no console was measured.
- After a missing view, a new hand, a pad change, or on the first update in which the hand may step again, only
  the newest sample is used.
- Two presses inside one update give one step and one `blocked`.
- A panel that resolves to its tile's alternate word (Pyra/Mythra's second fighter) is not qualified: the token
  and the fighter identity read unknown there.
- A block is inactive when the newest sample is older than 0.3 s at the publisher's clock reading, or when the
  owner callback had no view. That is the only meaning of `observation.active=0`.
- On the ordinary fighter select the state block has `seat` and `layout` unknown (seen in boot ts1; not
  investigated).
- Stage-select panels 4 to 6 of the ordinary grid are modelled in the host tests, not captured.

## Open

- Compliance: the checker exits 0 (`tile-cursor-state/gate-16d95d6/compliance-final/`, with the design trace
  `DESIGN-TRACE.md`). Four manual rows are recorded there as not met: the scoring scripts beside the evidence have
  no unit tests; one early build shared a target directory with another source copy; some early local commands
  ran without the memory and time guard; no model watcher was armed. Whether they stand in the way of a card
  install is the primary's decision.
- The route macros hold L3 + direction for two frames; a longer hold is a playbook decision, not made here.
- The README clips predate this branch.

## Next commands

```text
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
cargo skyline build --release
cargo skyline build --release --features diag
python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py . --nro <release NRO> --base 236c98d --trace <gate>/DESIGN-TRACE.md
```

## CPU coin take

L3 + B takes the offered CPU coin into P1's empty hand on the captured two-seat
fighter select; the game's B remains unchanged outside the scope. The new
bridge counters are `take.ack` and `take.applied`. Console pad-id recognition,
the CPU-held analogy and the fighter select's internal state are not measured;
the game's Back hold does not see the B of an L3 + B chord (`.text+0x1a2d760`),
so a full hand makes L3 + B a no-op also with Instant Back loaded; that pairing
is not run. Evidence and the requirement trace are under
`artifacts/phase23/tile-take/`.

- Code review of `e68d8ad` passed on 2026-11-22 (23:48:50Z in the inbox). The Back-hold change (`0353c5a`) is
  with the reviewer. No Eden proof yet: the rows `artifacts/phase23/tile-take/rows-tile-take.json` (tk00 to
  tk06 and tk04b) are handed to the combined run.
- Decided by the primary on 2026-11-23: the scope stays Training's fighter select; with an empty hand and no
  offered CPU coin the combination does nothing and never takes P1's own coin back; with a full hand L3 + B
  does nothing, so the game's Back hold does not see its B.
- User page: `docs/TAKE-CPU-COIN.md` and the README section "Take the CPU's coin: L3 + B", on the local branch
  `review/cpu-coin-docs` (docs only, on top of `0353c5a`; the NRO is that commit's). The clip and the Eden date
  are added there once the rows have passed.
