# wd-supervisor-seat — the proof run

The seat's four acceptance criteria, proven by one live spec drop into beehive on
2026-08-30. Recorded here because the evidence was a real run, not a test: nothing in
this repo's suite can assert that an agent opened in another repo and filed a row there.

**Run:** `run-6aebfcbfaafe8cac` · **Target:** beehive · **Pane:** `w1:pD` ·
**Preset:** `claude-sonnet` · **Correlation id:** `p-bea191e4` ·
**Sender:** waggledance@faa6945

## Step 0 — the door was shut, and that is what the gate proved

Before the flip, a real dispatch call against beehive returned:

    project beehive has not opted into orchestrator dispatch — enable it from the
    project's settings page

Zero agents spawned: `orchestration_allowed` is checked at `mcp.rs:937`, before preset
resolution and before any herdr call. This is the half of the proof that pins the gate
to the flag.

## Step 1 — the opt-in

`POST /api/projects/<id>/orchestration` with `enabled=on` — the same endpoint the
settings page posts — for all three registered projects. Each returned `303`. Read back
from the registry rather than assumed:

    waggledance|1
    jarvis|1
    beehive|1

## Step 2 — the drop

One dispatch carrying a file-and-stop task: write the spec into beehive's own docs tree,
register the proposed PBI, report, stop. No triage, no Lock, no routing, no merge.

## Step 3 — the four criteria

| # | Criterion | Evidence |
|---|---|---|
| 1 | A lead opens in the target repo carrying the spec, via the dispatch door | The lead ran in `~/Projects/goglbe/beehive` on pane `w1:pD` under preset `claude-sonnet`, and reported: "Done. Filed and stopped, as told." |
| 2 | The spec lands per the target's spec-drop convention | Both halves of beehive decision `12deaa34`: `docs/discovery/slp-human-up/wd-seat-live.md` written (68 lines) **and** PBI `p-bea191e4` filed `[proposed]`, its CoS opening `from waggledance@faa6945:` |
| 3 | The run is visible in `waggledance_runs` | `run-6aebfcbfaafe8cac`, `project_id: beehive`, `preset_label: claude-sonnet`, with `spec-drop p-bea191e4 from waggledance@faa6945` as line 1 of its stored task |
| 4 | Merge to main stays human-only | beehive `main` at `48317a234236dbd3e4f3412b493496f1a6881c80` captured before the dispatch and read back identical after |

## What went wrong, and what it teaches

**The first send pasted the task but never submitted it.** The composer held the full
task text while the run sat at `working` across two 60-second `await` polls with a
byte-identical delta. Nothing had landed in beehive: no spec file, no PBI row.

The cause is visible in that delta — the Claude CLI was still painting its startup
banner and an "Update installed · Restart to update" notice when the send arrived, so
the submit keystroke went to the startup UI instead of the composer. `dispatch_run` sends
with submit=true and does not verify that the composer actually cleared, so the run
looked healthy while being stalled.

Recovered with `herdr pane send-keys w1:pD enter`; the lead then ran correctly and
finished in 33 seconds.

Worth knowing for anyone using the seat:

- A run stuck at `working` with an unchanging `delta` that still shows the task text in
  the composer is an unsubmitted send, not a slow agent. The tell is the `>` prompt
  holding the task rather than a response above it.
- `await` reporting `working` is not evidence the agent ever received the task.
- A freshly spawned agent is at its most fragile in its first seconds; a spawn into a CLI
  with an update notice pending is the reproducible case.

This is a gap in the dispatch path, not in the seat, and is filed as its own backlog row
rather than fixed here.
