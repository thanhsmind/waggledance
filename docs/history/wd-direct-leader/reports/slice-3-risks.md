# Slice 3 risks consult — wd-direct-leader (2026-10-22)

Seat: hat-risks (fable), over `slice-3.md` and `slice-3-cells.json` before plan rev 2.
Seat confidence 88.

| Finding | Landed in |
|---|---|
| `seat.rs` is reached only from `cli.rs` and `main.rs`; the helpers it shares already live in `wake.rs` | no change: the cut is clean |
| `Config` is `#[serde(default)]` without `deny_unknown_fields`, so a `[supervisor]` table still loads; its pinning tests must go | `slice-3.md` Remove: the pinning tests listed; wdl-10 deletes them |
| One live leader still carries `waggledance.via=supervisor` and will read `daemon` | `slice-3.md` Remove: accepted |
| `README.md` and the doctor row in `docs/specs/doctor.md` still promise `supervisor ensure` | wdl-11 files and verify cover README.md and the doctor row |
| The installed `waggledance-supervisor` skill stays stale until doctor refreshes it | Host step 4: `waggledance doctor --fix` |
| A Paseo heartbeat on the supervisor agent would survive a code revert | Host step 1: delete any heartbeat or schedule on it before archiving |
| `~/.waggledance/supervisor/` would be orphaned | Host step 3: kept on disk as the old ledger, never deleted by the feature |
