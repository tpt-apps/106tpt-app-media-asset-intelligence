# Changelog

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
