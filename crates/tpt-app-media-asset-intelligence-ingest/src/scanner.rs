//! Recursive filesystem scanning with failure isolation (spec §7, §17, §18).
//!
//! The scanner walks archive roots deterministically (sorted per directory),
//! classifies every file against the previous index state (new / changed /
//! unchanged — the basis of incremental, resumable indexing, spec §18), and
//! isolates failures: one unreadable entry becomes a recorded [`ScanIssue`],
//! never a crashed run (spec §17, §26 step 24).
//!
//! Directory symlinks and junctions are **not** followed — NAS trees routinely
//! contain cycles, and following them risks unbounded walks. File symlinks are
//! followed (`std::fs::metadata` semantics).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::scan::{ScanOutcome, ScannedFile};

/// Validates an archive root before scanning (spec §17: strict path validation,
/// especially for watch-folder and NAS-mounted roots).
///
/// # Errors
///
/// Names the violated rule: relative path, missing path, or not a directory.
pub fn validate_root(root: &Path) -> Result<(), ScanConfigError> {
    if !root.is_absolute() {
        return Err(ScanConfigError::NotAbsolute(root.to_path_buf()));
    }
    let meta = std::fs::metadata(root).map_err(|e| ScanConfigError::Unreadable {
        path: root.to_path_buf(),
        reason: e.to_string(),
    })?;
    if !meta.is_dir() {
        return Err(ScanConfigError::NotADirectory(root.to_path_buf()));
    }
    Ok(())
}

/// Why an archive root cannot be scanned (spec §14 CONFIGURATION_ERROR input).
#[derive(Debug, thiserror::Error)]
pub enum ScanConfigError {
    /// Roots must be absolute (spec §17 path validation).
    #[error("archive root must be an absolute path: {0}")]
    NotAbsolute(PathBuf),
    /// The root does not exist or its metadata is unreadable.
    #[error("archive root is unreadable: {path}: {reason}")]
    Unreadable {
        /// The offending root.
        path: PathBuf,
        /// Underlying I/O error text.
        reason: String,
    },
    /// The root exists but is not a directory.
    #[error("archive root is not a directory: {0}")]
    NotADirectory(PathBuf),
}

/// A non-fatal problem recorded during a scan (spec §17: safe handling;
/// one bad entry never aborts the run).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanIssue {
    /// The path that could not be processed.
    pub path: PathBuf,
    /// What went wrong, for the archive-health details view (spec §12).
    pub reason: String,
}

/// The result of scanning a set of archive roots.
#[derive(Debug, Default)]
pub struct ScanRun {
    /// Every file found, in deterministic (per-directory sorted) order.
    pub files: Vec<ScannedEntry>,
    /// Non-fatal problems encountered; the run continued past each one.
    pub issues: Vec<ScanIssue>,
}

impl ScanRun {
    /// Entries whose files must flow through the pipeline this run (spec §18).
    pub fn needing_processing(&self) -> impl Iterator<Item = &ScannedEntry> {
        self.files.iter().filter(|e| e.outcome.needs_processing())
    }

    /// Number of files that must be processed this run.
    pub fn processing_count(&self) -> usize {
        self.needing_processing().count()
    }
}

/// A scanned file plus its incremental decision against the previous index
/// state (spec §18, §25: an interrupted scan must not reprocess unchanged
/// assets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEntry {
    /// The scanned file.
    pub file: ScannedFile,
    /// Whether this run must process it.
    pub outcome: ScanOutcome,
}

/// Walks the archive roots and classifies every file against the previous
/// index state.
///
/// `previous_record` returns the indexed `(size, mtime)` for a path, or `None`
/// when the path is not indexed. Roots are validated up front: an invalid root
/// is a configuration error (spec §14 CONFIGURATION_ERROR), while problems
/// *within* a walk are isolated into [`ScanRun::issues`].
///
/// # Errors
///
/// Returns [`ScanConfigError`] if any root fails [`validate_root`].
pub fn scan_roots(
    roots: &[PathBuf],
    previous_record: impl Fn(&Path) -> Option<(u64, SystemTime)>,
) -> Result<ScanRun, ScanConfigError> {
    for root in roots {
        validate_root(root)?;
    }

    let mut run = ScanRun::default();
    for root in roots {
        walk_root(root, &previous_record, &mut run);
    }
    // Deterministic order regardless of filesystem enumeration order: sort by
    // path text. (Per-directory reads are also sorted; this catches the
    // cross-directory level.)
    run.files.sort_by(|a, b| a.file.path.cmp(&b.file.path));
    Ok(run)
}

fn walk_root(
    root: &Path,
    previous_record: &impl Fn(&Path) -> Option<(u64, SystemTime)>,
    run: &mut ScanRun,
) {
    // Explicit stack instead of recursion: deep project trees must not risk
    // stack exhaustion (spec §26 step 24 hardening).
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                run.issues.push(ScanIssue {
                    path: dir,
                    reason: e.to_string(),
                });
                continue;
            }
        };

        // Sort entries within a directory for deterministic walk order.
        let mut named: BTreeMap<std::ffi::OsString, std::fs::DirEntry> = BTreeMap::new();
        for entry in entries {
            match entry {
                Ok(entry) => {
                    named.insert(entry.file_name(), entry);
                }
                Err(e) => run.issues.push(ScanIssue {
                    path: dir.clone(),
                    reason: e.to_string(),
                }),
            }
        }

        for (_, entry) in named {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                run.issues.push(ScanIssue {
                    path,
                    reason: "unreadable entry type".to_string(),
                });
                continue;
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if file_type.is_symlink() {
                // Directory symlinks/junctions are not followed (cycle risk on
                // NAS trees); symlinked files fall through to the file branch.
                if std::fs::metadata(&path)
                    .map(|m| m.is_dir())
                    .unwrap_or(false)
                {
                    continue;
                }
            }

            match ScannedFile::from_path(path.clone()) {
                Ok(file) => {
                    let outcome = match previous_record(&path) {
                        None => ScanOutcome::New,
                        Some((size, mtime))
                            if size == file.size_bytes && mtime == file.modified_time =>
                        {
                            ScanOutcome::Unchanged
                        }
                        Some((previous_size_bytes, previous_modified)) => ScanOutcome::Changed {
                            previous_size_bytes,
                            previous_modified,
                        },
                    };
                    run.files.push(ScannedEntry { file, outcome });
                }
                Err(e) => run.issues.push(ScanIssue {
                    path,
                    reason: e.to_string(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_archive(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join("tpt-mai-scanner-tests");
        let dir = base.join(format!("{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn no_previous(_: &Path) -> Option<(u64, SystemTime)> {
        None
    }

    #[test]
    fn rejects_relative_missing_and_file_roots() {
        assert!(matches!(
            validate_root(Path::new("relative/path")),
            Err(ScanConfigError::NotAbsolute(_))
        ));

        let missing = temp_archive("missing-root").join("nope");
        assert!(matches!(
            validate_root(&missing),
            Err(ScanConfigError::Unreadable { .. })
        ));

        let file_root = temp_archive("file-root");
        let file = file_root.join("afile.txt");
        fs::write(&file, b"x").unwrap();
        assert!(matches!(
            validate_root(&file),
            Err(ScanConfigError::NotADirectory(_))
        ));
    }

    #[test]
    fn walks_recursively_and_sorts_deterministically() {
        let root = temp_archive("recursive");
        fs::create_dir_all(root.join("b_proj")).unwrap();
        fs::create_dir_all(root.join("a_proj/sub")).unwrap();
        fs::write(root.join("b_proj/2.wav"), b"bb").unwrap();
        fs::write(root.join("a_proj/1.wav"), b"aa").unwrap();
        fs::write(root.join("a_proj/sub/3.wav"), b"cc").unwrap();

        let run = scan_roots(std::slice::from_ref(&root), no_previous).unwrap();
        assert_eq!(run.issues.len(), 0);
        let paths: Vec<String> = run
            .files
            .iter()
            .map(|e| e.file.path.to_string_lossy().replace('\\', "/"))
            .collect();
        let root_text = root.to_string_lossy().replace('\\', "/");
        assert_eq!(
            paths,
            vec![
                format!("{root_text}/a_proj/1.wav"),
                format!("{root_text}/a_proj/sub/3.wav"),
                format!("{root_text}/b_proj/2.wav"),
            ],
            "sorted by path text regardless of enumeration order"
        );
        assert_eq!(run.processing_count(), 3);
    }

    #[test]
    fn classifies_new_changed_unchanged_against_previous_state() {
        let root = temp_archive("incremental");
        fs::create_dir_all(&root).unwrap();
        let a = root.join("a.wav");
        let b = root.join("b.wav");
        fs::write(&a, b"aaa").unwrap();
        fs::write(&b, b"bbb").unwrap();

        // First run: everything new.
        let first = scan_roots(std::slice::from_ref(&root), no_previous).unwrap();
        assert_eq!(first.files.len(), 2);
        assert_eq!(first.processing_count(), 2);

        // Snapshot the indexed state as the scanner would record it.
        let mut indexed: BTreeMap<PathBuf, (u64, SystemTime)> = BTreeMap::new();
        for entry in &first.files {
            indexed.insert(
                entry.file.path.clone(),
                (entry.file.size_bytes, entry.file.modified_time),
            );
        }
        let previous = |p: &Path| indexed.get(p).copied();

        // Second run, no changes: everything unchanged, nothing to process.
        let second = scan_roots(std::slice::from_ref(&root), previous).unwrap();
        assert_eq!(second.processing_count(), 0);
        assert!(second
            .files
            .iter()
            .all(|e| e.outcome == ScanOutcome::Unchanged));

        // Modify one file: exactly that one is Changed, the other stays Unchanged.
        fs::write(&a, b"aaax").unwrap();
        let third = scan_roots(std::slice::from_ref(&root), previous).unwrap();
        assert_eq!(third.processing_count(), 1);
        let changed = third
            .files
            .iter()
            .find(|e| e.file.path == a)
            .expect("modified file present");
        assert!(matches!(changed.outcome, ScanOutcome::Changed { .. }));
    }

    #[test]
    fn skips_directory_symlinks_without_dying_on_them() {
        let root = temp_archive("symlinks");
        let real = root.join("real");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("clip.wav"), b"x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, root.join("loop")).unwrap();
        }
        #[cfg(windows)]
        {
            // Junctions are the Windows equivalent; ignore if unprivileged.
            let _ = std::os::windows::fs::symlink_dir(&real, root.join("loop"));
        }
        let run = scan_roots(std::slice::from_ref(&root), no_previous).unwrap();
        let wav_count = run
            .files
            .iter()
            .filter(|e| e.file.path.extension().is_some_and(|e| e == "wav"))
            .count();
        assert_eq!(wav_count, 1, "the linked directory must be visited once");
    }

    #[test]
    fn empty_root_yields_empty_run() {
        let root = temp_archive("empty");
        let run = scan_roots(&[root], no_previous).unwrap();
        assert_eq!(run.files.len(), 0);
        assert_eq!(run.issues.len(), 0);
    }
}
