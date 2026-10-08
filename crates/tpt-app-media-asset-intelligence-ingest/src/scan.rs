<<<<<<< HEAD
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tpt_app_media_asset_intelligence_core::fingerprint_bytes;
use walkdir::WalkDir;

/// A file discovered during a scan with its fingerprint.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub fingerprint: String,
}

/// Fingerprint one file's bytes (best-effort: unreadable files yield
/// an empty fingerprint; the scan error path reports them).
pub fn scan_fingerprint(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => fingerprint_bytes(&bytes),
        Err(_) => String::new(),
    }
}

/// Strict path validation for watch-folder/NAS roots (spec §17).
pub fn validate_root(root: &Path) -> Result<PathBuf, String> {
    if root.as_os_str().is_empty() {
        return Err("root path is empty".to_string());
    }
    if root.components().count() == 0 {
        return Err("root path has no components".to_string());
    }
    Ok(root.to_path_buf())
}

/// Recursively scan roots. A corrupt/unreadable file is recorded in
/// `errors` — it never aborts the scan (spec §17).
pub fn scan_roots(roots: &[PathBuf]) -> (Vec<ScannedFile>, Vec<String>) {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for root in roots {
        if let Err(e) = validate_root(root) {
            errors.push(format!("{}: {e}", root.display()));
            continue;
        }
        if !root.exists() {
            errors.push(format!("{}: missing root", root.display()));
            continue;
        }
        for entry in WalkDir::new(root).into_iter() {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    errors.push(format!("walk error: {e}"));
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            match fs::read(&path) {
                Ok(bytes) => {
                    found.push(ScannedFile {
                        path,
                        size_bytes: bytes.len() as u64,
                        fingerprint: fingerprint_bytes(&bytes),
                    });
                }
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
    }
    // Deterministic order: Pass 1 results surface predictably (§18 target).
    found.sort_by(|a, b| a.path.cmp(&b.path));
    (found, errors)
}

/// Group scanned files by fingerprint (feeds exact-hash dedupe, spec §8).
pub fn group_by_fingerprint(files: &[ScannedFile]) -> HashMap<&str, Vec<&ScannedFile>> {
    let mut map: HashMap<&str, Vec<&ScannedFile>> = HashMap::new();
    for f in files {
        map.entry(f.fingerprint.as_str()).or_default().push(f);
    }
    map
=======
//! Filesystem scan results (spec §7, §12, §18).
//!
//! A scan walks an archive's roots and, for each file, decides whether it is
//! new, changed (size/mtime moved), unchanged, or problematic. Incremental and
//! resumable indexing (spec §18, §25) fall out of this classification: an
//! interrupted scan reprocesses only new/changed files, never unchanged ones.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;
use tpt_app_media_asset_intelligence_core::MediaType;

/// A file observed during a filesystem scan, before any processing (spec §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedFile {
    /// Absolute path as observed on disk.
    pub path: PathBuf,
    /// File size in bytes at scan time.
    pub size_bytes: u64,
    /// File modification time at scan time.
    pub modified_time: SystemTime,
    /// Cheap extension-based classification, refined later by probing (spec §7).
    pub media_type: MediaType,
}

impl ScannedFile {
    /// Creates a scan record from file attributes and the extension classifier.
    ///
    /// # Errors
    ///
    /// Returns [`ScanError::UnreadableMetadata`] when the file's size or mtime
    /// cannot be read (vanished mid-scan, permission denied). Malformed *media*
    /// is not an error here — it is classified and handled by later stages
    /// (spec §17).
    pub fn from_path(path: PathBuf) -> Result<Self, ScanError> {
        let meta = std::fs::metadata(&path).map_err(|e| ScanError::UnreadableMetadata {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        let media_type = crate::classify_by_extension(&path);
        let modified_time = meta.modified().map_err(|e| ScanError::UnreadableMetadata {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        Ok(Self {
            path,
            size_bytes: meta.len(),
            modified_time,
            media_type,
        })
    }
}

/// Why a scanned file needs (or does not need) processing by this run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum ScanOutcome {
    /// Not in the index yet; process through all stages (spec §7).
    New,
    /// Size or mtime changed since the index recorded it; reprocess (spec §18).
    Changed {
        /// Size recorded by the previous index entry.
        previous_size_bytes: u64,
        /// Mtime recorded by the previous index entry.
        previous_modified: SystemTime,
    },
    /// Identical size and mtime to the indexed record; skip unchanged (spec §18, §25).
    Unchanged,
}

impl ScanOutcome {
    /// Returns `true` when the file must flow through the pipeline stages.
    pub const fn needs_processing(&self) -> bool {
        matches!(self, ScanOutcome::New | ScanOutcome::Changed { .. })
    }
}

/// Errors surfaced while collecting scan metadata.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The file existed during directory enumeration but its metadata is unreadable.
    #[error("unreadable metadata for {path}: {reason}")]
    UnreadableMetadata {
        /// The offending path.
        path: PathBuf,
        /// Underlying I/O error text.
        reason: String,
    },
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
}

#[cfg(test)]
mod tests {
    use super::*;
<<<<<<< HEAD
    use std::io::Write;

    #[test]
    fn empty_root_rejected() {
        assert!(validate_root(Path::new("")).is_err());
    }

    #[test]
    fn missing_root_reported_not_panicked() {
        let (found, errors) = scan_roots(&[PathBuf::from("__no_such_dir__")]);
        assert!(found.is_empty());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn scan_roundtrip_and_grouping() {
        let dir = std::env::temp_dir().join("mai-ingest-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.bin"), b"same-bytes").unwrap();
        fs::write(dir.join("b.bin"), b"same-bytes").unwrap();
        fs::write(dir.join("c.bin"), b"other-bytes").unwrap();
        let (found, errors) = scan_roots(std::slice::from_ref(&dir));
        assert!(errors.is_empty());
        assert_eq!(found.len(), 3);
        let groups = group_by_fingerprint(&found);
        assert_eq!(groups.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_unreadable_path_does_not_abort_scan() {
        let dir = std::env::temp_dir().join("mai-ingest-corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut f = fs::File::create(dir.join("ok.bin")).unwrap();
        f.write_all(b"ok").unwrap();
        let (found, _) = scan_roots(std::slice::from_ref(&dir.join("ok.bin")));
        // plus an explicit missing-root entry covered below
        let (found2, errors2) = scan_roots(&[PathBuf::from("__missing__")]);
        assert!(found.len() + found2.len() >= 1);
        assert_eq!(errors2.len(), 1);
        let _ = fs::remove_dir_all(&dir);
=======

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tpt-mai-ingest-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, b"scan test payload").unwrap();
        p
    }

    #[test]
    fn scans_a_readable_file() {
        let path = temp_file("readable.mov");
        let scanned = ScannedFile::from_path(path.clone()).unwrap();
        assert_eq!(scanned.path, path);
        assert_eq!(scanned.size_bytes, 17);
        assert_eq!(scanned.media_type, MediaType::Video);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_files_report_unreadable_metadata() {
        let err = ScannedFile::from_path(PathBuf::from("Z:/definitely/missing/file.mkv"))
            .expect_err("missing file must error");
        assert!(
            err.to_string().contains("unreadable metadata"),
            "got: {err}"
        );
    }

    #[test]
    fn outcomes_decide_processing() {
        assert!(ScanOutcome::New.needs_processing());
        assert!(ScanOutcome::Changed {
            previous_size_bytes: 1,
            previous_modified: SystemTime::UNIX_EPOCH,
        }
        .needs_processing());
        assert!(!ScanOutcome::Unchanged.needs_processing());
    }

    #[test]
    fn outcome_serializes_with_tag() {
        let json = serde_json::to_string(&ScanOutcome::Unchanged).unwrap();
        assert_eq!(json, "{\"outcome\":\"unchanged\"}");
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
    }
}
