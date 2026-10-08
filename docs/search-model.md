# Search Model

<<<<<<< HEAD
Layer 1 (always): filename/path/extension, codec/container/resolution/duration, dates, manual tags.
Layer 2 (once processed): local-model tag labels, user notes, transcripts when enabled.
Layer 3 (Phase 2, opt-in): local embeddings by default; cloud embeddings strictly opt-in (§10.1 flow).

Query syntax: `field:value` clauses joined by AND, e.g. `codec:av1 AND tag:interview`.
Every result explains its match (matched field/tag/score, §3.5).
=======
Source of truth: [`../spec.txt`](../spec.txt) §10, §14, §3.5.

## Layers — reliability never depends on optional AI

| Layer | What it matches | Availability |
| :--- | :--- | :--- |
| 1 — Deterministic metadata | filename, path, extension, codec, container, resolution, frame rate, duration, dates, manual tags | always, within seconds of scan start (§7.1, §18) |
| 2 — Full text | local-model tag labels, transcripts (when enabled), user notes | once processed |
| 3 — Semantic/similarity | embeddings — local models by default, cloud strictly opt-in | optional (Phase 2, §10.1, §21) |

A user should never wait for full processing before performing a basic search (§7.1); the most reliable results never depend on optional AI (§10).

## Query language

One grammar for the GUI search bar and the CLI (§3.6, §14), implemented in `-search`:

```text
tpt-media-asset-intel search --archive main --query "codec:prores AND tag:interview"
```

- Terms: `word`, `field:value`, `"quoted phrase"`.
- Operators: uppercase `AND`, `OR`, `NOT`; adjacent terms imply `AND`; parentheses group.
- Fields are lowercased on parse; values are not — `CODEC:ProRes` searches codec `ProRes`.
- Lowercase `and` is a plain searchable word.
- Empty query / `*` match everything.
- Errors carry a byte position and never panic (the parser is a §19.5 fuzz target).

## Explainability

Every result explains why it matched — matched field, matched tag, or a 0.0–1.0 similarity score — never an unambiguous yes/no (§3.5, §13.2). Semantic matches in particular are similarity-scored, not binary verdicts.

## Cloud boundary

Layer 3 with a cloud provider exists only in Phase 2, behind the §10.1 disclosure/confirmation flow, per archive or per import job. With cloud AI disabled, no search code path performs network I/O — asserted by the §19.4 AI-boundary tests.
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
