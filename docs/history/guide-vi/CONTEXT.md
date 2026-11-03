# guide-vi — locked context

A built-in Vietnamese guide to bee, served at `/guide` from inside the
waggledance binary and reached from the top bar menu on every page.

## Locked decisions

**guide-vi D1 — plain HTML, never markdown.** A chapter is an authored HTML
fragment under `crates/waggledance/assets/guide/vi/`, embedded with
`include_str!` and rendered verbatim. The markdown pipeline is not involved:
it rewrites links against a project root the guide has none of, and it cannot
carry a labelled SVG whose boxes are the chapter links. The diagram is the
method, not decoration.

**guide-vi D2 — the eli5 method.** Every chapter opens on a picture and only
then spends words. Concrete before abstract; an analogy before a definition;
never a paragraph that could have been a diagram. The reader is assumed to
know nothing about bee and to be impatient.

**guide-vi D3 — Vietnamese, addressed to one person.** Second person
singular ("bạn"). Short sentences. Bee's own English terms are kept in
`<code>` where they are what the reader will actually see on screen (`cell`,
`gate`, `worktree`, `claim`), with the Vietnamese meaning given once beside
them — a reader who only learns the Vietnamese word cannot read a bee message.

**guide-vi D4 — the chapters are a graph, not a stack.** Every chapter
cross-links to at least two others by `/guide/<slug>`, and ends with a
"Đọc tiếp" list. A test refuses any `/guide/<slug>` that names no chapter.

**guide-vi D5 — the guide reads nothing at runtime.** No store, no project,
no disk, no network. It answers identically on a host with nothing
registered.

## The fourteen chapters and their slugs

| # | slug | what it answers |
|---|---|---|
| 1 | `bee-la-gi` | who bee's user really is (the agent, not the human) |
| 2 | `tu-vung` | ~60 terms, one line each |
| 3 | `cong-gate` | the five gates and the bypass ladder |
| 4 | `kho-store` | what lives under `.bee/` |
| 5 | `phien-session` | preamble, heartbeat, handoff, compaction |
| 6 | `hooks-guards` | what actually stops a write |
| 7 | `worktree` | main / feature worktree / granted / staging |
| 8 | `vong-doi` | orient → shaping → planning → cells → execution → close |
| 9 | `cell-lane` | the work unit, the six lanes, the proof line |
| 10 | `giao-viec` | dispatch, workers, herding |
| 11 | `phoi-hop` | claims, reservations, holds |
| 12 | `bo-nho` | capture, decisions, knowledge, backlog |
| 13 | `config` | every key of `.bee/config.json` |
| 14 | `dung-hieu-qua` | the practical recipes |

## Where the facts come from

`/home/thanhsmind/Projects/goglbe/beehive/docs/product-description/` — read
the document that owns your chapter's subject and take the facts from it. It
is the authority; never invent a number, a key name, a TTL or a level.

| chapter | source |
|---|---|
| `tu-vung` | `glossary.md` |
| `cong-gate` | `foundations/gates.md` |
| `kho-store` | `foundations/store.md` |
| `phien-session` | `foundations/session.md` |
| `hooks-guards` | `foundations/guards.md`, `cross-cutting/failure.md` |
| `worktree` | `foundations/worktrees.md` |
| `vong-doi` | `lifecycle/*.md` (all six) |
| `cell-lane` | `lifecycle/cells.md`, `lifecycle/execution.md` |
| `giao-viec` | `delegation/dispatch.md`, `workers.md`, `herding.md` |
| `phoi-hop` | `coordination/reservations.md`, `sessions.md` |
| `bo-nho` | `memory/*.md` |
| `config` | `cross-cutting/configuration.md`, plus `goal.md`'s established facts |

## The exemplar

`crates/waggledance/assets/guide/vi/bee-la-gi.html` is written. Match its
voice, its density and its markup. Read it before writing a line.

## The markup vocabulary — use these classes, invent none

A fragment starts straight into content: no `<html>`, `<head>`, `<body>`,
`<main>`, and no `<h1>` (the page renders the chapter title itself).

- `<h2>` for a section, `<h3>` under it. `<p>`, `<ul>/<ol>/<li>`,
  `<strong>`, `<em>`, `<code>` plain.
- A figure:
  ```html
  <figure class="guide-fig">
    <svg viewBox="0 0 820 240" role="img" aria-labelledby="fig-x-title">
      <title id="fig-x-title">One sentence saying what the picture shows.</title>
      …
    </svg>
    <p class="guide-fig__cap">The caption states the point, it does not repeat the labels.</p>
  </figure>
  ```
- Inside an SVG use only these classes, never a literal colour and never a
  `style` attribute: `fig-box` (a plain box), `fig-box--on` (the box being
  talked about), `fig-line` (any stroke), `fig-t` (a label), `fig-t--sm`
  (a small grey label), `fig-t--mono`, and the four tints `fig-fill-gate`
  (human/stop), `fig-fill-agent` (the agent), `fig-fill-ok` (good outcome),
  `fig-fill-stop` (refusal). Both themes then come free.
- Make an SVG box a link by wrapping it: `<a href="/guide/slug"> <rect …/>
  <text …/> </a>`.
- A callout: `<div class="guide-note">` with an optional
  `<span class="guide-note__title">…</span>` first. Variants
  `guide-note--warn`, `guide-note--stop`, `guide-note--ok`.
- A table: `<div class="guide-table-wrap"><table class="guide-table">…`.
- A block to type verbatim: `<pre class="guide-code">…</pre>` (a comment
  inside it can be wrapped in `<span class="c">…</span>`).
- Prev/next is rendered by the page. Do not write one.

## Shape of a chapter

1. One or two sentences of hook — a question or a concrete situation.
2. The opening figure.
3. Two to five `<h2>` sections. Tables for anything with more than three
   parallel cases. At least one more figure in a long chapter.
4. At least one `guide-note` naming the thing beginners get wrong.
5. A closing `<h2>Đọc tiếp</h2>` with a `<ul>` of two to four
   `/guide/<slug>` links, each with a half-sentence saying why go there.

Length: 150–320 lines per chapter. `config` may run longer — it is a
reference.

## Proof

`cargo test -p waggledance guide` — the guide's own tests check every
cross-link resolves, that no fragment ships page scaffolding, and that no
chapter is a stub.
