//! Wired engine (`--features tpt`): real probing/import through
//! `tpt-av-asset` + kinetix/cadence decode, with the open-codec allowlist
//! enforced on top (spec §7, Codec Scope).
//!
//! Engine home layout (separate from the source archive, spec §16):
//! ```text
//! {home}/
//! ├── assets.redb        # tpt-av-asset embedded database
//! └── derivatives/       # tpt-av-asset CacheStorage root
//!     ├── waveforms/
//!     ├── thumbnails/
//!     └── proxies/
//! ```

pub mod direct;
pub mod engine;

pub use engine::{DerivativePaths, Engine};
