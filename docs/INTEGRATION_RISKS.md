# Integration Risks (Phase 0 verification)

Checked at scaffold time; none resolvable from a fresh workspace:

- `tpt-av-asset` (§5.1): no local checkout, no crates.io entry — capabilities not enumerated; re-check before wiring.
- `tpt-kinetix` (§5.2): expected pinned git dep `github.com/tpt-solutions/tpt-kinetix` — unproven; must additionally confirm open-codec-only decode (no H.264/AAC decoders pulled in).
- `tpt-cadence` (§5.2): same as above; unresolved for sibling `tpt-app-voice-studio` at time of writing.
- `tpt-visual` (§5.2): same as above (scene/perceptual/similarity seams are stubbed).
- `tpt-voice` (§5.2): most optional, out of MVP (§20) — Phase 2 concern.
- `tpt-av-test` (§5.2): golden/fuzz harnesses unavailable; local fixture helpers in `-test` stand in.

The workspace builds, tests, and runs fully offline without them. Wire each behind the
documented seam (derivative generation → av-asset, frame/scoring → kinetix/visual,
waveform/audio → cadence) and verify with `cargo-deny`/dependency review that no
patent-encumbered decoders enter the tree.
