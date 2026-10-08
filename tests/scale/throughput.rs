//! Scale regression: indexing throughput over a large synthetic archive
//! (§19.3, §26 step 21). The `synthetic-archive-large` fixture is generated
//! at runtime in a temp dir (no committed binaries) — generator shape and
//! counts are documented in `fixtures/synthetic-archive-large/README.md`.
//!
//! Guards: every generated file is indexed, exact-hash dedupe groups the
//! duplicate cluster at scale, Pass-1 order is deterministic, and
//! throughput stays above a floor (catches gross regressions, not flaky
//! sub-second noise).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tpt_app_media_asset_intelligence_dedupe::exact_duplicates;
use tpt_app_media_asset_intelligence_ingest::scan_roots;
use uuid::Uuid;

/// Unique files in the generated archive (spec §18 targets tens of
/// thousands per run; the full 100k+ benchmark runs against a real
/// archive, §26 step 25).
const UNIQUE_FILES: usize = 10_000;
/// Exact-duplicate cluster size to verify dedupe grouping at scale.
const DUPLICATE_CLUSTER: usize = 25;
/// Total indexed files expected.
const EXPECTED_FILES: usize = UNIQUE_FILES + DUPLICATE_CLUSTER;
/// Minimum Pass-1 throughput to hold the line on (lenient: catches
/// catastrophic regressions only, tolerant of slow CI runners).
const MIN_FILES_PER_SEC: f64 = 200.0;

fn make_fixture(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    for i in 0..UNIQUE_FILES {
        std::fs::write(
            root.join(format!("clip_{i:06}.bin")),
            format!("unique-content-{i:020}"),
        )
        .unwrap();
    }
    let dup_dir = root.join("duplicates");
    std::fs::create_dir_all(&dup_dir).unwrap();
    for i in 0..DUPLICATE_CLUSTER {
        std::fs::write(
            dup_dir.join(format!("copy_{i:02}.bin")),
            b"identical-block-of-bytes",
        )
        .unwrap();
    }
}

fn fixture_dir() -> PathBuf {
    std::env::temp_dir().join("mai-synthetic-archive-large")
}

#[test]
fn scale_throughput_regression() {
    let dir = fixture_dir();
    let _ = std::fs::remove_dir_all(&dir);
    make_fixture(&dir);

    let started = std::time::Instant::now();
    let (found, errors) = scan_roots(std::slice::from_ref(&dir));
    let elapsed = started.elapsed();

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        errors.is_empty(),
        "scan of the generated fixture must be clean: {errors:?}"
    );
    assert_eq!(
        found.len(),
        EXPECTED_FILES,
        "every generated file must be indexed"
    );

    // Exact-hash dedupe at scale: exactly one group, the duplicate cluster.
    let map: HashMap<Uuid, String> = found
        .iter()
        .enumerate()
        .map(|(i, f)| (Uuid::from_u128(i as u128), f.fingerprint.clone()))
        .collect();
    let groups = exact_duplicates(&map);
    assert_eq!(groups.len(), 1, "only the duplicate cluster is a group");
    assert_eq!(groups[0].asset_ids.len(), DUPLICATE_CLUSTER);

    // Deterministic Pass 1 order (spec §18 target).
    assert!(
        found.windows(2).all(|w| w[0].path <= w[1].path),
        "scan output must be path-sorted"
    );

    let files_per_sec = EXPECTED_FILES as f64 / elapsed.as_secs_f64();
    eprintln!("indexed {EXPECTED_FILES} files in {elapsed:?} ({files_per_sec:.0} files/s)");
    assert!(
        files_per_sec >= MIN_FILES_PER_SEC,
        "throughput regression: {files_per_sec:.0} files/s < {MIN_FILES_PER_SEC}"
    );
}
