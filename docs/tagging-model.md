# Tagging Model

Source of truth: [`../spec.txt`](../spec.txt) §11, §3.5, §13.5.

## Sources

| Source | Meaning | Confidence |
| :--- | :--- | :--- |
| `Manual` | entered by a user | none (user statements are not probabilities) |
| `LocalModel { model, version }` | bundled offline model — thumbnails, sampled frames, waveform audio events | required, 0.0–1.0 |
| `CloudModel { provider, model }` | opt-in cloud model, Phase 2, behind §10.1 disclosure | per provider |

Every tag displays its source and, where applicable, its confidence:

```
person        (local-model, 0.91)
outdoor       (local-model, 0.88)
press-event   (cloud-model, 0.95)
interview     (manual)
```

Model and version are recorded per tag, so a re-tagging run can attribute labels to exact model runs (§3.5).

## Rules

1. **Editable and deletable regardless of source** (§11): users can edit or delete any tag, manual or model-generated.
2. **Rejected-tag memory** (§11): rejecting an auto-tag records it. Future re-tagging passes must not silently reintroduce it — a match is re-proposed only flagged as previously rejected. Memory keys are (asset, label, source-kind) so a model *update* still gets flagged rather than silently suppressed.
3. **Offline by default** (§3.3, §17): local-model tagging is fully offline; cloud tagging cannot run unless the per-archive setting was explicitly enabled after the §10.1 disclosure.
4. **Revocation is safe** (§10.1, §19.4): disabling cloud AI never deletes or invalidates previously generated tags — local results are untouched.

## Review flow (§13.5)

The tagging review screen queues auto-generated tags with accept/reject/edit actions and bulk operations; the rejected-tag memory is visible so users can see what will be flagged.

## What lands where

Tags are stored in SQLite with source/confidence (§16) and feed Layer 2 full-text search (§10) via the tag references on `SearchIndexEntry` (§6.7).
