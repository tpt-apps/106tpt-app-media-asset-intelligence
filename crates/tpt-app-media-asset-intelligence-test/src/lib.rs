//! Test helpers: synthetic archive manifests for golden/scale tests (spec §19).

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let mut m = FixtureManifest::default();
        m.add("a.mkv", b"bytes");
        assert_eq!(m.files.len(), 1);
    }
}
