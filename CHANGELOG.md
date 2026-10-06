# Changelog

All notable changes to TPT Media Asset Intelligence are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Dual MIT OR Apache-2.0 licensing, `deny.toml` license/advisory policy, and the Cargo workspace with eleven engine crates plus an excluded Tauri desktop shell (spec §4).
- Domain model per spec §6: `Archive`/`AiSettings` with cloud fields defaulting to disabled, `Asset` with SHA-256 `AssetFingerprint` and deterministic `TechnicalMetadata`, `Derivative`, `Tag`/`TagSource` with confidence and rejected-tag memory, `DuplicateGroup` with review status and keeper selection, `Scene`, and `SearchIndexEntry`.
- `tpt-av-asset` foundation wiring in the ingest crate, with a capability map (spec §5.1) and compile-time reachability tests.
- Two-pass pipeline stage model (spec §7.1) and filesystem scan outcome classification for incremental/resumable indexing (spec §18).
- Dedupe report with reclaimable-waste arithmetic shared by the CLI and the Archive Health dashboard (spec §8, §12).
- Search query language parser (AST, positioned errors, display round-trip) shared by GUI and CLI (spec §10, §14); a designated fuzz target (spec §19.5).
- Archive health issue model and snapshot summaries (spec §12).
- CLI skeleton with the stable exit-code contract (0–6, spec §14), clap definitions, `cloud_ai_used` result envelopes, and cloud-tagging gated behind an explicit two-flag confirmation.
- Service crate with the §15 local API invariants encoded: loopback-only bind, disabled by default.
- Fixture directories per spec §19.2 (`synthetic-archive-small`, `synthetic-archive-large`, `duplicates`, `corrupt`, `mixed-formats`) and the `tests/` tree (integration/golden/scale).
- CI: fmt, clippy, test, cargo-deny (Linux reference platform; foundation siblings cloned in the expected layout).
- Documentation skeleton: `docs/architecture.md`, `index-model.md`, `search-model.md`, `tagging-model.md`, `ai-disclosure.md`, `archive-health.md`.
