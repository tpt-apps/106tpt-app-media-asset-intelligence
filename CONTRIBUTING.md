<<<<<<< HEAD
# Contributing

1. Read `spec.txt` first — it is the source of truth; `todo.md` `§` references point back to it.
2. Keep indexing/search/tagging fully local by default; cloud AI is Phase 2 and strictly opt-in.
3. Never modify, move, rename, or delete source files without explicit user confirmation (§3.4).
4. Every tag records source (manual/local/cloud), confidence, and model version (§3.5).
5. Corrupt/malformed media must never crash indexing (§17); return errors, keep scanning.
6. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo deny check`.
7. Every production bug gets a permanent regression fixture (§19.6).
=======
# Contributing to TPT Media Asset Intelligence

Thank you for helping improve TPT Media Asset Intelligence — a local-first media archive indexing, search, dedupe and archive-health platform.

## Ground rules

1. **Local-first is non-negotiable** (spec §3.1, §17). No mandatory network access for indexing, deterministic search or local-model tagging. No cloud upload of original media, ever, under any setting. No external telemetry of filenames, tags or archive contents.
2. **Non-destructive to source archives** (spec §3.4). The application never modifies, moves, renames or deletes original files without an explicit, reviewed user action. Dedupe suggests; humans decide.
3. **Determinism** (spec §3.2). Technical metadata extraction, scene detection and duplicate hashing must produce identical output for identical input bytes and version. If your change alters engine output, bump `ENGINE_VERSION` in `tpt-app-media-asset-intelligence-core`.
4. **AI is additive** (spec §3.3). Cloud AI is off by default, enabled per feature, disclosed per §10.1, and revocable without losing local results. The AI-boundary tests (§19.4) must stay green.
5. **Explain everything** (spec §3.5). Every tag carries source + confidence; every match explains itself. Never present model output as unambiguous fact.

## Development setup

- Stable Rust (see `rust-version` in [Cargo.toml](Cargo.toml)).
- The TPT foundation crates are sibling path dependencies (`../tpt-av-asset` etc.); CI clones them into the expected layout — mirror that locally or adjust `[workspace.dependencies]` paths in a PR only with strong reason.
- `cargo build --workspace && cargo test --workspace` must pass. The Tauri shell is excluded from the default workspace and is not required for engine work.

## Before you open a PR

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check        # license/advisory policy (deny.toml)
```

## Testing expectations (spec §19)

- Every pipeline stage has unit tests covering valid, invalid, boundary and malformed input (§19.1).
- Golden fixture archives live in `fixtures/` with documented expected index counts, duplicate groups and scene boundaries (§19.2). If your change is intentional output-changing, update the fixture expectations in the same PR and say why.
- **Every production bug produces a permanent regression fixture** (§19.6). Reference the issue in the test name.
- Media/metadata parsers, the archive-config and search-query parsers, and CLI arguments are fuzz targets (§19.5). If you touch a parser, keep it panic-free on arbitrary input.
- Disabling cloud AI must never delete or invalidate previously generated local-model tags (§19.4) — there are tests; keep them passing.

## Commit style

Short imperative subject (`scene: merge strobe cuts below min length`), body explaining the *why*. Reference spec sections (`§9`) and issues where relevant.

## Licensing

By contributing you agree your work is dual licensed [MIT OR Apache-2.0](LICENSE-MIT) like the rest of the project.
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
