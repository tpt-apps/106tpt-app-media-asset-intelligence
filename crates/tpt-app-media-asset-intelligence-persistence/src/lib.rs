//! SQLite persistence for the full index (spec §16, §26 step 14).
//!
//! Stores archives/roots/settings, assets (paths + fingerprints, never raw
//! media), derivatives (paths only), tags with source/confidence/version,
//! duplicate groups + review status, scenes, archive-health snapshots, user
//! preferences, and job state.
//!
//! Cached derivatives always live outside the database and outside the
//! original archive (enforced elsewhere by the model: derivatives carry
//! paths only).
//!
//! Database layout (see [`schema`]):
//!
//! ```text
//! archives            archive roots + AI/cloud settings (spec §6.1)
//! archive_roots       positional root paths per archive
//! preferences         user preferences + AI-enablement (key/value)
//! assets              paths + fingerprints + technical metadata (JSON)
//! derivatives         (asset, kind, path)
//! tags                label, source, confidence, model_version, rejected
//! duplicate_groups    match kind, review status, keeper selection
//! duplicate_group_members  positional member asset ids
//! scenes              per-asset scene boundaries
//! health_snapshots    archive-health history (spec §12 trends)
//! jobs                job-queue state (spec §13.7)
//! ```

pub mod job;
pub mod schema;

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tpt_app_media_asset_intelligence_health::HealthSnapshot;
use tpt_app_media_asset_intelligence_model::{
    Archive, ArchiveId, Asset, AssetId, Derivative, DerivativeKind, DuplicateGroup, MatchKind,
    MediaType, Tag, TagSource,
};
use uuid::Uuid;

use crate::job::{Job, JobKind, JobStatus};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot decode persisted value: {0}")]
    Decode(String),
}

/// SQLite store over the §16 schema. `Store` is cheap to clone (the
/// connection is not, so methods take `&self` and use `&Connection`).
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (creating parents + schema if missing) a database at `path`.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// In-memory store, for tests and ephemeral usage.
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch(schema::DDL)?;
        conn.execute_batch(&format!(
            "PRAGMA user_version = {};",
            schema::SCHEMA_VERSION
        ))?;
        Ok(Self { conn })
    }

    // ---- archives & roots -------------------------------------------------

    /// Upsert an archive and its roots/settings in one transaction.
    pub fn upsert_archive(&self, archive: &Archive) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO archives (
                 id, name, watch_enabled,
                 ai_local_tagging, ai_cloud_tagging, ai_cloud_provider,
                 ai_cloud_semantic_search
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 watch_enabled = excluded.watch_enabled,
                 ai_local_tagging = excluded.ai_local_tagging,
                 ai_cloud_tagging = excluded.ai_cloud_tagging,
                 ai_cloud_provider = excluded.ai_cloud_provider,
                 ai_cloud_semantic_search = excluded.ai_cloud_semantic_search",
            params![
                archive.id.to_string(),
                archive.name,
                archive.watch_enabled as i64,
                archive.ai_settings.local_tagging_enabled as i64,
                archive.ai_settings.cloud_tagging_enabled as i64,
                archive.ai_settings.cloud_provider,
                archive.ai_settings.cloud_semantic_search_enabled as i64,
            ],
        )?;
        tx.execute(
            "DELETE FROM archive_roots WHERE archive_id = ?1",
            params![archive.id.to_string()],
        )?;
        for (pos, root) in archive.roots.iter().enumerate() {
            tx.execute(
                "INSERT INTO archive_roots (archive_id, pos, path) VALUES (?1, ?2, ?3)",
                params![archive.id.to_string(), pos as i64, root.to_string_lossy()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_archive(&self, id: ArchiveId) -> Result<Option<Archive>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT id, name, watch_enabled,
                        ai_local_tagging, ai_cloud_tagging, ai_cloud_provider,
                        ai_cloud_semantic_search
                 FROM archives WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)? != 0,
                        row.get::<_, i64>(3)? != 0,
                        row.get::<_, i64>(4)? != 0,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, i64>(6)? != 0,
                    ))
                },
            )
            .optional()?;
        let Some((id, name, watch, local, cloud, provider, semantic)) = row else {
            return Ok(None);
        };
        let id = Uuid::parse_str(&id).map_err(|e| StoreError::Decode(e.to_string()))?;
        let roots = self.roots_for(id)?;
        Ok(Some(Archive {
            id,
            name,
            roots,
            watch_enabled: watch,
            ai_settings: tpt_app_media_asset_intelligence_model::AiSettings {
                local_tagging_enabled: local,
                cloud_tagging_enabled: cloud,
                cloud_provider: provider,
                cloud_semantic_search_enabled: semantic,
            },
        }))
    }

    pub fn list_archives(&self) -> Result<Vec<Archive>, StoreError> {
        let ids = list_ids(&self.conn, "SELECT id FROM archives ORDER BY name")?;
        ids.into_iter()
            .map(|id| {
                Uuid::parse_str(&id)
                    .map_err(|e| StoreError::Decode(e.to_string()))
                    .and_then(|u| self.get_archive(u))
                    .and_then(|a| {
                        a.ok_or_else(|| StoreError::Decode("archive id on file has no row".into()))
                    })
            })
            .collect()
    }

    pub fn delete_archive(&self, id: ArchiveId) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM archives WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    fn roots_for(&self, archive_id: ArchiveId) -> Result<Vec<std::path::PathBuf>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM archive_roots WHERE archive_id = ?1 ORDER BY pos")?;
        let rows = stmt
            .query_map(params![archive_id.to_string()], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().map(std::path::PathBuf::from).collect())
    }

    // ---- user preferences / AI enablement --------------------------------

    pub fn set_preference(&self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_preference(&self, key: &str) -> Result<Option<String>, StoreError> {
        self.conn
            .query_row(
                "SELECT value FROM preferences WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    // ---- assets -----------------------------------------------------------

    pub fn upsert_asset(&self, asset: &Asset) -> Result<(), StoreError> {
        let technical = serde_json::to_string(&asset.technical_metadata)?;
        self.conn.execute(
            "INSERT INTO assets (
                 id, archive_id, path, fingerprint, size_bytes, modified_time,
                 media_type, technical
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 archive_id = excluded.archive_id,
                 path = excluded.path,
                 fingerprint = excluded.fingerprint,
                 size_bytes = excluded.size_bytes,
                 modified_time = excluded.modified_time,
                 media_type = excluded.media_type,
                 technical = excluded.technical",
            params![
                asset.id.to_string(),
                asset.archive.to_string(),
                asset.path.to_string_lossy(),
                asset.fingerprint,
                asset.size_bytes as i64,
                asset.modified_time.to_rfc3339(),
                media_type_to_int(asset.media_type),
                technical,
            ],
        )?;
        Ok(())
    }

    pub fn get_asset(&self, id: AssetId) -> Result<Option<Asset>, StoreError> {
        self.conn
            .query_row(
                &format!("{} WHERE id = ?1", asset_select_sql()),
                params![id.to_string()],
                asset_from_row,
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn assets_for_archive(&self, archive: ArchiveId) -> Result<Vec<Asset>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "{} WHERE archive_id = ?1 ORDER BY path",
            asset_select_sql()
        ))?;
        let rows = stmt
            .query_map(params![archive.to_string()], asset_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn delete_asset(&self, id: AssetId) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM assets WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    // ---- derivatives ------------------------------------------------------

    pub fn upsert_derivative(&self, d: &Derivative) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO derivatives (asset_id, kind, path) VALUES (?1, ?2, ?3)
             ON CONFLICT(asset_id, kind) DO UPDATE SET path = excluded.path",
            params![
                d.asset_id.to_string(),
                derivative_kind_to_int(d.kind),
                d.path.to_string_lossy()
            ],
        )?;
        Ok(())
    }

    pub fn derivatives_for_asset(&self, asset_id: AssetId) -> Result<Vec<Derivative>, StoreError> {
        let mut out = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT kind, path FROM derivatives WHERE asset_id = ?1 ORDER BY kind")?;
        let mut q = stmt.query(params![asset_id.to_string()])?;
        while let Some(row) = q.next()? {
            let kind = derivative_kind_from_int(row.get(0)?)?;
            out.push(Derivative {
                asset_id,
                kind,
                path: std::path::PathBuf::from(row.get::<_, String>(1)?),
            });
        }
        Ok(out)
    }

    // ---- tags -------------------------------------------------------------

    pub fn add_tag(&self, tag: &Tag) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO tags (asset_id, label, source, confidence, model_version, rejected)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                tag.asset_id.to_string(),
                tag.label,
                tag_source_to_int(tag.source),
                tag.confidence,
                tag.model_version,
                tag.rejected as i64,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a persisted tag by its database id (manual edit path, spec §11).
    pub fn delete_tag(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM tags WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Flip the rejected flag (rejected-tag memory, spec §11).
    pub fn set_tag_rejected(&self, id: i64, rejected: bool) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE tags SET rejected = ?1 WHERE id = ?2",
            params![rejected as i64, id],
        )?;
        Ok(())
    }

    pub fn tags_for_asset(&self, asset_id: AssetId) -> Result<Vec<(i64, Tag)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, label, source, confidence, model_version, rejected
             FROM tags WHERE asset_id = ?1 ORDER BY id",
        )?;
        let mut out = Vec::new();
        let mut q = stmt.query(params![asset_id.to_string()])?;
        while let Some(row) = q.next()? {
            let id = row.get(0)?;
            let tag = Tag {
                asset_id,
                label: row.get(1)?,
                source: tag_source_from_int(row.get(2)?)?,
                confidence: row.get(3)?,
                model_version: row.get(4)?,
                rejected: row.get::<_, i64>(5)? != 0,
            };
            out.push((id, tag));
        }
        Ok(out)
    }

    // ---- duplicate groups -------------------------------------------------

    pub fn upsert_duplicate_group(&self, group: &DuplicateGroup) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO duplicate_groups (id, match_kind, reviewed, keeper)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 match_kind = excluded.match_kind,
                 reviewed = excluded.reviewed,
                 keeper = excluded.keeper",
            params![
                group.id.to_string(),
                match_kind_to_int(group.match_kind),
                group.reviewed as i64,
                group.keeper.map(|k| k.to_string()),
            ],
        )?;
        tx.execute(
            "DELETE FROM duplicate_group_members WHERE group_id = ?1",
            params![group.id.to_string()],
        )?;
        for (pos, asset_id) in group.asset_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO duplicate_group_members (group_id, asset_id, pos)
                 VALUES (?1, ?2, ?3)",
                params![group.id.to_string(), asset_id.to_string(), pos as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn duplicate_groups_for_asset(
        &self,
        asset_id: AssetId,
    ) -> Result<Vec<DuplicateGroup>, StoreError> {
        let ids: Vec<String> = self
            .conn
            .prepare(
                "SELECT g.id FROM duplicate_groups g
                 JOIN duplicate_group_members m ON m.group_id = g.id
                 WHERE m.asset_id = ?1",
            )?
            .query_map(params![asset_id.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for id in ids {
            let gid = Uuid::parse_str(&id).map_err(|e| StoreError::Decode(e.to_string()))?;
            if let Some(group) = self.get_duplicate_group(gid)? {
                out.push(group);
            }
        }
        Ok(out)
    }

    fn get_duplicate_group(&self, id: Uuid) -> Result<Option<DuplicateGroup>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT match_kind, reviewed, keeper FROM duplicate_groups WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)? != 0,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((match_kind, reviewed, keeper)) = row else {
            return Ok(None);
        };
        let asset_ids: Result<Vec<String>, rusqlite::Error> = self
            .conn
            .prepare(
                "SELECT asset_id FROM duplicate_group_members
                 WHERE group_id = ?1 ORDER BY pos",
            )?
            .query_map(params![id.to_string()], |r| r.get::<_, String>(0))?
            .collect();
        let asset_ids = asset_ids?
            .into_iter()
            .map(|s| Uuid::parse_str(&s).ok())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| StoreError::Decode("duplicate member id".into()))?;
        let keeper = keeper
            .map(|k| Uuid::parse_str(&k))
            .transpose()
            .map_err(|e| StoreError::Decode(e.to_string()))?;
        Ok(Some(DuplicateGroup {
            id,
            asset_ids,
            match_kind: match_kind_from_int(match_kind)?,
            reviewed,
            keeper,
        }))
    }

    /// Mark a group reviewed and optionally pick a keeper (undo-safe, §8).
    pub fn review_group(&self, id: Uuid, keeper: Option<AssetId>) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE duplicate_groups SET reviewed = 1, keeper = ?1 WHERE id = ?2",
            params![keeper.map(|k| k.to_string()), id.to_string()],
        )?;
        Ok(())
    }

    pub fn delete_duplicate_group(&self, id: Uuid) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM duplicate_groups WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    /// All duplicate groups whose members belong to an archive (used by the
    /// CLI dedupe summary and the archive health view).
    pub fn duplicate_groups_for_archive(
        &self,
        archive: ArchiveId,
    ) -> Result<Vec<DuplicateGroup>, StoreError> {
        let ids: Vec<String> = self
            .conn
            .prepare(
                "SELECT DISTINCT g.id FROM duplicate_groups g
                 JOIN duplicate_group_members m ON m.group_id = g.id
                 JOIN assets a ON a.id = m.asset_id
                 WHERE a.archive_id = ?1",
            )?
            .query_map(params![archive.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for id in ids {
            let gid = Uuid::parse_str(&id).map_err(|e| StoreError::Decode(e.to_string()))?;
            if let Some(group) = self.get_duplicate_group(gid)? {
                out.push(group);
            }
        }
        out.sort_by_key(|g| g.id);
        Ok(out)
    }

    /// Every duplicate group in the store (service + health views).
    pub fn list_duplicate_groups(&self) -> Result<Vec<DuplicateGroup>, StoreError> {
        let ids: Vec<String> = self
            .conn
            .prepare("SELECT id FROM duplicate_groups")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut out = Vec::new();
        for id in ids {
            let gid = Uuid::parse_str(&id).map_err(|e| StoreError::Decode(e.to_string()))?;
            if let Some(group) = self.get_duplicate_group(gid)? {
                out.push(group);
            }
        }
        out.sort_by_key(|g| g.id);
        Ok(out)
    }

    // ---- scenes -----------------------------------------------------------

    pub fn replace_scenes(
        &self,
        asset_id: AssetId,
        scenes: &[tpt_app_media_asset_intelligence_model::Scene],
    ) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM scenes WHERE asset_id = ?1",
            params![asset_id.to_string()],
        )?;
        for scene in scenes {
            tx.execute(
                "INSERT INTO scenes (asset_id, idx, start_secs, end_secs)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    asset_id.to_string(),
                    scene.index as i64,
                    scene.start_secs,
                    scene.end_secs,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn scenes_for_asset(
        &self,
        asset_id: AssetId,
    ) -> Result<Vec<tpt_app_media_asset_intelligence_model::Scene>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT idx, start_secs, end_secs FROM scenes WHERE asset_id = ?1 ORDER BY idx",
        )?;
        let mut out = Vec::new();
        let mut q = stmt.query(params![asset_id.to_string()])?;
        while let Some(row) = q.next()? {
            out.push(tpt_app_media_asset_intelligence_model::Scene {
                asset_id,
                index: row.get(0)?,
                start_secs: row.get(1)?,
                end_secs: row.get(2)?,
            });
        }
        Ok(out)
    }

    // ---- archive-health snapshots ----------------------------------------

    pub fn insert_health_snapshot(
        &self,
        archive_id: Option<ArchiveId>,
        snapshot: &HealthSnapshot,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO health_snapshots (
                 archive_id, taken_at, total_assets,
                 missing_files, corrupt_assets, orphaned_derivatives,
                 duplicate_waste_bytes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                archive_id.map(|a| a.to_string()),
                snapshot.taken_at.to_rfc3339(),
                snapshot.total_assets as i64,
                serde_json::to_string(&snapshot.missing_files)?,
                serde_json::to_string(&snapshot.corrupt_assets)?,
                serde_json::to_string(&snapshot.orphaned_derivatives)?,
                snapshot.duplicate_waste_bytes as i64,
            ],
        )?;
        Ok(())
    }

    pub fn recent_health_snapshots(
        &self,
        archive_id: Option<ArchiveId>,
        limit: i64,
    ) -> Result<Vec<HealthSnapshot>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT taken_at, total_assets,
                    missing_files, corrupt_assets,
                    orphaned_derivatives, duplicate_waste_bytes
             FROM health_snapshots {}
             ORDER BY taken_at DESC LIMIT ?1",
            match archive_id {
                Some(_) => "WHERE archive_id = ?2",
                None => "",
            }
        ))?;
        let id_str = archive_id.map(|id| id.to_string());
        let params: Vec<&dyn rusqlite::ToSql> = match &id_str {
            Some(id) => vec![&limit, id],
            None => vec![&limit],
        };
        let mut out = Vec::new();
        let mut q = stmt.query(rusqlite::params_from_iter(params))?;
        while let Some(row) = q.next()? {
            let taken_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(0)?)
                .map_err(|e| StoreError::Decode(e.to_string()))?
                .with_timezone(&Utc);
            let missing_files = serde_json::from_str(&row.get::<_, String>(2)?)?;
            let corrupt_assets = serde_json::from_str(&row.get::<_, String>(3)?)?;
            let orphaned_derivatives = serde_json::from_str(&row.get::<_, String>(4)?)?;
            out.push(HealthSnapshot {
                taken_at,
                total_assets: row.get(1)?,
                missing_files,
                corrupt_assets,
                orphaned_derivatives,
                duplicate_waste_bytes: row.get::<_, i64>(5)? as u64,
            });
        }
        Ok(out)
    }

    // ---- jobs -------------------------------------------------------------

    pub fn put_job(&self, job: &Job) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO jobs (
                 id, archive_id, kind, status, progress, error, note, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                 archive_id = excluded.archive_id,
                 kind = excluded.kind,
                 status = excluded.status,
                 progress = excluded.progress,
                 error = excluded.error,
                 note = excluded.note,
                 created_at = excluded.created_at,
                 updated_at = excluded.updated_at",
            params![
                job.id.to_string(),
                job.archive_id.map(|a| a.to_string()),
                job_kind_to_str(job.kind),
                job_status_to_str(job.status),
                job.progress,
                job.error,
                job.note,
                job.created_at.to_rfc3339(),
                job.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Convenience for queue workers: flip a job's status and error in one
    /// write without replacing the full record.
    pub fn set_status(
        &self,
        id: Uuid,
        status: JobStatus,
        error: Option<String>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE jobs SET status = ?2, error = ?3, updated_at = ?4 WHERE id = ?1",
            params![
                id.to_string(),
                job_status_to_str(status),
                error,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Convenience for queue workers: bump a job's progress fraction.
    pub fn set_progress(&self, id: Uuid, progress: f64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE jobs SET progress = ?2, updated_at = ?3 WHERE id = ?1",
            params![
                id.to_string(),
                progress.clamp(0.0, 1.0),
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_job(&self, id: Uuid) -> Result<Option<Job>, StoreError> {
        self.conn
            .query_row(
                "SELECT id, archive_id, kind, status, progress, error, note, created_at, updated_at
                 FROM jobs WHERE id = ?1",
                params![id.to_string()],
                job_from_row,
            )
            .optional()
            .map_err(StoreError::from)
    }

    pub fn list_jobs(&self) -> Result<Vec<Job>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, archive_id, kind, status, progress, error, note, created_at, updated_at
                      FROM jobs ORDER BY created_at",
        )?;
        let mut out = Vec::new();
        let mut q = stmt.query([])?;
        while let Some(row) = q.next()? {
            out.push(job_from_row(row)?);
        }
        Ok(out)
    }
}

fn list_ids(conn: &Connection, sql: &str) -> Result<Vec<String>, rusqlite::Error> {
    conn.prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect()
}

// ---- column mapping helpers ---------------------------------------------

fn asset_select_sql() -> String {
    "SELECT id, archive_id, path, fingerprint, size_bytes, modified_time, media_type, technical
     FROM assets"
        .to_string()
}

fn asset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Asset> {
    let id = Uuid::parse_str(&row.get::<_, String>(0)?)
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(0))?;
    let archive = Uuid::parse_str(&row.get::<_, String>(1)?)
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(1))?;
    let modified = row.get::<_, String>(5)?;
    let technical = row.get::<_, String>(7)?;
    Ok(Asset {
        id,
        archive,
        path: std::path::PathBuf::from(row.get::<_, String>(2)?),
        fingerprint: row.get(3)?,
        size_bytes: row.get::<_, i64>(4)? as u64,
        modified_time: DateTime::parse_from_rfc3339(&modified)
            .map_err(|_| rusqlite::Error::InvalidColumnIndex(5))?
            .with_timezone(&Utc),
        media_type: media_type_from_int(row.get(6)?)
            .map_err(|_| rusqlite::Error::InvalidColumnIndex(6))?,
        technical_metadata: serde_json::from_str(&technical)
            .map_err(|_| rusqlite::Error::InvalidColumnIndex(7))?,
    })
}

fn media_type_to_int(m: MediaType) -> i64 {
    match m {
        MediaType::Video => 0,
        MediaType::Audio => 1,
        MediaType::Unknown => 2,
    }
}

fn media_type_from_int(i: i64) -> Result<MediaType, StoreError> {
    match i {
        0 => Ok(MediaType::Video),
        1 => Ok(MediaType::Audio),
        2 => Ok(MediaType::Unknown),
        other => Err(StoreError::Decode(format!("invalid media_type {other}"))),
    }
}

fn tag_source_to_int(s: TagSource) -> i64 {
    match s {
        TagSource::Manual => 0,
        TagSource::LocalModel => 1,
        TagSource::CloudModel => 2,
    }
}

fn tag_source_from_int(i: i64) -> Result<TagSource, StoreError> {
    match i {
        0 => Ok(TagSource::Manual),
        1 => Ok(TagSource::LocalModel),
        2 => Ok(TagSource::CloudModel),
        other => Err(StoreError::Decode(format!("invalid tag source {other}"))),
    }
}

fn derivative_kind_to_int(k: DerivativeKind) -> i64 {
    match k {
        DerivativeKind::Thumbnail => 0,
        DerivativeKind::Proxy => 1,
        DerivativeKind::Waveform => 2,
    }
}

fn derivative_kind_from_int(i: i64) -> Result<DerivativeKind, StoreError> {
    match i {
        0 => Ok(DerivativeKind::Thumbnail),
        1 => Ok(DerivativeKind::Proxy),
        2 => Ok(DerivativeKind::Waveform),
        other => Err(StoreError::Decode(format!(
            "invalid derivative kind {other}"
        ))),
    }
}

fn match_kind_to_int(k: MatchKind) -> i64 {
    match k {
        MatchKind::ExactHash => 0,
        MatchKind::Perceptual => 1,
        MatchKind::AudioFingerprint => 2,
    }
}

fn match_kind_from_int(i: i64) -> Result<MatchKind, StoreError> {
    match i {
        0 => Ok(MatchKind::ExactHash),
        1 => Ok(MatchKind::Perceptual),
        2 => Ok(MatchKind::AudioFingerprint),
        other => Err(StoreError::Decode(format!("invalid match kind {other}"))),
    }
}

fn job_kind_to_str(k: JobKind) -> &'static str {
    match k {
        JobKind::Index => "index",
        JobKind::Derivatives => "derivatives",
        JobKind::Tagging => "tagging",
    }
}

fn job_status_to_str(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Queued => "queued",
        JobStatus::Active => "active",
        JobStatus::Paused => "paused",
        JobStatus::Cancelled => "cancelled",
        JobStatus::Done => "done",
        JobStatus::Failed => "failed",
    }
}

fn job_kind_from_str(s: &str) -> Result<JobKind, StoreError> {
    match s {
        "index" => Ok(JobKind::Index),
        "derivatives" => Ok(JobKind::Derivatives),
        "tagging" => Ok(JobKind::Tagging),
        other => Err(StoreError::Decode(format!("invalid job kind {other}"))),
    }
}

fn job_status_from_str(s: &str) -> Result<JobStatus, StoreError> {
    match s {
        "queued" => Ok(JobStatus::Queued),
        "active" => Ok(JobStatus::Active),
        "paused" => Ok(JobStatus::Paused),
        "cancelled" => Ok(JobStatus::Cancelled),
        "done" => Ok(JobStatus::Done),
        "failed" => Ok(JobStatus::Failed),
        other => Err(StoreError::Decode(format!("invalid job status {other}"))),
    }
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let id = Uuid::parse_str(&row.get::<_, String>(0)?)
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(0))?;
    let archive = row
        .get::<_, Option<String>>(1)?
        .map(|a| Uuid::parse_str(&a))
        .transpose()
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(1))?;
    let created_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(7))?
        .with_timezone(&Utc);
    let updated_at = DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
        .map_err(|_| rusqlite::Error::InvalidColumnIndex(8))?
        .with_timezone(&Utc);
    Ok(Job {
        id,
        archive_id: archive,
        kind: job_kind_from_str(&row.get::<_, String>(2)?)
            .map_err(|_| rusqlite::Error::InvalidColumnIndex(2))?,
        status: job_status_from_str(&row.get::<_, String>(3)?)
            .map_err(|_| rusqlite::Error::InvalidColumnIndex(3))?,
        progress: row.get(4)?,
        error: row.get(5)?,
        note: row.get(6)?,
        created_at,
        updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_app_media_asset_intelligence_health::find_missing;
    use tpt_app_media_asset_intelligence_model::{
        AiSettings, Asset, Derivative, DuplicateGroup, MatchKind, MediaType, Scene, Tag, TagSource,
        TechnicalMetadata,
    };

    fn store() -> Store {
        Store::in_memory().expect("in-memory store opens")
    }

    fn asset(id: Uuid, archive: Uuid, name: &str, supported: bool) -> Asset {
        Asset {
            id,
            archive,
            path: std::path::PathBuf::from(format!("C:\\archive\\{name}")),
            fingerprint: format!("fp-{name}"),
            size_bytes: 10,
            modified_time: Utc::now(),
            media_type: MediaType::Video,
            technical_metadata: TechnicalMetadata {
                codec: "av1".into(),
                container: "matroska".into(),
                supported,
                width: Some(1920),
                height: Some(1080),
                duration_secs: Some(12.5),
            },
        }
    }

    /// Seed an archive + asset (FK parents), returning `(archive_id, asset_id)`.
    fn seed(s: &Store) -> (Uuid, Uuid) {
        let archive = Uuid::new_v4();
        s.upsert_archive(&Archive {
            id: archive,
            name: "main".into(),
            roots: vec![std::path::PathBuf::from("C:\\archive")],
            watch_enabled: false,
            ai_settings: AiSettings::default(),
        })
        .unwrap();
        let asset_id = Uuid::new_v4();
        s.upsert_asset(&asset(asset_id, archive, "clip.mkv", true))
            .unwrap();
        (archive, asset_id)
    }

    #[test]
    fn archive_roundtrip() {
        let s = store();
        let id = Uuid::new_v4();
        let a = Archive {
            id,
            name: "main".into(),
            roots: vec!["C:\\archive".into(), "C:\\behind".into()],
            watch_enabled: true,
            ai_settings: AiSettings::default(),
        };
        s.upsert_archive(&a).unwrap();
        assert_eq!(s.get_archive(id).unwrap().unwrap().name, "main");
        assert_eq!(s.get_archive(id).unwrap().unwrap().roots.len(), 2);
        assert_eq!(s.list_archives().unwrap().len(), 1);
        s.delete_archive(id).unwrap();
        assert!(s.get_archive(id).unwrap().is_none());
    }

    #[test]
    fn cloud_settings_roundtrip() {
        let s = store();
        let id = Uuid::new_v4();
        let ai = AiSettings {
            cloud_tagging_enabled: true,
            cloud_provider: Some("local-test".into()),
            ..AiSettings::default()
        };
        s.upsert_archive(&Archive {
            id,
            name: "x".into(),
            roots: vec![],
            watch_enabled: false,
            ai_settings: ai,
        })
        .unwrap();
        let ai = s.get_archive(id).unwrap().unwrap().ai_settings;
        assert!(ai.cloud_tagging_enabled);
        assert_eq!(ai.cloud_provider.as_deref(), Some("local-test"));
    }

    #[test]
    fn preferences_roundtrip_and_upsert() {
        let s = store();
        assert!(s.get_preference("ui.theme").unwrap().is_none());
        s.set_preference("ui.theme", "dark").unwrap();
        s.set_preference("ui.theme", "system").unwrap();
        assert_eq!(
            s.get_preference("ui.theme").unwrap().as_deref(),
            Some("system")
        );
    }

    #[test]
    fn asset_roundtrip_with_technical() {
        let s = store();
        let archive = Uuid::new_v4();
        let id = Uuid::new_v4();
        s.upsert_archive(&Archive {
            id: archive,
            name: "a".into(),
            roots: vec![],
            watch_enabled: false,
            ai_settings: AiSettings::default(),
        })
        .unwrap();
        s.upsert_asset(&asset(id, archive, "clip.mkv", true))
            .unwrap();
        let out = s.get_asset(id).unwrap().unwrap();
        assert_eq!(out.fingerprint, "fp-clip.mkv");
        assert_eq!(out.technical_metadata.width, Some(1920));
        assert_eq!(out.technical_metadata.codec, "av1");
        assert!(out.technical_metadata.supported);
        assert_eq!(s.assets_for_archive(archive).unwrap().len(), 1);
        // update path (relink) preserves id
        let mut moved = asset(id, archive, "renamed.mkv", true);
        moved.technical_metadata.supported = false;
        s.upsert_asset(&moved).unwrap();
        let out = s.get_asset(id).unwrap().unwrap();
        assert!(!out.technical_metadata.supported);
        s.delete_asset(id).unwrap();
        assert!(s.get_asset(id).unwrap().is_none());
    }

    #[test]
    fn derivatives_roundtrip() {
        let s = store();
        let (_, aid) = seed(&s);
        s.upsert_derivative(&Derivative {
            asset_id: aid,
            kind: DerivativeKind::Waveform,
            path: "C:\\cache\\a.wave".into(),
        })
        .unwrap();
        let out = s.derivatives_for_asset(aid).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, DerivativeKind::Waveform);
    }

    #[test]
    fn tags_roundtrip_and_rejection() {
        let s = store();
        let (_, asset) = seed(&s);
        let id = s
            .add_tag(&Tag {
                asset_id: asset,
                label: "interview".into(),
                source: TagSource::LocalModel,
                confidence: Some(0.9),
                model_version: Some("local-v1".into()),
                rejected: false,
            })
            .unwrap();
        let tags = s.tags_for_asset(asset).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, id);
        assert_eq!(tags[0].1.source, TagSource::LocalModel);
        s.set_tag_rejected(id, true).unwrap();
        assert!(s.tags_for_asset(asset).unwrap()[0].1.rejected);
        s.delete_tag(id).unwrap();
        assert!(s.tags_for_asset(asset).unwrap().is_empty());
    }

    #[test]
    fn duplicate_group_roundtrip_and_review() {
        let s = store();
        let (archive, a) = seed(&s);
        let b = Uuid::new_v4();
        s.upsert_asset(&asset(b, archive, "copy.mkv", true))
            .unwrap();
        let gid = Uuid::new_v4();
        s.upsert_duplicate_group(&DuplicateGroup {
            id: gid,
            asset_ids: vec![a, b],
            match_kind: MatchKind::ExactHash,
            reviewed: false,
            keeper: None,
        })
        .unwrap();
        let groups = s.duplicate_groups_for_asset(b).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].asset_ids.len(), 2);
        s.review_group(gid, Some(a)).unwrap();
        let group = s.duplicate_groups_for_asset(b).unwrap().pop().unwrap();
        assert!(group.reviewed);
        assert_eq!(group.keeper, Some(a));
        s.delete_duplicate_group(gid).unwrap();
        assert!(s.duplicate_groups_for_asset(b).unwrap().is_empty());
    }

    #[test]
    fn duplicate_groups_listable_per_archive_and_globally() {
        let s = store();
        let (archive_a, a1) = seed(&s);
        let a2 = Uuid::new_v4();
        s.upsert_asset(&asset(a2, archive_a, "copy-a.mkv", true))
            .unwrap();
        let g_a = DuplicateGroup {
            id: Uuid::new_v4(),
            asset_ids: vec![a1, a2],
            match_kind: MatchKind::ExactHash,
            reviewed: false,
            keeper: None,
        };
        s.upsert_duplicate_group(&g_a).unwrap();

        let archive_b = Uuid::new_v4();
        s.upsert_archive(&Archive {
            id: archive_b,
            name: "second".into(),
            roots: vec![std::path::PathBuf::from("C:\\archive2")],
            watch_enabled: false,
            ai_settings: AiSettings::default(),
        })
        .unwrap();
        let b1 = Uuid::new_v4();
        s.upsert_asset(&asset(b1, archive_b, "clip-b.mkv", true))
            .unwrap();
        let b2 = Uuid::new_v4();
        s.upsert_asset(&asset(b2, archive_b, "copy-b.mkv", true))
            .unwrap();
        let g_b = DuplicateGroup {
            id: Uuid::new_v4(),
            asset_ids: vec![b1, b2],
            match_kind: MatchKind::ExactHash,
            reviewed: false,
            keeper: None,
        };
        s.upsert_duplicate_group(&g_b).unwrap();

        let for_a = s.duplicate_groups_for_archive(archive_a).unwrap();
        assert_eq!(for_a.len(), 1);
        assert_eq!(for_a[0].id, g_a.id);
        assert!(s.duplicate_groups_for_archive(archive_b).unwrap().len() == 1);
        let all = s.list_duplicate_groups().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn scenes_roundtrip() {
        let s = store();
        let (_, aid) = seed(&s);
        let scenes = vec![
            Scene {
                asset_id: aid,
                index: 0,
                start_secs: 0.0,
                end_secs: Some(5.0),
            },
            Scene {
                asset_id: aid,
                index: 1,
                start_secs: 5.0,
                end_secs: Some(10.0),
            },
        ];
        s.replace_scenes(aid, &scenes).unwrap();
        let out = s.scenes_for_asset(aid).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].start_secs, 5.0);
    }

    #[test]
    fn health_snapshot_roundtrip() {
        let s = store();
        let assets = vec![asset(Uuid::new_v4(), Uuid::new_v4(), "gone.mkv", false)];
        let snap = HealthSnapshot {
            taken_at: Utc::now(),
            total_assets: 1,
            missing_files: find_missing(&assets).iter().map(|a| a.id).collect(),
            corrupt_assets: vec![],
            orphaned_derivatives: vec!["C:\\cache\\orphan.thumb".into()],
            duplicate_waste_bytes: 4096,
        };
        s.insert_health_snapshot(None, &snap).unwrap();
        let snaps = s.recent_health_snapshots(None, 10).unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0].missing_files.len(), 1);
        assert_eq!(snaps[0].orphaned_derivatives.len(), 1);
        assert_eq!(snaps[0].duplicate_waste_bytes, 4096);
    }

    #[test]
    fn job_roundtrip_and_state_update() {
        let s = store();
        let (aid, _) = seed(&s);
        let jid = Uuid::new_v4();
        let job = Job::new(Some(aid), JobKind::Index);
        let job = Job { id: jid, ..job };
        s.put_job(&job).unwrap();
        let out = s.get_job(jid).unwrap().unwrap();
        assert_eq!(out.kind, JobKind::Index);
        assert_eq!(out.status, JobStatus::Queued);
        assert_eq!(s.list_jobs().unwrap().len(), 1);
        let updated = Job {
            status: JobStatus::Active,
            progress: 0.5,
            updated_at: Utc::now(),
            ..out
        };
        s.put_job(&updated).unwrap();
        let out = s.get_job(jid).unwrap().unwrap();
        assert_eq!(out.status, JobStatus::Active);
        assert_eq!(out.progress, 0.5);
    }
}
