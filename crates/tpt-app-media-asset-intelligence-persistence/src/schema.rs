//! SQLite schema and migrations (spec §16). Idempotent `CREATE TABLE IF
//! NOT EXISTS` + `PRAGMA user_version` for forward-compatible evolution.

pub const SCHEMA_VERSION: u32 = 1;

pub const DDL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS archives (
    id                      TEXT PRIMARY KEY,
    name                    TEXT NOT NULL,
    watch_enabled           INTEGER NOT NULL DEFAULT 0,
    ai_local_tagging        INTEGER NOT NULL DEFAULT 1,
    ai_cloud_tagging        INTEGER NOT NULL DEFAULT 0,
    ai_cloud_provider       TEXT,
    ai_cloud_semantic_search INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS archive_roots (
    archive_id              TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    pos                     INTEGER NOT NULL,
    path                    TEXT NOT NULL,
    PRIMARY KEY (archive_id, pos)
);

CREATE TABLE IF NOT EXISTS preferences (
    key                     TEXT PRIMARY KEY,
    value                   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assets (
    id                      TEXT PRIMARY KEY,
    archive_id              TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    path                    TEXT NOT NULL UNIQUE,
    fingerprint             TEXT NOT NULL,
    size_bytes              INTEGER NOT NULL,
    modified_time           TEXT NOT NULL,
    media_type              INTEGER NOT NULL,
    technical               TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_assets_archive ON assets(archive_id);
CREATE INDEX IF NOT EXISTS idx_assets_fingerprint ON assets(fingerprint);

CREATE TABLE IF NOT EXISTS derivatives (
    asset_id                TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    kind                    INTEGER NOT NULL,
    path                    TEXT NOT NULL,
    PRIMARY KEY (asset_id, kind)
);

CREATE TABLE IF NOT EXISTS tags (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id                TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    label                   TEXT NOT NULL,
    source                  INTEGER NOT NULL,
    confidence              REAL,
    model_version           TEXT,
    rejected                INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_tags_asset ON tags(asset_id);

CREATE TABLE IF NOT EXISTS duplicate_groups (
    id                      TEXT PRIMARY KEY,
    match_kind              INTEGER NOT NULL,
    reviewed                INTEGER NOT NULL DEFAULT 0,
    keeper                  TEXT
);

CREATE TABLE IF NOT EXISTS duplicate_group_members (
    group_id                TEXT NOT NULL REFERENCES duplicate_groups(id) ON DELETE CASCADE,
    asset_id                TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    pos                     INTEGER NOT NULL,
    PRIMARY KEY (group_id, asset_id)
);

CREATE TABLE IF NOT EXISTS scenes (
    asset_id                TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    idx                     INTEGER NOT NULL,
    start_secs              REAL NOT NULL,
    end_secs                REAL,
    PRIMARY KEY (asset_id, idx)
);

CREATE TABLE IF NOT EXISTS health_snapshots (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    archive_id              TEXT REFERENCES archives(id) ON DELETE CASCADE,
    taken_at                TEXT NOT NULL,
    total_assets            INTEGER NOT NULL,
    missing_files           TEXT NOT NULL,
    corrupt_assets          TEXT NOT NULL,
    orphaned_derivatives    TEXT NOT NULL,
    duplicate_waste_bytes   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_health_archive_time
    ON health_snapshots(archive_id, taken_at);

CREATE TABLE IF NOT EXISTS jobs (
    id                      TEXT PRIMARY KEY,
    archive_id              TEXT REFERENCES archives(id) ON DELETE SET NULL,
    kind                    TEXT NOT NULL,
    status                  TEXT NOT NULL,
    progress                REAL NOT NULL DEFAULT 0,
    error                   TEXT,
    note                    TEXT,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
"#;
