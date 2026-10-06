//! Fuzzing (`--features tpt`): parser never-panics proofs reusing
//! `tpt-av-test-fuzz` (§19.5). Deterministic proptest cases (no libFuzzer
//! needed); corpus regressions live alongside.

#[cfg(feature = "tpt")]
mod wired {
    use proptest::prelude::*;
    use tpt_app_media_asset_intelligence_search::parse_query;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn search_query_never_panics(s in "\\PC{0,64}") {
            tpt_av_test_fuzz::fuzz_parser_never_panics!(parser: parse_query, input: &s);
        }

        #[test]
        fn codec_allowlist_never_panics(s in "\\PC{0,32}") {
            tpt_av_test_fuzz::fuzz_parser_never_panics!(
                parser: tpt_app_media_asset_intelligence_core::is_supported_codec,
                input: &s
            );
        }

        #[test]
        fn mkv_demux_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
            tpt_av_test_fuzz::fuzz_parser_never_panics!(
                parser: parse_mkv,
                input: &bytes
            );
        }
    }

    #[allow(dead_code)]
    fn parse_mkv(bytes: &[u8]) -> Result<(), ()> {
        let _ = tpt_kinetix_demux::MkvDemuxer::new(bytes.to_vec());
        Ok(())
    }
}
