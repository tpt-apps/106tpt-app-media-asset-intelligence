//! Integration: scan -> dedupe -> search pipeline over synthetic fixtures.
use std::collections::HashMap;
use tpt_app_media_asset_intelligence_dedupe::exact_duplicates;
use tpt_app_media_asset_intelligence_ingest::scan_roots;

#[test]
fn pipeline_scans_and_groups() {
    let dir = std::env::temp_dir().join("mai-pipeline-it");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.bin"), b"dup").unwrap();
    std::fs::write(dir.join("b.bin"), b"dup").unwrap();
    let (found, errors) = scan_roots(std::slice::from_ref(&dir));
    assert!(errors.is_empty());
    let fps: HashMap<_, _> = found
        .iter()
        .enumerate()
        .map(|(i, f)| (uuid::Uuid::from_u128(i as u128), f.fingerprint.clone()))
        .collect();
    assert_eq!(exact_duplicates(&fps).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
