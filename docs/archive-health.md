# Archive Health

<<<<<<< HEAD
Collection-level health (distinct from per-file QC): missing files, broken/relinked paths,
unreadable/corrupt assets, orphaned derivatives, storage growth + duplicate-waste trends (§12).
Out-of-scope-codec assets are flagged here as `unsupported` rather than errors.
=======
Source of truth: [`../spec.txt`](../spec.txt) §12, §13.6, §16.

## Scope

Archive health covers the health of the archive **as a collection**. Per-file quality control is TPT Media QC's role — this product references rather than duplicates it (§12).

## Checks

| Check | Meaning | Spec |
| :--- | :--- | :--- |
| Missing file | indexed but no longer present on disk | §12 |
| Moved file | recorded path gone, but a fingerprint match exists at a new path (broken/relinked) | §12 |
| Corrupt/unreadable asset | failed safe handling during indexing; recorded, never a crash | §12, §17 |
| Orphaned derivative | thumbnail/proxy/waveform with no matching source asset | §12 |
| Storage growth / duplicate waste | derivative growth and reclaimable duplicate bytes, tracked over time | §12 |

Duplicate-waste totals use one definition shared with the dedupe report (§8): per group, member sizes minus one baseline copy (the chosen keeper, else the largest member). The arithmetic lives in `tpt-app-media-asset-intelligence-dedupe::report` so the CLI and dashboard can never disagree.

## Snapshots (§16)

Findings are stored as **archive health snapshots** over time, so the dashboard can trend missing/corrupt counts and storage waste — the §12 example dashboard:

```
Archive Health
──────────────────────────────
Total assets            48,201
Missing files                12
Corrupt/unreadable            3
Duplicate waste           1.8 TB
Orphaned derivatives         41

[View details]
```

## Positioning

Archive health is what makes the index *maintainable*: it is Level 3 of the differentiation roadmap — "can I find duplicates and broken files, and reclaim wasted storage?" (§22) — and a Facility-tier differentiator for advanced reporting (§23), which is not built until customer demand is evidenced (§23, §27).
>>>>>>> f59474f40b216520196c8150f8405ef66c08859d
