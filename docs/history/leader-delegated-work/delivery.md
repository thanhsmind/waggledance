# Leader Delegated Work — Delivery

A Paseo leader that ends a turn while work it handed out still runs keeps its run `working`, stays unarchived and unnudged, and a reply after its run ended still reaches the caller.

## What shipped

| Cell | Change | Commit |
|------|--------|--------|
| ldw-1 | `leader_holds` table, `waggledance hold` command, `hold_max_minutes` (default 120), result mail clears the hold | 89e253e |
| ldw-2 | Held leader's run stays `working`, idle sweep and heartbeat skip it, hold line on non-steer prompts, expiry noted in the transcript | dc94402 |
| ldw-3 | Late reply passed on once to the caller | 1095c66 |
| ldw-4 | Knowledge in the two leader patterns | this commit |

## Decisions kept

D1 to D5 as locked in CONTEXT.md. No new run status was added.

## Known limit

A message a person types into a leader's Paseo chat after its run ended `done` gets a reply passed on once as a late reply.

## Not proven here

The live whole-path proof is the leader's.

## Knowledge

`docs/knowledge/patterns/paseo-leader-dispatch.md`, `docs/knowledge/patterns/wd-direct-leader.md`.
