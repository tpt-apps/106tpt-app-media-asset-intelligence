# Changelog

<<<<<<< HEAD
All notable changes to this project will be documented in this file.

## [0.1.0] — Unreleased

- Scaffold 12-crate Cargo workspace (core/model/ingest/dedupe/scenes/tagging/search/health/cli/service/tauri/test).
- Open-codec-only MVP scope: AV1/VP9/VP8/Theora/FFV1, Opus/Vorbis/FLAC/PCM; unsupported-codec path that indexes-but-flags.
- Domain model (§6), resumable scanner, exact-hash dedupe, deterministic scene splitter, gated tagging, Layer 1/2 search, health checks, CLI with stable exit codes, localhost-only service surface.
- Docs skeleton, CI (build/test/clippy/fmt + cargo-deny), fixture/test directories.
- Bumped `tpt-kinetix` pin from `dab6415` (pre-master, only fetchable by SHA) to current master `b904ff3`; `-ingest`/`-model` (`--features tpt`) and the `-test --features tpt` fuzz/reference suites all green against the new rev.
- Testing suite: proptest fuzz targets (10) + regression corpus, scale throughput regression (10k files), AI-boundary integration tests (6).
- SQLite persistence crate (§16): DDL v1 with archives/roots/assets/derivatives/tags/duplicate-groups/scenes/health-snapshots/jobs/preferences, full store API with FK cascades, `PRAGMA user_version` migration hook, plus on-disk durability integration tests.
- Job queue executor crate (§13.7, §18): indexing/derivative/tagging jobs with pause/resume/cancel, persisted transitions, crash recovery (interrupted `Active` jobs requeue and re-run idempotently), `available_parallelism()`-bounded worker pool, real scanner-backed `Index` workers that persist assets + archive-health snapshots; end-to-end resume integration test.
- Hardening + failure isolation (§26 step 24): production code paths are panic-free — CLI serialization degrades to `INTERNAL_ERROR`, `list_archives` propagates decode errors, queue mutex-poison maps to `QueueError::LockPoisoned`, and a test proves a panicking worker is contained while the queue stays usable.
- Regression-fixture policy (§19.6) documented in `docs/regression-policy.md`; CI now lints all targets clippy-clean (`--all-targets`).
- Windows release packaging (§26 step 25): `scripts/package-windows.ps1` + `.github/workflows/release.yml` (full gate then versioned zip artifact on `v*` tags; publishing to GitHub Releases/registries intentionally unwired).
- CLI now runs the same engine as the GUI (§14): `index` uses `--data-dir`/`TMAI_DATA_DIR`/platform app-data, persists a durable SQLite archive through the job queue (assets + per-run health snapshots + reconciled exact-duplicate groups); `search` matches the persisted archive with reasons; `dedupe` recomputes and persists groups. `Store::duplicate_groups_for_archive` / `list_duplicate_groups` added for this and the health/service views.
- Local API HTTP binding (§15): dependency-free `ServiceServer` bound to 127.0.0.1 only, disabled by default, serving `/archives/:id/search`, `/assets/:id`, `/archives/:id/reindex`, `/jobs/:id`, `/health` over the queue+store, with an end-to-end lifecycle test (reindex → job → search → asset → 404). The operation logic lives in `service::handlers`, shared with the desktop command layer.
- Desktop command layer (§13 engine): `tauri::TauriCommands` reuses `service::handlers` for open-local data dir, reindex, run queue, job status, search, asset, health and health trend on the same `tptmai.sqlite` the CLI uses; UI screens §13.1–§13.7 remain pending.
- Archive-health trend reporting (§21 engine): `HealthTrend` rows with per-snapshot intervention counts + deltas and defensive chronological sorting; fed by the per-run snapshots the index workers already persist.
=======
All notable changes to TPT Media Asset Intelligence are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Pass 1 engine, end to end** (spec §26 steps 3–6, 8): recursive filesystem scanning with failure isolation and strict root validation (§7, §17), streaming SHA-256 fingerprinting (§7, §18), deterministic technical metadata extraction on the tpt-av-asset foundation — kinetix video, cadence audio (§3.2, §5.2) — an in-memory `ArchiveIndex` assigning asset ids and feeding incremental rescans (§18), Layer 1 query evaluation with per-result explanations (§10, §13.2), and exact-hash duplicate grouping with the shared waste report (§8).
- Engine integration test (`-ingest/tests/engine.rs`) that builds a synthetic archive with the foundation's test-media helpers (real WAV + lossless-video decodes, no ffmpeg required), drives scan → probe → index → dedupe → search, and proves unchanged rescans reprocess nothing (§18, §25).

### Changed

- `AiSettings` now derives `Default`; a test pins the §6.1 all-disabled defaults.
- `MediaType` parsing moved to the `FromStr` trait.

### Added (Phase 0)

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
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
