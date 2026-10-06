# Index Model

SQLite index (§16): archives/roots/settings, assets (paths + fingerprints, never raw media),
derivatives (paths only, stored separately from DB and source archive), tags
(source/confidence/version + rejected memory), duplicate groups (reviewed/keeper),
scenes, health snapshots, AI-enablement preferences.

Resumability: `ResumeLog` keyed by fingerprint — interrupted scans skip unchanged assets (§18).
