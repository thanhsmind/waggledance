---
area: bee-cockpit
updated: 2026-08-25
sources: [feature-close, agent-board, bee-artifact-rename, archive-visibility, feature-hub, board-declutter, board-trim, feature-titles, hub-fallbacks, detail-desc-wrap, cross-board, board-drop-live, card-terminals, gate-stop-superseded, console-theme-kanban, console-rail-orchestrator, console-phone-layout, bee-agent-activity, board-new-task, board-topbar-polish, rail-icons, card-agent-logos, rail-collapse, rail-agents-compact, herdr-session-liveness, session-work-line, board-live-morph, project-color-identity]
decisions: []
coverage: partial
---

# Bee Cockpit

A read-only surface inside waggledance that shows what the bee harness is doing in a
registered project: which features are waiting on a person right now, which are
in progress, which have already shipped, and the backlog and review queue
behind any of it.

Throughout this surface's own pages, the name shown to a person is **Bee
Artifact** — page titles, the top bar, and other on-page UI text all read that
way. This is a display label only: the command a person types, and the
identifiers this product exposes to other tools, remain **waggledance**.

Technology-agnostic: this describes behavior and rules, not the Rust that implements
them. Code entry points are listed in `reading-map.md`.

## Who it is for

Someone running bee across several projects who cannot read bee's own store. bee keeps
a thorough record — cells, lanes, sessions, backlog, reviews, decisions — spread across
JSON and JSONL files in each project's `.bee/` directory. That record is precise and
unreadable. This surface answers, in plain language, the questions a project manager
actually asks: what has been built, what is being worked on, what comes next, and where
is it stuck.

## Where it appears

A project qualifies for the bee surface when **both** hold:

1. It is registered with waggledance (it appears in the project registry).
2. Its root directory contains a `.bee/` directory.

A registered project without `.bee/` behaves exactly as it always did — opening it
still goes straight to its entry document. No bee link appears, and requesting a bee
page for it returns not-found rather than an empty bee page. The absence of a store is
never rendered as an empty dashboard.

A qualifying project gains an entry point on its home page leading to its board.

On a wide desktop screen the board pages — the home board and a project's own
bee board — span the full width of the window instead of stopping at the reading
column other pages keep; feature and item detail pages keep the narrower column.
Card titles render in the interface's own sans face rather than the reading face,
so a wall of cards scans like a control surface, not a document.

## The cross-project board

The same surface also exists once for everything at once. The viewer's own front
page — the page a person lands on before choosing a project — is one frame in
three parts: a top bar, a left rail of what is running and what is registered,
and, filling the rest, the Features board rolled up across **every** qualifying
project.

1. The **top bar**, whose right slot carries a single **Orchestrator** link: the
   board's entry point, and the only element there that marks itself the current
   page, which it does while the board is showing. It is a real address
   (`/?tab=kanban`) and not a script-driven control, so it works with scripting
   off and survives the page's own reloads. While current it reads as a quiet
   ink wash, never a loud filled pill. Nothing sits beneath the top bar in
   place of the retired row of tabs; the `tab=` addresses — `kanban`, `projects`,
   `terminals` — all keep resolving exactly as they did, and `/?tab=projects`
   still lands on the board.
   Beside Orchestrator sits **+ New task**, the board's one way to put work in
   (decisions `82078151`, `cb52bbd1`, `6e29ccc5`): it opens an in-page dialog
   with a task box — a real multi-line field with a comfortable minimum
   height — and a project picker listing the top-level registered projects
   (never their worktree branches), preselecting the rail's filtered project
   when one is set, else the first. Enter files the task, Shift+Enter breaks a
   line; Cancel, Esc or a click on the scrim closes it. The first non-empty
   line becomes the item's title (clipped to 200 characters) and the rest its
   conditions of satisfaction; the item lands as a proposed backlog item in
   that project, which the Todo column already renders, and the page reloads so
   it appears. A refusal — nothing typed, an unknown project, a project not set
   up with bee, or bee itself refusing — shows inline under the task box and
   keeps the typed text. On a handset both buttons hide; the bottom tab bar
   carries the board instead. That bar is folded away by default — a small
   handle on the bottom edge shows or hides it, and the choice is remembered
   per browser (`localStorage["waggledance-tabbar-open"]`); with scripting
   off the bar stays visible and the handle never appears.
2. The **left rail**, described below. It is the same rail on the board and on
   the terminals view, so reaching another agent, or another project, never
   depends on which of the two a reader happens to be looking at.
3. **Features**, across all projects.

The rail reads top down in the order the question does — who is working right
now, then what there is to work on:

- **Agents**, the live agent terminals, its heading led by a pin glyph. Same
  inventory and same order as the
  terminals view's own switcher: blocked first, then working, then the rest.
  Each row reads as two lines — a status dot, the project it belongs to, and
  its workspace and tab on the first; the feature it works, led by a purpose
  icon, on the second — and leads to that terminal. A row whose terminal a live agent
  session claims also names that agent's own state and the feature it is
  working; a row no such session claims reads exactly as it did before. That address names the pane and nothing
  else — a terminal's own title, session name and working directory never appear
  on this page. The group's heading is itself a link to the terminals view, and
  the group renders whether or not the terminal supervisor is running, so that
  view's own account of a stopped supervisor stays reachable; with no agent
  running the group shows one muted **No agents running** line rather than
  disappearing. The rail no longer carries a Board row — it pointed at the page
  it was already on, and the Orchestrator link answers that now.
- **Projects**, the registered projects, each one a group that collapses. The
  summary is the project's name line: a folder glyph, its dot, its name, and a
  **…** disclosure menu offering **Docs** (the project's reading pages) and
  **Remove** (unregister, with the usual confirmation) — the menu, not a bare
  remove control, and it stays reachable while the group is closed. A worktree
  branch row keeps only its dot, never a glyph.
  Everything a collapsed group should fold away sits in the body — the markdown
  count and last-seen line, the badges, and the worktree branches. Groups open
  by default; the set a reader leaves closed is remembered for that browser
  alone, keyed by project. Collapsing itself works with scripting off; only the
  memory of it needs scripting. Typing in the rail's filter forces every still
  matching group open, because a match hidden inside a closed group reads as no
  match at all, and clearing the filter restores the reader's own collapsed set.
  The filter matches the row's own words, never the menu's labels.
- On a wide screen the whole rail folds: a chevron at its top collapses it to a
  narrow strip (about 44px) and back, and that choice is remembered for the
  browser alone. The handset drawer is a different mechanism and is untouched
  by it.

At most one thing in this frame is marked the current page: the Orchestrator
link while the board is showing, or, on the terminals view, the pinned row of
the terminal actually on screen.

On a handset the same frame answers, narrowed rather than replaced: one page,
one address, one reading, laid out in a single column below the narrow-screen
width the rest of this surface already uses. What changes is where the frame's
parts sit, never what they say:

- The rail leaves the flow and becomes a drawer over the page, opened from a
  **Projects** item and dismissed by tapping the page behind it. It needs no
  scripting, so it survives the reload the page performs on its own, and a
  reader's collapsed project groups are still collapsed when the drawer opens.
- A **bottom bar** carries four items — **Board**, **Agents**, **Projects**,
  **Settings** — and marks the section on screen by more than colour alone.
  Board and Agents are the frame's two existing addresses, Projects is the
  drawer, Settings the settings page. The Orchestrator link stands down at
  this width because the Board item already answers it, which keeps the rule
  above intact: exactly one thing still marks itself the current page.
- The board opens with three figures — **working**, **need you** and
  **mergeable** — each read straight off the groups the board has just drawn:
  what is in progress, how much of that is sitting at a gate or a paused note,
  and what is ready to merge. Each figure jumps to the section it counts, and
  a figure of zero is drawn faint rather than hidden. The waiting mark is
  carried by in-progress work alone, so **need you** lands on In Progress,
  and it counts an agent that needs approval and one that needs an answer
  alike (see "The agent at work on a card," below).
- The groups stack top down in the order the question is asked: whatever
  carries a waiting mark first, then ready to merge, then in progress, then
  the rest, with the archive still a collapsed bar at the bottom. At this
  width only, a group holding no cards is not drawn at all — screen height is
  the scarce thing on a phone, and an empty header spends it saying nothing.
  At full width every group keeps its place and its honest empty line.

None of this adds a way to act. There is no floating compose button and no
notification bell on the phone layout, because the surface still only reads a
project's store. Above the narrow width the page is exactly what it was: no
bottom bar, no figures, and the rail back in the frame.

There is deliberately no cross-project Live strip. Presence across many projects
answered nothing the Features columns did not already answer, while taking the
top of the page to do it; the Live strip stays where it earns its place, on a
single project's own board, where it answers who is at work in *this* project.

Qualification is the same rule stated above, and it decides only what feeds the
Features section: a registered project without a `.bee/` store still appears in
the rail, exactly as before. When no project qualifies at all, the Features
section renders its own empty state rather than vanishing — the Orchestrator
link and the rail belong to the frame rather than to the board, so the terminals
view stays reachable whether or not any project carries bee metadata; the board
simply shows it has nothing yet, and the project list still reads exactly as
before in the rail.

If any `.bee` file could not be read while rolling up the board, the front page
carries one concise warning strip in its header — naming how many files failed and
that the counts below may be incomplete — so a corrupt store degrades loudly
instead of silently skewing every number. When every file read cleanly, no strip
appears; the strip never grows back into the removed multi-item panel.

The rolled-up board reads the same way its per-project counterpart does, with
three differences that follow from having more than one project in view:

- Every entry names the project it belongs to. Without that, a merged list is
  unreadable.
- The lists stay **flat**. Features are never grouped into per-project blocks and
  a project never gets a row of its own. The question this page answers first is
  "what is waiting on me", and the project is a detail of the answer, not the axis
  it is organised by.
- Ordering, counting, and truncating all happen **after** the merge, over the
  combined sequence — never per project and then stitched together.

Reading every project's store is more work than reading one, so it is done off to
the side and for all projects at once, never one after another while the page
waits. A project's rolled-up reading is also remembered between requests: a repeat
render reuses the last reading unless that project's store or its feature history
has actually changed since, so opening the front page again is cheap and only a
project that moved is read afresh. The per-project board is untouched by any of
this: it keeps its own address and renders exactly what it rendered before.

The front page also refreshes itself only when the change concerns it: an edit
somewhere in a project that has nothing to do with the board — a note, a readme —
no longer reloads the board, which reloads on its own feature-history content, not
on every file a project happens to hold.

When that change does concern it, the board no longer throws the page away. Both
board surfaces — the home board and a project's own board — refetch their own
content, match the incoming cards against the ones already on screen by each card's
stable key (the thing's own detail link), and patch the differences in place. Three
things survive that the old reload destroyed: anything already running inside a card
keeps running rather than being rebuilt, every card and column the reader had opened
stays open, and a card that changed column or position slides from where it was to
where it now belongs instead of jumping.

Motion is spent on the card as a whole and on nothing else: a card that moves slides,
a card that arrives fades in, and nothing inside a card animates. Every failure path
— an unreadable response, a shape the reconciler does not recognise — falls back to a
full page reload, so the worst case is exactly the behaviour this replaced.

Matching by key settles what happens to the things that have one; the reconcile owns
what is left over too. Not everything the board draws carries a key of its own — a
row's action controls sit beside the row rather than inside it, so no key ever claims
them. Anything standing in the patched area without a key is the reconcile's own
leftover and is removed on every pass. Without that rule each patch left one more copy
of those controls beside the last, and a column that emptied still showed the controls
of the rows that had left it. The arrival fade belongs to keyed things only, which is
the same rule as before read from the other side: motion marks a card or a row
appearing, never a control reappearing beside one.

## The reading order — and why it is the feature

The board answers, top to bottom, in a fixed order:

1. A top bar naming the project and the instant this snapshot was read.
2. **Live** — a single dense strip of what is running at this moment, one line each.
3. The **Feature Hub** — one card per feature, grouped by whether it is waiting on a
   person, still in progress, or finished.
4. **Finished (shipped) list** — the complete, permanent record of every feature that
   has fully shipped, collapsed by default so it never crowds out the live work above
   it.
5. **Backlog & Review** — the backlog of proposed work and the state of every review
   candidate, for the reader who wants to go one level deeper than the headline view.

This order is not decoration; it is the feature. It mirrors the sequence a project
manager actually asks the questions in: first "what needs me and what's moving right
now," then "what's already done," and only after that, backlog and review detail. A
section with nothing to show never disappears and never disturbs this order — it
renders its own honest empty line in its place (see "Honesty rules that hold
everywhere," below).

## Live

Between the top bar and the Hub sits one narrow presence strip answering a single
question — what is running right now:

- One line per live session, naming the lane it is working, that lane's current stop,
  how long ago it last reported in, and the checkout it is working in (the main
  checkout is named as such rather than left blank). A session with no lane of its own
  is labelled by the project's active feature, or plainly as having no active lane.
  When the session's own record says what it was *asked* to do, the line names that
  too: the title of the work, its status as a word beside the line and as an attribute
  on it, and the acceptance the work is measured by, carried on the row's title. Three
  rules hold on that reading:

  - The whole conversation is deliberately never carried. The record holds it, capped
    at eight thousand characters per session and re-read on every snapshot; a row shows
    a title and a card shows the acceptance, and neither needs the transcript.
  - A status the reader does not recognise is carried through verbatim rather than
    blanked, so a newer bee that invents a fifth status cannot empty the row on an
    older viewer.
  - A session record carrying no work at all renders exactly as it did before this
    existed. Most records in the wild predate the field, and a blank column reads as a
    rendering fault rather than as an absence.
- One line per branch checkout, naming the branch and the feature it carries, or
  saying it carries none. A checkout that could not be read says so and why, rather
  than being dropped or guessed at.
- Nothing running at all → one line saying so. The strip never disappears.

This strip is deliberately thin: it carries presence, not detail — no worker rosters,
no per-file contention, no health scoring. Those readings still exist for the drill-down
pages; this section is the glance that tells a reader whether anyone is at work before
they read anything else.

## Feature Hub

The board's main section is feature-centric: one card per feature, never one card per
cell. Every feature the store knows about — whether it still has live work or has
already shipped — is placed into exactly one of five columns, left to right —
**Todo, In Progress, Review, Compound, Ready to merge** — or into the **Archive**
bar beneath them. Membership is decided in one fixed priority order, so a feature
never appears twice and never in two columns at once. Each column opens with the
same header: a status dot in the column's own lane hue, the column's name, and a
right-aligned count in the monospace face; the In Progress header also carries a
"waiting" chip, only when a real count of features waiting on a person backs it.
In Progress is the one column that renders cards; the other four render dense rows.
A dense row reads as one continuous text run — title, then its facts — that wraps in
full rather than truncating, so nothing a row says is ever hidden behind an ellipsis.

- **Todo** — a feature whose cells exist, are all still open, and none is claimed.
- **In Progress** — every feature that has live work — a cell being worked, the
  checkout's active feature, a live session naming it, or a granted worktree of its
  own — and is not finished-and-idle. A card here additionally carries one line
  reading **Waiting on you —** followed by the reason, when its live work is sitting
  at a gate that has not yet been approved, or when it carries a paused handoff
  note; there is no separate Waiting-on-you column any more. Three things never
  raise that line: the independent-review gate never counts as a stop (see
  "Independent review is always invoked," below); a handoff explicitly recorded as
  a clean, already-claimed handover to the next piece of work — a `planned-next`
  handoff — never counts as waiting either; and an unapproved gate that a **later**
  gate has already been approved past is not a stop at all. Work that reached the
  execution gate has plainly been through the earlier ones whatever their flags
  say, and naming an earlier one would claim a decision is owed that nobody is
  waiting on. A feature whose interview genuinely stopped for an answer — nothing
  approved after it — still carries the line, which is the case it exists to catch.
- **Review** — a feature with no live work that still holds an unsettled review
  candidate.
- **Compound** — a feature whose own phase is exactly the compounding step.
- **Ready to merge** — a feature whose work is finished in its own worktree and is
  waiting only for the human to land it: the worktree is still open, the execution
  gate was approved, and every cell that was not dropped is capped (a worktree with
  no cells at all is never ready). The card says whether the acceptance test is
  already approved or still pending, and how long the feature has been ready
  (its latest cap); approved cards come first. Once bee records its own
  merge-readiness fact, that fact outranks this derivation. The card names its
  worktree branch as a line of its own — the one fact a merge needs — the same
  branch line an In Progress card carries (decision `8b057354`).
- **Archive** (Finished) — a feature that has either fully shipped (compounding
  complete) or been archived. Finished is not a column: it is a collapsed bar that
  spans the board beneath the five columns, labelled ARCHIVE and stating its true
  count while closed; opening it shows the finished rows. A row here carries a chip
  naming the state of its worktree: still the main checkout, an open worktree, or one
  already merged. Finished is checked before Review, so a closed feature stays closed
  even while it carries an unresolved review candidate.

A card's branch line names only a real branch. A feature worked in the main
checkout has no branch, so its card carries no branch line at all — "main
checkout" is the absence of a branch, never a branch named *Main* beside a
branch glyph; the Finished chip above is where that state is still read.

Every card title is led by one glyph so a column reads by shape before by word:
the column's own state mark — an open circle for Todo, an activity mark for In
Progress, an eye for Waiting on you, stacked layers for Compound, a merge mark
for Ready to merge, a check for Finished — tinted in that column's lane hue.
When an agent session is attached to the card, that agent's own mark replaces
the column glyph (Claude, OpenAI/Codex and Gemini each in their vendor colour,
a generic prompt mark for any other agent); an active session (working or
blocked) outranks a quiet one, and a card with no agent pane keeps the column
glyph. The agent's style name never spells a program name on a page where the
terminal switch is off.

A card shows the feature's human title with its slug as a subtitle beneath it, its
Feature Boundary sentence, and a description. The title and description are read from
the first of these that exists: the feature's own `CONTEXT.md` heading and boundary
text, then the most recent decision scoped to the feature, then the title of its first
cell. A long description wraps and is visually clamped — on the card, and on the
feature's own detail page too — rather than overflowing or forcing either page to
scroll sideways. A card links onward to the feature's own detail page.

On the board that spans every project, a card's subtitle names its project rather
than the feature's slug, and the card carries that project's own fixed accent
colour — the same colour for the same project on every visit, so a reader
scanning many projects groups the cards by eye before reading a word. That colour
is the one coloured thing on the card. A project's own board does not repeat the
project name it is already titled with: a card there keeps the feature's slug and
keeps its worktree visible. A card with no recorded title names its worktree
state in the subtitle instead of dropping the line, with no slug half and no
separator left behind.

### Live signals on a card

Every card ends in one footer line: on the left a small ring that is hollow, half,
or filled as its cells go from none to some to all capped, beside "n/m cells"; on
the right the last-activity time. That time reports the freshest of two clocks: the feature's own
cell claim and cap times, and — only on the card of the checkout's currently active
feature — the checkout-wide activity stamp that the agent refreshes on every action.
Work in progress before any cell is claimed therefore moves the active card instead
of leaving it frozen; a checkout whose records predate the stamp simply falls back to
cell times alone. When the active checkout's most recent agent action happened within
the last two minutes, that card additionally shows a small animated "working now"
pulse dot beside the activity line — present only while the action is that fresh,
absent otherwise and on every non-active card.

The active feature's card also carries a colored badge naming the run state of its
work — shaping, awaiting approval, running, blocked, or done — with awaiting
approval rendered as the most prominent of them, because it is the one state where a
person is the blocker. A checkout that records no run state shows no badge, and no
other card ever borrows the active feature's badge.

Separately from those active-only signals, any card whose feature owes deferred
follow-up work (captures, promotions, reviews queued for later) shows a count badge
naming how many items wait; hovering it reveals each item's kind and reason. Debt is
matched to the feature that owes it, so sibling cards of the same project each show
exactly their own, and a card with nothing owed shows no badge at all. All of these
signals live on the cards themselves — none of them brings back a board-level panel,
and none of them moves a card between groups.

### The agent at work on a card

Beside those signals, a card whose feature has a live agent session says what
that agent is doing right now. The reading comes from bee's own per-session
record inside the project's store — read, never written, like everything else
on this surface — and it reaches a card through the feature the session says
it is working. When several sessions name the same feature, the one that spoke
last speaks for the card: the line names a single agent, so it names the
freshest.

The line reads **agent: state · cell · quiet age**, and it takes a full-width
row of its own beneath the title; the cell title wraps inside the card and is
never clipped to one line:

- **State** is one of five words — *working*, *idle*, *exited*, and the two
  that call for a person: *needs an answer* and *needs approval*. The word
  itself is always printed beside its colour, so the state never speaks by
  colour alone, and a state a newer bee invents is carried through as it was
  rather than forced into one of the five. The colour lives on the agent's own
  mark rather than on a separate dot, and where a collapsed card prints no word
  the mark carries the state in its accessible name — see the Appearance spec,
  R8 and R9.
- **Cell** is the title of the cell the agent holds, falling back to its bare
  id when the store no longer knows the title. An absent cell — or an absent
  feature anywhere this reading appears — renders as a dash, never as a blank
  and never as an error.
- **Quiet age** is how long ago that record last spoke: seconds under a
  minute, whole minutes above it.

A record that has not spoken for more than ninety seconds also carries a
muted **no signal** marker. Its state still reads; what changes is that
nothing counts it as a call for a person.

A record stays on its pane for as long as the session is alive, and alive
has two witnesses — either one suffices. The first is bee's own heartbeat,
thirty minutes. The second is herdr still hosting that session id in the
pane: a session blocked on a gate question writes nothing while the person
is away, so its heartbeat alone would age out inside the hour and the pane
would fall back to herdr's own reading of that dialog, which is *idle*.
herdr listing the id is proof the agent is still there, so the record keeps
speaking and *needs approval* stays on the pane until the answer lands. A
stale record nobody hosts is history and leaves the pane to herdr.

Two rules follow this reading wherever else it surfaces:

- **"Need you" means both kinds of call.** A session that needs approval and
  one that needs an answer each count as needing a person — in the In Progress
  group's waiting chip, in the phone layout's *need you* figure, in the pinned
  rows and in the agents listing alike, all from one shared rule, so no two of
  them can ever disagree. A session that has stopped speaking is counted by
  none of them.
- **The agent's own state outranks the screen.** Where the agent's reported
  state and the terminal supervisor's screen-derived status both describe the
  same terminal, the reported state decides the dot's colour, the word printed
  beside it, the order In Progress cards sit in, and the order agents are
  listed in. A terminal no agent session claims reads exactly as it always did.

Worker nicknames are never rendered anywhere in this: the line says what the
agent is doing and which cell it holds, never who it is.

### The terminals running behind a card

On the cross-project board, an In Progress card also carries one marker per
terminal session running in that feature's own checkout, each showing that
session's state and the program it runs, and each opening that session's own
terminal view. A collapsed card badges only the sessions that are working or
blocked; the quiet ones move inside the card's expandable body, and the card
states how many it folded away so a closed card never understates what is
behind it. Finished rows carry none.

Which sessions those are is decided first by the session, then by the checkout.
A session's feature is resolved by the strongest evidence first — the cell it
holds, else the branch checkout its directory sits in, else the lane it is bound
to, else its own activity record (for an unbound session, the project's active
feature) — and its activity record names the terminal it occupies, so a session
that speaks is marked on that one feature's card and on no other — even when it
runs in the project's main checkout beside sessions working other features. Only
a terminal no session claims — a shell, an agent bee never saw — falls back to
the checkout directory, the one thing both sides share. So:

- A feature with its own branch checkout is marked with the sessions running
  inside that checkout, and with no others. For such a feature the marking is
  exact.
- A feature working in the project's main checkout is marked with the sessions
  there that name it, plus the unclaimed terminals of that checkout — which
  every other main-checkout feature of that project shares. The markers are
  therefore still labelled as the terminals of *this checkout*, never as the
  feature's own; the unclaimed share keeps that label honest.

A feature with no session in its checkout shows no marker and no empty frame for
them. When the terminal switch is off, or the session host cannot be reached or
does not answer promptly, no card carries a marker and the board renders exactly
as it otherwise would — the same rule the project list already follows. The
per-project board carries no such markers.

### How each group is ordered, and how Finished is cut short

Within the In Progress column, features are listed by name.

Finished is different, because it is the group that grows without bound — a
long-running project accumulates hundreds of finished features while the other two
groups stay small. Two rules keep it readable:

- **Order.** Finished features that carry a ship time come first, most recently
  finished at the top, each showing that time. Every remaining finished feature
  follows, listed by name. A feature has a ship time only when the whole of its
  finished work is timed; a feature whose record is partly timed counts as untimed
  rather than being placed on a half-known date. On the cross-project board this
  is one shared sequence across every project, not one sequence per project.
- **Length.** The group shows ten entries and puts the rest behind a control that
  names how many are hidden and reveals the next ten at a time. Nothing is ever
  dropped — only folded. On the cross-project board the ten and the hidden count
  are both taken from the merged total.

## Finished (shipped) list

Every feature that has fully shipped, collapsed by default behind one summary line
that states the true count of finished features and finished cells even while the
list itself stays closed — collapsing a list is never allowed to understate what it
holds. Opening it shows one compact line per feature — never one card per cell —
naming its cell count and, when both of its timestamps are on record, its time to
finish. This list is never capped or truncated: every feature that has shipped is
named here, no matter how many there are. A project with nothing finished yet shows a
single honest line instead of a collapsible, zeroed list.

A feature that has fully shipped appears twice on the board by design: once as a row
in the Feature Hub's own Archive bar, and again as a line in this list. The two are
not competing claims about the same thing — the Hub groups every feature, live or
shipped, by its current status at a glance, while this list is the complete, standing
record of everything that has ever shipped, and is the one place that record is
guaranteed never to be capped.

## Backlog & Review

Three sub-views in one supporting panel:

- **PBIs by status.** Backlog work items are event-sourced — the same item can appear
  many times as its status changes over time, and its **current** status is whatever
  its most recent recorded entry says. The browsable list shows only **open** items —
  anything already done or declined is left out of the list itself — capped at a
  recent 20; a project with more open items than that shows the true total of open
  items alongside the visible subset ("Showing X of Y") rather than looking smaller
  than its real open backlog. The status chips above the list still carry the full
  count for every status, done and declined included, so closed work is never erased
  from the numbers, only from the scrollable list. Each item can be expanded to reveal
  a further line of its own detail. A project with no open items — whether because it
  has no backlog at all, or because everything in it is already done or declined —
  says so plainly with its own empty state.
- **Findings by severity.** Each recorded finding carries a severity of P1, P2 or P3;
  they are summarised by severity, with P1 given visual weight because a P1 finding
  blocks, and the same bounded-recent-slice-with-true-total treatment as PBIs. A
  project with no findings yet says so plainly.
- **Review queue by state.** Every review candidate the store has recorded is placed
  into exactly one of three states by joining it against every recorded review
  session: **unreviewed** (it has never appeared in any session), **in review** (it
  appears in a session whose decision has not yet settled), or **settled** (it appears
  in a session whose decision reached approved or blocked). The count of open P1
  findings is called out first. Every sentence in this panel words independent review
  as something the project's owner invokes — nothing here ever implies review is
  already running or already queued as pending automatic work.

A store with **zero** recorded review candidates is genuinely ambiguous — it is the
same shape whether a project has never run a review at all, or whether every candidate
has already been folded and rolled off the list — so this one case says review state is
unknown rather than rendering three zeroes as if they were a real measurement. From the
moment even one candidate is recorded, every count is real and computed, including a
genuine zero for "in review" or "settled" when every recorded candidate really is
unreviewed.

## Drilling in

Every feature card links to its own feature detail page.

- A **feature detail page** shows the same human title, slug subtitle, Feature
  Boundary sentence, and clamped/wrapped description as its Hub card, alongside a row
  of chips summarising its current state. A **Docs row** links to every markdown file
  recorded under the feature's own history docs. Below that sit three tabs:
  **Activity**, **Todos**, and **Terminal** — the Terminal tab lists the project's own
  live agent-terminal panes; a plain worker list is no longer shown anywhere in this
  UI.

  A feature's own detail page is not archive-free the way the Hub and the Finished
  list are: it merges a feature's archived cells (the record `bee close` produces)
  together with its still-live ones. A feature whose only remaining cells are archived
  still shows correctly — it reads Closed, and its done-cell count includes the
  archived ones rather than reading zero.
- A **cell page** shows that cell in full: what it is, what proves it, its state and
  lane, the files it touches, the decisions it cites, its required outcomes, and its
  whole execution trace — who ran it, when it was claimed and capped, its outcome,
  recorded deviations, and its test result. Cells are not surfaced directly on the
  board any more; each is reached through its own feature's detail page. A cell page
  resolves an archived cell exactly as it does a live one.

An unknown cell or feature name returns a clean not-found, never a blank page.

## Two guarantees that make this board safe to point at a project

### It never writes to a project's store

The surface never writes to a project's `.bee/` directory itself. It approves no gate,
claims no cell, edits no backlog item, and ends no session. Those actions belong to the
bee CLI and to the live sessions that own that state; a dashboard that wrote there would
race a running agent. The one thing the board can put into a project — a task filed
from **+ New task** — goes through that project's own bee CLI, run at the project
root, exactly as an agent would file it; a project that carries no bee gets a refusal,
never a substitute writer (decision `cb52bbd1`).

This is enforced, not merely intended: the project's entire `.bee/` tree — every file
and every directory in it — is compared byte for byte before a request and after it,
for every kind of page this surface renders. A caching layer, or any code path that
merely listed a directory in a way that could create one as a side effect, would fail
that check.

### It renders nothing that identifies a filesystem outside the project

This surface carries no authentication of its own — nothing in waggledance does, including
the agent terminal (see the Agent terminal spec) — and can be bound to a non-loopback
address. bee's store is full of absolute paths — the files a cell touches,
a worker's identity, a session's transcript, a workspace root. **None of them may reach
the page.** A field that is itself entirely a path is rendered relative to the project
root, or dropped.

Free text is a harder case, and this board holds the guarantee there too: several
fields the board renders are free-form prose from the store — a feature's own boundary
sentence and description, a review finding's own description, a backlog item's own
detail — and any of these can have an absolute path typed into the middle of a
sentence, not as the whole field. Every one of those fields is scanned for an absolute
path embedded anywhere inside it, and any path found is reduced the same way a
wholly-path field would be, while the words around it survive untouched. A project's
operator can write "see /home/them/notes.txt for context" into a feature's description
and that sentence still renders, with the path portion alone reduced.

A string shaped like an application route rather than a filesystem path — for example
`/p/:id/_bee` — is not treated as a path by this scan and is left exactly as written;
only a string that resolves as a genuine absolute filesystem path is reduced.

A feature name is itself free text from the store, and this board has to join a
feature name onto a location on disk to list the markdown files recorded under that
feature's own history docs, for the Docs row on its detail page. Before any such join
happens, the name is validated: no path separators, no `..` segment, no leading dot,
no control characters, and not already an absolute path of any shape. A name that
fails validation is never looked up at all — the join is never attempted, so a
maliciously- or accidentally-shaped feature name can never make this board read, or
claim to check, anything outside the project it was asked about.

The tests that guard both halves of this guarantee assert against the **fixture's own
root path**, not against a literal that merely looks like a production path — a check
written against one hardcoded prefix would pass green while a real page leaked a real
path verbatim. See `docs/history/learnings/20260805-toothless-security-assertions.md`.

## Honesty rules that hold everywhere

Four rules apply across every section of this board, not just the ones above that
happen to illustrate them:

- **A dropped cell counts toward no total and no denominator, anywhere.** Not in a
  feature's own progress, not in its cell count on the Finished list, not in whether a
  feature counts as shipped. It never shipped, so counting it as done would inflate
  the picture; it is simply absent from every count that would otherwise include it.
- **A capped or truncated list always states its true total beside the visible
  subset.** A real store can be large — hundreds of backlog rows and findings are
  normal — so detail lists are bounded to a recent slice, and whenever that slice is
  smaller than the true total, the panel says so. The one list on this board that is
  never capped is the finished-features list — nothing that has shipped is ever
  silently left off it.
- **Nothing to measure renders as a stated absence, never as a zero or a division
  artifact.** "No features have shipped yet," "no live cells recorded for this feature
  yet," "no open backlog items yet" — these are the shape this rule takes. A number
  that is genuinely, computably zero — a bucket that really does hold no cells right
  now, or a feature whose real, measured time to finish happens to round to a very
  small figure — is not what this rule forbids; it forbids manufacturing a number
  where there was no measurement to take.
- **A store that cannot be fully read says so.** Any single unreadable file — missing,
  empty, truncated, or malformed — degrades the page to a partial view that names what
  could not be read, rather than silently dropping or miscounting the data it held. It
  never takes down the page, and a malformed line among otherwise-good lines loses
  only itself. A project with no store at all is a different, earlier case — presence,
  not degradation — and is a clean not-found, never an empty dashboard (see "Where it
  appears," above).

## Independent review is always invoked

Wherever this board mentions independent review — the Feature Hub's definition of
"waiting on you," and the review queue panel in Backlog & Review — it is worded the
same way: review is something the project's owner invokes, never a stage the board
implies is already running, already queued, or pending on its own. A gate that has not
yet been through review reads as "not yet approved," never as "review in progress"; a
candidate that has never appeared in a session reads as "never reviewed," never as
"awaiting automatic review." This holds even when the count of unreviewed work is
large enough that a different phrasing might read as more urgent — the wording never
implies the board itself is doing, or about to do, that work.

## Bounded output

The snapshot is rebuilt on every request, and a real store is large — hundreds of
backlog rows and thousands of decision events are normal. Detail lists are capped at a
small recent slice, and each panel states its true total when it is showing a capped
subset (see "Honesty rules that hold everywhere," above).

The Feature Hub and the standalone Finished list are archive-free: only live cells
feed a feature's progress and its shipped status there. A feature's own detail page,
and a cell page, are not — see "Drilling in," above.

## Scope

This surface covers **one project at a time**. A single page aggregating every
registered project — total active projects, velocity across all of them, which lanes
run where across the fleet — is a separate, later feature.
