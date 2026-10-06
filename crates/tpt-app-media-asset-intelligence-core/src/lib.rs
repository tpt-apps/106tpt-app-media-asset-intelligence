//! Shared primitives for TPT Media Asset Intelligence (spec §6).
//!
//! This crate holds the small, dependency-light types every other crate needs:
//! strongly typed identifiers for the domain model, the [`MediaType`]
//! classification, the content [`AssetFingerprint`] and the engine version string
//! stamped into derivatives and index records (spec §6.3, §3.2).
//!
//! Like the rest of the engine boundary, this crate performs no I/O, no
//! networking and no clock reads: the same inputs produce the same values on
//! every machine (spec §3.2).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod fingerprint;
pub mod id;
pub mod media_type;
pub mod version;

pub use error::{CoreError, Result};
pub use fingerprint::AssetFingerprint;
pub use id::{ArchiveId, AssetId, DerivativeId, DuplicateGroupId, SceneId, TagId};
pub use media_type::MediaType;
pub use version::ENGINE_VERSION;
