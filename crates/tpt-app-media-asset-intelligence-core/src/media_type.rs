//! Broad media classification for indexed assets (spec §6.2).
//!
//! The archive indexes video, audio and image material. [`MediaType`] is the
//! coarse classification derived from container/extension probing during ingest;
//! technical detail (codec, resolution, duration) lives in the model crate's
//! `TechnicalMetadata`, not here.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The broad kind of media an indexed file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MediaType {
    /// A video container with at least one video stream.
    Video,
    /// An audio-only file (music, interview, effect).
    Audio,
    /// A still image.
    Image,
    /// A file that carries none of the above (documents, project files, sidecars).
    Other,
}

impl MediaType {
    /// The kebab-case token used in search filters and JSON output (spec §14).
    pub const fn as_str(self) -> &'static str {
        match self {
            MediaType::Video => "video",
            MediaType::Audio => "audio",
            MediaType::Image => "image",
            MediaType::Other => "other",
        }
    }
}

impl std::str::FromStr for MediaType {
    type Err = ();

    /// Parses the kebab-case token back into a [`MediaType`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "video" => Ok(MediaType::Video),
            "audio" => Ok(MediaType::Audio),
            "image" => Ok(MediaType::Image),
            "other" => Ok(MediaType::Other),
            _ => Err(()),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_kebab_case_token() {
        for kind in [
            MediaType::Video,
            MediaType::Audio,
            MediaType::Image,
            MediaType::Other,
        ] {
            assert_eq!(kind.as_str().parse::<MediaType>(), Ok(kind), "{kind}");
        }
    }

    #[test]
    fn unknown_tokens_do_not_parse() {
        assert!("document".parse::<MediaType>().is_err());
        assert!("".parse::<MediaType>().is_err());
        assert!(
            "VIDEO".parse::<MediaType>().is_err(),
            "tokens are lowercase"
        );
    }

    #[test]
    fn serializes_as_kebab_case() {
        assert_eq!(
            serde_json::to_string(&MediaType::Video).unwrap(),
            "\"video\""
        );
        assert_eq!(
            serde_json::to_string(&MediaType::Other).unwrap(),
            "\"other\""
        );
    }
}
