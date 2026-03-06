//! Persistent application configuration: `~/.config/fang/config.toml`
//!
//! Compatible with the `[ai]` section that PR #18 (AI integration) introduces.
//! New sections (e.g. `[layout]`) can be added here without breaking the AI section.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Layout settings ──────────────────────────────────────────────────────────

/// Panel size configuration.
///
/// Widths are expressed as percentages (1–90) so the layout remains responsive
/// regardless of terminal width.  `sidebar_width` is in columns (a fixed length
/// because the tree panel rarely benefits from scaling proportionally).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Sidebar tree panel width in columns (default: 22).
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,

    /// File-list panel width as a percentage of the usable area after the
    /// sidebar has been subtracted (default: 25, meaning preview gets ~75 %).
    #[serde(default = "default_file_list_pct")]
    pub file_list_pct: u16,

    /// File-list panel percentage when the sidebar is hidden (default: 33).
    #[serde(default = "default_file_list_nosidebar_pct")]
    pub file_list_nosidebar_pct: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            sidebar_width: default_sidebar_width(),
            file_list_pct: default_file_list_pct(),
            file_list_nosidebar_pct: default_file_list_nosidebar_pct(),
        }
    }
}

fn default_sidebar_width() -> u16 {
    22
}
fn default_file_list_pct() -> u16 {
    25
}
fn default_file_list_nosidebar_pct() -> u16 {
    33
}

// ── Top-level config file ─────────────────────────────────────────────────────

/// Full config file — all sections are optional so the file is forwards-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Panel size settings.
    #[serde(default)]
    pub layout: LayoutConfig,
    // `[ai]` section lives in commands/ai.rs (introduced by PR #18) and will
    // be merged here once that PR lands.
}

// ── Persistence ───────────────────────────────────────────────────────────────

/// Returns `~/.config/fang/config.toml`, or `None` on unsupported platforms.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("fang").join("config.toml"))
}

/// Load config from disk, returning defaults on any error.
pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}

/// Persist `config` to `~/.config/fang/config.toml`.
/// Creates parent directories if they don't exist.
pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("cannot determine config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

// ── Editable settings list ────────────────────────────────────────────────────

/// A single editable setting shown in the settings panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntry {
    pub key: &'static str,
    pub description: &'static str,
    pub value: SettingValue,
    pub min: u16,
    pub max: u16,
}

/// The value of a single setting (all are integers for now).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingValue(pub u16);

impl SettingValue {
    pub fn increment(&mut self, max: u16) {
        if self.0 < max {
            self.0 += 1;
        }
    }
    pub fn decrement(&mut self, min: u16) {
        if self.0 > min {
            self.0 -= 1;
        }
    }
}

/// Build the list of settings from a `Config` snapshot.
pub fn entries_from_config(cfg: &Config) -> Vec<SettingEntry> {
    vec![
        SettingEntry {
            key: "layout.sidebar_width",
            description: "Sidebar panel width (columns)",
            value: SettingValue(cfg.layout.sidebar_width),
            min: 10,
            max: 60,
        },
        SettingEntry {
            key: "layout.file_list_pct",
            description: "File list width (%, sidebar visible)",
            value: SettingValue(cfg.layout.file_list_pct),
            min: 10,
            max: 60,
        },
        SettingEntry {
            key: "layout.file_list_nosidebar_pct",
            description: "File list width (%, no sidebar)",
            value: SettingValue(cfg.layout.file_list_nosidebar_pct),
            min: 10,
            max: 70,
        },
    ]
}

/// Apply edited `entries` back into a `Config`.
pub fn apply_entries(cfg: &mut Config, entries: &[SettingEntry]) {
    for e in entries {
        match e.key {
            "layout.sidebar_width" => cfg.layout.sidebar_width = e.value.0,
            "layout.file_list_pct" => cfg.layout.file_list_pct = e.value.0,
            "layout.file_list_nosidebar_pct" => cfg.layout.file_list_nosidebar_pct = e.value.0,
            _ => {}
        }
    }
}
