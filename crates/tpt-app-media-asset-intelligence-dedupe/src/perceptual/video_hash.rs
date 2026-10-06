//! Video near-duplicates: 64-bit dHash over 9×8-downscaled luma of
//! `VideoSource` RGBA frames; Hamming distance ≤ threshold counts as a
//! near-duplicate. Deterministic, fully offline.

use std::path::Path;

/// 64-bit perceptual hash of one RGBA frame (dHash on luma).
pub fn dhash_rgba(data: &[u8], width: u32, height: u32) -> u64 {
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    let mut small = [0u8; 72];
    for y in 0..8 {
        for x in 0..9 {
            let sx = (x * w / 9).min(w - 1);
            let sy = (y * h / 8).min(h - 1);
            let idx = (sy * w + sx) * 4;
            let (r, g, b) = (
                data.get(idx).copied().unwrap_or(0) as u32,
                data.get(idx + 1).copied().unwrap_or(0) as u32,
                data.get(idx + 2).copied().unwrap_or(0) as u32,
            );
            small[y * 9 + x] = ((r * 30 + g * 59 + b * 11) / 100) as u8;
        }
    }
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            if small[y * 9 + x] > small[y * 9 + x + 1] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    hash
}

/// Hamming distance between two dHashes.
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Hash the middle frame of `path` via `VideoSource`. `None` when the
/// file has no decodable video.
pub fn hash_video_file(path: &Path) -> Option<u64> {
    let mut src = tpt_av_asset_cache::video::open_video(path).ok()?;
    let info = src.info().clone();
    src.seek_to(info.duration_secs / 2.0);
    let frame = src.next_frame().ok()??;
    Some(dhash_rgba(&frame.data, frame.width, frame.height))
}

/// Group paths whose video dHashes are within `threshold` Hamming distance.
pub fn perceptual_groups(paths: &[&Path], threshold: u32) -> Vec<Vec<std::path::PathBuf>> {
    let hashes: Vec<(&Path, Option<u64>)> =
        paths.iter().map(|p| (*p, hash_video_file(p))).collect();
    let mut groups: Vec<Vec<std::path::PathBuf>> = Vec::new();
    let mut used = vec![false; hashes.len()];
    for (i, (pa, ha)) in hashes.iter().enumerate() {
        if used[i] || ha.is_none() {
            continue;
        }
        let mut group = vec![(*pa).to_path_buf()];
        used[i] = true;
        for (j, (pb, hb)) in hashes.iter().enumerate().skip(i + 1) {
            if used[j] {
                continue;
            }
            if let (Some(a), Some(b)) = (ha, hb) {
                if hamming(*a, *b) <= threshold {
                    group.push((*pb).to_path_buf());
                    used[j] = true;
                }
            }
        }
        if group.len() > 1 {
            groups.push(group);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_frames_hash_equal() {
        let data = vec![128u8; 64 * 48 * 4];
        assert_eq!(dhash_rgba(&data, 64, 48), dhash_rgba(&data, 64, 48));
    }

    #[test]
    fn solid_frames_hash_zero() {
        let data = vec![200u8; 32 * 24 * 4];
        assert_eq!(dhash_rgba(&data, 32, 24), 0);
    }
}
