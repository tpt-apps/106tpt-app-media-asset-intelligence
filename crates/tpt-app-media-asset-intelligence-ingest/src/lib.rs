//! Filesystem scanning, fingerprinting, resumable indexing, Pass 1 bootstrap (spec §7, §18).

pub mod resume;
pub mod scan;

pub use resume::{IndexedState, ResumeLog};
pub use scan::{scan_roots, ScannedFile};
