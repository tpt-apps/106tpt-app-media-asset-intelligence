//! Strongly typed identifiers for the domain model (spec §6).
//!
//! Each identifier is a newtype over `u64` so the compiler prevents mixing, say,
//! an [`AssetId`] into an API expecting a [`TagId`]. All identifiers serialize as
//! plain integers and display in a stable `prefix-N` form for logs and CLI output.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($(#[$doc:meta])* $name:ident, $prefix:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(u64);

        impl $name {
            /// Creates an identifier from a raw integer.
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the raw integer value.
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "-{}"), self.0)
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value)
            }
        }
    };
}

define_id!(
    /// Identifies a named collection of indexed filesystem roots (spec §6.1).
    ArchiveId,
    "archive"
);
define_id!(
    /// Identifies a single indexed media file within an archive (spec §6.2).
    AssetId,
    "asset"
);
define_id!(
    /// Identifies a generated thumbnail/proxy/waveform (spec §6.3).
    DerivativeId,
    "derivative"
);
define_id!(
    /// Identifies a manual, local-model or cloud-model tag (spec §6.4).
    TagId,
    "tag"
);
define_id!(
    /// Identifies a reviewable duplicate group (spec §6.5).
    DuplicateGroupId,
    "dupgroup"
);
define_id!(
    /// Identifies a detected scene boundary range (spec §6.6).
    SceneId,
    "scene"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_display_with_stable_prefixes() {
        assert_eq!(ArchiveId::new(1).to_string(), "archive-1");
        assert_eq!(AssetId::new(42).to_string(), "asset-42");
        assert_eq!(DerivativeId::new(3).to_string(), "derivative-3");
        assert_eq!(TagId::new(9).to_string(), "tag-9");
        assert_eq!(DuplicateGroupId::new(14).to_string(), "dupgroup-14");
        assert_eq!(SceneId::new(7).to_string(), "scene-7");
    }

    #[test]
    fn ids_serialize_as_plain_integers() {
        assert_eq!(serde_json::to_string(&AssetId::new(5)).unwrap(), "5");
        assert_eq!(
            serde_json::from_str::<AssetId>("5").unwrap(),
            AssetId::new(5)
        );
    }

    #[test]
    fn ids_order_by_raw_value() {
        let mut ids = [AssetId::new(30), AssetId::new(10), AssetId::new(20)];
        ids.sort();
        assert_eq!(ids, [AssetId::new(10), AssetId::new(20), AssetId::new(30)]);
    }

    #[test]
    fn different_id_types_are_distinct_types() {
        // Compile-time separation; runtime sanity check that values do not coerce.
        let asset = AssetId::new(1);
        let tag = TagId::new(1);
        assert_ne!(asset.get(), tag.get() + 1);
        assert_eq!(asset.get(), tag.get());
    }
}
