//! Fuzzing (`--features tpt`): parser never-panics proofs reusing
//! `tpt-av-test-fuzz` (§19.5). Deterministic proptest cases (no libFuzzer
//! needed); regression corpus runs alongside (§19.5, §19.6).
//!
//! Covered surfaces: media container/metadata parsers (MKV demux + cadence
//! audio readers), the extension probe, the search-query parser, and CLI
//! argument parsing (clap) — §19.5, §26 step 23.

use clap::Parser as _;
use proptest::prelude::*;
use std::io::{Cursor, Read};
use std::path::Path;
use tpt_app_media_asset_intelligence_cli::Cli;
use tpt_app_media_asset_intelligence_core::is_supported_codec;
use tpt_app_media_asset_intelligence_ingest::probe_file;
use tpt_app_media_asset_intelligence_search::parse_query;
use tpt_av_cadence_core::{Decoder, FormatReader};
use tpt_av_test_fuzz::fuzz_parser_never_panics;
use tpt_av_test_fuzz::seed::{determinism_config, Seed};

proptest! {
    #![proptest_config(determinism_config(Seed::default()))]

    #[test]
    fn search_query_never_panics(s in "\\PC{0,64}") {
        fuzz_parser_never_panics!(parser: parse_query, input: &s);
    }

    #[test]
    fn codec_allowlist_never_panics(s in "\\PC{0,32}") {
        fuzz_parser_never_panics!(parser: is_supported_codec, input: &s);
    }

    #[test]
    fn probe_file_never_panics(name in "\\PC{0,128}") {
        fuzz_parser_never_panics!(parser: probe_named, input: &name);
    }

    #[test]
    fn cli_args_never_panic(args in proptest::collection::vec("\\PC{0,24}", 1..12)) {
        fuzz_parser_never_panics!(parser: parse_cli, input: args);
    }

    #[test]
    fn mkv_demux_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        fuzz_parser_never_panics!(parser: parse_mkv, input: &bytes);
    }

    #[test]
    fn wav_reader_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        fuzz_parser_never_panics!(parser: parse_wav, input: &bytes);
    }

    #[test]
    fn flac_reader_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        fuzz_parser_never_panics!(parser: parse_flac, input: &bytes);
    }

    #[test]
    fn ogg_opus_reader_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        fuzz_parser_never_panics!(parser: parse_ogg_opus, input: &bytes);
    }

    #[test]
    fn vorbis_reader_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        fuzz_parser_never_panics!(parser: parse_vorbis, input: &bytes);
    }
}

/// Regression corpus of known-bad inputs must never panic a media parser
/// (panics surface from `tpt_av_test_fuzz::corpus::run` as a hard failure).
#[test]
fn regression_corpus_never_panics() {
    let parsers: [MediaParser; 5] = [
        parse_mkv,
        parse_wav,
        parse_flac,
        parse_ogg_opus,
        parse_vorbis,
    ];
    for parser in parsers {
        let parsed = move |bytes: &[u8]| -> Result<(), String> {
            let _ = parser(bytes);
            Ok(())
        };
        tpt_av_test_fuzz::corpus::run(&parsed)
            .expect("tpt-av-test regression corpus must never panic a media parser");
    }
}

fn probe_named(name: &str) -> Result<(), ()> {
    let _ = probe_file(Path::new(name));
    Ok(())
}

type MediaParser = fn(&[u8]) -> Result<(), ()>;

fn parse_cli(args: Vec<String>) -> Result<(), ()> {
    let _ = Cli::try_parse_from(args);
    Ok(())
}

fn parse_mkv(bytes: &[u8]) -> Result<(), ()> {
    let _ = tpt_kinetix_demux::MkvDemuxer::new(bytes.to_vec());
    Ok(())
}

fn reader(bytes: &[u8]) -> Box<dyn Read + Send> {
    Box::new(Cursor::new(bytes.to_vec()))
}

fn parse_wav(bytes: &[u8]) -> Result<(), ()> {
    if let Ok(r) = tpt_av_cadence_wav::WavReader::open(reader(bytes)) {
        let _ = r.info().clone();
    }
    Ok(())
}

fn parse_flac(bytes: &[u8]) -> Result<(), ()> {
    if let Ok(r) = tpt_av_cadence_flac::FlacReader::open(reader(bytes)) {
        let _ = r.info().clone();
    }
    Ok(())
}

fn parse_ogg_opus(bytes: &[u8]) -> Result<(), ()> {
    if let Ok(r) = tpt_av_cadence_opus::OggOpusReader::open(reader(bytes)) {
        let _ = r.info().clone();
    }
    Ok(())
}

fn parse_vorbis(bytes: &[u8]) -> Result<(), ()> {
    if let Ok(r) = tpt_av_cadence_vorbis::VorbisDecoder::open(reader(bytes)) {
        let _ = r.info().clone();
    }
    Ok(())
}
