//! Watch-folder bridge (`--features tpt`): `MediaWatcher` over archive
//! roots, translating debounced `FileEvent`s into rescan hints (spec
//! §7 watch folders, §18 resumability via fingerprint log).

use std::path::{Path, PathBuf};

/// A rescan hint produced from a watcher event.
#[derive(Debug, Clone)]
pub struct RescanHint {
    pub path: PathBuf,
    pub deleted: bool,
}

/// Attach a watcher to `roots`. Returns the watcher; call `poll_hints`
/// to drain debounced events without blocking.
pub fn watch_roots(roots: &[PathBuf]) -> Result<tpt_av_asset_watcher::MediaWatcher, String> {
    let mut watcher = tpt_av_asset_watcher::MediaWatcher::new().map_err(|e| e.to_string())?;
    for root in roots {
        watcher.watch(root).map_err(|e| e.to_string())?;
    }
    Ok(watcher)
}

/// Drain all currently-available debounced events into rescan hints.
pub fn poll_hints(watcher: &mut tpt_av_asset_watcher::MediaWatcher) -> Vec<RescanHint> {
    let mut hints = Vec::new();
    while let Ok(Some(event)) = watcher.try_recv() {
        let deleted = matches!(
            event.event_type,
            tpt_av_asset_watcher::FileEventType::Deleted
                | tpt_av_asset_watcher::FileEventType::Renamed { .. }
        );
        hints.push(RescanHint {
            path: event.path.clone(),
            deleted,
        });
    }
    hints
}

/// Validate a watch root with the same strict rules as the scanner.
pub fn validate_watch_root(root: &Path) -> Result<PathBuf, String> {
    crate::scan::validate_root(root)
}
