---
type: bee.pattern
title: "A JS-only behavior can still be proven, with a DOM probe over the real asset"
description: "Practice: behavior that only exists once app.js runs is routinely capped on a string assertion that cannot see it — a node script that builds the rendered DOM and loads the real asset proves the behavior itself, red before and green after."
timestamp: 2026-08-31
bee:
  id: a-js-only-behavior-can-still-be-proven
  lifecycle: active
  areas: [agent-terminal, changes-diff-screen]
  sources: [.bee/cells/pkr-1.json, .bee/cells/dfc-1.json]
  polarity: practice
  signature: "behavior lives in app.js, proof asserts markup"
---

# A JS-only behavior can still be proven, with a DOM probe over the real asset

## The situation

The change lives in `app.js`. The repo's tests are Rust, and what they can
reach is the served string — so the cap ends up asserting that a button was
rendered, which is not the claim. The claim was that pressing it folds the
section, or that a sender kicks a refresh. A `views.rs` string assertion can
never show either, and "verified by reading the code" is not a proof line.

## The practice

Build the rendered DOM in a scratch node script, load the **real** `app.js`
(the asset the server ships, never a copy or a paraphrase of it), drive the
behavior, and assert on the result.

`pane-kick-refresh` did exactly this: a scratchpad node DOM probe over the real
`app.js` **failed 7 checks against the pre-change file and passed them after**.
Red first, then green, on the actual asset — the shape of a proof, not of a
demonstration.

## Why it keeps getting skipped

Because the honest alternative looks close enough. `diff-file-collapse` shipped
a fold button whose whole point is what happens on click, proved the server
half live, ran `node --check` on `app.js`, and still had to record that the
click itself was never exercised — the browser extension was not connected and
the machine had no headless browser. `node --check` is a syntax gate; it says
nothing about behavior.

The probe needs no browser and no new dependency. Reach for it whenever the
sentence describing what shipped has a verb in it that only fires in a browser.

## Sibling

The server half of the same problem — proving what the browser actually
receives without displacing the user's daemon — is
[[a-second-daemon-proves-it-without-taking-the-users]]. Together they cover the
whole path; either alone leaves a seam.
