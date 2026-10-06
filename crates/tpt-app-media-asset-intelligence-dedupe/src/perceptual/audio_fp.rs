//! Audio-fingerprint matching: zero-crossing + RMS fingerprint over
//! cadence PCM windows; Euclidean distance ≤ threshold counts as a match.

use std::path::Path;

/// Compact audio fingerprint over the first `windows` PCM windows.
pub fn fingerprint_audio(pcm: &[f32], channels: usize, windows: usize) -> Vec<f32> {
    let ch = channels.max(1);
    let win = 1024 * ch;
    let mut out = Vec::new();
    for w in 0..windows {
        let start = w * win;
        if start + win > pcm.len() {
            break;
        }
        let s = &pcm[start..start + win];
        let mut crossings = 0u32;
        let mut energy = 0f64;
        let mono: Vec<f32> = s
            .chunks_exact(ch)
            .map(|f| f.iter().sum::<f32>() / ch as f32)
            .collect();
        for pair in mono.windows(2) {
            if (pair[0] < 0.0) != (pair[1] < 0.0) {
                crossings += 1;
            }
            energy += (pair[1] as f64) * (pair[1] as f64);
        }
        out.push(crossings as f32 / win as f32);
        out.push((energy / win as f64).sqrt() as f32);
    }
    out
}

/// Euclidean distance between fingerprints.
pub fn fingerprint_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

/// Fingerprint an audio file via cadence readers. `None` when undecodable.
pub fn fingerprint_audio_file(path: &Path) -> Option<Vec<f32>> {
    let mut reader = tpt_av_asset_cache::audio::open_audio(path).ok()?;
    let channels = reader.info().channels as usize;
    let mut pcm = vec![0.0f32; 4096 * channels.max(1)];
    let mut all = Vec::new();
    loop {
        match reader.decode(&mut pcm) {
            Ok(0) => break,
            Ok(n) => all.extend_from_slice(&pcm[..n * channels]),
            Err(_) => break,
        }
    }
    if all.is_empty() {
        return None;
    }
    Some(fingerprint_audio(&all, channels, 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic() {
        let pcm: Vec<f32> = (0..8192).map(|i| (i as f32 * 0.01).sin()).collect();
        assert_eq!(fingerprint_audio(&pcm, 1, 4), fingerprint_audio(&pcm, 1, 4));
    }

    #[test]
    fn distance_zero_for_identical() {
        let pcm = vec![0.1f32; 8192];
        let f = fingerprint_audio(&pcm, 1, 4);
        assert_eq!(fingerprint_distance(&f, &f), 0.0);
    }
}
