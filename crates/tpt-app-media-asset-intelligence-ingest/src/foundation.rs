//! Mapping from `tpt-av-asset` foundation capabilities to product
//! responsibilities (spec §5.1).
//!
//! Media Asset Intelligence is, in large part, the commercial surface built on
//! top of `tpt-av-asset`'s infrastructure. This module re-exports the surface
//! this product consumes so the integration is enforced by the build, and so
//! there is one place documenting *what comes from where* rather than letting
//! the product duplicate foundation work (spec §5: "the commercial application
//! must add product-level indexing, search, tagging, and archive-health logic
//! on top of this foundation rather than duplicating decode or caching work").
//!
//! | Foundation capability (spec §5.1)          | Provided by                        |
//! | :---                                       | :---                               |
//! | Persistent asset metadata / embedded DB    | [`tpt_av_asset_db::AssetDb`]       |
//! | Thumbnail cache                            | [`tpt_av_asset_cache::ThumbnailCache`] |
//! | Waveform cache                             | [`tpt_av_asset_cache::WaveformReader`] |
//! | Proxy generation                           | [`tpt_av_asset_proxy::ProxyGenerator`] |
//! | Filesystem monitoring / watch folders      | [`tpt_av_asset_watcher::MediaWatcher`] |
//! | Background jobs, resumable processing      | [`tpt_av_asset_pipeline::ProcessingPipeline`] |
//! | Cache invalidation                         | [`tpt_av_asset_cache::invalidate_asset`] |
//!
//! Note: the foundation's `tpt_av_asset_utils::AssetId` and `MediaInfo` are the
//! *storage-layer* identifiers and probe results. The product's domain types
//! ( [`tpt_app_media_asset_intelligence_model::Asset`] ) live in the model crate;
//! Phase 1 (§26 steps 2–3) adds the mapping between the two layers.

/// Re-exports of the consumed foundation surface.
pub mod exports {
    pub use tpt_av_asset_cache::{
        invalidate_asset, CacheStorage, Thumbnail, ThumbnailCache, ThumbnailGenerator,
        WaveformReader,
    };
    pub use tpt_av_asset_db::{AssetDb, CacheType, JobRecord, JobState};
    pub use tpt_av_asset_pipeline::{
        probe_media_info, AssetImporter, AudioProxyJob, Job, JobId, JobQueue, ProcessingPipeline,
        ProgressTracker, Scheduler, ThumbnailJob, VideoProxyJob, WaveformJob,
    };
    pub use tpt_av_asset_proxy::{ProxyGenerator, ProxyProfile};
    pub use tpt_av_asset_utils::{
        AssetError, MediaInfo, ProgressEvent, ProgressReporter, TimeRange,
    };
    pub use tpt_av_asset_watcher::{FileEvent, FileEventType, MediaWatcher};
}

/// Compile-time proof that the foundation resolves and links (spec todo Phase 0:
/// "confirm `tpt-av-asset` is reachable and enumerate its capabilities").
#[cfg(test)]
mod reachability {
    use super::exports::*;

    #[test]
    fn foundation_types_are_constructible_or_nameable() {
        // Touch each re-exported item so the build fails if a foundation crate
        // drops or renames the surface this product depends on.
        let _queue: Option<JobQueue> = None;
        let _job_id: Option<JobId> = None;
        let _profile: Option<ProxyProfile> = None;
        let _db: Option<AssetDb> = None;

        // Enum trait values.
        let _ = JobState::Pending;
        let _ = CacheType::VideoProxy;
        let _ = FileEventType::Created;

        // Function items exist and are nameable.
        let _ = probe_media_info as fn(&std::path::Path) -> Result<MediaInfo, AssetError>;
    }
}
