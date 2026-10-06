# Search Model

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
