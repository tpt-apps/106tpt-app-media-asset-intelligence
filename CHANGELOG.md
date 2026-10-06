# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] — Unreleased

- Scaffold 12-crate Cargo workspace (core/model/ingest/dedupe/scenes/tagging/search/health/cli/service/tauri/test).
- Open-codec-only MVP scope: AV1/VP9/VP8/Theora/FFV1, Opus/Vorbis/FLAC/PCM; unsupported-codec path that indexes-but-flags.
- Domain model (§6), resumable scanner, exact-hash dedupe, deterministic scene splitter, gated tagging, Layer 1/2 search, health checks, CLI with stable exit codes, localhost-only service surface.
- Docs skeleton, CI (build/test/clippy/fmt + cargo-deny), fixture/test directories.
- `tpt-av-asset`/`kinetix`/`cadence`/`visual`/`voice`/`av-test` flagged as unresolved integration risks.
