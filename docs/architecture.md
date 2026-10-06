# Architecture

TPT Media Asset Intelligence is a local-first media archive indexing, search, dedupe and archive-health platform. Source of truth: [`../spec.txt`](../spec.txt); this document summarises how the codebase realises it.

## The one architectural boundary

The **engine** — everything except the Tauri shell — is usable without a GUI (spec §3.6): large archives can be indexed by the CLI or a headless service on a machine with no GUI session.

```
              Media Asset Intelligence Application
                          |
             +------------+-------------+
             |                          |
          Desktop (Tauri)           CLI / Service
             |                          |
             +------------+-------------+
                          |
                  Indexing/Search Engine
                          |
        +------------------+------------------+
        |                  |                  |
    Technical           Derivative          Tagging/
    metadata            generation          Search index
    extraction          (thumb/proxy/
                         waveform)
```

## Crates

| Crate | Role | Spec |
| :--- | :--- | :--- |
| `-core` | Identifiers, media types, SHA-256 fingerprints, engine version | §6 |
| `-model` | Domain model: archives, assets, derivatives, tags, groups, scenes, index entries | §6 |
| `-ingest` | Scanning, fingerprinting, technical metadata — built on `tpt-av-asset` | §7, §5.1 |
| `-dedupe` | Layered duplicate detection into reviewable groups and reports | §8 |
| `-scenes` | Deterministic scene-change detection configuration and pipeline | §9 |
| `-tagging` | Offline local-model tagging, manual tags, rejected-tag memory | §11 |
| `-search` | Shared query language and layered search | §10 |
| `-health` | Archive-health issues and snapshots | §12 |
| `-cli` | `tpt-media-asset-intel` batch interface, exit-code contract | §14 |
| `-service` | Background job host; optional localhost-only API (disabled by default) | §13.7, §15 |
| `-tauri` | Desktop shell (excluded from default build) | §13 |
| `-test` | Fixture discovery and synthetic archive helpers | §19 |

## Foundation, not duplication

The product builds on the TPT AV foundation rather than duplicating decode or caching work (spec §5):

| Responsibility | Foundation crate |
| :--- | :--- |
| Persistent asset metadata, embedded DB, job state | `tpt-av-asset-db` |
| Thumbnail/waveform caches, invalidation | `tpt-av-asset-cache` |
| Proxy generation | `tpt-av-asset-proxy` |
| Watch folders | `tpt-av-asset-watcher` |
| Background/resumable jobs | `tpt-av-asset-pipeline` |
| Video decode, frame access, technical metadata | `tpt-kinetix` (pinned git rev) |
| Audio decode, waveform | `tpt-cadence` (pinned git rev) |
| Scene change / perceptual similarity | `tpt-visual` |
| Golden fixtures, fuzzing harnesses | `tpt-av-test` |
| Transcription/diarisation (Phase 2, optional) | `tpt-voice` (not yet reachable — flagged risk) |

Local development consumes these as sibling path dependencies; the foundation workspaces themselves pin `tpt-kinetix`/`tpt-cadence` by git rev, which also proves the git-dependency route resolves (spec Phase 0 verification).

## The ingestion pipeline

Two passes (spec §7.1): Pass 1 (scan → fingerprint → technical metadata → metadata search index) is fast and always runs, so basic search returns within seconds of a scan starting regardless of archive size (§18). Pass 2 (derivatives → duplicates → scenes → tagging → full-text index) enriches progressively in the background without blocking the UI (§25).

Indexing is incremental and resumable (§18): a scan classifies each file as new, changed (size/mtime moved) or unchanged; interrupted runs reprocess only what needs processing.

## Persistence and derivatives

SQLite stores archives/roots/settings, asset paths + fingerprints (never raw media), derivative paths only, tags with source/confidence, duplicate groups with review status, health snapshots and preferences (§16). Cached derivative files live separately from the database and from the original archive.

## Security/privacy invariants

Enforced by tests wherever possible (§17, §19.4):

- No network call during indexing/tagging/search while cloud AI is disabled (the default).
- Cloud AI enablement is per-feature, disclosed (§10.1), never silent.
- The local API binds loopback only and is disabled by default (`-service` crate).
- Malformed media is handled safely; parsers are fuzz targets (§19.5).
- No automatic reorganisation/renaming/deletion of source files.
