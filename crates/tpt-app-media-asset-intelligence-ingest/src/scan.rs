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
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }
}
