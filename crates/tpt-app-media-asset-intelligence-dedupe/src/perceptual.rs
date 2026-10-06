//! Perceptual near-duplicate + audio-fingerprint matching (`--features tpt`).

pub mod audio_fp;
pub mod video_hash;

pub use audio_fp::{fingerprint_audio, fingerprint_audio_file, fingerprint_distance};
pub use video_hash::{dhash_rgba, hamming, hash_video_file, perceptual_groups};
