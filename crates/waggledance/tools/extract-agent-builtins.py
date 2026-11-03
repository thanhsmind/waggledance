#!/usr/bin/env python3
"""Extract Claude Code's registered slash commands from the installed binary.

Each command is a minified object literal carrying `type:"local"|"local-jsx"|
"prompt"`, a `name:"..."`, and usually a `description:"..."`. Anchor on the
type field so tool definitions, CLI flags and npm package blurbs (which also
carry name/description pairs) stay out.

The trap this parser exists to avoid: a command object with NO description of
its own will happily borrow the next object's if the read window is not
bounded — that is how /login ended up described as "Sign out". Every field is
therefore read only from the slice between this object's own `name:` and the
next `name:`/`type:` marker.
"""
import re
import sys
import json

path = sys.argv[1]
blob = open(path, "rb").read().decode("utf-8", "replace")

anchor = re.compile(r'type:"(?:local|local-jsx|prompt)"')
name_re = re.compile(r'name:"([a-z][a-z0-9-]*)"')
next_marker = re.compile(r'(?:name:"|type:")')
desc_re = re.compile(r'description:"((?:[^"\\]|\\.){3,240})"')
hint_re = re.compile(r'argumentHint:"((?:[^"\\]|\\.){0,80})"')
hidden_re = re.compile(r'isHidden(?:\(\))?\s*:\s*(?:!0|true)')

found = {}
for m in anchor.finditer(blob):
    head = blob[m.end():m.end() + 600]
    n = name_re.search(head)
    if not n or n.start() > 220:
        continue
    name = n.group(1)
    # Everything after this object's own name, cut at the next object's
    # marker — no borrowing a neighbour's description or hint.
    rest = head[n.end():]
    nxt = next_marker.search(rest)
    body = rest[: nxt.start()] if nxt else rest
    # `description` can also sit BEFORE the name inside the same object; that
    # slice is bounded by the anchor itself, so it is safe to read too.
    before = head[: n.start()]
    d = desc_re.search(body) or desc_re.search(before)
    h = hint_re.search(body) or hint_re.search(before)
    rec = {
        "name": name,
        "description": d.group(1) if d else None,
        "argument_hint": h.group(1) if h else None,
        "hidden": bool(hidden_re.search(body) or hidden_re.search(before)),
    }
    prev = found.get(name)
    if prev is None or (rec["description"] and not prev["description"]):
        found[name] = rec

rows = sorted(found.values(), key=lambda r: r["name"])
print(json.dumps(rows, indent=1, ensure_ascii=False))
print(
    f"# total {len(rows)}, with description {sum(1 for r in rows if r['description'])}, "
    f"hidden-marked {sum(1 for r in rows if r['hidden'])}",
    file=sys.stderr,
)
