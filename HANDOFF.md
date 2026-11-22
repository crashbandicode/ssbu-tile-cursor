# Handoff: ssbu-tile-cursor, branch `tile/state-moving` (2026-11-22)

## Branch and commits

- `tile/state-moving`, based on `tile/random-panels` 75a90e3, which is based on `tile/css-sss` 2f0a56c.
- Behaviour head: 66bad13. The commits after it change comments, docs and the Cargo feature list only
  (`git log 66bad13..`). Not pushed, not merged, no tag.

## What the branch adds to 2f0a56c

1. 75a90e3: Random Battlefield and Random Omega qualify on the ordinary stage select (their native preview forms
   are 1 and 2).
2. d24a61f: five state lines after the 38 existing ones: `phase`, `target.at`, `target.kind`, `target.name`, and
   (9f828a5) `blocked`.
3. 9f828a5 to 66bad13: the pad's 16 newest samples are read at each update, so a chord shorter than the gap
   between two updates can still step. Every press among the samples taken gets one outcome in that update: the
   first press while the hand may step is the step (`to=<n>`, or `to=none` at an edge); every other press is
   counted and logged as `blocked`.

## Release NRO

- 98,304 bytes, SHA-256 `21f6b629767764216e9cb9b1b75975c89cbc1909b93ec8cc0864126b1ba03179`,
  build ID `9e9b38df8faf7c26b56291d089da72e2d25f5644f2353abec0920fd8220fcd45`. Built from 66bad13; the later
  commits were gated to reproduce it byte for byte.

## Evidence

Under `smash-training-lobby/artifacts/phase23/`:

- `tile-cursor-random-panels/`: `capture-1` (native preview records), `gate-75a90e3`, `eden-check-75a90e3`.
- `tile-cursor-state/gate-*`: one folder per green commit, each with its red receipts, test, clippy, no-default
  and release logs, the NRO and the compliance reports.
- `tile-cursor-state/eden-check-d24a61f/` (boots ts1, ts2): phase and target lines on every published block.
- `tile-cursor-state/top-edge-d24a61f/te1`: 108 route presses on d24a61f, 14 without an outcome.
- `tile-cursor-state/top-edge-9f828a5/te2`, `te3`: the same presses, 108 of 108, on two hosts.
- `tile-cursor-state/top-edge-03bd88c/te4`, `top-edge-66bad13/te5`: 108 of 108 and the `blocked` path; te5 is on
  the release NRO above.
- Reviews are in `INBOX-opus-rc6-integration.md` (review-games-astra-3, 2026-11-22 08:43:02Z and 09:01:33Z).

## Live state

- No Eden run and no reservation belong to this branch.
- The NRO is offered to the bridge's Eden loads only. It is not on the Switch card.

## Known limits

- `events.log` is written through a 64-event ring and can drop lines; the state counters `ack`, `applied` and
  `blocked` are the record.
- A chord that is no longer among the 16 newest samples at the next update is not seen. In te4 and te5 Eden
  delivered about 16.7 samples per displayed frame; no other host and no console was measured.
- After a missing view, a new hand, a pad change, or on the first update in which the hand may step again, only
  the newest sample is used.
- Two presses inside one update give one step and one `blocked`.
- On the ordinary fighter select the state block has `seat` and `layout` unknown (seen in boot ts1; not
  investigated).
- Stage-select panels 4 to 6 of the ordinary grid are modelled in the host tests, not captured.

## Open

- Compliance: the reviewed sheet in the newest gate folder lists the manual rows nobody has verified yet. Clean
  compliance is required before a card install.
- The route macros hold L3 + direction for two frames; a longer hold is a playbook decision, not made here.
- The README clips predate this branch.

## Next commands

```text
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
cargo skyline build --release
cargo skyline build --release --features diag
python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py . --nro <release NRO> --base 75a90e3
```
