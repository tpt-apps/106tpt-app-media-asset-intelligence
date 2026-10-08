<<<<<<< HEAD
# synthetic-archive-large (open codecs only)

Scale/performance fixture (§19.3): generated set of distinct-byte files for
indexing-throughput and memory regression tests.

Counts documented on generation:

- Regenerated at runtime by `tests/scale/throughput.rs` into a temp dir
  (no committed binaries).
- `UNIQUE_FILES = 10_000` unique files, `clip_000000.bin`… `clip_009999.bin`.
- `DUPLICATE_CLUSTER = 25` exact duplicates under `duplicates/` (one
  `sha256` fingerprint cluster) — golden expectation: **10_025 indexed
  assets, exactly 1 exact-hash duplicate group of size 25**.
- Fixture bytes are content-formatted (`unique-content-…`,
  `identical-block-of-bytes`) so fingerprints are deterministic.

Run: `cargo test --test scale_throughput`.
=======
# synthetic-archive-large

Large synthetic archive for scale/performance regression testing (spec §19.3, §26 step 21): indexing throughput and memory usage on a hundreds-of-thousands-of-files archive (§18).

**Status: to be generated in Phase 1.** Generated on demand by the scale test harness (too large to commit); the generator and expected aggregate counts are versioned here.
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
