//! TPT Media Asset Intelligence desktop shell (spec §13).
//!
//! A native Tauri wrapper around the engine: the webview renders and holds no
//! business logic (spec §3.6). Every screen — archive browser, search, asset
//! inspector, duplicate review, tagging review, health dashboard, indexing
//! queue — delegates to engine crates. Phase 1 (spec §26 steps 17–19) lands the
//! screens; this shell already proves the window opens and can call the engine.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Returns the engine version for the About panel and support bundles (spec §6.3).
#[tauri::command]
fn engine_version() -> String {
    tpt_app_media_asset_intelligence_core::ENGINE_VERSION.to_string()
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![engine_version])
        .run(tauri::generate_context!())
        .expect("error while running TPT Media Asset Intelligence");
}
