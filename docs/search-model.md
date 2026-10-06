# Search Model

Layer 1 (always): filename/path/extension, codec/container/resolution/duration, dates, manual tags.
Layer 2 (once processed): local-model tag labels, user notes, transcripts when enabled.
Layer 3 (Phase 2, opt-in): local embeddings by default; cloud embeddings strictly opt-in (§10.1 flow).

Query syntax: `field:value` clauses joined by AND, e.g. `codec:av1 AND tag:interview`.
Every result explains its match (matched field/tag/score, §3.5).
