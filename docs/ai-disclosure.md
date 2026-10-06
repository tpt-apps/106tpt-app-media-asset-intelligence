# AI Disclosure

How TPT Media Asset Intelligence draws — and enforces — the boundary between local and cloud AI. Source of truth: [`../spec.txt`](../spec.txt) §3.3, §10.1, §11, §17, §19.4.

## The default: everything local, everything offline

Indexing, deterministic metadata search, duplicate detection, scene detection, and local-model tagging require **zero** network access (§3.3). The user can fully disable outbound connectivity and still index, search, tag (local models) and browse the entire archive (§3.1).

## The boundary: cloud AI is opt-in, per feature

Cloud-assisted tagging and cloud-assisted semantic search are Phase 2 features, each:

- **off by default**;
- enabled **per feature** — never by one global "enable AI" switch, never silently by an update or a "recommended settings" prompt;
- accompanied by an itemised disclosure of what data is sent, to whom, and when (§10.1), for example:

> Enabling cloud semantic search will send:
> - a compact visual/audio embedding derived from each processed asset (not the original media file)
> - to: [selected provider]
>
> This setting is OFF by default and can be disabled at any time. Previously generated local results are not affected.

- confirmed **per archive (or per import job)**;
- **revocable at any time** — disabling cloud AI does not delete or invalidate previously generated local-model results (§10.1).

## What never leaves the machine

Under any setting (§17):

- original media is never uploaded — cloud features send only derived data such as compact embeddings;
- filenames, tags and archive contents are never telemetered;
- the local API binds loopback only and is disabled by default (§15).

## Model provenance (§3.5, §11)

Every model-generated tag records its source (`LocalModel{model, version}` or `CloudModel{provider, model}`) and confidence. Rejected auto-tags are remembered and flagged on re-proposal rather than silently reintroduced (§11).

## Enforcement (§19.4)

Automated AI-boundary tests assert:

1. no network call occurs during indexing, tagging or search with cloud AI disabled (the default);
2. enabling cloud AI requires the documented disclosure/confirmation flow — in the CLI, cloud tagging additionally requires the explicit `--cloud --cloud-confirmed` flag pair (§14);
3. disabling cloud AI after use leaves prior local-model tags intact.

These are Phase 0 scaffolds where the relevant features are Phase 2 (e.g. network-interception tests land with the first cloud integration); the settings defaults, gating flags and CLI contract already exist and are unit-tested.
