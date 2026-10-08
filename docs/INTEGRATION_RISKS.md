# Integration Risks (Phase 0 verification)

Resolved 2026-10-06 — all six TPT foundation repos are live on GitHub and
wired as rev-pinned git dependencies (ecosystem convention per
`tpt-av-asset/Cargo.toml` and `tpt-visual/Cargo.toml`). Default
`cargo test` stays offline-fast; `--features tpt` enables the wired build.

| Crate | Rev (master HEAD at wiring) | Used for |
|---|---|---|
| `tpt-av-asset` | `bb257b3` | AssetId/MediaInfo, AssetDb, CacheStorage, pipeline/importer, MediaWatcher |
| `tpt-kinetix` | `b904ff3` (bumped 2026-10-07 from `dab6415`, which was pre-master) | MKV/WebM demux (`MkvDemuxer`), AV1/VP9 decoder crates available |
| `tpt-cadence` | `95ff6bf` | WAV/FLAC/Ogg-Opus/Vorbis readers via `FormatReader` |
| `tpt-visual` | `7f79eb9` | `VideoFrame`/pixel types (scene/perceptual operate on decoded RGBA) |
| `tpt-voice` | `05ffff3` | Phase 2 only (out of MVP §20) — not wired |
| `tpt-av-test` | `4571941` | `reference` + `fuzz` harnesses (dev-deps of `-test`) |

`tpt-kinetix` also appears as a transitive pin (`9747a2b`) from inside
`tpt-av-test`'s own dependency tree; source revs coexist in the lockfile.

Codec-scope verification: kinetix ships open codecs (AV1/VP9) plus a
separate `out-kinetix-h264` crate that is NOT in our dependency tree;
cadence ships WAV/AIFF/FLAC/AAC/Opus/MP3/Vorbis — our ingest only
references the WAV/FLAC/Ogg/Opus/Vorbis readers, never AAC/MP3. The
`-core` allowlist enforces this at runtime regardless of what decoders
exist upstream. Patent-encumbered paths (H.264 MP4, AAC) probe as
recognised-but-`unsupported`: indexed by path/fingerprint, flagged in
health, never a crash.
