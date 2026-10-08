# Architecture

Engine/presentation split (§3.6): deterministic indexing/search core under `crates/`,
hosted by CLI, background service, or Tauri desktop shell.

- `-core`: open-codec allowlist (AV1/VP9/VP8/Theora/FFV1; Opus/Vorbis/FLAC/PCM), SHA-256 fingerprinting, error type.
- `-model`: Archive/AiSettings (cloud off by default), Asset, Derivative, Tag (source/confidence/version), DuplicateGroup, Scene, SearchIndexEntry (§6).
- `-ingest`: recursive scanner with strict root validation; corrupt files recorded, never fatal (§17); `ResumeLog` for resumability (§18).
- `-dedupe`: exact-hash groups now; perceptual (tpt-visual) and audio-fingerprint (tpt-cadence) seams reserved (§8).
- `-scenes`: deterministic score-threshold splitter; frame scoring plugs in via tpt-visual (§9).
- `-tagging`: local-model + manual tags; `CloudTaggingGate` makes cloud tagging uncallable without an explicit flag; rejected-tag memory (§11).
- `-search`: Layer 1 deterministic + Layer 2 full-text; Layer 3 semantic is Phase 2 (§10).
- `-health`: missing/corrupt/orphan/waste checks (§12).
- `-persistence`: SQLite store, DDL v1 (`PRAGMA user_version`), archives/roots/assets (paths+fingerprints only)/derivatives (paths only)/tags/duplicate-groups/scenes/health-snapshots/jobs/preferences; FK cascades; in-memory and on-disk modes (§16).
- `-queue`: job queue executor over the store — indexing/derivative/tagging jobs with pause/resume/cancel, crash recovery (interrupted `Active` jobs requeue), `available_parallelism()`-bounded worker pool, deterministic resumable asset ids; `Index` runs the real scanner (§13.7, §18).
- `-cli`: same-engine shell over the queue+store (`--data-dir`/`TMAI_DATA_DIR`/platform app-data): `index` enqueues a durable job and persists assets + health snapshots + duplicate groups; `search` and `dedupe` read the persisted archive (§14).
- `-service`: localhost-only (127.0.0.1), disabled-by-default, dependency-free HTTP/1.1 surface over the queue+store: `/archives/:id/search`, `/assets/:id`, `/archives/:id/reindex`, `/jobs/:id`, `/health` (§15). Exposes `-service::handlers` — the shared operation logic the HTTP routes and the desktop commands both call.
- `-tauri`: same-engine desktop command layer (`TauriCommands` over `-service::handlers`; DB shared with the CLI); §13.1–§13.7 screens pending the GUI runtime.

Quality gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` on every push. Production code paths are panic-free (queue mutex-poison maps to `QueueError::LockPoisoned`; see §26 step 24). Regression-fixture policy in `docs/regression-policy.md` (§19.6): one bug, one permanent test.

Out-of-scope codecs index by path/fingerprint as `unsupported`, skip derivatives/scene/audio work, and surface in health — never crash.
