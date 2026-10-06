# Index Model

How an archive becomes an index. Source of truth: [`../spec.txt`](../spec.txt) §6, §7, §16, §18.

## Entities

- **Archive** (§6.1) — a named set of filesystem roots plus watch/AI settings. Cloud-AI fields default to disabled/None; there is no combined "enable AI" switch.
- **Asset** (§6.2) — one indexed media file: archive-normalized path, SHA-256 content fingerprint, size, mtime, broad media type, deterministic technical metadata. The index stores paths and fingerprints, **never raw media** (§16).
- **Derivative** (§6.3) — thumbnail/proxy/waveform record: kind, cache-relative path, and the engine version that generated it (drives cache invalidation). Derivative *files* live in a separate cache, outside the database and outside the archive (§16).
- **Tag** (§6.4) — label + provenance (`Manual`, `LocalModel{model, version}`, `CloudModel{provider, model}`) + confidence where applicable (§3.5).
- **DuplicateGroup** (§6.5) — members, match kind, similarity, review status, chosen keeper. Creation enforces ≥2 distinct members, similarity ∈ [0,1], and exact-hash similarity = 1.0.
- **Scene** (§6.6) — asset-local `start < end` time range plus detection method.
- **SearchIndexEntry** (§6.7) — text fields, tag references, optional embedding + its source.

## Paths

Archive-normalized paths use forward slashes and are relative to a root, so an index stays meaningful when a root moves between mounts (§12 broken/relinked-path detection). Strict path validation for watch-folder and NAS roots happens at the ingest boundary (§17).

## Fingerprints

The fingerprint is a SHA-256 over the file's bytes, the basis of exact-hash duplicate detection (§8): equal fingerprints mean byte-identical files regardless of name or location. Hex (64 lowercase characters) is the stable external representation.

## Determinism and versions

Technical metadata extraction produces the same result for the same bytes and application version (§3.2). Derivatives record `generated_with_version`; any change that can alter engine output — including detector defaults — bumps `ENGINE_VERSION` in `-core`.

## Incremental and resumable indexing

A scan classifies each file:

| Outcome | Meaning | Action (§18, §25) |
| :--- | :--- | :--- |
| `new` | not in the index | process through all stages |
| `changed` | size or mtime moved | reprocess |
| `unchanged` | identical size + mtime | skip |

An interrupted scan (crash, restart, unplugged NAS) resumes without re-processing unchanged, already-indexed assets — a §25 Definition-of-Done requirement.

## What is stored where

| Data | Stored in | Notes |
| :--- | :--- | :--- |
| Archives, roots, AI settings | SQLite | §16 |
| Asset paths + fingerprints | SQLite | never raw media |
| Derivative records | SQLite | paths only |
| Derivative files | derivative cache directory | separate from DB and archive |
| Tags (with source/confidence) | SQLite | §6.4 |
| Duplicate groups + review status | SQLite | §8 |
| Health snapshots | SQLite | §12, trend data |
| Preferences / AI enablement | SQLite | per archive |
