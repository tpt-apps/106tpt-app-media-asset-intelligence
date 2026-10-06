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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
