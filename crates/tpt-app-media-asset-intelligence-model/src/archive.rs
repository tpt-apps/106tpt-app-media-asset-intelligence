//! Archives and their AI-enablement settings (spec §6.1, §10.1).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tpt_app_media_asset_intelligence_core::ArchiveId;

/// Per-archive AI feature switches (spec §6.1).
///
/// Cloud-related fields default to disabled/None. There is intentionally no
/// combined "enable AI" constructor: each cloud feature is enabled individually
/// through its dedicated method, which the application layer only calls after
/// the per-feature §10.1 disclosure/confirmation flow.
///
/// The derived `Default` IS the §6.1 default: every switch `false`, provider
/// `None`. A test asserts this so a future field cannot silently change it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSettings {
    /// Bundled local-model auto-tagging; fully offline (spec §11).
    pub local_tagging_enabled: bool,
    /// Cloud-assisted tagging; strictly opt-in (spec §11, §10.1).
    pub cloud_tagging_enabled: bool,
    /// The cloud provider used by any enabled cloud feature; `None` until chosen.
    pub cloud_provider: Option<CloudAiProvider>,
    /// Cloud-assisted semantic search; strictly opt-in (spec §10 Layer 3, §10.1).
    pub cloud_semantic_search_enabled: bool,
}

impl AiSettings {
    /// Returns `true` if any cloud AI feature is enabled for this archive.
    ///
    /// This is the single check every engine surface (GUI, CLI, API) uses to
    /// prove the offline guarantee (spec §17, §19.4): when it is `false`, no
    /// code path may perform a network call.
    pub fn any_cloud_feature_enabled(&self) -> bool {
        self.cloud_tagging_enabled || self.cloud_semantic_search_enabled
    }

    /// Enables cloud tagging for the given provider after the §10.1 disclosure
    /// flow has been confirmed by the user.
    ///
    /// The application layer must only call this in direct response to the
    /// user's explicit confirmation; the model deliberately cannot distinguish
    /// "confirmed" from "called", so tests assert the call sites instead
    /// (spec §19.4).
    pub fn enable_cloud_tagging(&mut self, provider: CloudAiProvider) {
        self.cloud_tagging_enabled = true;
        self.cloud_provider = Some(provider);
    }

    /// Enables cloud semantic search for the given provider after the §10.1
    /// disclosure flow has been confirmed by the user.
    pub fn enable_cloud_semantic_search(&mut self, provider: CloudAiProvider) {
        self.cloud_semantic_search_enabled = true;
        self.cloud_provider = Some(provider);
    }

    /// Disables both cloud features.
    ///
    /// Disabling cloud AI does not delete or invalidate previously generated
    /// local-model results (spec §10.1, §19.4); this method only flips switches.
    pub fn disable_cloud(&mut self) {
        self.cloud_tagging_enabled = false;
        self.cloud_semantic_search_enabled = false;
        self.cloud_provider = None;
    }
}

/// A supported cloud AI provider for the opt-in Phase 2 features (spec §10, §21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloudAiProvider {
    /// The placeholder provider configured per deployment; no real integration exists yet.
    Unspecified,
}

impl CloudAiProvider {
    /// The kebab-case token used in settings files and CLI output (spec §14).
    pub const fn as_str(self) -> &'static str {
        match self {
            CloudAiProvider::Unspecified => "unspecified",
        }
    }
}

/// An archive: a named set of filesystem roots indexed together (spec §6.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Archive {
    /// Stable identifier within the persistent index.
    pub id: ArchiveId,
    /// Human-facing name shown in the UI and accepted by `--archive` in the CLI (spec §14).
    pub name: String,
    /// Archive roots: local disks, NAS shares or project folders (spec §7).
    pub roots: Vec<PathBuf>,
    /// Whether watch-folder monitoring picks up new files automatically (spec §5.1, §7).
    pub watch_enabled: bool,
    /// Per-archive AI feature switches (spec §6.1).
    pub ai_settings: AiSettings,
}

impl Archive {
    /// Creates a new archive with watch folders disabled and all AI features off.
    ///
    /// Cloud fields default to disabled/None (spec §6.1); callers opt in
    /// per-feature via [`AiSettings`] only after the §10.1 disclosure flow.
    pub fn new(id: u64, name: impl Into<String>, roots: Vec<PathBuf>) -> Self {
        Self {
            id: ArchiveId::new(id),
            name: name.into(),
            roots,
            watch_enabled: false,
            ai_settings: AiSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive() -> Archive {
        Archive::new(1, "Main", vec![PathBuf::from("/mnt/media-nas/projects")])
    }

    #[test]
    fn new_archives_have_all_ai_features_disabled() {
        let a = archive();
        assert!(!a.ai_settings.local_tagging_enabled);
        assert!(!a.ai_settings.cloud_tagging_enabled);
        assert!(!a.ai_settings.cloud_semantic_search_enabled);
        assert!(a.ai_settings.cloud_provider.is_none());
        assert!(!a.watch_enabled, "watch folders are opt-in");
    }

    #[test]
    fn default_ai_settings_match_spec_defaults() {
        let s = AiSettings::default();
        assert!(!s.local_tagging_enabled);
        assert!(!s.cloud_tagging_enabled);
        assert!(s.cloud_provider.is_none());
        assert!(!s.cloud_semantic_search_enabled);
    }

    #[test]
    fn any_cloud_feature_reflects_each_switch() {
        let mut s = AiSettings::default();
        assert!(!s.any_cloud_feature_enabled());

        s.cloud_tagging_enabled = true;
        assert!(s.any_cloud_feature_enabled());

        s.cloud_tagging_enabled = false;
        s.cloud_semantic_search_enabled = true;
        assert!(s.any_cloud_feature_enabled());
    }

    #[test]
    fn enable_and_disable_round_trip() {
        let mut a = archive();
        a.ai_settings
            .enable_cloud_tagging(CloudAiProvider::Unspecified);
        assert!(a.ai_settings.cloud_tagging_enabled);
        assert_eq!(
            a.ai_settings.cloud_provider,
            Some(CloudAiProvider::Unspecified)
        );

        a.ai_settings.disable_cloud();
        assert!(!a.ai_settings.any_cloud_feature_enabled());
        assert!(a.ai_settings.cloud_provider.is_none());
    }

    #[test]
    fn serializes_with_kebab_case_provider() {
        let json = serde_json::to_string(&AiSettings::default()).unwrap();
        assert!(json.contains("\"cloud_provider\":null"));
        assert!(serde_json::from_str::<AiSettings>(&json).unwrap() == AiSettings::default());
    }
}
