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
- `-cli`/`-service`/`-tauri`: same-engine shells (§13–§15).

Out-of-scope codecs index by path/fingerprint as `unsupported`, skip derivatives/scene/audio work, and surface in health — never crash.
