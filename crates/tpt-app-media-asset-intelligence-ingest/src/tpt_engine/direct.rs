//! Direct kinetix (video containers) + cadence (audio) probing for files
//! the av-asset cache routing doesn't cover yet. Returns the media info
//! plus the open-codec allowlist verdict.

use tpt_app_media_asset_intelligence_core::is_supported_codec;

/// Direct probe: MKV/WebM tracks via kinetix, audio via cadence readers,
/// MP4/MOV recognised-but-unsupported. `None` = unrecognised.
pub fn direct_probe(path: &std::path::Path) -> Option<(tpt_av_asset_utils::MediaInfo, bool)> {
    use tpt_av_asset_utils::{AssetId, MediaInfo};
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(mkv) = tpt_kinetix_demux::MkvDemuxer::new(bytes) {
            if let Some(track) = mkv.tracks().first() {
                let (media_type, codec) = codec_from_mkv(track);
                let supported = is_supported_codec(&codec);
                let id = AssetId::from_path(path).ok()?;
                let mut info = MediaInfo::new(id, path, media_type);
                match media_type {
                    tpt_av_asset_utils::MediaType::Video => {
                        info.video = Some(tpt_av_asset_utils::VideoInfo {
                            width: 0,
                            height: 0,
                            frame_rate: 0.0,
                            codec,
                            pixel_format: String::new(),
                            bit_rate: None,
                            frame_count: 0,
                            duration_secs: 0.0,
                        });
                    }
                    tpt_av_asset_utils::MediaType::Audio => {
                        info.audio = Some(tpt_av_asset_utils::AudioInfo {
                            sample_rate: 0,
                            channels: 0,
                            bit_depth: 0,
                            codec,
                            bit_rate: None,
                            duration_secs: 0.0,
                        });
                    }
                    tpt_av_asset_utils::MediaType::Image => {}
                }
                return Some((info, supported));
            }
        }
    }
    if let Some((codec, rate, channels, bits, frames)) = probe_audio_direct(path) {
        let supported = is_supported_codec(&codec);
        let id = AssetId::from_path(path).ok()?;
        let duration = frames.map(|f| f as f64 / rate.max(1) as f64);
        let mut info = MediaInfo::new(id, path, tpt_av_asset_utils::MediaType::Audio);
        info.duration_secs = duration;
        info.audio = Some(tpt_av_asset_utils::AudioInfo {
            sample_rate: rate,
            channels,
            bit_depth: bits,
            codec,
            bit_rate: None,
            duration_secs: duration.unwrap_or(0.0),
        });
        return Some((info, supported));
    }
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "mp4" | "mov" | "m4a"))
    {
        let id = AssetId::from_path(path).ok()?;
        let info = MediaInfo::new(id, path, tpt_av_asset_utils::MediaType::Video);
        return Some((info, false));
    }
    None
}

fn codec_from_mkv(
    track: &tpt_kinetix_demux::mkv::MkvTrack,
) -> (tpt_av_asset_utils::MediaType, String) {
    use tpt_av_asset_utils::MediaType as MT;
    match track.codec_id.as_str() {
        "V_AV1" => (MT::Video, "av1".into()),
        "V_VP9" => (MT::Video, "vp9".into()),
        "V_VP8" => (MT::Video, "vp8".into()),
        "V_THEORA" => (MT::Video, "theora".into()),
        "V_FFV1" => (MT::Video, "ffv1".into()),
        "A_OPUS" => (MT::Audio, "opus".into()),
        "A_VORBIS" => (MT::Audio, "vorbis".into()),
        "A_FLAC" => (MT::Audio, "flac".into()),
        "A_PCM" | "A_PCM/INT/LIT" => (MT::Audio, "pcm".into()),
        other => (MT::Video, other.to_ascii_lowercase()),
    }
}

fn probe_audio_direct(path: &std::path::Path) -> Option<(String, u32, u16, u16, Option<u64>)> {
    use tpt_av_cadence_core::{Decoder, FormatReader};
    let open = || {
        std::fs::File::open(path)
            .ok()
            .map(|f| Box::new(f) as Box<dyn std::io::Read + Send>)
    };
    if let Some(src) = open() {
        if let Ok(r) = tpt_av_cadence_wav::WavReader::open(src) {
            let i = r.info().clone();
            return Some((
                "wav".into(),
                i.sample_rate,
                i.channels,
                i.bit_depth,
                i.total_frames,
            ));
        }
    }
    if let Some(src) = open() {
        if let Ok(r) = tpt_av_cadence_flac::FlacReader::open(src) {
            let i = r.info().clone();
            return Some((
                "flac".into(),
                i.sample_rate,
                i.channels,
                i.bit_depth,
                i.total_frames,
            ));
        }
    }
    if let Some(src) = open() {
        if let Ok(r) = tpt_av_cadence_opus::OggOpusReader::open(src) {
            let i = r.info().clone();
            return Some((
                "opus".into(),
                i.sample_rate,
                i.channels,
                i.bit_depth,
                i.total_frames,
            ));
        }
    }
    if let Some(src) = open() {
        if let Ok(r) = tpt_av_cadence_vorbis::VorbisDecoder::open(src) {
            let i = r.info().clone();
            return Some((
                "vorbis".into(),
                i.sample_rate,
                i.channels,
                i.bit_depth,
                i.total_frames,
            ));
        }
    }
    None
}
