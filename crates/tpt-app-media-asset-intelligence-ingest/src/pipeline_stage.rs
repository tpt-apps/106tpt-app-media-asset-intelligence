//! Pipeline stages and the two-pass execution model (spec §7, §7.1).
//!
//! Every unit of work in the indexing job queue belongs to exactly one stage
//! (spec §13.7) and exactly one pass. Pass 1 stages run before anything
//! expensive so basic search results surface within seconds of a scan starting
//! (spec §7.1, §18); Pass 2 stages progressively enrich the index in the
//! background without blocking the UI (spec §25).

use serde::{Deserialize, Serialize};
use std::fmt;

/// The two pipeline passes (spec §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PipelinePass {
    /// Fast, always runs: scan, fingerprint, technical metadata, metadata search index.
    Pass1,
    /// Background enrichment: derivatives, duplicates, scenes, tagging, index update.
    Pass2,
}

/// One stage of the ingestion pipeline, in pipeline order (spec §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PipelineStage {
    /// Filesystem/watch-folder scan (spec §7).
    Scan,
    /// Content fingerprinting (spec §7).
    Fingerprint,
    /// Technical metadata extraction (spec §7, deterministic per §3.2).
    TechnicalMetadata,
    /// Pass 1 search-index update: metadata/filename (spec §7.1, §10 Layer 1).
    SearchIndexMetadata,
    /// Thumbnail/proxy/waveform generation (spec §7).
    Derivatives,
    /// Duplicate and near-duplicate detection (spec §8).
    DuplicateDetection,
    /// Scene-change detection (spec §9).
    SceneDetection,
    /// Local-model tagging by default; optional cloud tagging is Phase 2 (spec §11, §20).
    Tagging,
    /// Pass 2 search-index update: full-text over tags/notes (spec §10 Layer 2).
    SearchIndexFullText,
}

impl PipelineStage {
    /// The pass this stage belongs to (spec §7.1).
    pub const fn pass(self) -> PipelinePass {
        match self {
            PipelineStage::Scan
            | PipelineStage::Fingerprint
            | PipelineStage::TechnicalMetadata
            | PipelineStage::SearchIndexMetadata => PipelinePass::Pass1,
            PipelineStage::Derivatives
            | PipelineStage::DuplicateDetection
            | PipelineStage::SceneDetection
            | PipelineStage::Tagging
            | PipelineStage::SearchIndexFullText => PipelinePass::Pass2,
        }
    }

    /// All stages in pipeline order (spec §7 diagram).
    pub const ALL: [PipelineStage; 9] = [
        PipelineStage::Scan,
        PipelineStage::Fingerprint,
        PipelineStage::TechnicalMetadata,
        PipelineStage::SearchIndexMetadata,
        PipelineStage::Derivatives,
        PipelineStage::DuplicateDetection,
        PipelineStage::SceneDetection,
        PipelineStage::Tagging,
        PipelineStage::SearchIndexFullText,
    ];
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PipelineStage::Scan => "scan",
            PipelineStage::Fingerprint => "fingerprint",
            PipelineStage::TechnicalMetadata => "technical-metadata",
            PipelineStage::SearchIndexMetadata => "search-index-metadata",
            PipelineStage::Derivatives => "derivatives",
            PipelineStage::DuplicateDetection => "duplicate-detection",
            PipelineStage::SceneDetection => "scene-detection",
            PipelineStage::Tagging => "tagging",
            PipelineStage::SearchIndexFullText => "search-index-full-text",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass1_stages_come_before_pass2_stages() {
        let max_pass1 = PipelineStage::ALL
            .iter()
            .filter(|s| s.pass() == PipelinePass::Pass1)
            .map(|s| PipelineStage::ALL.iter().position(|x| x == s).unwrap())
            .max()
            .unwrap();
        let min_pass2 = PipelineStage::ALL
            .iter()
            .filter(|s| s.pass() == PipelinePass::Pass2)
            .map(|s| PipelineStage::ALL.iter().position(|x| x == s).unwrap())
            .min()
            .unwrap();
        assert!(
            max_pass1 < min_pass2,
            "every Pass 1 stage must precede every Pass 2 stage (spec §7.1)"
        );
    }

    #[test]
    fn display_and_serde_use_the_same_tokens() {
        for stage in PipelineStage::ALL {
            let token = stage.to_string();
            assert_eq!(
                serde_json::to_string(&stage).unwrap(),
                format!("\"{token}\"")
            );
            let round: PipelineStage = serde_json::from_str(&format!("\"{token}\"")).unwrap();
            assert_eq!(round, stage);
        }
    }
}
