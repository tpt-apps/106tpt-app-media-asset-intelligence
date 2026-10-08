//! Job-queue state persisted by the store (spec §13.7). The queue executor
//! itself lands alongside the job screen; this is the durable record of
//! active/queued/paused/cancelled jobs.

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Archive scanning + fingerprinting.
    Index,
    /// Thumbnail/proxy/waveform generation.
    Derivatives,
    /// Local-model (or gated cloud) tagging.
    Tagging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Active,
    Paused,
    Cancelled,
    Done,
    Failed,
}

/// A durable job record; `progress` is a 0.0..=1.0 fraction.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub archive_id: Option<Uuid>,
    pub kind: JobKind,
    pub status: JobStatus,
    pub progress: f64,
    pub error: Option<String>,
    /// Completion note (e.g. "indexed 42 asset(s)"), set on `Done`.
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// A fresh queued job.
    pub fn new(archive_id: Option<Uuid>, kind: JobKind) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            archive_id,
            kind,
            status: JobStatus::Queued,
            progress: 0.0,
            error: None,
            note: None,
            created_at: now,
            updated_at: now,
        }
    }
}
