# TPT Media Asset Intelligence

**Find, understand, organise, and process your entire media archive — without uploading it anywhere.**

TPT Media Asset Intelligence is a native, local-first application that indexes a media archive — local disks, NAS shares, project folders accumulated over years — and turns it into something searchable, understood, and maintainable: technical metadata, thumbnails, waveforms, proxies, duplicate groups, scene boundaries, tags, and (optionally) transcripts and speaker labels.

Part of the TPT Apps family. Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Why

Every production company with a growing media archive eventually asks: *what do we actually have, where is it, and how do I find the exact clip I need?*

This product is professional archive infrastructure — not a video editor, not an enterprise MAM platform requiring IT deployment, not a file manager. It turns terabytes of unsorted or loosely organised media into a searchable, health-monitored archive, **on the hardware the customer already owns**.

## Principles

- **Local-first** ([spec §3.1](spec.txt)): the index and all derivatives live on the user's own storage. No account, no hosted index, no mandatory internet.
- **Deterministic analysis** (§3.2): the same input bytes and version produce the same technical metadata, always.
- **AI is additive, not required** (§3.3): indexing, deterministic search, dedupe, scene detection and local-model tagging work with zero cloud AI. Cloud AI is off by default, per-feature, disclosed, and revocable.
- **Non-destructive** (§3.4): the app indexes and generates derivatives; it never modifies, moves, renames or deletes source files without an explicit, reviewed user action.
- **Every tag and match is explained** (§3.5): sources, confidence values, similarity scores — never unambiguous fact.
- **Engine and presentation are separate** (§3.6): the same engine backs the desktop app, the CLI, and headless service deployment.

## What it does

| Area | Summary | Spec |
| :--- | :--- | :--- |
| Ingestion | Filesystem/watch-folder scanning, fingerprinting, technical metadata, incremental/resumable indexing | §7 |
| Search | Layered: deterministic metadata → full-text over tags/notes → optional semantic | §10 |
| Dedupe | Exact hash → perceptual → audio fingerprint; reviewable groups, never auto-delete | §8 |
| Scenes | Deterministic frame-difference/perceptual-hash scene changes | §9 |
| Tagging | Offline local-model auto-tagging with visible source/confidence + manual tags | §11 |
| Archive health | Missing/corrupt/moved assets, orphaned derivatives, duplicate-waste trends | §12 |
| Automation | CLI (`index`, `search`, `dedupe`, `tag`) with a stable exit-code contract | §14 |

Cloud AI features (semantic search, cloud tagging) and transcription are **explicitly out of MVP scope** and arrive in Phase 2 behind the §10.1 disclosure flow.

## Repository layout

```
crates/
├── tpt-app-media-asset-intelligence-core      Shared primitives: ids, fingerprints, media types
├── tpt-app-media-asset-intelligence-model     Domain model (archives, assets, tags, groups…)
├── tpt-app-media-asset-intelligence-ingest    Scanning, fingerprinting, metadata (on tpt-av-asset)
├── tpt-app-media-asset-intelligence-dedupe    Duplicate detection groups + reports
├── tpt-app-media-asset-intelligence-scenes    Deterministic scene detection config
├── tpt-app-media-asset-intelligence-tagging   Local-model tagging + rejected-tag memory
├── tpt-app-media-asset-intelligence-search    Query language + layered search
├── tpt-app-media-asset-intelligence-health    Archive health snapshots
├── tpt-app-media-asset-intelligence-cli       tpt-media-asset-intel batch interface
├── tpt-app-media-asset-intelligence-service   Headless host + optional localhost API
├── tpt-app-media-asset-intelligence-tauri     Desktop shell (excluded from default build)
└── tpt-app-media-asset-intelligence-test      Fixture helpers
fixtures/   Golden fixture archives (small/large/duplicates/corrupt/mixed-formats)
tests/      Integration, golden and scale test entry points
docs/       Architecture and model documentation
```

The engine boundary matters: everything except the Tauri shell is engine, usable without a GUI (spec §3.6).

## Building

Requires a stable Rust toolchain (see `rust-version` in [Cargo.toml](Cargo.toml)).

```sh
cargo build --workspace
cargo test --workspace
```

The foundation crates (`tpt-av-asset` and friends) are consumed as sibling path dependencies for local development (see `[workspace.dependencies]`); the CI workflow clones them next to this checkout. The desktop shell builds separately:

```sh
cargo build --manifest-path crates/tpt-app-media-asset-intelligence-tauri/Cargo.toml
```

## Quickstart (CLI)

```sh
tpt-media-asset-intel index --archive ./archive-config.yaml --roots /mnt/media-nas/projects
tpt-media-asset-intel search --archive main --query "codec:prores AND tag:interview"
tpt-media-asset-intel dedupe --archive main --report duplicates.json
tpt-media-asset-intel tag --archive main --local-only
```

Machine-readable output always includes `cloud_ai_used`, and the exit-code contract (0 SUCCESS … 6 INTERNAL_ERROR) is stable (spec §14).

## Status

Phase 0 (workspace, licensing, foundation verification, scaffolds) is complete; see [todo.md](todo.md) for the live task list and [CHANGELOG.md](CHANGELOG.md) for history. The MVP build per spec §20 is in progress.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Every production bug produces a permanent regression fixture (spec §19.6).
