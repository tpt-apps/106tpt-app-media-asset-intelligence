//! localhost-only service surface (spec §15). Disabled by default; binds
//! 127.0.0.1 only, never externally. A minimal dependency-free HTTP/1.1
//! implementation over the engine's queue + store.

//! The [`handlers`] module holds the shared operation logic so the local HTTP
//! service, the desktop command layer and the CLI all run one engine.

use serde::Serialize;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

use tpt_app_media_asset_intelligence_model::{Asset, SearchIndexEntry};
use tpt_app_media_asset_intelligence_persistence::Store;
use tpt_app_media_asset_intelligence_queue::JobQueue;

/// Shared engine operations for the service + desktop command layer (§13/§15).
pub mod handlers {
    use super::*;
    use tpt_app_media_asset_intelligence_search::{explain_match, matches, parse_query};

    #[derive(Debug, Clone, thiserror::Error)]
    pub enum HandlerError {
        #[error("store unavailable")]
        StoreUnavailable,
        #[error("store read failed: {0}")]
        StoreRead(String),
        #[error("invalid query: {0}")]
        BadQuery(String),
        #[error("invalid id: {0}")]
        BadId(String),
        #[error("not found")]
        NotFound,
        #[error("enqueue failed: {0}")]
        Enqueue(String),
    }

    pub type HResult<T> = Result<T, HandlerError>;

    #[derive(Debug, Serialize)]
    pub struct HealthResponse {
        pub ok: bool,
        pub archives: usize,
    }

    #[derive(Debug, Serialize)]
    pub struct ReindexResponse {
        pub job_id: Uuid,
    }

    #[derive(Debug, Serialize, serde::Deserialize)]
    pub struct MatchHit {
        pub asset_id: Uuid,
        pub path: String,
        pub reasons: Vec<String>,
    }

    #[derive(Debug, Serialize)]
    pub struct JobView {
        pub id: Uuid,
        pub status: String,
        pub progress: f64,
        pub note: Option<String>,
        pub error: Option<String>,
    }

    impl JobView {
        fn from_job(job: &tpt_app_media_asset_intelligence_persistence::job::Job) -> Self {
            Self {
                id: job.id,
                status: format!("{:?}", job.status),
                progress: job.progress,
                note: job.note.clone(),
                error: job.error.clone(),
            }
        }
    }

    pub fn lock(store: &Mutex<Store>) -> HResult<MutexGuard<'_, Store>> {
        store.lock().map_err(|_| HandlerError::StoreUnavailable)
    }

    fn parse_id(raw: &str) -> HResult<Uuid> {
        Uuid::parse_str(raw).map_err(|e| HandlerError::BadId(e.to_string()))
    }

    fn entry_for_asset(store: &Store, asset: &Asset) -> SearchIndexEntry {
        let filename = asset
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tags = store
            .tags_for_asset(asset.id)
            .map(|tags| {
                tags.into_iter()
                    .filter(|(_, t)| !t.rejected)
                    .map(|(_, t)| t.label)
                    .collect()
            })
            .unwrap_or_default();
        SearchIndexEntry {
            asset_id: asset.id,
            filename,
            codec: asset.technical_metadata.codec.clone(),
            tags,
            notes: String::new(),
        }
    }

    pub fn health(queue: &JobQueue) -> HResult<HealthResponse> {
        let guard = lock(queue.store())?;
        let archives = guard
            .list_archives()
            .map(|a| a.len())
            .map_err(|e| HandlerError::StoreRead(e.to_string()))?;
        Ok(HealthResponse { ok: true, archives })
    }

    pub fn search(queue: &JobQueue, archive_id: &str, query: &str) -> HResult<Vec<MatchHit>> {
        let parsed_query = parse_query(query).map_err(|e| HandlerError::BadQuery(e.to_string()))?;
        let id = parse_id(archive_id)?;
        let guard = lock(queue.store())?;
        let assets = guard
            .assets_for_archive(id)
            .map_err(|e| HandlerError::StoreRead(e.to_string()))?;
        let mut hits = Vec::new();
        for a in &assets {
            let entry = entry_for_asset(&guard, a);
            if matches(&entry, &parsed_query) {
                hits.push(MatchHit {
                    asset_id: a.id,
                    path: a.path.display().to_string(),
                    reasons: explain_match(&entry, &parsed_query),
                });
            }
        }
        Ok(hits)
    }

    pub fn asset(queue: &JobQueue, asset_id: &str) -> HResult<Asset> {
        let id = parse_id(asset_id)?;
        let guard = lock(queue.store())?;
        guard
            .get_asset(id)
            .ok()
            .flatten()
            .ok_or(HandlerError::NotFound)
    }

    pub fn reindex(queue: &JobQueue, archive_id: &str) -> HResult<ReindexResponse> {
        let id = parse_id(archive_id)?;
        let guard = lock(queue.store())?;
        let missing = guard.get_archive(id).ok().flatten().is_none();
        drop(guard);
        if missing {
            return Err(HandlerError::NotFound);
        }
        queue
            .enqueue_index(id)
            .map(|job_id| ReindexResponse { job_id })
            .map_err(|e| HandlerError::Enqueue(e.to_string()))
    }

    pub fn job(queue: &JobQueue, job_id: &str) -> HResult<JobView> {
        let id = parse_id(job_id)?;
        let guard = lock(queue.store())?;
        guard
            .get_job(id)
            .ok()
            .flatten()
            .map(|j| JobView::from_job(&j))
            .ok_or(HandlerError::NotFound)
    }

    /// Health-trend rows over the archive's snapshot history (spec §21),
    /// oldest first; `None` for the archive means global history.
    pub fn health_trend(
        queue: &JobQueue,
        archive_id: Option<&str>,
        limit: i64,
    ) -> HResult<Vec<tpt_app_media_asset_intelligence_health::HealthTrend>> {
        let id = match archive_id {
            Some(raw) => Some(parse_id(raw)?),
            None => None,
        };
        let guard = lock(queue.store())?;
        let snaps = guard
            .recent_health_snapshots(id, limit)
            .map_err(|e| HandlerError::StoreRead(e.to_string()))?;
        Ok(tpt_app_media_asset_intelligence_health::health_trend(
            &snaps,
        ))
    }
}

/// Routes exposed when the operator explicitly enables the API.
pub const ROUTES: &[&str] = &[
    "GET /archives/:id/search",
    "GET /assets/:id",
    "POST /archives/:id/reindex",
    "GET /jobs/:id",
    "GET /health",
];

/// Service is always disabled unless explicitly enabled.
pub fn is_enabled(explicit_flag: bool) -> bool {
    explicit_flag
}

fn to_json<T: Serialize>(value: &T) -> Vec<u8> {
    match serde_json::to_string(value) {
        Ok(s) => json_response(200, &s),
        Err(_) => json_response(500, "{\"error\":\"serialization failed\"}"),
    }
}

fn error_response(err: &handlers::HandlerError) -> Vec<u8> {
    let status = match err {
        handlers::HandlerError::BadId(_) | handlers::HandlerError::BadQuery(_) => 400,
        handlers::HandlerError::NotFound => 404,
        handlers::HandlerError::StoreUnavailable => 503,
        _ => 500,
    };
    json_response(status, &format!("{{\"error\":\"{err}\"}}"))
}

/// A localhost-only HTTP/1.1 listener over a job queue (spec §15).
pub struct ServiceServer {
    queue: Arc<JobQueue>,
    listener: TcpListener,
}

impl ServiceServer {
    /// Bind 127.0.0.1 only; `port: 0` selects an ephemeral port.
    pub fn local(queue: JobQueue, port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        Ok(Self {
            queue: Arc::new(queue),
            listener,
        })
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// Handle connections until `stop` becomes true. Each connection is
    /// handled synchronously on this thread (fine for a local control API).
    pub fn serve_until(&self, stop: &std::sync::atomic::AtomicBool) -> std::io::Result<()> {
        self.listener.set_nonblocking(true)?;
        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            match self.listener.accept() {
                Ok((stream, _)) => self.handle(stream),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn handle(&self, mut stream: TcpStream) {
        let response = self.route(&stream);
        let _ = stream.write_all(&response);
        let _ = stream.flush();
    }

    fn read_request_head(stream: &TcpStream) -> Option<(String, String, String, String)> {
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).ok()?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next()?.to_string();
        let path_full = parts.next()?.to_string();
        let version = parts.next()?.to_string();
        let (path, query) = match path_full.split_once('?') {
            Some((p, q)) => (p.to_string(), q.to_string()),
            None => (path_full.clone(), String::new()),
        };
        // Drain the headers (we take no body).
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).ok()? == 0 || line.trim().is_empty() {
                break;
            }
        }
        Some((method, path, query, version))
    }

    fn query_param(query: &str, key: &str) -> Option<String> {
        query.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            (k == key).then(|| v.to_string())
        })
    }

    fn route(&self, stream: &TcpStream) -> Vec<u8> {
        let Some((method, path, query, _version)) = Self::read_request_head(stream) else {
            return json_response(400, "{\"error\":\"bad request\"}");
        };
        let segments: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let result = match (method.as_str(), segments.as_slice()) {
            ("GET", ["health"]) => handlers::health(&self.queue).map(|h| to_json(&h)),
            ("GET", ["archives", id, "search"]) => match Self::query_param(&query, "query") {
                Some(q) => handlers::search(&self.queue, id, &q).map(|hits| to_json(&hits)),
                None => Ok(json_response(400, "{\"error\":\"missing ?query=\"}")),
            },
            ("GET", ["assets", id]) => handlers::asset(&self.queue, id).map(|a| to_json(&a)),
            ("POST", ["archives", id, "reindex"]) => {
                handlers::reindex(&self.queue, id).map(|r| to_json(&r))
            }
            ("GET", ["jobs", id]) => handlers::job(&self.queue, id).map(|j| to_json(&j)),
            _ => Ok(json_response(404, "{\"error\":\"not found\"}")),
        };
        match result {
            Ok(bytes) => bytes,
            Err(e) => error_response(&e),
        }
    }
}

fn json_response(status: u16, body: &str) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Status",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, Read};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    fn seed_queue() -> (JobQueue, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("mai-svc-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let root = dir.join("archive");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("interview.mkv"), [0u8; 32]).unwrap();
        std::fs::write(root.join("broll.mkv"), [1u8; 32]).unwrap();
        let db = dir.join("svc.sqlite");
        let queue = JobQueue::open(&db).unwrap();
        (queue, dir)
    }

    fn http_get(port: u16, raw: &str) -> (Vec<String>, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        let mut headers = Vec::new();
        let mut body = String::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let trimmed = line.trim_end().to_string();
            if trimmed.is_empty() {
                break;
            }
            headers.push(trimmed);
        }
        reader.read_to_string(&mut body).unwrap();
        (headers, body)
    }

    fn spawn_server(server: ServiceServer, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let _ = server.serve_until(&stop);
        })
    }

    #[test]
    fn health_reports_archive_count() {
        let (queue, dir) = seed_queue();
        let server = ServiceServer::local(queue.clone_ref(), 0).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = stop.clone();
        let port = server.port();
        let thread = spawn_server(server, stop_t);

        let (headers, body) = http_get(port, "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(headers[0].starts_with("HTTP/1.1 200"));
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["archives"], 0);

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        thread.join().unwrap();
        drop(queue);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn full_request_lifecycle_reindex_then_job_then_search() {
        let (queue, dir) = seed_queue();
        let archive = {
            let store = queue.store().lock().unwrap();
            let a = tpt_app_media_asset_intelligence_model::Archive {
                id: Uuid::new_v4(),
                name: "news".into(),
                roots: vec![dir.join("archive")],
                watch_enabled: false,
                ai_settings: Default::default(),
            };
            store.upsert_archive(&a).unwrap();
            a
        };
        let server = ServiceServer::local(queue.clone_ref(), 0).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = stop.clone();
        let port = server.port();
        let thread = spawn_server(server, stop_t);

        // Reindex via the API, run the queue, then read the job and search.
        let reindex = format!(
            "POST /archives/{}/reindex HTTP/1.1\r\nHost: localhost\r\n\r\n",
            archive.id
        );
        let (headers, body) = http_get(port, &reindex);
        assert!(headers[0].starts_with("HTTP/1.1 200"), "{headers:?} {body}");
        let job_id: Uuid = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["job_id"].clone(),
        )
        .unwrap();
        queue.run_until_idle().unwrap();

        let job_req = format!("GET /jobs/{job_id} HTTP/1.1\r\nHost: localhost\r\n\r\n");
        let (_headers, body) = http_get(port, &job_req);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["status"], "Done");
        assert_eq!(v["progress"], 1.0);

        let search_req = format!(
            "GET /archives/{}/search?query=interview HTTP/1.1\r\nHost: localhost\r\n\r\n",
            archive.id
        );
        let (_headers, body) = http_get(port, &search_req);
        let hits: Vec<handlers::MatchHit> = serde_json::from_str(&body).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].path.contains("interview.mkv"));
        assert!(!hits[0].reasons.is_empty());

        let asset_req = format!(
            "GET /assets/{} HTTP/1.1\r\nHost: localhost\r\n\r\n",
            hits[0].asset_id
        );
        let (_headers, body) = http_get(port, &asset_req);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(v["path"].as_str().unwrap().contains("interview.mkv"));

        let missing = http_get(
            port,
            "GET /assets/00000000-0000-0000-0000-000000000000 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(missing.0[0].starts_with("HTTP/1.1 404"));

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        thread.join().unwrap();
        drop(queue);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reroutes_and_gates() {
        assert_eq!(ROUTES.len(), 5);
        assert!(!is_enabled(false));
        assert!(is_enabled(true));
    }
}
