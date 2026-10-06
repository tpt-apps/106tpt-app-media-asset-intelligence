//! Desktop command surface for the Tauri shell (spec §13).
//! Real window/menu wiring lands with the Tauri frontend; this crate
//! owns the command contracts so CLI/service/GUI share one engine.

use tpt_app_media_asset_intelligence_model::AssetId;

#[derive(Debug, Clone)]
pub enum UiCommand {
    OpenArchive { archive_id: String },
    InspectAsset { asset_id: AssetId },
    RunSearch { query: String },
    ReviewDuplicates { group_id: String },
    OpenHealth,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn commands_carry_ids() {
        let cmd = UiCommand::InspectAsset {
            asset_id: Uuid::new_v4(),
        };
        assert!(matches!(cmd, UiCommand::InspectAsset { .. }));
    }
}
