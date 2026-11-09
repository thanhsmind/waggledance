# Spec drop — sup-20261105-a7d3

**Provenance:** from waggledance@830a1726f
**Filed:** 2026-11-05
**Correlation id:** sup-20261105-a7d3 (same id as the registered PBI)
**Origin:** beehive, decision d9007aca (2026-11-05)

## Request text (verbatim)

> waggledance_await: a summary-only mode (from beehive, decision d9007aca, 2026-11-05)
>
> Why: Claude Code cut background-subagent status cost by about 99% when a small model
> wrote the status line from short notes instead of re-reading the whole subagent
> conversation. In beehive, a leader paid session-model tokens to read worker logs,
> which pushed one session to 350k tokens. beehive now has a rule: a leader never reads
> a worker's transcript, pane log or activity stream to learn its progress — it reads
> one status line (beehive adds `bee herding status --job <id>` for its own workers).
>
> The ask: today `waggledance_await` returns `status` plus the full transcript `delta`
> since dispatch. Add a mode (for example a `summary: true` parameter, or a
> `detail: "line"|"full"` field — your call) that returns `status` plus ONE short status
> line instead of the delta. The line should come from short progress notes or the run's
> final result, summarised by a cheap model or taken verbatim when there are few notes —
> never by re-reading the whole transcript on the caller's model. The full delta stays
> available as today for callers who need it. Acceptance: a caller polling a run in
> summary mode receives one line (at most about 120 characters) per call and never the
> transcript delta; with no notes yet it says so plainly.

## Acceptance (as stated by the requester)

- A caller polling a run in summary mode gets `status` plus one line of at most about
  120 characters on every call.
- The response in summary mode never carries the transcript `delta`.
- With no progress notes yet, the line says so plainly.
- The full-delta behaviour stays available, unchanged, for callers that ask for it.

## Notes

- The parameter shape (`summary: true` or `detail: "line"|"full"`) is left to this
  repo's triage, by the requester's own words.
- The line's source is open: short progress notes or the run's final result, summarised
  by a cheap model or taken verbatim when notes are few. This repo has no
  progress-notes channel for dispatched runs today as far as this drop knows; whether
  one is needed is a triage question, not decided here.
- Domain neighbours already proposed, not merged or superseded here:
  `p-1f6d0ebf` (await reports done from screen stability) and `board-run-reaper`
  (board runs are never awaited). Whether this drop touches either is this repo's own
  triage call.
