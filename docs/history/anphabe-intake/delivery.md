# anphabe intake — Delivery

- Config: an `[intake]` section, off by default, with the user's email as a host value and never a literal in code (D2, D7).
- Ledger: table `intake_items` from migration 15 with four verdicts, so a retry cannot dispatch twice and a held item is announced once (D4, D5).
- Pass: one Haiku run classifies, and Rust owns the threshold, the twin check and the dispatch with `leader: true` and `request_id` `intake-<key>`; the model never dispatches (D3, D4).
- No writes to anphabe: the allowed tool list holds only the three mcp-gateway tools (D6).
- Daemon loop: `intake::run` joins `TerminalBackground`, starts only when enabled with an email set, first pass after 60 seconds, then on the interval (D3, D7).
- Held items: one `intake` notifier message per tick, marked notified only after the send succeeds (D5).
- Agent verbs: `intake run` makes one pass by hand and `intake list` shows the ledger, both listed in `schema` and `search` (D7).
- Knowledge: the supervisor-daemon area gains the intake ledger and intake pass concepts, anchored to the code.
