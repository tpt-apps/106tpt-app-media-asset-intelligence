//! Filesystem scanning, fingerprinting, resumable indexing, Pass 1 bootstrap (spec §7, §18).

pub mod probe;
pub mod resume;
pub mod scan;
#[cfg(feature = "tpt")]
pub mod tpt_engine;
#[cfg(feature = "tpt")]
pub mod watch;

pub use probe::{probe_file, ProbedFile};
pub use resume::{IndexedState, ResumeLog};
pub use scan::{scan_roots, ScannedFile};
