<<<<<<< HEAD
//! Test helpers: synthetic archive manifests for golden/scale tests (spec §19).

#[cfg(all(test, feature = "tpt"))]
pub mod fuzz;

use std::collections::HashMap;

/// Expected counts for a synthetic fixture archive.
#[derive(Debug, Clone)]
pub struct FixtureExpectation {
    pub expected_assets: usize,
    pub expected_duplicate_groups: usize,
    pub expected_scenes: usize,
}

/// Manifest describing a synthetic archive fixture.
#[derive(Debug, Clone, Default)]
pub struct FixtureManifest {
    pub files: HashMap<String, Vec<u8>>,
}

impl FixtureManifest {
    pub fn add(&mut self, name: &str, bytes: &[u8]) {
        self.files.insert(name.to_string(), bytes.to_vec());
    }
=======
//! Test support for TPT Media Asset Intelligence (spec §19).
//!
//! Golden fixture archives live in the repository under `fixtures/` (spec
//! §19.2); this crate is how tests find them, regardless of the working
//! directory cargo happens to run from:
//!
//! | Fixture                      | Purpose (spec §19.2)                          |
//! | :---                         | :---                                          |
//! | `synthetic-archive-small`    | small archive with known expected results     |
//! | `synthetic-archive-large`    | scale/performance regression source (§19.3)   |
//! | `duplicates`                 | exact/perceptual/audio duplicate groups       |
//! | `corrupt`                    | malformed media that must not crash indexing  |
//! | `mixed-formats`              | container/codec breadth                       |
//!
//! Each fixture ships a `README.md` documenting its expected index counts,
//! duplicate groups and scene boundaries (spec §19.2); Phase 1 turns those
//! documents into asserted golden tests.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::path::{Path, PathBuf};

/// Returns the repository root (the directory containing `fixtures/`).
///
/// Walks upward from `CARGO_MANIFEST_DIR` of this crate, which is stable for
/// both workspace builds and out-of-tree test harnesses.
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors() {
        if ancestor.join("fixtures").is_dir() && ancestor.join("Cargo.toml").is_file() {
            return ancestor.to_path_buf();
        }
    }
    manifest
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}

/// Returns the path of one of the five golden fixture archives (spec §19.2).
///
/// # Panics
///
/// Panics for an unknown fixture name — a typo in a test, not a runtime condition.
pub fn fixture(name: &str) -> PathBuf {
    const KNOWN: [&str; 5] = [
        "synthetic-archive-small",
        "synthetic-archive-large",
        "duplicates",
        "corrupt",
        "mixed-formats",
    ];
    assert!(
        KNOWN.contains(&name),
        "unknown fixture `{name}`; expected one of {KNOWN:?}"
    );
    repo_root().join("fixtures").join(name)
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
<<<<<<< HEAD
    fn manifest_roundtrip() {
        let mut m = FixtureManifest::default();
        m.add("a.mkv", b"bytes");
        assert_eq!(m.files.len(), 1);
=======
    fn repo_root_contains_fixtures() {
        let root = repo_root();
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("fixtures").is_dir());
    }

    #[test]
    fn all_five_fixtures_exist_with_readmes() {
        for name in [
            "synthetic-archive-small",
            "synthetic-archive-large",
            "duplicates",
            "corrupt",
            "mixed-formats",
        ] {
            let dir = fixture(name);
            assert!(dir.is_dir(), "fixture {name} missing at {}", dir.display());
            assert!(
                dir.join("README.md").is_file(),
                "fixture {name} must document expected results (spec §19.2)"
            );
        }
    }

    #[test]
    #[should_panic(expected = "unknown fixture")]
    fn unknown_fixture_names_panic_in_tests() {
        let _ = fixture("not-a-fixture");
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
