use std::collections::{HashMap, HashSet};

/// Fingerprint log of already-indexed assets so an interrupted scan
/// resumes without reprocessing unchanged files (spec §18).
#[derive(Debug, Default)]
pub struct ResumeLog {
    indexed: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexedState {
    pub already_indexed: bool,
}

impl ResumeLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_indexed(&mut self, fingerprint: &str, path: &str) {
        self.indexed
            .insert(fingerprint.to_string(), path.to_string());
    }

    pub fn state(&self, fingerprint: &str) -> IndexedState {
        IndexedState {
            already_indexed: self.indexed.contains_key(fingerprint),
        }
    }

    /// Fingerprints from `candidates` not yet indexed.
    pub fn pending<'a>(&self, candidates: &'a [String]) -> Vec<&'a str> {
        let known: HashSet<&str> = self.indexed.keys().map(String::as_str).collect();
        candidates
            .iter()
            .filter(|c| !known.contains(c.as_str()))
            .map(String::as_str)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_skips_indexed_fingerprints() {
        let mut log = ResumeLog::new();
        log.mark_indexed("fp1", "/a");
        assert!(log.state("fp1").already_indexed);
        assert!(!log.state("fp2").already_indexed);
        assert_eq!(log.pending(&["fp1".into(), "fp2".into()]), vec!["fp2"]);
    }

    #[test]
    fn empty_log_marks_everything_pending() {
        let log = ResumeLog::new();
        assert_eq!(log.pending(&["x".into()]), vec!["x"]);
    }
}
