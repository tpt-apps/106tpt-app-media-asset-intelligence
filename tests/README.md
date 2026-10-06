# Test entry points (spec §19)

| Directory | Purpose |
| :--- | :--- |
| `integration/` | cross-crate pipeline tests: scan → index → search → report |
| `golden/` | assertions against `fixtures/` expected results (§19.2) |
| `scale/` | large-archive throughput/memory regression (§19.3) |

Unit tests live inside each crate (`§19.1`); fuzz targets live with the parsers they exercise and run on nightly (§19.5). Every production bug produces a permanent regression fixture (§19.6).

**Status: populated in Phase 1** (§26 steps 20–23) as the pipeline stages land.
