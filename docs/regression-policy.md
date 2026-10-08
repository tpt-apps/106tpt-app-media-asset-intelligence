# Regression-Fixture Policy (§19.6)

Every production bug ships with a permanent regression test. A bug is not
"fixed" until a test that fails on the old code is committed alongside the fix
and passes on the new code.

## Rules

1. **One bug → one test.** Writing the regression test is part of the fix, not
   an afterthought. If the bug is a crash, the test asserts no panic; if it is
   wrong output, it asserts the corrected value; if it is a data-integrity
   issue, it asserts the invariant (e.g. a `Store` round-trip).
2. **Test at the lowest layer that can reproduce it.** A `search` bug goes in
   `-search`, a `scan_roots` bug in `-ingest`, a wrong row in `-persistence`,
   a job-transition bug in `-queue`. Use the crate's `#[cfg(test)]` module, or
   `tests/integration/<area>.rs` in the `-test` crate when the bug spans
   crates (persistence durability, queue crash-resume).
3. **Fixtures are permanent.** Byte fixtures live in `fixtures/` (§19.2),
   never generated ad hoc inside a test body. Generated test input (scale
   archives) is pinned and documented in the adjacent `README.md`. The
   `tpt-av-test` fuzz corpus is version-pinned in `Cargo.toml`; a new corpus
   case arrives as a permanent pinned revision, never "just run once".
4. **Fuzz properties for parser/CLI input.** Any parser bug that can be
   triggered by malformed input adds a property or a pinned seed case to
   `-test/src/fuzz.rs` as well as the repro test (§19.5).
5. **No fix without CI green.** CI runs `cargo test --workspace` and
   `cargo clippy --all-targets -- -D warnings` on every push; the regression
   test is part of that gate, so it runs forever.
6. **Track regressions in this repo's history only.** Changelog entries are
   added for the bug fix; the test is the durable record. A regression exempts
   itself from fixtures only when the repro is inherently environmental
   (e.g. Windows cleanup after open file handles): then the test asserts the
   *behavior boundary* instead (such tests are marked with a comment noting
   the non-fixture reason).

## Where things live

| Concern | Home |
| --- | --- |
| Archive/golden/fixture files | `fixtures/{synthetic-archive-small,synthetic-archive-large,duplicates,corrupt,mixed-formats}/` |
| Cross-crate on-disk regression tests | `tests/integration/persistence.rs`, `tests/integration/queue.rs` (in `-test` crate) |
| Scale/performance regressions | `tests/scale/throughput.rs` (§19.3) |
| Parser fuzzing + pinned seeds | `-test/src/fuzz.rs`, plus the pinned `tpt-av-test-fuzz` corpus |
| Media-crash guarantees | `-test/src/fuzz.rs` `tpt-av-test` prop: corpus must never panic a parser |