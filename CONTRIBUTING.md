# Contributing

1. Read `spec.txt` first — it is the source of truth; `todo.md` `§` references point back to it.
2. Keep indexing/search/tagging fully local by default; cloud AI is Phase 2 and strictly opt-in.
3. Never modify, move, rename, or delete source files without explicit user confirmation (§3.4).
4. Every tag records source (manual/local/cloud), confidence, and model version (§3.5).
5. Corrupt/malformed media must never crash indexing (§17); return errors, keep scanning.
6. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo deny check`.
7. Every production bug gets a permanent regression fixture (§19.6).
