//! End-to-end Pass 1 engine test (spec §7.1, §26 steps 4–6).
//!
//! Builds a synthetic archive on disk with the foundation's test-media
//! helpers, then drives the real pipeline: scan → fingerprint → probe → index
//! → exact-duplicate detection → Pass 1 search — and finally proves the
//! incremental guarantee (§18, §25): an unchanged rescan reprocesses nothing.

use std::path::PathBuf;

use tempfile::TempDir;
use tpt_app_media_asset_intelligence_core::{ArchiveId, AssetFingerprint, MediaType};
use tpt_app_media_asset_intelligence_dedupe::{DedupeReport, WastedSummary};
use tpt_app_media_asset_intelligence_ingest::{probe, scan_roots, ArchiveIndex, ProbeError};
use tpt_app_media_asset_intelligence_model::{DuplicateGroup, MatchKind};
use tpt_app_media_asset_intelligence_search::{evaluate, parse_query, SearchDoc};

/// Builds the synthetic archive:
///
/// ```text
/// <root>/project_a/interview_final.wav       (1 s, 48 kHz stereo)
/// <root>/project_a/interview_final_copy.wav  (byte-identical copy)
/// <root>/project_b/room_tone.wav             (0.5 s, 44.1 kHz mono)
/// <root>/project_b/render.proxy              (80-frame lossless video)
/// <root>/notes.txt                           (no decoder: unsupported)
/// ```
fn build_archive(root: &std::path::Path) -> PathBuf {
    tpt_av_asset_test_media::write_test_wav(
        &root.join("project_a/interview_final.wav"),
        1.0,
        48_000,
        2,
    )
    .unwrap();
    std::fs::copy(
        root.join("project_a/interview_final.wav"),
        root.join("project_a/interview_final_copy.wav"),
    )
    .unwrap();
    tpt_av_asset_test_media::write_test_wav(&root.join("project_b/room_tone.wav"), 0.5, 44_100, 1)
        .unwrap();
    tpt_av_asset_test_media::write_proxy_video(
        &root.join("project_b/render.proxy"),
        320,
        240,
        25.0,
        80,
        tpt_av_asset_test_media::gradient_painter,
    )
    .unwrap();
    std::fs::write(root.join("notes.txt"), b"not media at all").unwrap();
    root.to_path_buf()
}

/// Runs Pass 1 over the archive: scan, then index every new/changed file.
/// Returns the probe failures — the corrupt/unreadable inputs archive health
/// will report (spec §12, §17).
fn run_pass1(index: &mut ArchiveIndex, root: &std::path::Path) -> Vec<(PathBuf, ProbeError)> {
    let run = scan_roots(&[root.to_path_buf()], |p| index.previous_record(p)).unwrap();
    let mut failures = Vec::new();
    for entry in run.needing_processing() {
        match probe(&entry.file.path) {
            Ok(probed) => {
                index.index_scanned(&entry.file, &probed).unwrap();
            }
            Err(e) => failures.push((entry.file.path.clone(), e)),
        }
    }
    failures
}

#[test]
fn pass1_scans_probes_indexes_and_searches_a_synthetic_archive() {
    let tmp = TempDir::new().unwrap();
    let root = build_archive(tmp.path());

    let mut index = ArchiveIndex::new(ArchiveId::new(1));
    let failures = run_pass1(&mut index, &root);

    // The text file is unsupported, not corrupt: indexing continues (§17).
    assert_eq!(failures.len(), 1, "exactly notes.txt must be unsupported");
    assert!(failures[0].1.is_unsupported_format());
    assert!(failures[0].0.ends_with("notes.txt"));
    assert_eq!(index.len(), 4, "three wavs + one proxy video indexed");

    // Technical metadata came from the real decoders (§3.2, §26 step 5).
    let interview = index
        .get_by_path(&root.join("project_a/interview_final.wav"))
        .unwrap();
    assert_eq!(interview.media_type, MediaType::Audio);
    assert_eq!(
        interview.technical_metadata.container.as_deref(),
        Some("wav")
    );
    assert!(
        interview
            .technical_metadata
            .audio_codec
            .as_deref()
            .is_some_and(|c| c.eq_ignore_ascii_case("wav")),
        "the foundation reports the wav codec under its container-style name, got {:?}",
        interview.technical_metadata.audio_codec
    );

    let video = index
        .get_by_path(&root.join("project_b/render.proxy"))
        .unwrap();
    assert_eq!(video.technical_metadata.width, Some(320));
    assert_eq!(video.technical_metadata.height, Some(240));
    // 80 frames at 25 fps = 3200 ms.
    assert_eq!(video.technical_metadata.duration_ms, Some(3_200));

    // Exact-hash duplicate detection (§8 layer 1): the byte-identical copy.
    let members = index.assets_with_fingerprint(&interview.fingerprint);
    assert_eq!(members.len(), 2, "master + copy share the fingerprint");
    let group = DuplicateGroup::new(1, members, MatchKind::ExactHash, 1.0).unwrap();
    let sizes: std::collections::BTreeMap<u64, u64> =
        index.assets().map(|a| (a.id.get(), a.size_bytes)).collect();
    let report = DedupeReport::build("synthetic", vec![group], |id| sizes.get(&id.get()).copied());
    assert_eq!(
        report.wasted,
        WastedSummary {
            groups: 1,
            assets_in_groups: 2,
            // Both members are byte-identical, so one copy's bytes are waste.
            reclaimable_bytes: interview.size_bytes,
        }
    );

    // Pass 1 search (§26 step 6): filename, path and technical fields.
    let docs: Vec<SearchDoc> = index.assets().map(SearchDoc::from_asset).collect();

    let hits = evaluate(&parse_query("interview_final").unwrap(), &docs);
    assert_eq!(hits.len(), 2, "master + copy by filename substring");
    assert!(hits.iter().all(|h| h
        .explanations
        .iter()
        .any(|e| e.contains("filename") || e.contains("path"))));

    let hits = evaluate(&parse_query("ext:wav AND path:project_b").unwrap(), &docs);
    assert_eq!(hits.len(), 1);
    assert!(hits[0]
        .explanations
        .iter()
        .any(|e| e.contains("ext") && e.contains("wav")));

    let hits = evaluate(&parse_query("resolution:320x240").unwrap(), &docs);
    assert_eq!(hits.len(), 1, "the proxy video by decoded resolution");

    // Fingerprint hex survives the report boundary (CLI output form, §14).
    assert_eq!(
        AssetFingerprint::from_hex(&interview.fingerprint.to_hex()).as_ref(),
        Some(&interview.fingerprint)
    );
}

#[test]
fn unchanged_rescans_reprocess_nothing_and_changed_files_alone_are_picked_up() {
    // §18, §25: an interrupted scan (a fresh run against the same archive)
    // must not reprocess unchanged, already-indexed assets — and must pick up
    // exactly the files that changed.
    let tmp = TempDir::new().unwrap();
    let root = build_archive(tmp.path());

    let mut index = ArchiveIndex::new(ArchiveId::new(1));
    let failures = run_pass1(&mut index, &root);
    assert_eq!(failures.len(), 1);
    assert_eq!(index.len(), 4);

    // No changes: the rescan (comparing against the index, as the persisted
    // store of §26 step 14 will) finds nothing new to process — except
    // notes.txt, which was never indexed (no decoder): unsupported files stay
    // pending until archive health records them (§12).
    let run = scan_roots(std::slice::from_ref(&root), |p| index.previous_record(p)).unwrap();
    assert_eq!(run.issues.len(), 0);
    let pending: Vec<&tpt_app_media_asset_intelligence_ingest::ScannedEntry> =
        run.needing_processing().collect();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].file.path.ends_with("notes.txt"));

    // Modify exactly one file: append a byte (size and mtime both move).
    let interview = root.join("project_a/interview_final.wav");
    let mut bytes = std::fs::read(&interview).unwrap();
    bytes.push(0);
    std::fs::write(&interview, &bytes).unwrap();

    let run = scan_roots(std::slice::from_ref(&root), |p| index.previous_record(p)).unwrap();
    let changed: Vec<&tpt_app_media_asset_intelligence_ingest::ScannedEntry> =
        run.needing_processing().collect();
    assert_eq!(changed.len(), 2, "the changed wav + the pending notes.txt");
    let wav = changed
        .iter()
        .find(|e| e.file.path.ends_with("interview_final.wav"))
        .expect("the changed wav is picked up");
    assert!(matches!(
        wav.outcome,
        tpt_app_media_asset_intelligence_ingest::ScanOutcome::Changed { .. }
    ));
    // Every *indexed* asset stays untouched: no other media file reprocessed.
    assert!(changed.iter().all(
        |e| e.file.path.ends_with("notes.txt") || e.file.path.ends_with("interview_final.wav")
    ));
}
