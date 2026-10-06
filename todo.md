# tpt-app-media-asset-intelligence — Project Todo

TPT Media Asset Intelligence — local-first media archive indexing, search, dedupe, and archive-health platform (TPT Solutions). License: dual **MIT OR Apache-2.0**. Source of truth: `spec.txt`; all `§` references point back to it.

---

## Phase 0: Repository, Licensing & Foundation Verification (§4, §5, §16, §17)

- [x] Confirm `tpt-av-asset` (primary dependency, §5.1) is reachable from this workspace (path or git dependency) and enumerate its existing asset/derivative/job/cache capabilities to avoid duplicating them — *consumed as a sibling path dep; capability map + compile-time reachability test in `crates/tpt-app-media-asset-intelligence-ingest/src/foundation.rs`*
- [x] Confirm `tpt-kinetix` (§5.2) resolves as a pinned git dependency (`github.com/tpt-solutions/tpt-kinetix`, per the rev-pinning pattern already used in `tpt-av-asset/Cargo.toml` and `tpt-visual/Cargo.toml`) — flag as an integration risk until proven resolvable from a fresh workspace — *resolved: our fresh workspace fetched and built the pinned revs (`9747a2b`) transitively through `tpt-av-asset`'s pins*
- [x] Confirm `tpt-cadence` (§5.2, audio decode/waveform/technical metadata) is reachable from this workspace — flag as an integration risk (unresolved for sibling `tpt-app-voice-studio` at time of writing per its `todo.md`; re-check current status before wiring real `Cargo.toml` paths) — *resolved via `tpt-av-asset`'s git-rev pins (`72794ef`); built in this workspace*
- [x] Confirm `tpt-visual` (§5.2, scene-change/perceptual-duplicate/visual-similarity) is reachable from this workspace — *`tpt-av-visual-utils` (frame/pixel/time primitives) wired into `-scenes` with a reachability test; the GPU compositor stack is deliberately not consumed*
- [ ] Confirm `tpt-voice` (§5.2, optional transcription/diarisation) is reachable from this workspace — flag as an integration risk since it is the most optional/least-proven integration and is explicitly out of MVP scope (§20) — **Status: NOT reachable.** No local repo exists and none was found on this machine; out of MVP scope (§20), so this only blocks the Phase 2 integration (§21)
- [ ] Confirm `tpt-av-test` (§5.2, golden fixtures/fuzzing harnesses) is reachable from this workspace for reuse — *repo present locally and public on GitHub; remaining: wire as a dev-dependency when the fuzz/golden work lands (§19.5, §26 steps 20–23)*
- [x] Initialize git repository, add `.gitignore` (Rust/Cargo template) — *repository already initialized; `.gitignore` covers target/, editor dirs and runtime derivative caches*
- [x] Create `LICENSE-MIT` and `LICENSE-APACHE` (dual license, copyright holder TPT Solutions)
- [x] Set `license = "MIT OR Apache-2.0"` in workspace `Cargo.toml`
- [x] Create `deny.toml` (cargo-deny license/advisory enforcement) — *permissive-only allow list; C/copyleft media stacks banned; TPT git sources allowed; validated with cargo-deny locally and in CI*
- [x] Create Cargo workspace `Cargo.toml` (members per §4: `-core`, `-model`, `-ingest`, `-dedupe`, `-scenes`, `-tagging`, `-search`, `-health`, `-cli`, `-service`, `-tauri`, `-test`) — *11 workspace members; `-tauri` excluded (Tauri/WebView2 must not be an engine prerequisite), mirroring `tpt-app-av-automation`*
- [x] Scaffold `tpt-app-media-asset-intelligence-core` crate — *ids, media types, SHA-256 fingerprint, ENGINE_VERSION (§6)*
- [x] Scaffold `tpt-app-media-asset-intelligence-model` crate (domain model per §6) — *full §6 domain model implemented with serde + invariant tests (cloud fields default disabled, exact-hash similarity 1.0, rejected-tag matching)*
- [x] Scaffold `tpt-app-media-asset-intelligence-ingest` crate — *foundation wiring + capability map, pipeline-stage/two-pass model (§7.1), scan outcome classification (§18)*
- [x] Scaffold `tpt-app-media-asset-intelligence-dedupe` crate — *detection layers + JSON report with shared reclaimable-waste arithmetic (§8, §12)*
- [x] Scaffold `tpt-app-media-asset-intelligence-scenes` crate — *deterministic detector config (part of the §3.2 contract) + tpt-visual wiring*
- [x] Scaffold `tpt-app-media-asset-intelligence-tagging` crate — *rejected-tag memory with flagged-reproposal semantics (§11)*
- [x] Scaffold `tpt-app-media-asset-intelligence-search` crate — *shared query language: AST, parser with positioned errors, display round-trip (§14); designated §19.5 fuzz target*
- [x] Scaffold `tpt-app-media-asset-intelligence-health` crate — *health issue kinds + snapshots (§12)*
- [x] Scaffold `tpt-app-media-asset-intelligence-cli` crate — *clap definitions, stable exit-code contract 0–6 (§14), `cloud_ai_used` envelopes, cloud tagging behind `--cloud --cloud-confirmed`*
- [x] Scaffold `tpt-app-media-asset-intelligence-service` crate — *§15 local API invariants encoded: loopback-only bind, disabled by default*
- [x] Scaffold `tpt-app-media-asset-intelligence-tauri` crate — *buildable shell (manifest, conf, capabilities, placeholder UI, engine-version command); excluded from workspace/CI*
- [x] Scaffold `tpt-app-media-asset-intelligence-test` crate — *fixture discovery for the five §19.2 archives*
- [x] Create `README.md` (product overview, positioning, quickstart)
- [x] Create `CONTRIBUTING.md`
- [x] Create `CHANGELOG.md`
- [x] Create `docs/` skeleton: `architecture.md`, `index-model.md`, `search-model.md`, `tagging-model.md`, `ai-disclosure.md`, `archive-health.md`
- [x] Set up CI (GitHub Actions): build, test, clippy, fmt check — `.github/workflows/ci.yml` (Linux reference platform; clones sibling `tpt-av-asset` into the path-dep layout)
- [x] Add `cargo-deny check` to CI
- [x] Scaffold `fixtures/` directories: `synthetic-archive-small/`, `synthetic-archive-large/`, `duplicates/`, `corrupt/`, `mixed-formats/` — §19.2 — *directories + purpose READMEs; synthetic media and documented expectations are generated with the golden suite in Phase 1 (§26 step 20)*
- [x] Scaffold `tests/` directories: `integration/`, `golden/`, `scale/` — *with READMEs; populated in Phase 1 (§26 steps 20–23)*

---

## Phase 1: MVP Build

Goal: deliver the full MVP per §20 and Definition of Done per §25, following the recommended implementation order in §26.

### Domain Model
- [ ] Implement `Archive` and `AiSettings` types, cloud fields defaulting to disabled/None — §6.1
- [ ] Implement `Asset` type (fingerprint, size, technical metadata) — §6.2
- [ ] Implement `Derivative`/`DerivativeKind` types (Thumbnail/Proxy/Waveform) — §6.3
- [ ] Implement `Tag`/`TagSource` types (Manual/LocalModel/CloudModel) with confidence — §6.4
- [ ] Implement `DuplicateGroup`/`MatchKind` types — §6.5
- [ ] Implement `Scene` type — §6.6
- [ ] Implement `SearchIndexEntry` type — §6.7

### Ingestion & Technical Metadata
- [ ] Integrate `tpt-av-asset` and build the archive/asset domain model on top of it — §26 steps 2–3
- [ ] Implement filesystem/watch-folder scanning and recursive import of existing archive structures — §7, §26 step 4
- [ ] Implement fingerprinting — §7
- [ ] Implement technical metadata extraction (codec/container/resolution/duration/stream layout) via `tpt-kinetix`/`tpt-cadence`, deterministic per §3.2 — §7, §26 step 5
- [ ] Implement incremental/resumable indexing so an interrupted scan does not reprocess unchanged assets — §18, §25

### Pass 1 Search
- [ ] Implement Pass 1 metadata/filename/path search, surfacing results within seconds of scan start — §7.1, §10 Layer 1, §26 step 6

### Derivative Generation
- [ ] Implement thumbnail/proxy/waveform derivative generation via `tpt-av-asset`, running in the background without blocking the UI — §26 step 7

### Deduplication
- [ ] Implement exact-hash duplicate detection — §8, §26 step 8
- [ ] Implement perceptual near-duplicate detection via `tpt-visual` — §8, §26 step 9
- [ ] Implement audio fingerprint duplicate matching (`tpt-cadence`/DSP-based) — §8
- [ ] Implement reviewable duplicate groups UI/data model with keeper selection, reviewed/unreviewed status, and no automatic deletion or undo-safe review flow — §8

### Scene Detection
- [ ] Implement deterministic frame-difference/perceptual-hash scene-change detection via `tpt-visual` — §9, §26 step 10

### Tagging
- [ ] Implement local-model automatic tagging (object/scene/label from thumbnails/frames, basic audio-event tags from waveforms), entirely offline, with source/confidence/model-version tracking — §11, §26 step 11
- [ ] Implement manual tagging (create/edit/delete regardless of tag source) — §11
- [ ] Implement rejected-tag memory so future re-tagging passes flag previously rejected auto-tags instead of silently reintroducing them — §11

### Full-Text Search
- [ ] Implement full-text search over local-model tag labels and user notes — §10 Layer 2, §26 step 12

### Archive Health
- [ ] Implement missing-file detection (indexed but no longer present on disk) — §12
- [ ] Implement broken/relinked path detection (moved files) — §12
- [ ] Implement unreadable/corrupt asset detection during indexing — §12
- [ ] Implement orphaned-derivative detection (derivative with no matching source asset) — §12
- [ ] Implement storage growth / duplicate-waste trend summary — §12, §26 step 13

### Persistence
- [ ] Implement SQLite persistence for archives/roots/settings, assets (paths+fingerprints, not raw media), derivatives (paths only), tags, duplicate groups, archive-health snapshots, and user/AI-enablement preferences — §16, §26 step 14
- [ ] Ensure cached derivatives are stored separately from the database and from the original archive — §16

### Job Queue & Resumability
- [ ] Implement the indexing job queue (active/queued indexing, derivative-generation, tagging jobs) with pause/resume/cancel — §13.7, §26 step 15
- [ ] Ensure concurrency uses available CPU cores for parallel metadata extraction/derivative generation while search stays responsive during background scans — §18

### CLI
- [ ] Implement CLI `index` command (`--archive`, `--roots`) using the same engine as the GUI — §14, §26 step 16
- [ ] Implement CLI `search` command with query syntax (e.g. `codec:prores AND tag:interview`) — §14
- [ ] Implement CLI `dedupe` command with JSON report output — §14
- [ ] Implement CLI `tag` command, local-only by default, requiring an explicit flag to use any cloud model — §14
- [ ] Implement machine-readable (JSON) result output including `cloud_ai_used` field — §14
- [ ] Implement the stable exit-code contract (0 SUCCESS, 1 PARTIAL_SUCCESS, 2 INDEXING_FAILED, 3 SEARCH_FAILED, 4 CONFIGURATION_ERROR, 5 INPUT_ERROR, 6 INTERNAL_ERROR) — §14

### Desktop UI (Tauri)
- [ ] Implement Archive Browser (grid/list, thumbnails, filter by technical metadata/tags/folder) — §13.1, §26 step 17
- [ ] Implement Search screen (unified search bar, filter chips, per-result match explanation) — §13.2, §26 step 17
- [ ] Implement Asset Inspector (file info, technical metadata, tags with source/confidence, derivative previews, duplicate-group membership) — §13.3, §26 step 17
- [ ] Implement Duplicate Review screen (keeper selection, reviewed/unreviewed tracking) — §13.4, §26 step 18
- [ ] Implement Tagging Review screen (accept/reject/edit, bulk operations) — §13.5, §26 step 18
- [ ] Implement Archive Health Dashboard — §13.6, §26 step 19
- [ ] Implement Indexing Queue screen (active/queued jobs, pause/resume/cancel) — §13.7, §26 step 19

### Local API (optional)
- [ ] Implement optional localhost-only API (127.0.0.1, never bound externally, disabled by default) with `/archives/:id/search`, `/assets/:id`, `/archives/:id/reindex`, `/jobs/:id`, `/health` — §15

### Security & Privacy
- [ ] Ensure no mandatory network access for indexing, deterministic search, or local-model tagging — §17
- [ ] Ensure no cloud upload of original media under any setting; cloud AI (Phase 2) sends only derived data — §17
- [ ] Ensure no external telemetry of filenames, tags, or archive contents — §17
- [ ] Implement safe handling of malformed/corrupt media during indexing (no crash) — §17
- [ ] Implement strict path validation for watch-folder and NAS-mounted roots — §17
- [ ] Confirm no automatic reorganisation, renaming, or deletion of source files without explicit user confirmation — §3.4, §17

### Testing
- [ ] Unit tests per pipeline stage (fingerprinting, metadata extraction, derivative generation, duplicate detection, scene detection, tagging, search indexing): valid/invalid/boundary/malformed cases — §19.1
- [ ] Build golden fixture archives with documented expected index counts, duplicate groups, and scene boundaries across `synthetic-archive-small/`, `synthetic-archive-large/`, `duplicates/`, `corrupt/`, `mixed-formats/` — §19.2, §26 step 20
- [ ] Build large synthetic-archive scale/performance regression test (indexing throughput, memory usage) — §19.3, §26 step 21
- [ ] Implement AI-boundary tests: no network call during indexing/tagging/search with cloud AI disabled; enabling cloud AI requires the disclosure/confirmation flow; disabling cloud AI does not delete/invalidate prior local-model tags — §19.4, §26 step 22
- [ ] Fuzz media container/metadata parsers, archive-config and search-query parsers, and CLI arguments, reusing `tpt-av-test` where possible — §19.5, §26 step 23
- [ ] Establish regression-fixture policy: every production bug produces a permanent regression test — §19.6

### Hardening, Packaging & Beta
- [ ] Harden error handling and failure isolation — §26 step 24
- [ ] Package Windows release — §26 step 25
- [ ] Validate against a real large local/NAS archive — §26 step 25
- [ ] Test clean-machine installation without development tooling — §25
- [ ] Run a private beta with a real production company or archive team — §26

---

## Phase 2: Post-MVP Expansion (§21 Phase 2)

- [ ] Implement opt-in cloud-assisted tagging, subject to the §10.1 disclosure/confirmation flow — §11, §21
- [ ] Implement opt-in cloud-assisted semantic search, subject to the §10.1 disclosure/confirmation flow — §10 Layer 3, §21
- [ ] Implement local embedding-based semantic/similarity search (default local model) — §10 Layer 3, §21
- [ ] Integrate `tpt-voice` for transcription and speaker diarisation, toggleable per-archive/per-import, feeding full-text and speaker-based search — §5.2, §21
- [ ] Implement import of TPT Media QC pass/fail findings as searchable, filterable asset metadata — §5.3, §21
- [ ] Implement import of TPT Media Forensics findings as archive metadata — §5.3, §21
- [ ] Reuse existing TPT Voice Studio transcripts where a project has already been transcribed, instead of re-transcribing — §5.3
- [ ] Implement richer archive-health trend reporting over time — §21

---

## Phase 3: Team & Platform Features (§21 Phase 3)

- [ ] Implement multi-user/facility permissions and shared archive access — §21, §23 (Facility tier)
- [ ] Implement plugin SDK for custom local taggers/classifiers — §21
- [ ] Implement distributed/NAS-cluster indexing for very large, multi-site archives — §21
- [ ] Do not build Facility-tier multi-user features until customer demand is evidenced — §23, §27
