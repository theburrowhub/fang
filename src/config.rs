//! Persistent application configuration: `~/.config/fang/config.toml`
//!
//! Compatible with the `[ai]` section that PR #18 (AI integration) introduces.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Layout settings ──────────────────────────────────────────────────────────

/// Panel size configuration — all values are **percentages of terminal width**.
///
/// Invariant: `sidebar_pct + file_list_pct + preview_pct == 100`
///
/// `preview_pct` is **derived** (`100 - sidebar_pct - file_list_pct`) and is
/// not stored on disk; it is computed on the fly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Sidebar tree panel width (%, default 15).
    #[serde(default = "default_sidebar_pct")]
    pub sidebar_pct: u16,

    /// File-list panel width (%, default 20).
    /// Preview automatically gets `100 - sidebar_pct - file_list_pct`.
    #[serde(default = "default_file_list_pct")]
    pub file_list_pct: u16,
}

impl LayoutConfig {
    /// Derived preview percentage — always `100 - sidebar - file_list`.
    pub fn preview_pct(&self) -> u16 {
        100u16
            .saturating_sub(self.sidebar_pct)
            .saturating_sub(self.file_list_pct)
    }

    /// Clamp values so `sidebar + file_list <= 95` (leaving ≥ 5 % for preview).
    pub fn clamp(&mut self) {
        self.sidebar_pct = self.sidebar_pct.clamp(5, 50);
        self.file_list_pct = self.file_list_pct.clamp(5, 50);
        // Ensure total ≤ 95
        if self.sidebar_pct + self.file_list_pct > 95 {
            self.file_list_pct = 95 - self.sidebar_pct;
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            sidebar_pct: default_sidebar_pct(),
            file_list_pct: default_file_list_pct(),
        }
    }
}

fn default_sidebar_pct() -> u16 {
    15
}
fn default_file_list_pct() -> u16 {
    20
}

// ── Top-level config file ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub layout: LayoutConfig,
    // `[ai]` section — introduced by PR #18, merged here once that PR lands.
}

// ── Persistence ───────────────────────────────────────────────────────────────

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("fang").join("config.toml"))
}

pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Config::default(),
    };
    let mut cfg: Config = toml::from_str(&content).unwrap_or_default();
    cfg.layout.clamp();
    cfg
}

pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("cannot determine config directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

// ── Editable settings list ────────────────────────────────────────────────────

/// Whether an entry is editable or derived (read-only, shown in gray).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Editable { min: u16, max: u16 },
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntry {
    pub key: &'static str,
    pub description: &'static str,
    pub value: u16,
    pub kind: EntryKind,
}

impl SettingEntry {
    pub fn increment(&mut self) {
        if let EntryKind::Editable { min: _, max } = self.kind {
            if self.value < max {
                self.value += 1;
            }
        }
    }
    pub fn decrement(&mut self) {
        if let EntryKind::Editable { min, max: _ } = self.kind {
            if self.value > min {
                self.value -= 1;
            }
        }
    }
    pub fn is_editable(&self) -> bool {
        matches!(self.kind, EntryKind::Editable { .. })
    }
}

/// Build the editable entries list from a `Config` snapshot.
/// The preview entry is derived and kept in sync automatically.
pub fn entries_from_config(cfg: &Config) -> Vec<SettingEntry> {
    let preview = cfg.layout.preview_pct();
    vec![
        SettingEntry {
            key: "layout.sidebar_pct",
            description: "Sidebar",
            value: cfg.layout.sidebar_pct,
            kind: EntryKind::Editable { min: 5, max: 50 },
        },
        SettingEntry {
            key: "layout.file_list_pct",
            description: "File list",
            value: cfg.layout.file_list_pct,
            kind: EntryKind::Editable { min: 5, max: 50 },
        },
        SettingEntry {
            key: "layout.preview_pct",
            description: "Preview",
            value: preview,
            kind: EntryKind::Derived,
        },
    ]
}

/// Sync edited entries back into `cfg` and re-derive the preview value.
pub fn apply_entries(cfg: &mut Config, entries: &[SettingEntry]) {
    for e in entries {
        match e.key {
            "layout.sidebar_pct" => cfg.layout.sidebar_pct = e.value,
            "layout.file_list_pct" => cfg.layout.file_list_pct = e.value,
            _ => {}
        }
    }
    cfg.layout.clamp();
}

/// After editing sidebar or file_list, re-compute the derived preview entry
/// so the modal shows the up-to-date value without closing and reopening.
pub fn refresh_derived(entries: &mut [SettingEntry], cfg: &Config) {
    let preview = cfg.layout.preview_pct();
    if let Some(e) = entries.iter_mut().find(|e| e.key == "layout.preview_pct") {
        e.value = preview;
    }
}
