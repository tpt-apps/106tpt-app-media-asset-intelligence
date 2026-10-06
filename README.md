# TPT Media Asset Intelligence

Local-first media archive indexing, search, dedupe, and archive-health platform (TPT Solutions).
Dual license: MIT OR Apache-2.0. Source of truth: `spec.txt`.

## What it is

Index terabytes of unsorted media (local disks, NAS shares, project folders) into something
searchable, understood, and maintainable — technical metadata, thumbnails, waveforms, proxies,
duplicate groups, scene boundaries, tags — **without uploading anything anywhere**.

## Supported codecs (MVP: open, royalty-free only)

- Video: AV1, VP9, VP8, Theora, FFV1
- Audio: Opus, Vorbis, FLAC, PCM/WAV
- Containers: Matroska/WebM, Ogg, WAV, FLAC (MP4 only where it carries an open codec, e.g. AV1)

Out-of-scope files (H.264/AAC/ProRes, …) are still indexed by path/fingerprint, recorded as
`unsupported`, and flagged in archive health. Derivatives, scene detection, and audio
fingerprinting are skipped for them. Indexing never crashes on them.

## Quickstart

```powershell
cargo build
cargo test
cargo run -p tpt-app-media-asset-intelligence-cli -- index --archive main --roots D:\footage
cargo run -p tpt-app-media-asset-intelligence-cli -- search --archive main --query "codec:av1 AND tag:interview"
cargo run -p tpt-app-media-asset-intelligence-cli -- dedupe --archive main --report duplicates.json
```

## Layout

- `crates/*-core` — codec allowlist, fingerprinting, error type
- `crates/*-model` — Archive/Asset/Derivative/Tag/DuplicateGroup/Scene domain model (§6)
- `crates/*-ingest` — filesystem scanning, fingerprinting, resumable indexing (§7, §18)
- `crates/*-dedupe`, `*-scenes`, `*-tagging`, `*-search`, `*-health` — pipeline stages (§8–§12)
- `crates/*-cli`, `*-service`, `*-tauri` — shells over the shared engine (§13–§15)
- `crates/*-test` — fixture helpers (§19)
- `fixtures/` — golden fixture archives (open codecs only; `mixed-formats/` also holds a few
  H.264/AAC files solely to verify the unsupported-codec path)
- `tests/` — integration, golden, scale suites

## Integration risks (Phase 0)

`tpt-av-asset`, `tpt-kinetix`, `tpt-cadence`, `tpt-visual`, `tpt-voice`, and `tpt-av-test`
were not reachable from this workspace at scaffold time — no local paths, no crates.io
entries. They are flagged as integration risks in `docs/INTEGRATION_RISKS.md`; the workspace
builds and tests fully offline without them, with seam crates ready to wire in.
